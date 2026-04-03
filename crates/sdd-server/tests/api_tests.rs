use axum::http::StatusCode;
use axum_test::TestServer;
use sdd_core::{
    compute_coverage, parse_requirements, parse_tasks, scan_directory, ScanState, ScanStatus,
};
use sdd_server::{
    routes::create_router,
    state::{AppConfig, AppState},
};
use std::{path::Path, sync::Arc};
use tokio::sync::RwLock;

fn fixture_path(rel: &str) -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = crates/sdd-server  →  ../.. = workspace root
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    root.join("fixtures").join(rel)
}

async fn test_server(fixture: &str) -> TestServer {
    let base = fixture_path(fixture);
    let config = AppConfig {
        requirements_path: base.join("requirements.yaml"),
        tasks_path: base.join("tasks.yaml"),
        source_path: base.clone(),
    };

    let reqs = parse_requirements(&config.requirements_path).unwrap();
    let tasks = parse_tasks(&config.tasks_path).unwrap();
    let annotations = scan_directory(&config.source_path).unwrap();
    let scan_result = compute_coverage(&reqs, &tasks, &annotations);

    let mut app_state = AppState::new(config);
    app_state.scan_result = Some(scan_result);
    app_state.scan_status = ScanStatus {
        status: ScanState::Completed,
        started_at: Some("2026-04-03T12:00:00Z".to_string()),
        completed_at: Some("2026-04-03T12:00:01Z".to_string()),
        duration: Some(1000),
    };

    let state = Arc::new(RwLock::new(app_state));
    TestServer::new(create_router(state))
}

async fn empty_server() -> TestServer {
    let base = fixture_path("valid-project");
    let config = AppConfig {
        requirements_path: base.join("requirements.yaml"),
        tasks_path: base.join("tasks.yaml"),
        source_path: base,
    };
    let state = Arc::new(RwLock::new(AppState::new(config)));
    TestServer::new(create_router(state))
}

// ---------------------------------------------------------------------------
// Healthcheck
// ---------------------------------------------------------------------------

// @req SCS-API-001
#[tokio::test]
async fn test_healthcheck() {
    let server = test_server("valid-project").await;
    let resp = server.get("/healthcheck").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["status"], "healthy");
    assert!(body["version"].is_string());
    assert!(body["timestamp"].is_string());
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

// @req SCS-API-002
#[tokio::test]
async fn test_stats() {
    let server = test_server("valid-project").await;
    let resp = server.get("/stats").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["requirements"]["total"], 3);
    assert!(body["coverage"].is_number());
    assert!(body["lastScanAt"].is_string());
}

// @req SCS-API-002
#[tokio::test]
async fn test_stats_no_scan_returns_503() {
    let server = empty_server().await;
    let resp = server.get("/stats").await;
    assert_eq!(resp.status_code(), StatusCode::SERVICE_UNAVAILABLE);
}

// ---------------------------------------------------------------------------
// Requirements
// ---------------------------------------------------------------------------

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_list() {
    let server = test_server("valid-project").await;
    let resp = server.get("/requirements").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.as_array().unwrap().len() == 3);
}

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_filter_by_status() {
    let server = test_server("valid-project").await;
    let resp = server
        .get("/requirements")
        .add_query_param("status", "covered")
        .await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["status"], "covered");
    assert_eq!(arr[0]["id"], "FR-TEST-001");
}

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_filter_by_type() {
    let server = test_server("valid-project").await;
    let resp = server
        .get("/requirements")
        .add_query_param("type", "FR")
        .await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body.as_array().unwrap().len(), 3);
}

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_sort_order() {
    let server = test_server("valid-project").await;
    let resp = server
        .get("/requirements")
        .add_query_param("sort", "id")
        .add_query_param("order", "desc")
        .await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let arr = body.as_array().unwrap();
    assert_eq!(arr[0]["id"], "FR-TEST-003");
    assert_eq!(arr[2]["id"], "FR-TEST-001");
}

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_invalid_status_returns_400() {
    let server = test_server("valid-project").await;
    let resp = server
        .get("/requirements")
        .add_query_param("status", "unknown")
        .await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["error"], "bad_request");
}

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_invalid_sort_returns_400() {
    let server = test_server("valid-project").await;
    let resp = server
        .get("/requirements")
        .add_query_param("sort", "garbage")
        .await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// Requirement detail
// ---------------------------------------------------------------------------

// @req SCS-API-004
#[tokio::test]
async fn test_requirement_detail() {
    let server = test_server("valid-project").await;
    let resp = server.get("/requirements/FR-TEST-001").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["id"], "FR-TEST-001");
    assert_eq!(body["status"], "covered");
    assert!(body["annotations"].is_array());
    assert!(body["tasks"].is_array());
    // FR-TEST-001 has at least 1 impl and 1 test annotation
    let annotations = body["annotations"].as_array().unwrap();
    assert!(!annotations.is_empty());
}

