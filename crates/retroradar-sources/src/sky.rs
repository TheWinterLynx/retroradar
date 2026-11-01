//! "What is it looking at": resolve a target name to a sky position and fetch a survey cutout of the
//! field. The picture is a DSS2 sky-survey image around the target, never an image taken by Hubble or
//! JWST, and the display says so.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::{Duration, Instant};

use retroradar_core::config::SkySources;
use retroradar_core::schedule::{FieldRequest, FieldState};

use crate::tasks::Event;

/// Largest response read for a picture or a program file, bytes.
const MAX_BODY: u64 = 8 * 1024 * 1024;
const ENDPOINT_PAUSE: Duration = Duration::from_secs(60);
const CDS_PRIMARY: &str = "https://alasky.cds.unistra.fr/hips-image-services/hips2fits";
const CDS_MIRROR: &str = "https://alaskybis.cds.unistra.fr/hips-image-services/hips2fits";

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub name: String,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub origin: String,
}

/// Percent-encode a query value.
fn encode(v: &str) -> String {
    v.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// `2MASS-J18022716+6043356` encodes its own position: hhmmss.ss and +ddmmss.s.
pub fn parse_2mass(name: &str) -> Option<Position> {
    let body = name
        .strip_prefix("2MASS-J")
        .or_else(|| name.strip_prefix("2MASS J"))?;
    let split = body.find(['+', '-'])?;
    let (ra, dec) = body.split_at(split);
    let sign = if dec.starts_with('-') { -1.0 } else { 1.0 };
    let dec = &dec[1..];
    if ra.len() != 8
        || dec.len() != 7
        || !ra.bytes().all(|b| b.is_ascii_digit())
        || !dec.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let n = |s: &str| s.parse::<f64>().ok();
    let ra_h = n(&ra[0..2])? + n(&ra[2..4])? / 60.0 + n(&ra[4..8])? / 100.0 / 3600.0;
    let dec_d = n(&dec[0..2])? + n(&dec[2..4])? / 60.0 + n(&dec[4..7])? / 10.0 / 3600.0;
    Some(Position {
        name: name.to_owned(),
        ra_deg: ra_h * 15.0,
        dec_deg: sign * dec_d,
        origin: "2MASS designation".into(),
    })
}

/// The `%J` line of a Sesame reply: `%J 10.68470833 +41.26875000 = 00 42 44.330 +41 16 07.50`.
pub fn parse_sesame(text: &str, name: &str) -> Option<Position> {
    let line = text.lines().find(|l| l.starts_with("%J "))?;
    let mut it = line[3..].split_whitespace();
    let ra: f64 = it.next()?.parse().ok()?;
    let dec: f64 = it.next()?.parse().ok()?;
    ((0.0..360.0).contains(&ra) && (-90.0..=90.0).contains(&dec)).then(|| Position {
        name: name.to_owned(),
        ra_deg: ra,
        dec_deg: dec,
        origin: "Sesame (SIMBAD/NED/VizieR)".into(),
    })
}

/// Fixed targets of a Hubble Phase II program file: lines such as
/// `1   HD-35165   STAR, B3-B5 V-IV, BE   RA=05H21M16.8603S,   J2000 ...` followed by `DEC=-34D20'42.17"`.
pub fn parse_hst_targets(text: &str) -> Vec<Position> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(ra_at) = line.find("RA=") else {
            continue;
        };
        let Some(dec_line) = lines.get(i + 1).filter(|l| l.contains("DEC=")) else {
            continue;
        };
        let mut head = line[..ra_at].split_whitespace();
        let (Some(num), Some(name)) = (head.next(), head.next()) else {
            continue;
        };
        if num.parse::<u32>().is_err() {
            continue;
        }
        let ra = parse_hms(&line[ra_at + 3..]);
        let dec = dec_line
            .find("DEC=")
            .and_then(|d| parse_dms(&dec_line[d + 4..]));
        if let (Some(ra), Some(dec)) = (ra, dec) {
            out.push(Position {
                name: name.trim_start_matches('-').to_owned(),
                ra_deg: ra,
                dec_deg: dec,
                origin: "program Phase II file".into(),
            });
        }
    }
    out
}

