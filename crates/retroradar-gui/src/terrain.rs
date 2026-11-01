//! Bounded, progressive regional imagery. All network, disk and decode work is on a worker.
use crate::globe::{GlobePass, GlobeScene};
use glam::{Vec2, Vec4};
use retroradar_sources::earth::{Client, MAX_LEVEL, TILE_SIZE, Tile};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const SIDE: u32 = 8;
pub const PATCH_SIZE: u32 = SIDE * TILE_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Key {
    pub level: u32,
    pub x: u32,
    pub y: u32,
}
impl Key {
    pub fn bounds(self) -> [f32; 4] {
        let rows = (1 << self.level) as f32;
        [
            self.x as f32 / (2.0 * rows),
            self.y as f32 / rows,
            SIDE as f32 / (2.0 * rows),
            SIDE as f32 / rows,
        ]
    }
    fn tile(self, x: u32, y: u32) -> Tile {
        Tile {
            level: self.level,
            x: (self.x + x) % (2 << self.level),
            y: self.y + y,
        }
    }
}

fn bounds_key(min: Vec2, max: Vec2) -> Option<Key> {
    for level in (5..=MAX_LEVEL).rev() {
        let rows = (1 << level) as f32;
        let size = Vec2::new(rows * 2.0, rows);
        let a = (min * size).floor() - Vec2::ONE;
        let b = (max * size).ceil() + Vec2::ONE;
        if (b - a).max_element() <= SIDE as f32 {
            return Some(Key {
                level,
                x: (a.x as i64).rem_euclid((2 << level) as i64) as u32,
                y: (a.y as i64).clamp(0, (1 << level) - i64::from(SIDE)) as u32,
            });
        }
    }
    None
}

/// Ray picking uses the same sphere, camera and Earth-fixed rotation as the shader.
pub(crate) fn request(scene: &GlobeScene) -> Option<Key> {
    if scene.cam_pos.length() > 1.25 {
        return None;
    }
    let inv = scene.view_proj.inverse();
    let mut points = Vec::new();
    for y in -2..=2 {
        for x in -2..=2 {
            let n = inv * Vec4::new(x as f32 * 0.5, y as f32 * 0.5, 0.0, 1.0);
            let dir = (n.truncate() / n.w - scene.cam_pos).normalize();
            let b = scene.cam_pos.dot(dir);
            let disc = b * b - scene.cam_pos.length_squared() + 1.0;
            if disc < 0.0 || -b - disc.sqrt() <= 0.0 {
                continue;
            }
            let p = scene.cam_pos + dir * (-b - disc.sqrt());
            let e = (scene.ecef * p).normalize();
            points.push(Vec2::new(
                e.y.atan2(e.x) / std::f32::consts::TAU + 0.5,
                0.5 - e.z.clamp(-1.0, 1.0).asin() / std::f32::consts::PI,
            ));
        }
    }
    if points.len() < 9 {
        return None;
    }
    let origin = points[points.len() / 2].x;
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for mut p in points {
        p.x = origin + (p.x - origin + 0.5).rem_euclid(1.0) - 0.5;
        min = min.min(p);
        max = max.max(p);
    }
    bounds_key(min, max)
}

