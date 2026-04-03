use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Raw YAML deserialization types
// ---------------------------------------------------------------------------

/// Raw requirement entry deserialized directly from requirements.yaml.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawRequirement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Raw task entry deserialized directly from tasks.yaml.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTask {
    pub id: String,
    pub requirement_id: String,
    pub title: String,
    pub status: TaskStatus,
    pub assignee: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Enriched API types
// ---------------------------------------------------------------------------

/// Requirement enriched with a computed coverage status and type prefix.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub id: String,
    #[serde(rename = "type")]
    pub req_type: String,
    pub title: String,
    pub description: String,
    pub status: CoverageStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// Requirement with its full traceability chain (annotations + tasks).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementDetail {
    #[serde(flatten)]
    pub requirement: Requirement,
    pub annotations: Vec<Annotation>,
    pub tasks: Vec<Task>,
}

/// Task mapped from a RawTask with timestamps passed through unchanged.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub requirement_id: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A single `@req` annotation found in source code.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub file: String,
    pub line: usize,
    pub req_id: String,
    #[serde(rename = "type")]
    pub annotation_type: AnnotationType,
    pub snippet: String,
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Whether an annotation lives in implementation code or test code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnnotationType {
    #[serde(rename = "impl")]
    Impl,
    #[serde(rename = "test")]
    Test,
}

impl fmt::Display for AnnotationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnnotationType::Impl => write!(f, "impl"),
            AnnotationType::Test => write!(f, "test"),
        }
    }
}

impl FromStr for AnnotationType {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "impl" => Ok(AnnotationType::Impl),
            "test" => Ok(AnnotationType::Test),
            other => Err(format!(
                "invalid annotation type: '{other}'; expected 'impl' or 'test'"
            )),
        }
    }
}

/// Computed coverage status of a requirement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Covered,
    Partial,
    Missing,
}

impl fmt::Display for CoverageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoverageStatus::Covered => write!(f, "covered"),
            CoverageStatus::Partial => write!(f, "partial"),
            CoverageStatus::Missing => write!(f, "missing"),
        }
    }
}

impl FromStr for CoverageStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "covered" => Ok(CoverageStatus::Covered),
            "partial" => Ok(CoverageStatus::Partial),
            "missing" => Ok(CoverageStatus::Missing),
            other => Err(format!(
                "invalid coverage status: '{other}'; expected 'covered', 'partial', or 'missing'"
            )),
        }
    }
}

/// Work-item lifecycle status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Done,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Open => write!(f, "open"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Done => write!(f, "done"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "open" => Ok(TaskStatus::Open),
            "in_progress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            other => Err(format!(
                "invalid task status: '{other}'; expected 'open', 'in_progress', or 'done'"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Scan result & statistics
// ---------------------------------------------------------------------------

/// The complete output produced by a single scan run.
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Requirements enriched with coverage status.
    pub requirements: Vec<Requirement>,
    /// Annotations that reference a known requirement.
    pub annotations: Vec<Annotation>,
    /// Tasks that reference a known requirement.
    pub tasks: Vec<Task>,
    /// Annotations whose `reqId` is not in `requirements.yaml`.
    pub orphan_annotations: Vec<Annotation>,
    /// Tasks whose `requirementId` is not in `requirements.yaml`.
    pub orphan_tasks: Vec<Task>,
    /// Aggregate statistics for this scan.
    pub stats: Stats,
}

/// Project-wide aggregate statistics matching the API `Stats` schema.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub requirements: RequirementStats,
    pub annotations: AnnotationStats,
    pub tasks: TaskStats,
    /// Percentage of fully covered requirements (0–100).
    pub coverage: f64,
    pub last_scan_at: String,
}

/// Requirement breakdown within `Stats`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementStats {
    pub total: usize,
    pub by_type: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
}

/// Annotation breakdown within `Stats`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationStats {
    pub total: usize,
    #[serde(rename = "impl")]
    pub impl_count: usize,
    pub test: usize,
    pub orphans: usize,
}

/// Task breakdown within `Stats`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStats {
    pub total: usize,
    pub by_status: HashMap<String, usize>,
    pub orphans: usize,
}

// ---------------------------------------------------------------------------
// Scan lifecycle
// ---------------------------------------------------------------------------

/// Lifecycle state of the background scan.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanState {
    Idle,
    Scanning,
    Completed,
    Failed,
}

/// Full scan-status object returned by `GET /scan` and `POST /scan`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub status: ScanState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}