/// `05H21M16.8603S` to degrees.
fn parse_hms(s: &str) -> Option<f64> {
    let (h, rest) = s.split_once('H')?;
    let (m, rest) = rest.split_once('M')?;
    let sec = rest.split('S').next()?;
    Some(
        (h.trim().parse::<f64>().ok()?
            + m.parse::<f64>().ok()? / 60.0
            + sec.parse::<f64>().ok()? / 3600.0)
            * 15.0,
    )
}

/// `-34D20'42.17"` to degrees.
fn parse_dms(s: &str) -> Option<f64> {
    let (d, rest) = s.split_once('D')?;
    let (m, rest) = rest.split_once('\'')?;
    let sec = rest.split('"').next()?;
    let d = d.trim();
    let sign = if d.starts_with('-') { -1.0 } else { 1.0 };
    let deg = d.trim_start_matches(['+', '-']).parse::<f64>().ok()?;
    Some(sign * (deg + m.parse::<f64>().ok()? / 60.0 + sec.parse::<f64>().ok()? / 3600.0))
}

/// hips2fits request for a TAN cutout centred on the position.
pub fn cutout_url(cfg: &SkySources, ra_deg: f64, dec_deg: f64) -> String {
    format!(
        "{}?hips={}&width={px}&height={px}&fov={}&projection=TAN&coordsys=icrs&ra={ra_deg:.6}&dec={dec_deg:.6}&format=jpg",
        cfg.hips2fits_url,
        encode(&cfg.survey),
        cfg.fov_deg,
        px = cfg.size_px,
    )
}

/// Use the other documented CDS endpoint when a built-in endpoint fails.
/// Custom servers remain exclusive and never send their positions to a public mirror.
fn cutout_urls(cfg: &SkySources, ra_deg: f64, dec_deg: f64) -> Vec<String> {
    let primary = cutout_url(cfg, ra_deg, dec_deg);
    let mirror = match cfg.hips2fits_url.as_str() {
        CDS_PRIMARY => Some(CDS_MIRROR),
        CDS_MIRROR => Some(CDS_PRIMARY),
        _ => None,
    };
    let mut urls = vec![primary.clone()];
    if let Some(mirror) = mirror {
        urls.push(format!("{}{}", mirror, &primary[cfg.hips2fits_url.len()..]));
    }
    urls
}

fn endpoint(url: &str) -> &str {
    let authority_start = url.find("://").map_or(0, |i| i + 3);
    let end = url[authority_start..]
        .find('/')
        .map_or(url.len(), |i| authority_start + i);
    &url[..end]
}

fn jpeg_signature(body: &[u8]) -> bool {
    body.len() >= 4 && body[..2] == [0xFF, 0xD8]
}

/// Names a visit list uses for things that are not sky positions.
fn is_internal_name(name: &str) -> bool {
    let n = name.trim();
    n.is_empty()
        || n.to_ascii_lowercase().contains("apt")
        || n.starts_with("input_")
        || n.contains("-Background")
        || n.contains("-bkg")
}

struct Http {
    agent: ureq::Agent,
    user_agent: String,
    cache: PathBuf,
    max_age_s: f64,
    offline: bool,
    paused: RefCell<BTreeMap<String, Instant>>,
}

impl Http {
    fn cached(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.cache.join(key);
        let age = std::fs::metadata(&path)
            .ok()?
            .modified()
            .ok()?
            .elapsed()
            .ok()?;
        (age.as_secs_f64() < self.max_age_s)
            .then(|| std::fs::read(&path).ok())
            .flatten()
    }

    fn get(&self, key: &str, url: &str) -> Result<Vec<u8>, String> {
        self.get_many(key, &[url.to_owned()], false)
    }

