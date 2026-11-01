//! Versioned EOX Sentinel-2 2016 tiles (CC BY 4.0), fetched on a worker.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const TILE_SIZE: u32 = 256;
pub const MAX_LEVEL: u32 = 13; // ~10 m; higher advertised levels enlarge the same source.
pub const ATTRIBUTION: &str = "EOxCloudless https://cloudless.eox.at by EOX IT Services GmbH\nContains modified Copernicus Sentinel data 2016 | CC BY 4.0 https://creativecommons.org/licenses/by/4.0/";
const ROOT: &str = "https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless/default/WGS84";
const MAX_BYTES: u64 = 512 * 1024;
const CACHE_BUDGET: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    pub level: u32,
    pub x: u32,
    pub y: u32,
}
impl Tile {
    pub fn valid(self) -> bool {
        self.level <= MAX_LEVEL && self.x < (2 << self.level) && self.y < (1 << self.level)
    }
    pub fn url(self) -> Result<String, String> {
        if !self.valid() {
            return Err("invalid WGS84 tile coordinates".into());
        }
        Ok(format!("{ROOT}/{}/{}/{}.jpg", self.level, self.y, self.x))
    }
    fn filename(self) -> String {
        format!("{}-{}-{}.jpg", self.level, self.x, self.y)
    }
}
fn image_signature(bytes: &[u8]) -> bool {
    (bytes.len() >= 4 && bytes.starts_with(&[255, 216])) || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
}

