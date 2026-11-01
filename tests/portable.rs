//! Each child gets independent process-wide paths; tests never modify the user's profile.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "retroradar-doctor-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn doctor_resolves_portable_paths_without_writing_files() {
    let root = root().join("path with spaces");
    let out = Command::new(env!("CARGO_BIN_EXE_retroradar"))
        .args([
            "--data-dir",
            root.to_str().expect("UTF-8 temp path"),
            "doctor",
        ])
        .output()
        .expect("doctor starts");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("R01 DOCTOR OK"), "{text}");
    for (label, expected) in [
        ("config", root.join("config").join("config.toml")),
        ("cache", root.join("cache")),
        ("data", root.join("data")),
    ] {
        let prefix = format!("{label}: ");
        let reported = text
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .unwrap_or_else(|| panic!("missing {label} path in {text}"));
        assert_eq!(std::path::Path::new(reported), expected, "{text}");
    }
    assert!(!root.exists(), "doctor must not create directories");
}

#[test]
fn doctor_rejects_invalid_portable_configuration() {
    let root = root();
    std::fs::create_dir_all(root.join("config")).expect("create fixture");
    std::fs::write(root.join("config").join("config.toml"), "not valid TOML [")
        .expect("write fixture");
    let out = Command::new(env!("CARGO_BIN_EXE_retroradar"))
        .args([
            "--data-dir",
            root.to_str().expect("UTF-8 temp path"),
            "doctor",
        ])
        .output()
        .expect("doctor starts");
    std::fs::remove_dir_all(root).expect("remove fixture");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("loading configuration"));
}