    fn get_many(&self, key: &str, urls: &[String], jpeg: bool) -> Result<Vec<u8>, String> {
        if let Some(body) = self.cached(key).filter(|b| !jpeg || jpeg_signature(b)) {
            return Ok(body);
        }
        if self.offline {
            return Err("offline and not cached".into());
        }
        let mut errors = Vec::new();
        for url in urls {
            let host = endpoint(url);
            if self
                .paused
                .borrow()
                .get(host)
                .is_some_and(|until| *until > Instant::now())
            {
                errors.push(format!("{host}: retries paused after an endpoint failure"));
                continue;
            }
            let mut resp = match self
                .agent
                .get(url)
                .header("User-Agent", &self.user_agent)
                .call()
            {
                Ok(resp) => resp,
                Err(e) => {
                    self.pause(host);
                    errors.push(format!("{url}: {e}"));
                    continue;
                }
            };
            let status = resp.status().as_u16();
            if status != 200 {
                if status == 429 || status >= 500 {
                    self.pause(host);
                }
                errors.push(format!("{url}: HTTP {status}"));
                continue;
            }
            let body = match resp.body_mut().with_config().limit(MAX_BODY).read_to_vec() {
                Ok(body) => body,
                Err(e) => {
                    self.pause(host);
                    errors.push(format!("{url}: {e}"));
                    continue;
                }
            };
            if body.is_empty() || (jpeg && !jpeg_signature(&body)) {
                errors.push(format!("{url}: empty reply or invalid JPEG signature"));
                continue;
            }
            self.paused.borrow_mut().remove(host);
            std::fs::create_dir_all(&self.cache)
                .map_err(|e| format!("{}: {e}", self.cache.display()))?;
            std::fs::write(self.cache.join(key), &body)
                .map_err(|e| format!("{}: {e}", self.cache.display()))?;
            return Ok(body);
        }
        Err(errors.join("; "))
    }

    fn pause(&self, host: &str) {
        self.paused
            .borrow_mut()
            .insert(host.to_owned(), Instant::now() + ENDPOINT_PAUSE);
    }

    fn text(&self, key: &str, url: &str) -> Result<String, String> {
        String::from_utf8(self.get(key, url)?).map_err(|e| format!("{url}: not text: {e}"))
    }
}