pub struct Client {
    root: PathBuf,
    agent: ureq::Agent,
    offline: bool,
    next_request: Instant,
    paused: Option<Instant>,
    writes: u32,
}
impl Client {
    pub fn new(cache: &Path, offline: bool) -> Self {
        Self {
            root: cache.join("earth-eox-s2-2016-v1"),
            agent: ureq::Agent::config_builder()
                .tls_config(crate::tls::http_config())
                .timeout_global(Some(Duration::from_secs(8)))
                .build()
                .into(),
            offline,
            next_request: Instant::now(),
            paused: None,
            writes: 0,
        }
    }
    pub fn get(&mut self, tile: Tile) -> Result<Vec<u8>, String> {
        let url = tile.url()?;
        let path = self.root.join(tile.filename());
        if let Ok(meta) = std::fs::metadata(&path)
            && meta.len() <= MAX_BYTES
            && let Ok(bytes) = std::fs::read(&path)
            && image_signature(&bytes)
        {
            return Ok(bytes);
        }
        if self.offline {
            return Err("offline: detail tile not cached".into());
        }
        if self.paused.is_some_and(|t| t > Instant::now()) {
            return Err("detail provider cooling down".into());
        }
        // One request at a time, at most four per second; never prefetch a world.
        if let Some(delay) = self.next_request.checked_duration_since(Instant::now()) {
            std::thread::sleep(delay);
        }
        self.next_request = Instant::now() + Duration::from_millis(250);
        let result = (|| {
            let mut response = self
                .agent
                .get(&url)
                .header(
                    "User-Agent",
                    "retroradar/0.1.1 (https://github.com/TheWinterLynx/retroradar)",
                )
                .call()
                .map_err(|e| e.to_string())?;
            let bytes = response
                .body_mut()
                .with_config()
                .limit(MAX_BYTES)
                .read_to_vec()
                .map_err(|e| e.to_string())?;
            if !image_signature(&bytes) {
                return Err("detail provider returned an invalid image".into());
            }
            Ok(bytes)
        })();
        let bytes: Vec<u8> = match result {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(source = "EOX Sentinel-2 2016", error = %error, "regional imagery unavailable; keeping the global map");
                self.paused = Some(Instant::now() + Duration::from_secs(60));
                return Err(error);
            }
        };
        std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        // Single worker: an atomic replacement is unnecessary; a crash-truncated
        // file is rejected by the decoder and removed via invalidate().
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        self.writes += 1;
        if self.writes.is_multiple_of(16) {
            self.trim();
        }
        Ok(bytes)
    }
    pub fn invalidate(&self, tile: Tile) {
        let _ = std::fs::remove_file(self.root.join(tile.filename()));
    }
    fn trim(&self) {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return;
        };
        let mut files: Vec<_> = entries
            .flatten()
            .filter_map(|e| {
                let m = e.metadata().ok()?;
                m.is_file().then(|| (m.modified().ok(), m.len(), e.path()))
            })
            .collect();
        files.sort_by_key(|e| e.0);
        let mut total: u64 = files.iter().map(|e| e.1).sum();
        for (_, len, path) in files {
            if total <= CACHE_BUDGET {
                break;
            }
            if std::fs::remove_file(path).is_ok() {
                total = total.saturating_sub(len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matrix_urls_use_row_before_column_and_reject_out_of_range_coordinates() {
        assert_eq!(
            Tile {
                level: 13,
                x: 9048,
                y: 2666
            }
            .url()
            .unwrap(),
            format!("{ROOT}/13/2666/9048.jpg")
        );
        for tile in [
            Tile {
                level: 14,
                x: 0,
                y: 0,
            },
            Tile {
                level: 0,
                x: 2,
                y: 0,
            },
            Tile {
                level: 1,
                x: 0,
                y: 2,
            },
        ] {
            assert!(tile.url().is_err());
        }
    }
    #[test]
    #[ignore = "queries public EOX service; never part of the offline gate"]
    fn live_eox_native_tile_is_available_over_verified_tls() {
        let root = std::env::temp_dir().join(format!("retroradar-eox-live-{}", std::process::id()));
        let mut client = Client::new(&root, false);
        // Explicit trust for a diagnostic runner behind an intercepting proxy.
        // Test-only: normal clients keep Mozilla roots and cannot use this override.
        if let Some(path) = std::env::var_os("RETRORADAR_TEST_CA") {
            let pem = std::fs::read(path).unwrap();
            let certificate = ureq::tls::Certificate::from_pem(&pem).unwrap();
            let tls = ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .unversioned_rustls_crypto_provider(std::sync::Arc::new(crate::tls::provider()))
                .root_certs(ureq::tls::RootCerts::Specific(std::sync::Arc::new(vec![
                    certificate,
                ])))
                .build();
            client.agent = ureq::Agent::config_builder()
                .tls_config(tls)
                .timeout_global(Some(Duration::from_secs(8)))
                .build()
                .into();
        }
        let result = client.get(Tile {
            level: 13,
            x: 8023,
            y: 2257,
        });
        let _ = std::fs::remove_dir_all(root);
        let bytes = result.unwrap();
        assert!(image_signature(&bytes) && bytes.len() > 1000);
        println!("EOX 2016 verified-TLS native tile: {} bytes", bytes.len());
    }
    #[test]
    fn offline_cache_reuses_images_but_rejects_html_and_oversized_entries() {
        let root =
            std::env::temp_dir().join(format!("retroradar-detail-cache-{}", std::process::id()));
        let mut client = Client::new(&root, true);
        std::fs::create_dir_all(&client.root).unwrap();
        let tile = Tile {
            level: 0,
            x: 0,
            y: 0,
        };
        let path = client.root.join(tile.filename());
        std::fs::write(&path, [255, 216, 255, 217]).unwrap();
        assert_eq!(client.get(tile).unwrap(), [255, 216, 255, 217]);
        std::fs::write(&path, b"<html>error</html>").unwrap();
        assert!(client.get(tile).unwrap_err().contains("offline"));
        std::fs::File::create(&path)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert!(client.get(tile).unwrap_err().contains("offline"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn offline_cache_miss_and_invalid_coordinates_do_not_create_directories() {
        let root =
            std::env::temp_dir().join(format!("retroradar-detail-offline-{}", std::process::id()));
        let mut client = Client::new(&root, true);
        assert!(
            client
                .get(Tile {
                    level: 13,
                    x: 1,
                    y: 1
                })
                .unwrap_err()
                .contains("offline")
        );
        assert!(
            client
                .get(Tile {
                    level: 100,
                    x: 0,
                    y: 0
                })
                .is_err()
        );
        assert!(!root.exists());
    }
}
