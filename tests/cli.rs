use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_dir() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("prose-lint-test-{nonce}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn scans_directories_and_emits_json_array() {
    let dir = fixture_dir();
    fs::write(dir.join("a.md"), "It is worth noting that this works.").unwrap();
    fs::write(dir.join("ignored.rs"), "Importantly, this is code.").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "--format", "json"])
        .arg(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let reports: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reports.as_array().unwrap().len(), 1);
    assert!(reports[0]["path"].as_str().unwrap().ends_with("a.md"));
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn directory_scan_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    let dir = fixture_dir();
    fs::write(dir.join("real.md"), "Plain text.").unwrap();
    symlink(dir.join("real.md"), dir.join("linked.md")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "--format", "json"])
        .arg(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let reports: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reports.as_array().unwrap().len(), 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn broken_pipe_is_treated_as_normal_early_consumer_exit() {
    use std::process::Stdio;

    let dir = fixture_dir();
    let text = "It is important to note that this works.\n".repeat(20_000);
    let file = dir.join("large.md");
    fs::write(&file, text).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "--format", "json"])
        .arg(&file)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    assert!(status.success(), "broken pipe exit was {status:?}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn strict_mode_fails_only_for_high_confidence_findings() {
    let dir = fixture_dir();
    let weak = dir.join("weak.md");
    fs::write(&weak, "The method accentuates the result.").unwrap();
    let weak_status = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "--strict", "--all"])
        .arg(&weak)
        .status()
        .unwrap();
    assert!(weak_status.success());

    let strong = dir.join("strong.md");
    fs::write(&strong, "It is important to note that this works.").unwrap();
    let strong_status = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "--strict"])
        .arg(&strong)
        .status()
        .unwrap();
    assert_eq!(strong_status.code(), Some(1));
    fs::remove_dir_all(dir).unwrap();
}
