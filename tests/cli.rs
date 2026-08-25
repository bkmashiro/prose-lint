use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn fixture_dir() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "prose-lint-test-{}-{nonce}-{sequence}",
        std::process::id()
    ));
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

#[cfg(unix)]
#[test]
fn glob_scan_does_not_follow_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let dir = fixture_dir();
    let external = fixture_dir();
    fs::write(dir.join("real.md"), "Plain text.").unwrap();
    fs::write(external.join("outside.md"), "Importantly, outside.").unwrap();
    symlink(&external, dir.join("linked-dir")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "**/*.md", "--format", "json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reports: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let paths = reports.as_array().unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0]["path"], "real.md");

    let explicit = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "linked-dir/**/*.md", "--format", "json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        explicit.status.success(),
        "{}",
        String::from_utf8_lossy(&explicit.stderr)
    );
    let reports: serde_json::Value = serde_json::from_slice(&explicit.stdout).unwrap();
    assert_eq!(reports.as_array().unwrap().len(), 1);
    assert!(
        reports[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("linked-dir/outside.md")
    );
    fs::remove_dir_all(dir).unwrap();
    fs::remove_dir_all(external).unwrap();
}

#[test]
fn expands_globs_without_shell_help() {
    let dir = fixture_dir();
    fs::write(dir.join("a.typ"), "Plain Typst prose.").unwrap();
    fs::write(dir.join("b.typ"), "More Typst prose.").unwrap();
    fs::write(dir.join("ignored.md"), "Markdown prose.").unwrap();
    fs::create_dir(dir.join("chapter")).unwrap();
    fs::write(dir.join("chapter/nested.typ"), "Nested Typst prose.").unwrap();

    let top = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "*.typ", "--format", "json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        top.status.success(),
        "{}",
        String::from_utf8_lossy(&top.stderr)
    );
    let reports: serde_json::Value = serde_json::from_slice(&top.stdout).unwrap();
    let paths: Vec<_> = reports
        .as_array()
        .unwrap()
        .iter()
        .map(|report| report["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["a.typ", "b.typ"]);

    let nested = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "**/*.typ", "--format", "json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        nested.status.success(),
        "{}",
        String::from_utf8_lossy(&nested.stderr)
    );
    let reports: serde_json::Value = serde_json::from_slice(&nested.stdout).unwrap();
    assert!(reports.as_array().unwrap().iter().any(|report| {
        report["path"]
            .as_str()
            .unwrap()
            .ends_with("chapter/nested.typ")
    }));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unmatched_glob_reports_the_pattern() {
    let dir = fixture_dir();
    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "*.typ"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("pattern matched no paths: *.typ"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn discovers_nearest_repo_config_and_applies_extra_terms() {
    let workspace = fixture_dir();
    let repo = workspace.join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(
        repo.join(".prose-lint.json"),
        r#"{
  "extra_terms": [
    "magic surface",
    {
      "term": "lands cleanly",
      "severity": "high",
      "message": "Use the repository's concrete merge terminology.",
      "suggestion": "Name the operation."
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(
        repo.join("docs/guide.md"),
        "The magic surface works and the patch lands cleanly.",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "repo/docs/guide.md", "--format", "json", "--strict"])
        .current_dir(&workspace)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let reports: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let custom: Vec<_> = reports[0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["rule_id"] == "custom.repo-term")
        .collect();
    assert_eq!(custom.len(), 2);
    assert!(custom.iter().any(|finding| finding["severity"] == "medium"));
    assert!(custom.iter().any(|finding| finding["severity"] == "high"));
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn one_scan_uses_each_repositories_nearest_config() {
    let workspace = fixture_dir();
    for (repo_name, term) in [("repo-a", "alpha fog"), ("repo-b", "beta fog")] {
        let repo = workspace.join(repo_name);
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::write(
            repo.join(".prose-lint.json"),
            format!(r#"{{"extra_terms":["{term}"]}}"#),
        )
        .unwrap();
        fs::write(repo.join("guide.md"), "alpha fog and beta fog").unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", ".", "--format", "json"])
        .current_dir(&workspace)
        .output()
        .unwrap();
    assert!(output.status.success());
    let reports: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reports.as_array().unwrap().len(), 2);
    for report in reports.as_array().unwrap() {
        let matches: Vec<_> = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["rule_id"] == "custom.repo-term")
            .map(|finding| finding["matched"].as_str().unwrap())
            .collect();
        assert_eq!(matches.len(), 1);
        let path = report["path"].as_str().unwrap();
        assert_eq!(
            matches[0],
            if path.contains("repo-a") {
                "alpha fog"
            } else {
                "beta fog"
            }
        );
    }
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn explicit_config_overrides_auto_discovery() {
    let repo = fixture_dir();
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::write(
        repo.join(".prose-lint.json"),
        r#"{"extra_terms":["auto term"]}"#,
    )
    .unwrap();
    fs::write(
        repo.join("override.json"),
        r#"{"extra_terms":["forced term"]}"#,
    )
    .unwrap();
    fs::write(repo.join("guide.md"), "auto term and forced term").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args([
            "scan",
            "guide.md",
            "--config",
            "override.json",
            "--format",
            "json",
        ])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(output.status.success());
    let reports: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let matches: Vec<_> = reports[0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["rule_id"] == "custom.repo-term")
        .map(|finding| finding["matched"].as_str().unwrap())
        .collect();
    assert_eq!(matches, ["forced term"]);
    fs::remove_dir_all(repo).unwrap();
}

#[test]
fn malformed_repo_config_fails_with_its_path() {
    let repo = fixture_dir();
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::write(repo.join(".prose-lint.json"), r#"{"extra_terms":[""]}"#).unwrap();
    fs::write(repo.join("guide.md"), "Plain prose.").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_prose-lint"))
        .args(["scan", "guide.md"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(".prose-lint.json"));
    assert!(stderr.contains("must not be empty"));
    fs::remove_dir_all(repo).unwrap();
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