struct Update {
    key: Key,
    image: image::RgbaImage,
    loaded: u32,
    error: Option<String>,
}
#[derive(Default)]
struct State {
    desired: Option<Key>,
    generation: u64,
    latest: Option<Update>,
    closed: bool,
}
struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}
pub(crate) struct Stream {
    shared: Arc<Shared>,
    requested: Option<Key>,
    last_request: Instant,
    pub status: Option<String>,
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().closed = true;
        self.shared.wake.notify_one();
    }
}
impl Stream {
    pub fn new(cache: PathBuf, offline: bool) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
        });
        let worker = shared.clone();
        std::thread::spawn(move || {
            let mut client = Client::new(&cache, offline);
            let mut seen = 0;
            loop {
                let key = {
                    let mut state = worker.state.lock().unwrap();
                    while !state.closed && state.generation == seen {
                        state = worker.wake.wait(state).unwrap();
                    }
                    if state.closed {
                        return;
                    }
                    seen = state.generation;
                    state.desired
                };
                let Some(key) = key else {
                    continue;
                };
                let mut image = image::RgbaImage::new(PATCH_SIZE, PATCH_SIZE);
                let mut positions: Vec<_> = (0..SIDE)
                    .flat_map(|y| (0..SIDE).map(move |x| (x, y)))
                    .collect();
                // Centre first; missing edge tiles do not delay the area under the cursor.
                positions.sort_by_key(|&(x, y)| (x as i32 - 4).pow(2) + (y as i32 - 4).pow(2));
                let mut loaded = 0;
                let mut error = None;
                for (i, (x, y)) in positions.into_iter().enumerate() {
                    {
                        let s = worker.state.lock().unwrap();
                        if s.closed {
                            return;
                        }
                        if s.generation != seen {
                            break;
                        }
                    }
                    let tile = key.tile(x, y);
                    let previous_loaded = loaded;
                    match client.get(tile).and_then(|bytes| decode(&bytes)) {
                        Ok(pixels) => {
                            image::imageops::replace(
                                &mut image,
                                &pixels,
                                i64::from(x * TILE_SIZE),
                                i64::from(y * TILE_SIZE),
                            );
                            loaded += 1;
                        }
                        Err(e) => {
                            client.invalidate(tile);
                            error = Some(e);
                        }
                    }
                    if (loaded == 1 && loaded != previous_loaded)
                        || (loaded > 0 && i % 4 == 3)
                        || i == (SIDE * SIDE - 1) as usize
                    {
                        let pixels = image.clone();
                        let mut s = worker.state.lock().unwrap();
                        if s.generation == seen {
                            s.latest = Some(Update {
                                key,
                                image: pixels,
                                loaded,
                                error: error.clone(),
                            });
                        }
                    }
                }
            }
        });
        Self {
            shared,
            requested: None,
            last_request: Instant::now(),
            status: None,
        }
    }
    pub fn tick(&mut self, scene: Option<&GlobeScene>, globe: &mut GlobePass, queue: &wgpu::Queue) {
        let wanted = scene.and_then(request);
        if wanted != self.requested
            || (wanted.is_some() && self.last_request.elapsed() > Duration::from_secs(60))
        {
            self.requested = wanted;
            self.last_request = Instant::now();
            let mut state = self.shared.state.lock().unwrap();
            state.desired = wanted;
            state.generation += 1;
            state.latest = None;
            self.shared.wake.notify_one();
            self.status =
                wanted.map(|k| format!("DETAIL LOD {} | LOADING | EOX Sentinel-2 2016", k.level));
        }
        if wanted.is_none() {
            globe.clear_detail();
            self.status = None;
            return;
        }
        let update = self.shared.state.lock().unwrap().latest.take();
        if let Some(update) = update
            && Some(update.key) == wanted
        {
            let metres = 40_075_017.0 / ((2 << update.key.level) * TILE_SIZE) as f64;
            self.status = Some(format!(
                "DETAIL ~{metres:.0} m/px | {}/64 TILES{}",
                update.loaded,
                if update.error.is_some() {
                    " | MISSING TILES: GLOBAL MAP"
                } else {
                    ""
                }
            ));
            globe.upload_detail(queue, &update.image, update.key.bounds());
        }
    }
}
fn decode(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if reader.into_dimensions().map_err(|e| e.to_string())? != (TILE_SIZE, TILE_SIZE) {
        return Err("unexpected detail tile dimensions".into());
    }
    image::load_from_memory(bytes)
        .map(|i| i.to_rgba8())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patch_matrix_wraps_dateline_and_clamps_poles_without_invalid_tiles() {
        for (a, b) in [
            (Vec2::new(0.999, 0.4), Vec2::new(1.001, 0.401)),
            (Vec2::new(-0.001, 0.0), Vec2::new(0.001, 0.001)),
            (Vec2::new(0.5, 0.999), Vec2::new(0.501, 1.0)),
        ] {
            let k = bounds_key(a, b).unwrap();
            for y in 0..SIDE {
                for x in 0..SIDE {
                    assert!(k.tile(x, y).valid());
                }
            }
            assert!(k.bounds()[2] > 0.0);
        }
        let k = bounds_key(Vec2::new(0.5, 0.4), Vec2::new(0.5001, 0.4001)).unwrap();
        assert_eq!(k.level, MAX_LEVEL);
        assert!(bounds_key(Vec2::ZERO, Vec2::ONE).is_none());
    }
    #[test]
    fn cached_real_tile_is_decoded_on_worker_and_missing_tiles_remain_transparent() {
        let root =
            std::env::temp_dir().join(format!("retroradar-terrain-worker-{}", std::process::id()));
        let path = root.join("earth-eox-s2-2016-v1");
        std::fs::create_dir_all(&path).unwrap();
        let bytes = include_bytes!("../fixtures/earth/s2-2016-z13-y2257-x8023.jpg");
        std::fs::write(path.join("13-8023-2257.jpg"), bytes).unwrap();
        let stream = Stream::new(root.clone(), true);
        let key = Key {
            level: 13,
            x: 8020,
            y: 2254,
        };
        {
            let mut s = stream.shared.state.lock().unwrap();
            s.desired = Some(key);
            s.generation = 1;
        }
        stream.shared.wake.notify_one();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let update = stream.shared.state.lock().unwrap().latest.take();
            if let Some(u) = update
                && u.loaded == 1
            {
                let expected = decode(bytes).unwrap();
                for y in 0..TILE_SIZE {
                    for x in 0..TILE_SIZE {
                        assert_eq!(
                            u.image.get_pixel(3 * TILE_SIZE + x, 3 * TILE_SIZE + y),
                            expected.get_pixel(x, y)
                        );
                    }
                }
                assert_eq!(u.image.get_pixel(0, 0).0, [0; 4]);
                break;
            }
            assert!(Instant::now() < deadline, "cached tile never arrived");
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(stream);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn tiles_require_native_256_square_images_and_reject_html() {
        assert!(decode(b"<html>error</html>").is_err());
        assert!(decode(crate::assets::EARTH_JPEG).is_err());
    }
}
