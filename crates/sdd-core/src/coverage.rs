use crate::models::{
    Annotation, AnnotationStats, AnnotationType, CoverageStatus, RawRequirement, RawTask,
    Requirement, RequirementStats, ScanResult, Stats, Task, TaskStats,
};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// @req SCS-COV-001
/// @req SCS-COV-002
/// @req SCS-COV-003
/// @req SCS-COV-004
///
/// Ties everything together: computes per-requirement coverage status, detects
/// orphans, maps tasks, and produces aggregate statistics.
pub fn compute_coverage(
    requirements: &[RawRequirement],
    tasks: &[RawTask],
    annotations: &[Annotation],
) -> ScanResult {
    let req_ids: HashSet<&str> = requirements.iter().map(|r| r.id.as_str()).collect();

    // Partition annotations into valid and orphan.
    let (valid_annotations, orphan_annotations): (Vec<_>, Vec<_>) = annotations
        .iter()
        .cloned()
        .partition(|a| req_ids.contains(a.req_id.as_str()));

    // Map and partition tasks.
    let all_mapped_tasks: Vec<Task> = tasks.iter().map(task_from_raw).collect();
    let (valid_tasks, orphan_tasks): (Vec<_>, Vec<_>) = all_mapped_tasks
        .into_iter()
        .partition(|t| req_ids.contains(t.requirement_id.as_str()));

    // Index valid annotations by requirement ID.
    let mut impl_by_req: HashMap<&str, usize> = HashMap::new();
    let mut test_by_req: HashMap<&str, usize> = HashMap::new();
    for ann in &valid_annotations {
        let counter = match ann.annotation_type {
            AnnotationType::Impl => impl_by_req.entry(ann.req_id.as_str()).or_default(),
            AnnotationType::Test => test_by_req.entry(ann.req_id.as_str()).or_default(),
        };
        *counter += 1;
    }

    // Compute per-requirement coverage status.
    let requirements_out: Vec<Requirement> = requirements
        .iter()
        .map(|r| {
            let has_impl = impl_by_req.contains_key(r.id.as_str());
            let has_test = test_by_req.contains_key(r.id.as_str());
            let status = match (has_impl, has_test) {
                (true, true) => CoverageStatus::Covered,
                (true, false) => CoverageStatus::Partial,
                _ => CoverageStatus::Missing,
            };
            Requirement {
                id: r.id.clone(),
                req_type: extract_type_from_id(&r.id),
                title: r.title.clone(),
                description: r.description.clone(),
                status,
                created_at: r.created_at.clone(),
                updated_at: r.updated_at.clone(),
            }
        })
        .collect();

    let last_scan_at = chrono::Utc::now().to_rfc3339();
    let stats = compute_stats(
        &requirements_out,
        &valid_annotations,
        &orphan_annotations,
        &valid_tasks,
        &orphan_tasks,
        &last_scan_at,
    );

    ScanResult {
        requirements: requirements_out,
        annotations: valid_annotations,
        tasks: valid_tasks,
        orphan_annotations,
        orphan_tasks,
        stats,
    }
}

/// @req SCS-COV-004
///
/// Extracts the type prefix from a requirement ID.
/// `SCS-SCAN-001` → `SCS`, `FR-TEST-001` → `FR`.
pub fn extract_type_from_id(id: &str) -> String {
    id.split('-').next().unwrap_or(id).to_string()
}

