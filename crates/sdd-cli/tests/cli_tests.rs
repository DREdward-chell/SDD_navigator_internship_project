use assert_cmd::Command;
use std::path::Path;
use tempfile::tempdir;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture(rel: &str) -> std::path::PathBuf {
    workspace_root().join("fixtures").join(rel)
}

// ---------------------------------------------------------------------------
// CLI scan tests
// ---------------------------------------------------------------------------

// @req SCS-CLI-001
#[test]
fn test_cli_scan_valid_project_exits_zero() {
    let base = fixture("valid-project");
    let mut cmd = Command::cargo_bin("sdd-coverage").unwrap();
    cmd.current_dir(&base).args([
        "scan",
        "--requirements",
        "requirements.yaml",
        "--tasks",
        "tasks.yaml",
        "--source",
        ".",
    ]);
    cmd.assert().success();
}

// @req SCS-CLI-001
#[test]
fn test_cli_scan_prints_summary() {
    let base = fixture("valid-project");
    let mut cmd = Command::cargo_bin("sdd-coverage").unwrap();
    cmd.current_dir(&base).args([
        "scan",
        "--requirements",
        "requirements.yaml",
        "--tasks",
        "tasks.yaml",
        "--source",
        ".",
    ]);
    let output = cmd.assert().success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("Requirements:"),
        "should print requirements summary"
    );
    assert!(
        text.contains("Coverage:"),
        "should print coverage percentage"
    );
    assert!(
        text.contains("Annotations:"),
        "should print annotations summary"
    );
}

// @req SCS-CLI-001
#[test]
fn test_cli_strict_mode_fails_when_coverage_incomplete() {
    // valid-project has partial and missing requirements → strict should fail
    let base = fixture("valid-project");
    let mut cmd = Command::cargo_bin("sdd-coverage").unwrap();
    cmd.current_dir(&base).args([
        "scan",
        "--requirements",
        "requirements.yaml",
        "--tasks",
        "tasks.yaml",
        "--source",
        ".",
        "--strict",
    ]);
    cmd.assert().failure();
}

// @req SCS-CLI-001
#[test]
fn test_cli_strict_mode_passes_on_full_coverage() {
    // Create a minimal fully-covered project in a temp dir.
    let dir = tempdir().expect("temp dir");

    std::fs::write(
        dir.path().join("requirements.yaml"),
        "requirements:\n  - id: FR-FULL-001\n    title: Full\n    description: \"Fully covered.\"\n    createdAt: \"2026-04-03T10:00:00Z\"\n    updatedAt: \"2026-04-03T10:00:00Z\"\n",
    )
    .unwrap();

    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("main.rs"), "// @req FR-FULL-001\nfn main() {}\n").unwrap();

    let tests = dir.path().join("tests");
    std::fs::create_dir(&tests).unwrap();
    std::fs::write(
        tests.join("test_main.rs"),
        "// @req FR-FULL-001\n#[test]\nfn t() {}\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("sdd-coverage").unwrap();
    cmd.current_dir(dir.path()).args(["scan", "--strict"]);
    cmd.assert().success();
}

// @req SCS-CLI-001
#[test]
fn test_cli_strict_mode_fails_on_orphan_annotation() {
    let dir = tempdir().expect("temp dir");

    std::fs::write(
        dir.path().join("requirements.yaml"),
        "requirements:\n  - id: FR-REAL-001\n    title: Real\n    description: \"Real req.\"\n    createdAt: \"2026-04-03T10:00:00Z\"\n    updatedAt: \"2026-04-03T10:00:00Z\"\n",
    )
    .unwrap();

    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(
        src.join("main.rs"),
        "// @req FR-REAL-001\n// @req GHOST-001\nfn main() {}\n",
    )
    .unwrap();

    let tests = dir.path().join("tests");
    std::fs::create_dir(&tests).unwrap();
    std::fs::write(
        tests.join("test_main.rs"),
        "// @req FR-REAL-001\n#[test]\nfn t() {}\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("sdd-coverage").unwrap();
    cmd.current_dir(dir.path()).args(["scan", "--strict"]);
    cmd.assert().failure();
}

// ---------------------------------------------------------------------------
// Self-hosting test
// ---------------------------------------------------------------------------

// @req SCS-SELF-001
#[test]
fn test_self_hosting() {
    let root = workspace_root();
    let mut cmd = Command::cargo_bin("sdd-coverage").unwrap();
    cmd.current_dir(&root).args([
        "scan",
        "--requirements",
        "requirements.yaml",
        "--tasks",
        "tasks.yaml",
        "--source",
        ".",
        "--strict",
    ]);
    cmd.assert().success();
}

// ---------------------------------------------------------------------------
// Docker artifact test
// ---------------------------------------------------------------------------

// @req SCS-DOCKER-001
#[test]
fn test_dockerfile_exists() {
    let root = workspace_root();
    assert!(
        root.join("Dockerfile").exists(),
        "Dockerfile must exist at workspace root"
    );
}
