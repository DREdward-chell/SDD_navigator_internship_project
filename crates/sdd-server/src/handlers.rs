use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use sdd_core::{
    compute_coverage, parse_requirements, parse_tasks, scan_directory, AnnotationType,
    CoverageStatus, RequirementDetail, ScanState, ScanStatus, TaskStatus,
};
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;

use crate::{errors::ApiError, state::SharedState};

// ---------------------------------------------------------------------------
// Healthcheck
// ---------------------------------------------------------------------------

/// @req SCS-API-001
pub async fn healthcheck() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// @req SCS-API-002
pub async fn get_stats(State(state): State<SharedState>) -> impl IntoResponse {
    let guard = state.read().await;
    match guard.scan_result.as_ref() {
        Some(result) => Json(result.stats.clone()).into_response(),
        None => ApiError::service_unavailable(
            "No scan result available yet. A scan may be in progress.",
        )
        .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Requirements
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Default)]
pub struct RequirementsQuery {
    #[serde(rename = "type")]
    pub req_type: Option<String>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

/// @req SCS-API-003
pub async fn list_requirements(
    State(state): State<SharedState>,
    Query(query): Query<RequirementsQuery>,
) -> impl IntoResponse {
    let guard = state.read().await;
    let scan_result = match guard.scan_result.as_ref() {
        Some(r) => r,
        None => {
            return ApiError::service_unavailable(
                "No scan result available yet. A scan may be in progress.",
            )
            .into_response()
        }
    };

    // Validate query parameters.
    let status_filter = match &query.status {
        Some(s) => match CoverageStatus::from_str(s) {
            Ok(v) => Some(v),
            Err(_) => {
                return ApiError::bad_request(format!(
                    "invalid 'status' value: '{s}'; expected 'covered', 'partial', or 'missing'"
                ))
                .into_response()
            }
        },
        None => None,
    };

    let sort_field = query.sort.as_deref().unwrap_or("id");
    let order = query.order.as_deref().unwrap_or("asc");

    if !matches!(sort_field, "id" | "updatedAt") {
        return ApiError::bad_request(format!(
            "invalid 'sort' value: '{sort_field}'; expected 'id' or 'updatedAt'"
        ))
        .into_response();
    }
    if !matches!(order, "asc" | "desc") {
        return ApiError::bad_request(format!(
            "invalid 'order' value: '{order}'; expected 'asc' or 'desc'"
        ))
        .into_response();
    }

    // Filter.
    let mut filtered: Vec<_> = scan_result
        .requirements
        .iter()
        .filter(|r| {
            if let Some(t) = &query.req_type {
                if &r.req_type != t {
                    return false;
                }
            }
            if let Some(s) = &status_filter {
                if &r.status != s {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    // Sort.
    filtered.sort_by(|a, b| {
        let cmp = if sort_field == "updatedAt" {
            a.updated_at.cmp(&b.updated_at)
        } else {
            a.id.cmp(&b.id)
        };
        if order == "desc" {
            cmp.reverse()
        } else {
            cmp
        }
    });

    Json(filtered).into_response()
}

/// @req SCS-API-004
pub async fn get_requirement(
    State(state): State<SharedState>,
    Path(requirement_id): Path<String>,
) -> impl IntoResponse {
    let guard = state.read().await;
    let scan_result = match guard.scan_result.as_ref() {
        Some(r) => r,
        None => {
            return ApiError::service_unavailable(
                "No scan result available yet. A scan may be in progress.",
            )
            .into_response()
        }
    };

    let requirement = match scan_result
        .requirements
        .iter()
        .find(|r| r.id == requirement_id)
    {
        Some(r) => r.clone(),
        None => {
            return ApiError::not_found(format!("Requirement '{requirement_id}' not found"))
                .into_response()
        }
    };

    let annotations = scan_result
        .annotations
        .iter()
        .filter(|a| a.req_id == requirement_id)
        .cloned()
        .collect();

    let tasks = scan_result
        .tasks
        .iter()
        .filter(|t| t.requirement_id == requirement_id)
        .cloned()
        .collect();

    let detail = RequirementDetail {
        requirement,
        annotations,
        tasks,
    };

    Json(detail).into_response()
}

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Default)]
pub struct AnnotationsQuery {
    #[serde(rename = "type")]
    pub ann_type: Option<String>,
    pub orphans: Option<String>,
}

/// @req SCS-API-005
pub async fn list_annotations(
    State(state): State<SharedState>,
    Query(query): Query<AnnotationsQuery>,
) -> impl IntoResponse {
    let guard = state.read().await;
    let scan_result = match guard.scan_result.as_ref() {
        Some(r) => r,
        None => {
            return ApiError::service_unavailable(
                "No scan result available yet. A scan may be in progress.",
            )
            .into_response()
        }
    };

    // Validate orphans param.
    let only_orphans = match query.orphans.as_deref() {
        None | Some("false") => false,
        Some("true") => true,
        Some(v) => {
            return ApiError::bad_request(format!(
                "invalid 'orphans' value: '{v}'; expected 'true' or 'false'"
            ))
            .into_response()
        }
    };

    // Validate type filter.
    let type_filter = match &query.ann_type {
        Some(s) => match AnnotationType::from_str(s) {
            Ok(v) => Some(v),
            Err(_) => {
                return ApiError::bad_request(format!(
                    "invalid 'type' value: '{s}'; expected 'impl' or 'test'"
                ))
                .into_response()
            }
        },
        None => None,
    };

    // Select source: orphans only vs all.
    let mut annotations: Vec<_> = if only_orphans {
        scan_result.orphan_annotations.to_vec()
    } else {
        scan_result
            .annotations
            .iter()
            .chain(scan_result.orphan_annotations.iter())
            .cloned()
            .collect()
    };

    // Apply type filter.
    if let Some(t) = &type_filter {
        annotations.retain(|a| &a.annotation_type == t);
    }

    // Sort by (file, line) per spec.
    annotations.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    Json(annotations).into_response()
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Default)]
pub struct TasksQuery {
    pub status: Option<String>,
    pub orphans: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

/// @req SCS-API-006
pub async fn list_tasks(
    State(state): State<SharedState>,
    Query(query): Query<TasksQuery>,
) -> impl IntoResponse {
    let guard = state.read().await;
    let scan_result = match guard.scan_result.as_ref() {
        Some(r) => r,
        None => {
            return ApiError::service_unavailable(
                "No scan result available yet. A scan may be in progress.",
            )
            .into_response()
        }
    };

    // Validate params.
    let status_filter = match &query.status {
        Some(s) => match TaskStatus::from_str(s) {
            Ok(v) => Some(v),
            Err(_) => {
                return ApiError::bad_request(format!(
                    "invalid 'status' value: '{s}'; expected 'open', 'in_progress', or 'done'"
                ))
                .into_response()
            }
        },
        None => None,
    };

    let only_orphans = match query.orphans.as_deref() {
        None | Some("false") => false,
        Some("true") => true,
        Some(v) => {
            return ApiError::bad_request(format!(
                "invalid 'orphans' value: '{v}'; expected 'true' or 'false'"
            ))
            .into_response()
        }
    };

    let sort_field = query.sort.as_deref().unwrap_or("id");
    let order = query.order.as_deref().unwrap_or("asc");

    if !matches!(sort_field, "id" | "updatedAt") {
        return ApiError::bad_request(format!(
            "invalid 'sort' value: '{sort_field}'; expected 'id' or 'updatedAt'"
        ))
        .into_response();
    }
    if !matches!(order, "asc" | "desc") {
        return ApiError::bad_request(format!(
            "invalid 'order' value: '{order}'; expected 'asc' or 'desc'"
        ))
        .into_response();
    }

    // Select source.
    let mut tasks: Vec<_> = if only_orphans {
        scan_result.orphan_tasks.to_vec()
    } else {
        scan_result
            .tasks
            .iter()
            .chain(scan_result.orphan_tasks.iter())
            .cloned()
            .collect()
    };

    // Apply status filter.
    if let Some(s) = &status_filter {
        tasks.retain(|t| &t.status == s);
    }

    // Sort.
    tasks.sort_by(|a, b| {
        let cmp = if sort_field == "updatedAt" {
            a.updated_at.cmp(&b.updated_at)
        } else {
            a.id.cmp(&b.id)
        };
        if order == "desc" {
            cmp.reverse()
        } else {
            cmp
        }
    });

    Json(tasks).into_response()
}

// ---------------------------------------------------------------------------
// Scan lifecycle
// ---------------------------------------------------------------------------

/// @req SCS-API-007
pub async fn get_scan_status(State(state): State<SharedState>) -> impl IntoResponse {
    let guard = state.read().await;
    Json(guard.scan_status.clone()).into_response()
}

/// @req SCS-API-007
pub async fn trigger_scan(State(state): State<SharedState>) -> impl IntoResponse {
    let started_at = start_scan(&state).await;
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "scanning",
            "startedAt": started_at,
        })),
    )
}

