use sdd_core::{
    compute_coverage, is_test_file, parse_requirements, parse_tasks, scan_directory, CoreError,
    CoverageStatus,
};
use std::path::Path;

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
// Parser integration tests
// ---------------------------------------------------------------------------

// @req SCS-PARSE-001
#[test]
fn test_parse_requirements_valid() {
    let reqs = parse_requirements(&fixture("valid-project/requirements.yaml"))
        .expect("should parse valid requirements");
    assert_eq!(reqs.len(), 3);
    assert!(reqs.iter().all(|r| !r.id.is_empty()));
    assert!(reqs.iter().all(|r| !r.created_at.is_empty()));
    assert!(reqs.iter().all(|r| !r.updated_at.is_empty()));
}

// @req SCS-PARSE-001
#[test]
fn test_parse_requirements_malformed_returns_error() {
    let err = parse_requirements(&fixture("malformed-yaml/requirements.yaml")).unwrap_err();
    assert!(matches!(err, CoreError::Yaml { .. }));
}

// @req SCS-PARSE-001
#[test]
fn test_parse_requirements_missing_file_returns_io_error() {
    let err = parse_requirements(Path::new("/no/such/file.yaml")).unwrap_err();
    assert!(matches!(err, CoreError::Io { .. }));
}

// @req SCS-PARSE-002
#[test]
fn test_parse_tasks_valid() {
    let tasks =
        parse_tasks(&fixture("valid-project/tasks.yaml")).expect("should parse valid tasks");
    assert_eq!(tasks.len(), 2);
    assert!(tasks.iter().all(|t| !t.id.is_empty()));
    assert!(tasks.iter().all(|t| !t.requirement_id.is_empty()));
}

// @req SCS-PARSE-002
#[test]
fn test_parse_tasks_missing_file_returns_empty() {
    let tasks = parse_tasks(Path::new("/no/such/tasks.yaml"))
        .expect("missing tasks.yaml should return empty vec");
    assert!(tasks.is_empty());
}

// ---------------------------------------------------------------------------
// Scanner integration tests
// ---------------------------------------------------------------------------

// @req SCS-SCAN-001
#[test]
fn test_scan_finds_annotations_in_all_languages() {
    let base = fixture("mixed-languages");
    let annotations = scan_directory(&base).expect("should scan");
    let ids: std::collections::HashSet<&str> =
        annotations.iter().map(|a| a.req_id.as_str()).collect();
    for id in &[
        "FR-MIX-001",
        "FR-MIX-002",
        "FR-MIX-003",
        "FR-MIX-004",
        "FR-MIX-005",
        "FR-MIX-006",
    ] {
        assert!(ids.contains(id), "missing annotation for {id}");
    }
}

// @req SCS-SCAN-001
#[test]
fn test_scan_finds_nested_annotations() {
    let base = fixture("nested-dirs");
    let annotations = scan_directory(&base).expect("should scan");
    assert!(
        annotations.iter().any(|a| a.req_id == "FR-NEST-001"),
        "deeply nested annotation should be found"
    );
}

// @req SCS-SCAN-002
#[test]
fn test_classify_test_vs_impl() {
    assert!(is_test_file(Path::new("tests/foo.rs")));
    assert!(is_test_file(Path::new("src/tests/bar.rs")));
    assert!(is_test_file(Path::new("test_main.py")));
    assert!(is_test_file(Path::new("foo_test.go")));
    assert!(is_test_file(Path::new("foo.test.ts")));
    assert!(!is_test_file(Path::new("src/main.rs")));
    assert!(!is_test_file(Path::new("lib/utils.py")));
}

// @req SCS-SCAN-002
#[test]
fn test_test_file_annotations_classified_correctly() {
    let base = fixture("valid-project");
    let annotations = scan_directory(&base).expect("should scan");
    let test_anns: Vec<_> = annotations
        .iter()
        .filter(|a| a.annotation_type == sdd_core::AnnotationType::Test)
        .collect();
    assert!(!test_anns.is_empty(), "should find test annotations");
    assert!(
        test_anns.iter().all(|a| a.file.contains("tests")),
        "test annotations must come from test files"
    );
}

// ---------------------------------------------------------------------------
// Coverage integration tests
// ---------------------------------------------------------------------------

// @req SCS-COV-001
#[test]
fn test_coverage_status_computation() {
    let base = fixture("valid-project");
    let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
    let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
    let annotations = scan_directory(&base).unwrap();
    let result = compute_coverage(&reqs, &tasks, &annotations);

    let status_of = |id: &str| {
        result
            .requirements
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.status.clone())
            .expect("requirement should exist")
    };

    assert_eq!(status_of("FR-TEST-001"), CoverageStatus::Covered);
    assert_eq!(status_of("FR-TEST-002"), CoverageStatus::Partial);
    assert_eq!(status_of("FR-TEST-003"), CoverageStatus::Missing);
}

// @req SCS-COV-002
#[test]
fn test_orphan_annotation_detection_integration() {
    let base = fixture("orphans-project");
    let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
    let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
    let annotations = scan_directory(&base).unwrap();
    let result = compute_coverage(&reqs, &tasks, &annotations);

    assert!(
        !result.orphan_annotations.is_empty(),
        "should detect orphan annotations"
    );
    assert!(result
        .orphan_annotations
        .iter()
        .any(|a| a.req_id == "NONEXISTENT-001"));
}

// @req SCS-COV-003
#[test]
fn test_orphan_task_detection_integration() {
    let base = fixture("orphans-project");
    let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
    let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
    let annotations = scan_directory(&base).unwrap();
    let result = compute_coverage(&reqs, &tasks, &annotations);

    assert!(
        !result.orphan_tasks.is_empty(),
        "should detect orphan tasks"
    );
    assert!(result
        .orphan_tasks
        .iter()
        .any(|t| t.requirement_id == "NONEXISTENT-001"));
}

// @req SCS-COV-004
#[test]
fn test_stats_aggregation_integration() {
    let base = fixture("valid-project");
    let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
    let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
    let annotations = scan_directory(&base).unwrap();
    let result = compute_coverage(&reqs, &tasks, &annotations);
    let stats = &result.stats;

    assert_eq!(stats.requirements.total, 3);
    assert!(stats.coverage >= 0.0 && stats.coverage <= 100.0);
    assert!(stats.annotations.total >= 1);
    assert!(!stats.last_scan_at.is_empty());
}

// ---------------------------------------------------------------------------
// Error handling integration tests
// ---------------------------------------------------------------------------

// @req SCS-ERR-001
#[test]
fn test_empty_source_directory_is_not_an_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let annotations = scan_directory(dir.path()).expect("should not fail on empty dir");
    assert!(annotations.is_empty());
}

// @req SCS-ERR-001
#[test]
fn test_malformed_yaml_produces_descriptive_error() {
    let err = parse_requirements(&fixture("malformed-yaml/requirements.yaml")).unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty(), "error message should not be empty: {msg}");
    // Should reference the YAML error
    assert!(
        msg.contains("YAML")
            || msg.contains("yaml")
            || msg.contains("line")
            || msg.contains("parse"),
        "error should describe the YAML problem: {msg}"
    );
}