/// @req SCS-COV-004
///
/// Computes aggregate statistics from the scan artifacts.
pub fn compute_stats(
    requirements: &[Requirement],
    annotations: &[Annotation],
    orphan_annotations: &[Annotation],
    tasks: &[Task],
    orphan_tasks: &[Task],
    last_scan_at: &str,
) -> Stats {
    // Requirements stats.
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut by_status: HashMap<String, usize> = HashMap::new();
    for req in requirements {
        *by_type.entry(req.req_type.clone()).or_default() += 1;
        *by_status.entry(req.status.to_string()).or_default() += 1;
    }
    let covered_count = by_status.get("covered").copied().unwrap_or(0);
    let coverage = if requirements.is_empty() {
        0.0
    } else {
        (covered_count as f64 / requirements.len() as f64) * 100.0
    };

    // Annotation stats (total = valid only; orphans counted separately).
    let impl_count = annotations
        .iter()
        .filter(|a| a.annotation_type == AnnotationType::Impl)
        .count();
    let test_count = annotations
        .iter()
        .filter(|a| a.annotation_type == AnnotationType::Test)
        .count();

    // Task stats (total includes orphans).
    let total_tasks = tasks.len() + orphan_tasks.len();
    let mut task_by_status: HashMap<String, usize> = HashMap::new();
    for task in tasks.iter().chain(orphan_tasks.iter()) {
        *task_by_status.entry(task.status.to_string()).or_default() += 1;
    }

    Stats {
        requirements: RequirementStats {
            total: requirements.len(),
            by_type,
            by_status,
        },
        annotations: AnnotationStats {
            total: annotations.len(),
            impl_count,
            test: test_count,
            orphans: orphan_annotations.len(),
        },
        tasks: TaskStats {
            total: total_tasks,
            by_status: task_by_status,
            orphans: orphan_tasks.len(),
        },
        coverage,
        last_scan_at: last_scan_at.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn task_from_raw(raw: &RawTask) -> Task {
    Task {
        id: raw.id.clone(),
        requirement_id: raw.requirement_id.clone(),
        title: raw.title.clone(),
        status: raw.status.clone(),
        assignee: raw.assignee.clone(),
        created_at: raw.created_at.clone(),
        updated_at: raw.updated_at.clone(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_requirements, parse_tasks};
    use crate::scanner::scan_directory;

    fn fixture(rel: &str) -> std::path::PathBuf {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        root.join("fixtures").join(rel)
    }

    // @req SCS-COV-001
    #[test]
    fn test_coverage_calculation() {
        let base = fixture("valid-project");
        let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
        let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
        let annotations = scan_directory(&base).unwrap();

        let result = compute_coverage(&reqs, &tasks, &annotations);

        let find = |id: &str| {
            result
                .requirements
                .iter()
                .find(|r| r.id == id)
                .unwrap()
                .status
                .clone()
        };

        assert_eq!(find("FR-TEST-001"), CoverageStatus::Covered);
        assert_eq!(find("FR-TEST-002"), CoverageStatus::Partial);
        assert_eq!(find("FR-TEST-003"), CoverageStatus::Missing);
    }

    // @req SCS-COV-002
    #[test]
    fn test_orphan_annotation_detection() {
        let base = fixture("orphans-project");
        let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
        let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
        let annotations = scan_directory(&base).unwrap();

        let result = compute_coverage(&reqs, &tasks, &annotations);

        assert_eq!(result.orphan_annotations.len(), 1);
        assert_eq!(result.orphan_annotations[0].req_id, "NONEXISTENT-001");
    }

    // @req SCS-COV-003
    #[test]
    fn test_orphan_task_detection() {
        let base = fixture("orphans-project");
        let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
        let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
        let annotations = scan_directory(&base).unwrap();

        let result = compute_coverage(&reqs, &tasks, &annotations);

        assert_eq!(result.orphan_tasks.len(), 1);
        assert_eq!(result.orphan_tasks[0].requirement_id, "NONEXISTENT-001");
    }

    // @req SCS-COV-004
    #[test]
    fn test_stats_computation() {
        let base = fixture("valid-project");
        let reqs = parse_requirements(&base.join("requirements.yaml")).unwrap();
        let tasks = parse_tasks(&base.join("tasks.yaml")).unwrap();
        let annotations = scan_directory(&base).unwrap();

        let result = compute_coverage(&reqs, &tasks, &annotations);
        let stats = &result.stats;

        assert_eq!(stats.requirements.total, 3);
        assert_eq!(
            stats
                .requirements
                .by_status
                .get("covered")
                .copied()
                .unwrap_or(0),
            1
        );
        assert_eq!(
            stats
                .requirements
                .by_status
                .get("partial")
                .copied()
                .unwrap_or(0),
            1
        );
        assert_eq!(
            stats
                .requirements
                .by_status
                .get("missing")
                .copied()
                .unwrap_or(0),
            1
        );

        // FR-TEST-001 covered → 1/3 = 33.33%
        assert!((stats.coverage - 33.333_333).abs() < 0.001);

        // annotations: 2 impl (src/main.rs), 1 test (tests/test_main.rs)
        assert_eq!(stats.annotations.impl_count, 2);
        assert_eq!(stats.annotations.test, 1);
        assert_eq!(stats.annotations.total, 3);
        assert_eq!(stats.annotations.orphans, 0);

        // tasks: 2 total, 0 orphans
        assert_eq!(stats.tasks.total, 2);
        assert_eq!(stats.tasks.orphans, 0);
    }

    // @req SCS-COV-004
    #[test]
    fn test_extract_type_from_id() {
        assert_eq!(extract_type_from_id("SCS-SCAN-001"), "SCS");
        assert_eq!(extract_type_from_id("FR-TEST-001"), "FR");
        assert_eq!(extract_type_from_id("AR-001"), "AR");
        assert_eq!(extract_type_from_id("SINGLE"), "SINGLE");
    }

    // @req SCS-COV-001
    #[test]
    fn test_empty_requirements_zero_coverage() {
        let result = compute_coverage(&[], &[], &[]);
        assert_eq!(result.stats.coverage, 0.0);
        assert_eq!(result.stats.requirements.total, 0);
    }
}