fn safe_key(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Resolve a JWST target name to a position: its own designation if it has one, else Sesame, trying the
/// name as written and with hyphens read as spaces.
fn resolve_name(http: &Http, cfg: &SkySources, name: &str) -> Result<Position, String> {
    if let Some(p) = parse_2mass(name) {
        return Ok(p);
    }
    if is_internal_name(name) {
        return Err(format!(
            "{name} is a proposal-internal name, not a catalogued object"
        ));
    }
    let mut tried = Vec::new();
    for variant in [name.to_owned(), name.replace('-', " ")] {
        if tried.contains(&variant) {
            continue;
        }
        let url = cfg.sesame_url.replace("{name}", &encode(&variant));
        let text = http.text(&format!("sesame-{}.txt", safe_key(&variant)), &url)?;
        if let Some(p) = parse_sesame(&text, name) {
            return Ok(p);
        }
        tried.push(variant);
    }
    Err(format!("Sesame does not know {name:?}"))
}

fn resolve(
    http: &Http,
    cfg: &SkySources,
    req: &FieldRequest,
) -> Result<(Position, Vec<u8>), String> {
    let pos = match req {
        FieldRequest::Target(name) => resolve_name(http, cfg, name)?,
        FieldRequest::HstProgram(id) => {
            let url = cfg.hst_program_url.replace("{id}", id);
            let text = http.text(&format!("hstpro-{}.txt", safe_key(id)), &url)?;
            parse_hst_targets(&text).into_iter().next().ok_or_else(|| {
                "the program's Phase II file lists no fixed targets (solar-system or moving targets have no sky position)"
                    .to_owned()
            })?
        }
    };
    let urls = cutout_urls(cfg, pos.ra_deg, pos.dec_deg);
    let key = format!(
        "cutout-{:.4}-{:+.4}-{}.jpg",
        pos.ra_deg,
        pos.dec_deg,
        safe_key(&cfg.survey)
    );
    let jpeg = http.get_many(&key, &urls, true)?;
    Ok((pos, jpeg))
}

/// Serve picture requests until the channel closes. Each reply is an `Event::Field`.
pub fn run(
    cfg: SkySources,
    user_agent: String,
    timeout_s: f64,
    cache_dir: &Path,
    requests: &Receiver<(String, FieldRequest)>,
    tx: &SyncSender<Event>,
    offline: bool,
) {
    let http = Http {
        agent: ureq::Agent::config_builder()
            .tls_config(crate::tls::http_config())
            .timeout_global(Some(Duration::from_secs_f64(timeout_s)))
            .http_status_as_error(false)
            .build()
            .into(),
        user_agent,
        cache: cache_dir.join("sky"),
        max_age_s: cfg.cache_s,
        offline,
        paused: RefCell::new(BTreeMap::new()),
    };
    while let Ok((vehicle, req)) = requests.recv() {
        let state = if offline {
            // Cached pictures still work offline; the cache layer answers before any request is made.
            match resolve(&http, &cfg, &req) {
                Ok(r) => ready(&cfg, r),
                Err(_) => FieldState::Unavailable("offline and not cached".into()),
            }
        } else {
            match resolve(&http, &cfg, &req) {
                Ok(r) => ready(&cfg, r),
                Err(e) => {
                    tracing::warn!(?req, error = %e, "field picture unavailable");
                    FieldState::Unavailable(e)
                }
            }
        };
        if tx
            .send(Event::Field {
                vehicle,
                key: req.key(),
                state,
            })
            .is_err()
        {
            return;
        }
    }
}

fn ready(cfg: &SkySources, (pos, jpeg): (Position, Vec<u8>)) -> FieldState {
    FieldState::Ready {
        target: pos.name,
        ra_deg: pos.ra_deg,
        dec_deg: pos.dec_deg,
        fov_deg: cfg.fov_deg,
        origin: pos.origin,
        survey: cfg.survey.clone(),
        jpeg: Arc::new(jpeg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHttp(Http);

    impl Drop for TestHttp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0.cache);
        }
    }

    fn test_http(offline: bool) -> TestHttp {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        TestHttp(Http {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(2)))
                .http_status_as_error(false)
                .build()
                .into(),
            user_agent: "retroradar-test".into(),
            cache: std::env::temp_dir().join(format!(
                "retroradar-sky-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            )),
            max_age_s: 3600.0,
            offline,
            paused: RefCell::new(BTreeMap::new()),
        })
    }

    fn serve(replies: Vec<(u16, Vec<u8>)>) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::Shutdown;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/image", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            for (status, body) in replies {
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "mock HTTP request timed out");
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(e) => panic!("mock HTTP accept: {e}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                // Borrow one socket instead of keeping a cloned descriptor alive
                // while the other handle is closed on Windows.
                {
                    let mut reader = BufReader::new(&mut stream);
                    loop {
                        let mut line = String::new();
                        assert!(reader.read_line(&mut line).unwrap() > 0);
                        if line == "\r\n" {
                            break;
                        }
                    }
                }
                let mut response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes();
                response.extend_from_slice(&body);
                stream.write_all(&response).unwrap();
                // Send FIN only after the whole reply, then wait for the peer's
                // close before dropping the socket. A bare close can race with
                // the client's body read and produce WSAECONNRESET (10054).
                let peer_closed = |e: &std::io::Error| {
                    matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::NotConnected
                    )
                };
                match stream.shutdown(Shutdown::Write) {
                    Ok(()) => {}
                    Err(e) if peer_closed(&e) => continue,
                    Err(e) => panic!("mock HTTP send shutdown: {e}"),
                }
                let mut extra = [0u8; 1];
                match stream.read(&mut extra) {
                    Ok(0) => {}
                    // The client may cancel an HTTP error response without
                    // reading its body; its reset is not a server teardown bug.
                    Err(e) if peer_closed(&e) => {}
                    other => panic!("mock HTTP expected peer close after GET: {other:?}"),
                }
            }
        });
        (url, worker)
    }

    #[test]
    fn mock_http_delivers_complete_replies_then_clean_eof() {
        use std::io::{Read, Write};
        use std::net::{Shutdown, TcpStream};
        let replies = vec![
            (200, b"ready".to_vec()),
            (200, vec![0x5a; 256 * 1024]),
            (503, b"unavailable".to_vec()),
            (200, vec![]),
        ];
        let (url, server) = serve(replies.clone());
        let address = url
            .strip_prefix("http://")
            .unwrap()
            .strip_suffix("/image")
            .unwrap();
        for (i, (status, body)) in replies.iter().enumerate() {
            let mut stream = TcpStream::connect(address).unwrap();
            stream.set_nodelay(true).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            // Exercise request headers arriving in several writes, and peers
            // closing their send side either before or after reading the reply.
            for part in [
                "GET /image HTTP/1.1\r\n",
                "Host: localhost\r\n",
                "Connection: close\r\n",
                "\r\n",
            ] {
                stream.write_all(part.as_bytes()).unwrap();
            }
            if i % 2 == 0 {
                stream.shutdown(Shutdown::Write).unwrap();
            }
            let mut reply = Vec::new();
            // read_to_end must observe a clean FIN, not a connection reset.
            stream.read_to_end(&mut reply).unwrap();
            let mut expected = format!(
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            expected.extend_from_slice(body);
            assert_eq!(reply, expected);
        }
        server.join().unwrap();
    }

    #[test]
    fn documented_cds_mirror_preserves_query_and_custom_servers_remain_exclusive() {
        let mut cfg = cfg();
        let urls = cutout_urls(&cfg, 12.0, -20.0);
        assert_eq!(urls.len(), 2);
        assert!(urls[1].starts_with(CDS_MIRROR));
        assert_eq!(
            urls[0].split_once('?').unwrap().1,
            urls[1].split_once('?').unwrap().1
        );
        cfg.hips2fits_url = "https://private.example/cutout".into();
        assert_eq!(cutout_urls(&cfg, 12.0, -20.0).len(), 1);
    }

    #[test]
    fn sky_service_503_falls_back_and_reuses_the_successful_cache() {
        let fixture = test_http(false);
        let jpeg = vec![0xff, 0xd8, 0xff, 0xd9];
        let (primary, a) = serve(vec![(503, b"temporarily unavailable".to_vec())]);
        let (mirror, b) = serve(vec![(200, jpeg.clone())]);
        let urls = vec![primary, mirror];
        assert_eq!(
            fixture.0.get_many("picture.jpg", &urls, true).unwrap(),
            jpeg
        );
        a.join().unwrap();
        b.join().unwrap();
        // Both servers are gone: this succeeds only from the validated cache.
        assert_eq!(
            fixture.0.get_many("picture.jpg", &urls, true).unwrap(),
            jpeg
        );
    }

    #[test]
    fn error_pages_are_not_cached_as_pictures_and_the_mirror_is_tried() {
        let fixture = test_http(false);
        let (primary, a) = serve(vec![(200, b"<html>error</html>".to_vec())]);
        let (mirror, b) = serve(vec![(200, vec![0xff, 0xd8, 0xff, 0xd9])]);
        let body = fixture
            .0
            .get_many("picture.jpg", &[primary, mirror], true)
            .unwrap();
        assert!(jpeg_signature(&body));
        assert_eq!(
            std::fs::read(fixture.0.cache.join("picture.jpg")).unwrap(),
            body
        );
        a.join().unwrap();
        b.join().unwrap();
    }

    #[test]
    fn endpoint_failures_pause_other_targets_and_can_retry_after_the_pause() {
        let fixture = test_http(false);
        let (url, server) = serve(vec![(503, vec![]), (200, b"ready".to_vec())]);
        assert!(fixture.0.get("first", &url).unwrap_err().contains("503"));
        assert!(
            fixture
                .0
                .get("second", &url)
                .unwrap_err()
                .contains("retries paused")
        );
        fixture
            .0
            .paused
            .borrow_mut()
            .insert(endpoint(&url).to_owned(), Instant::now());
        assert_eq!(fixture.0.get("third", &url).unwrap(), b"ready");
        server.join().unwrap();
    }

    #[test]
    fn offline_sky_cache_misses_never_connect_and_cache_hits_still_work() {
        let fixture = test_http(true);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/image", listener.local_addr().unwrap());
        assert_eq!(
            fixture.0.get("missing", &url).unwrap_err(),
            "offline and not cached"
        );
        std::fs::create_dir_all(&fixture.0.cache).unwrap();
        std::fs::write(fixture.0.cache.join("cached"), b"cached text").unwrap();
        assert_eq!(fixture.0.get("cached", &url).unwrap(), b"cached text");
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[test]
    fn invalid_cached_images_are_rejected_without_network_in_offline_mode() {
        let fixture = test_http(true);
        std::fs::create_dir_all(&fixture.0.cache).unwrap();
        std::fs::write(fixture.0.cache.join("bad.jpg"), b"<html>error</html>").unwrap();
        assert_eq!(
            fixture
                .0
                .get_many("bad.jpg", &["http://127.0.0.1:1/image".into()], true)
                .unwrap_err(),
            "offline and not cached"
        );
    }

    const PRO: &str = "\
1   HD-35165     STAR, B3-B5 V-IV, BE    RA=05H21M16.8603S,               J2000             V = 6.09
                                         DEC=-34D20'42.17\"                                  erg/s
    Reference Frame: ICRS    Extended: NO
2   -ALF-ARA     STAR, B0-B2 V-IV, BE    RA=17H31M50.4915S,               J2000             V = 2.95
                                         DEC=+49D52'34.12\"
";

    fn cfg() -> SkySources {
        retroradar_core::config::Config::from_layers(retroradar_core::config::DEFAULT_CONFIG, None)
            .unwrap()
            .sources
            .sky
    }

    #[test]
    fn two_mass_names_carry_their_position() {
        let p = parse_2mass("2MASS-J18022716+6043356").unwrap();
        assert!((p.ra_deg - 270.6132).abs() < 1e-3, "{}", p.ra_deg); // 18h02m27.16s
        assert!((p.dec_deg - 60.7266).abs() < 1e-3, "{}", p.dec_deg); // +60d43m35.6s
        let s = parse_2mass("2MASS-J03313052-2739514").unwrap();
        assert!(
            s.dec_deg < -27.0 && s.dec_deg > -28.0,
            "southern declination is negative"
        );
        assert!(parse_2mass("V339-Del").is_none());
        assert!(
            parse_2mass("2MASS-J1802+60").is_none(),
            "short designations are refused, not guessed"
        );
    }

    #[test]
    fn sesame_replies_are_read_from_the_j_line() {
        let text = "# M31\n%@ @1575544\n%J 10.68470833 +41.26875000 = 00 42 44.330  +41 16 07.50\n";
        let p = parse_sesame(text, "M31").unwrap();
        assert_eq!((p.ra_deg, p.dec_deg), (10.68470833, 41.26875));
        assert!(parse_sesame("# nothing\n#!Sc=Simbad: not found\n", "x").is_none());
        assert!(
            parse_sesame("%J 400 +10", "x").is_none(),
            "out-of-range right ascension"
        );
    }

    #[test]
    fn hst_program_files_yield_fixed_targets_only() {
        let t = parse_hst_targets(PRO);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].name, "HD-35165");
        assert!((t[0].ra_deg - 80.3202).abs() < 1e-3, "{}", t[0].ra_deg); // 5h21m16.86s
        assert!((t[0].dec_deg - -34.34505).abs() < 1e-4, "{}", t[0].dec_deg);
        assert_eq!(
            t[1].name, "ALF-ARA",
            "the leading dash of a Greek-letter name is dropped"
        );
        assert!(t[1].dec_deg > 49.0);
        assert!(parse_hst_targets("TARGET NAME: IO\nSTD=JUPITER\n").is_empty());
    }

    #[test]
    fn cutout_urls_ask_for_a_tan_jpeg_around_the_target() {
        let u = cutout_url(&cfg(), 10.6847, 41.2687);
        assert!(u.starts_with("https://alasky.cds.unistra.fr/hips-image-services/hips2fits?hips=CDS%2FP%2FDSS2%2Fcolor"), "{u}");
        for part in [
            "width=512",
            "height=512",
            "fov=0.25",
            "projection=TAN",
            "coordsys=icrs",
            "ra=10.684700",
            "dec=41.268700",
            "format=jpg",
        ] {
            assert!(u.contains(part), "{part} in {u}");
        }
    }

    #[test]
    fn proposal_internal_names_are_not_sent_to_sesame() {
        for n in [
            "input_lulls_00",
            "GO9016_apt_v2",
            "V339-Del-Background",
            "Pa30-bkg",
            "",
        ] {
            assert!(is_internal_name(n), "{n}");
        }
        assert!(!is_internal_name("V339-Del") && !is_internal_name("HD-209458b"));
    }
}