// ---------------------------------------------------------------------------
// Shared scan helper
// ---------------------------------------------------------------------------

/// Aborts any running scan, sets status to Scanning, spawns a new scan task,
/// and returns the `startedAt` timestamp.
pub async fn start_scan(state: &SharedState) -> String {
    let started_at = Utc::now().to_rfc3339();

    {
        let mut guard = state.write().await;
        if let Some(handle) = guard.current_scan_handle.take() {
            handle.abort();
        }
        guard.scan_status = ScanStatus {
            status: ScanState::Scanning,
            started_at: Some(started_at.clone()),
            completed_at: None,
            duration: None,
        };
    }

    let state_clone = Arc::clone(state);
    let handle = tokio::spawn(run_scan(state_clone));

    {
        let mut guard = state.write().await;
        guard.current_scan_handle = Some(handle.abort_handle());
    }

    started_at
}

/// @req SCS-API-007
///
/// Background task: performs the full scan and updates shared state on completion.
pub(crate) async fn run_scan(state: SharedState) {
    let scan_start = std::time::Instant::now();
    let config = {
        let guard = state.read().await;
        guard.config.clone()
    };

    let result = tokio::task::spawn_blocking(move || {
        let reqs = parse_requirements(&config.requirements_path)?;
        let tasks = parse_tasks(&config.tasks_path)?;
        let annotations = scan_directory(&config.source_path)?;
        Ok::<_, sdd_core::CoreError>(compute_coverage(&reqs, &tasks, &annotations))
    })
    .await;

    let duration_ms = scan_start.elapsed().as_millis() as u64;
    let completed_at = Utc::now().to_rfc3339();

    let mut guard = state.write().await;
    match result {
        Ok(Ok(scan_result)) => {
            guard.scan_status.status = ScanState::Completed;
            guard.scan_status.completed_at = Some(completed_at);
            guard.scan_status.duration = Some(duration_ms);
            guard.scan_result = Some(scan_result);
        }
        Ok(Err(core_err)) => {
            tracing::error!("Scan failed: {core_err}");
            guard.scan_status.status = ScanState::Failed;
        }
        Err(join_err) => {
            if !join_err.is_cancelled() {
                tracing::error!("Scan task panicked: {join_err}");
                guard.scan_status.status = ScanState::Failed;
            }
            // Cancelled: the new trigger_scan has already updated status.
        }
    }
    guard.current_scan_handle = None;
}
