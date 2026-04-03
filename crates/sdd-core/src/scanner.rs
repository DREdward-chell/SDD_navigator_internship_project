use crate::error::{CoreError, Result};
use crate::models::{Annotation, AnnotationType};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Regex — compiled once at first use
// ---------------------------------------------------------------------------

fn annotation_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Safety: the pattern is a compile-time constant and is always valid.
    RE.get_or_init(|| {
        Regex::new(r"(?://|#)\s*@req\s+([\w-]+)")
            .expect("annotation regex is a valid compile-time constant")
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Returns the file extensions that the scanner processes.
pub fn supported_extensions() -> &'static [&'static str] {
    &["rs", "ts", "js", "py", "dart", "go"]
}

/// @req SCS-SCAN-002
///
/// Returns `true` if `path` should be treated as a test file based on its
/// path components and filename patterns.
pub fn is_test_file(path: &Path) -> bool {
    // Check every path component for a "tests" directory.
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            if name == "tests" {
                return true;
            }
        }
    }

    // Check filename patterns.
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if file_name.starts_with("test_") {
            return true;
        }
        if file_name.contains("_test.") {
            return true;
        }
        if file_name.contains(".test.") {
            return true;
        }
    }

    false
}

/// @req SCS-SCAN-001
///
/// Recursively scans `root` for `@req` annotations in all supported source
/// files. Hidden directories, `target/`, and `node_modules/` are skipped.
/// Permission errors on individual files are logged and skipped.
pub fn scan_directory(root: &Path) -> Result<Vec<Annotation>> {
    if !root.exists() {
        tracing::warn!(
            "Source directory does not exist: '{}', returning empty results",
            root.display()
        );
        return Ok(Vec::new());
    }

    let exts = supported_extensions();
    let mut annotations = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_skipped_dir(e))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Permission error while walking directory: {e}");
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !exts.contains(&ext) {
            continue;
        }

        match scan_file(path, root) {
            Ok(file_annotations) => annotations.extend(file_annotations),
            Err(e) => {
                tracing::warn!("Error scanning '{}': {e}", path.display());
            }
        }
    }

    Ok(annotations)
}

/// @req SCS-SCAN-001
///
/// Reads `path`, finds all `@req` annotations, extracts a snippet (the
/// annotation line plus up to 2 following lines), and classifies each as
/// `impl` or `test` based on the relative path from `relative_to`.
pub fn scan_file(path: &Path, relative_to: &Path) -> Result<Vec<Annotation>> {
    let content = std::fs::read_to_string(path).map_err(|e| CoreError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    let rel_path = path
        .strip_prefix(relative_to)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();

    let rel_path_obj = Path::new(&rel_path);
    let annotation_type = if is_test_file(rel_path_obj) {
        AnnotationType::Test
    } else {
        AnnotationType::Impl
    };

    let re = annotation_regex();
    let lines: Vec<&str> = content.lines().collect();
    let mut annotations = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = re.captures(line) {
            let req_id = caps[1].to_string();

            // Snippet: annotation line + up to 2 lines below it.
            let end = (i + 3).min(lines.len());
            let snippet = lines[i..end].join("\n");

            annotations.push(Annotation {
                file: rel_path.clone(),
                line: i + 1, // 1-indexed
                req_id,
                annotation_type: annotation_type.clone(),
                snippet,
            });
        }
    }

    Ok(annotations)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn is_skipped_dir(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || name == "target" || name == "node_modules"
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(rel: &str) -> std::path::PathBuf {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        root.join("fixtures").join(rel)
    }

    // @req SCS-SCAN-001
    #[test]
    fn test_scan_rust_file() {
        let base = fixture("valid-project");
        let file = base.join("src/main.rs");
        let annotations = scan_file(&file, &base).expect("should scan");
        assert_eq!(annotations.len(), 2);
        assert!(annotations
            .iter()
            .all(|a| a.annotation_type == AnnotationType::Impl));
        let ids: Vec<&str> = annotations.iter().map(|a| a.req_id.as_str()).collect();
        assert!(ids.contains(&"FR-TEST-001"));
        assert!(ids.contains(&"FR-TEST-002"));
    }

    // @req SCS-SCAN-001
    #[test]
    fn test_scan_python_file() {
        let base = fixture("mixed-languages");
        let file = base.join("src/main.py");
        let annotations = scan_file(&file, &base).expect("should scan");
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].req_id, "FR-MIX-003");
        assert_eq!(annotations[0].annotation_type, AnnotationType::Impl);
    }

    // @req SCS-SCAN-001
    #[test]
    fn test_scan_mixed_languages() {
        let base = fixture("mixed-languages");
        let annotations = scan_directory(&base).expect("should scan");
        let ids: std::collections::HashSet<&str> =
            annotations.iter().map(|a| a.req_id.as_str()).collect();
        assert!(ids.contains("FR-MIX-001"), "Rust not found");
        assert!(ids.contains("FR-MIX-002"), "TypeScript not found");
        assert!(ids.contains("FR-MIX-003"), "Python not found");
        assert!(ids.contains("FR-MIX-004"), "JavaScript not found");
        assert!(ids.contains("FR-MIX-005"), "Go not found");
        assert!(ids.contains("FR-MIX-006"), "Dart not found");
    }

    // @req SCS-SCAN-002
    #[test]
    fn test_classify_test_file_by_directory() {
        assert!(is_test_file(Path::new("tests/test_main.rs")));
        assert!(is_test_file(Path::new("src/tests/foo.rs")));
    }

    // @req SCS-SCAN-002
    #[test]
    fn test_classify_test_file_by_name() {
        assert!(is_test_file(Path::new("test_parser.py")));
        assert!(is_test_file(Path::new("parser_test.rs")));
        assert!(is_test_file(Path::new("parser.test.ts")));
    }

    // @req SCS-SCAN-002
    #[test]
    fn test_classify_impl_file() {
        assert!(!is_test_file(Path::new("src/main.rs")));
        assert!(!is_test_file(Path::new("src/handler.ts")));
        assert!(!is_test_file(Path::new("lib/utils.py")));
    }

    // @req SCS-SCAN-002
    #[test]
    fn test_test_annotation_type_from_tests_dir() {
        let base = fixture("valid-project");
        let file = base.join("tests/test_main.rs");
        let annotations = scan_file(&file, &base).expect("should scan");
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].annotation_type, AnnotationType::Test);
    }

    // @req SCS-SCAN-001
    #[test]
    fn test_snippet_captures_following_lines() {
        let base = fixture("valid-project");
        let file = base.join("src/main.rs");
        let annotations = scan_file(&file, &base).expect("should scan");
        // Each snippet should contain the @req line + following content.
        for ann in &annotations {
            assert!(ann.snippet.contains("@req"));
        }
    }

    // @req SCS-ERR-001
    #[test]
    fn test_empty_source_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let result = scan_directory(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // @req SCS-ERR-001
    #[test]
    fn test_nonexistent_directory_returns_empty() {
        let result = scan_directory(Path::new("/nonexistent/source/dir"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // @req SCS-SCAN-001
    #[test]
    fn test_nested_directories_scanned() {
        let base = fixture("nested-dirs");
        let annotations = scan_directory(&base).expect("should scan");
        assert!(!annotations.is_empty());
        assert!(annotations.iter().any(|a| a.req_id == "FR-NEST-001"));
    }
}