// @req SCS-API-004
#[tokio::test]
async fn test_requirement_not_found_returns_404() {
    let server = test_server("valid-project").await;
    let resp = server.get("/requirements/FR-UNKNOWN-999").await;
    assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["error"], "not_found");
    assert!(body["message"].as_str().unwrap().contains("FR-UNKNOWN-999"));
}

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

// @req SCS-API-005
#[tokio::test]
async fn test_annotations_list() {
    let server = test_server("valid-project").await;
    let resp = server.get("/annotations").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.as_array().unwrap().len() >= 2);
}

// @req SCS-API-005
#[tokio::test]
async fn test_annotations_filter_orphans() {
    let server = test_server("orphans-project").await;

    let resp_all = server.get("/annotations").await;
    resp_all.assert_status_ok();
    let all = resp_all.json::<serde_json::Value>();

    let resp_orphans = server
        .get("/annotations")
        .add_query_param("orphans", "true")
        .await;
    resp_orphans.assert_status_ok();
    let orphans = resp_orphans.json::<serde_json::Value>();

    // all should include more than orphans-only
    assert!(all.as_array().unwrap().len() > orphans.as_array().unwrap().len());
    // orphans should only contain NONEXISTENT-001
    let orphan_arr = orphans.as_array().unwrap();
    assert!(!orphan_arr.is_empty());
    assert!(orphan_arr.iter().all(|a| a["reqId"] == "NONEXISTENT-001"));
}

// @req SCS-API-005
#[tokio::test]
async fn test_annotations_filter_by_type() {
    let server = test_server("valid-project").await;
    let resp = server
        .get("/annotations")
        .add_query_param("type", "test")
        .await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let arr = body.as_array().unwrap();
    assert!(arr.iter().all(|a| a["type"] == "test"));
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

// @req SCS-API-006
#[tokio::test]
async fn test_tasks_list() {
    let server = test_server("valid-project").await;
    let resp = server.get("/tasks").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body.as_array().unwrap().len(), 2);
}

// @req SCS-API-006
#[tokio::test]
async fn test_tasks_filter_by_status() {
    let server = test_server("valid-project").await;
    let resp = server.get("/tasks").add_query_param("status", "done").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "TASK-001");
}

// @req SCS-API-006
#[tokio::test]
async fn test_tasks_filter_orphans() {
    let server = test_server("orphans-project").await;
    let resp = server
        .get("/tasks")
        .add_query_param("orphans", "true")
        .await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["requirementId"], "NONEXISTENT-001");
}

// ---------------------------------------------------------------------------
// Scan
// ---------------------------------------------------------------------------

// @req SCS-API-007
#[tokio::test]
async fn test_trigger_scan_returns_202() {
    let server = test_server("valid-project").await;
    let resp = server.post("/scan").await;
    assert_eq!(resp.status_code(), StatusCode::ACCEPTED);
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["status"], "scanning");
    assert!(body["startedAt"].is_string());
}

// @req SCS-API-007
#[tokio::test]
async fn test_scan_status() {
    let server = test_server("valid-project").await;
    let resp = server.get("/scan").await;
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    // Pre-populated state has Completed status.
    assert_eq!(body["status"], "completed");
    assert!(body["startedAt"].is_string());
    assert!(body["completedAt"].is_string());
    assert!(body["duration"].is_number());
}

// ---------------------------------------------------------------------------
// Error response format
// ---------------------------------------------------------------------------

// @req SCS-ERR-001
#[tokio::test]
async fn test_error_response_format() {
    let server = test_server("valid-project").await;
    let resp = server.get("/requirements/NONEXISTENT").await;
    assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
    let body = resp.json::<serde_json::Value>();
    // Must have "error" and "message" fields per the API Error schema.
    assert!(body["error"].is_string());
    assert!(body["message"].is_string());
}
