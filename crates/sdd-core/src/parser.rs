use crate::error::{CoreError, Result};
use crate::models::{RawRequirement, RawTask};
use serde::Deserialize;
use std::path::Path;

// ---------------------------------------------------------------------------
// File-level wrapper types (private — used only for deserialization)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RequirementsFile {
    requirements: Vec<RawRequirement>,
}

#[derive(Deserialize)]
struct TasksFile {
    tasks: Vec<RawTask>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// @req SCS-PARSE-001
///
/// Parses `requirements.yaml` at `path`, validating that every entry contains
/// the mandatory fields and valid ISO 8601 timestamps.
pub fn parse_requirements(path: &Path) -> Result<Vec<RawRequirement>> {
    let content = std::fs::read_to_string(path).map_err(|e| CoreError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    let file: RequirementsFile = serde_yaml::from_str(&content).map_err(|e| {
        let line = e.location().map(|loc| loc.line());
        CoreError::Yaml {
            line,
            message: e.to_string(),
        }
    })?;

    for req in &file.requirements {
        validate_requirement(req)?;
    }

    Ok(file.requirements)
}

/// @req SCS-PARSE-002
///
/// Parses `tasks.yaml` at `path`. If the file does not exist, returns an empty
/// list with a warning log — `tasks.yaml` is optional.
pub fn parse_tasks(path: &Path) -> Result<Vec<RawTask>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!(
                "tasks.yaml not found at '{}', treating as empty",
                path.display()
            );
            return Ok(Vec::new());
        }
        Err(e) => {
            return Err(CoreError::Io {
                path: path.display().to_string(),
                source: e,
            })
        }
    };

    let file: TasksFile = serde_yaml::from_str(&content).map_err(|e| {
        let line = e.location().map(|loc| loc.line());
        CoreError::Yaml {
            line,
            message: e.to_string(),
        }
    })?;

    for task in &file.tasks {
        validate_task(task)?;
    }

    Ok(file.tasks)
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_requirement(req: &RawRequirement) -> Result<()> {
    validate_timestamp(&req.created_at, "createdAt", &req.id)?;
    validate_timestamp(&req.updated_at, "updatedAt", &req.id)?;
    Ok(())
}

fn validate_task(task: &RawTask) -> Result<()> {
    validate_timestamp(&task.created_at, "createdAt", &task.id)?;
    validate_timestamp(&task.updated_at, "updatedAt", &task.id)?;
    Ok(())
}

fn validate_timestamp(value: &str, field: &str, id: &str) -> Result<()> {
    chrono::DateTime::parse_from_rfc3339(value).map_err(|_| CoreError::Validation {
        id: id.to_string(),
        message: format!("'{field}' must be a valid ISO 8601 date-time string, got '{value}'"),
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(rel: &str) -> std::path::PathBuf {
        // CARGO_MANIFEST_DIR = crates/sdd-core  →  ../.. = workspace root
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        root.join("fixtures").join(rel)
    }

    // @req SCS-PARSE-001
    #[test]
    fn test_parse_valid_requirements() {
        let path = fixture("valid-project/requirements.yaml");
        let reqs = parse_requirements(&path).expect("should parse");
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].id, "FR-TEST-001");
        assert_eq!(reqs[1].id, "FR-TEST-002");
        assert_eq!(reqs[2].id, "FR-TEST-003");
    }

    // @req SCS-PARSE-001
    #[test]
    fn test_parse_malformed_yaml_returns_error() {
        let path = fixture("malformed-yaml/requirements.yaml");
        let result = parse_requirements(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, CoreError::Yaml { .. }),
            "expected Yaml error, got: {err}"
        );
    }

    // @req SCS-PARSE-001
    #[test]
    fn test_parse_missing_requirements_returns_error() {
        let path = std::path::Path::new("/nonexistent/requirements.yaml");
        let result = parse_requirements(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::Io { .. }));
    }

    // @req SCS-PARSE-002
    #[test]
    fn test_parse_valid_tasks() {
        let path = fixture("valid-project/tasks.yaml");
        let tasks = parse_tasks(&path).expect("should parse");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "TASK-001");
        assert_eq!(tasks[0].requirement_id, "FR-TEST-001");
    }

    // @req SCS-PARSE-002
    #[test]
    fn test_parse_missing_tasks_returns_empty() {
        let path = std::path::Path::new("/nonexistent/tasks.yaml");
        let tasks = parse_tasks(path).expect("missing tasks.yaml should return empty vec");
        assert!(tasks.is_empty());
    }

    // @req SCS-ERR-001
    #[test]
    fn test_missing_requirements_file_error() {
        let path = std::path::Path::new("/no/such/requirements.yaml");
        let err = parse_requirements(path).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("/no/such/requirements.yaml"),
            "error should mention path: {msg}"
        );
    }
}
