# PLAN.md — SDD Coverage Service Build Plan

## How to Use This Plan

Execute one phase at a time. After each phase:
1. Run the verification checks listed in that phase.
2. Ensure all checks pass.
3. Update README.md as specified.
4. Commit with the exact message format shown.

Read `CLAUDE.md` for full architectural context, coding conventions, and anti-patterns.

**IMPORTANT**: Before writing any `Cargo.toml`, look up the latest stable version of each dependency on crates.io. Do NOT guess version numbers.

---

## Phase 1: Specification & Project Scaffold

### Goal
Set up the Cargo workspace, write `requirements.yaml` and `tasks.yaml` for the project itself, create all `Cargo.toml` files with correct dependencies, create test fixtures, and establish the GitHub Actions workflow.

### What to Create

#### 1.1 — `requirements.yaml` (project root)

Define the service's own requirements. Every requirement has `id`, `title`, `description` (with MUST/SHOULD directive), `createdAt`, and `updatedAt` (ISO 8601). Use `SCS-*` prefix. Set `createdAt` to the time you write the entry; set `updatedAt` to the same initially, then update it whenever you modify the entry.

```yaml
requirements:
  - id: SCS-PARSE-001
    title: Parse requirements.yaml
    description: "Service MUST read requirements.yaml from a configurable path and validate that every entry contains id, title, description, createdAt, and updatedAt fields."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-PARSE-002
    title: Parse tasks.yaml
    description: "Service MUST read tasks.yaml from a configurable path and validate that every entry contains id, requirementId, title, status, createdAt, and updatedAt fields."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-SCAN-001
    title: Scan @req annotations
    description: "Scanner MUST find all @req annotations in .rs, .ts, .js, .py, .dart, .go files using language-appropriate comment syntax (// or #)."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-SCAN-002
    title: Classify annotations as impl or test
    description: "Scanner MUST classify each annotation as impl or test based on file path patterns: files in /tests/ directories or matching test_*, *_test.*, *.test.* are test; all others are impl."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-COV-001
    title: Calculate per-requirement coverage
    description: "Service MUST compute coverage status for each requirement: covered (has impl AND test annotations), partial (impl only), missing (no annotations)."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-COV-002
    title: Detect orphan annotations
    description: "Service MUST identify annotations whose reqId does not match any requirement in requirements.yaml."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-COV-003
    title: Detect orphan tasks
    description: "Service MUST identify tasks whose requirementId does not match any requirement in requirements.yaml."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-COV-004
    title: Compute project-level statistics
    description: "Service MUST compute aggregate stats: total requirements, counts by type and status, annotation counts (impl/test/orphan), task counts by status with orphan count, and overall coverage percentage."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-API-001
    title: GET /healthcheck
    description: "Service MUST return JSON with status, version (from Cargo.toml), and ISO 8601 timestamp."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-API-002
    title: GET /stats
    description: "Service MUST return project-wide statistics matching the Stats schema from the OpenAPI spec."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-API-003
    title: GET /requirements with filtering and sorting
    description: "Service MUST return requirements list supporting ?type, ?status, ?sort (id|updatedAt), and ?order (asc|desc) query parameters. Invalid parameter values MUST return 400."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-API-004
    title: GET /requirements/{requirementId}
    description: "Service MUST return a single requirement with all linked annotations and tasks. Non-existent ID MUST return 404 with Error schema."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-API-005
    title: GET /annotations with filtering
    description: "Service MUST return annotations list supporting ?type (impl|test) and ?orphans (true|false) query parameters."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-API-006
    title: GET /tasks with filtering and sorting
    description: "Service MUST return tasks list supporting ?status, ?orphans, ?sort, and ?order query parameters."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-API-007
    title: POST /scan and GET /scan
    description: "POST /scan MUST trigger a background re-scan and return 202. GET /scan MUST return current scan status. A POST during an active scan MUST cancel and restart."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-ERR-001
    title: Error handling
    description: "Service MUST handle missing files (clear error with path), malformed YAML (error with line number), empty source directories (warning, not crash), and permission errors (skip with warning). Service MUST NOT panic under any input."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-CLI-001
    title: CLI scan command
    description: "CLI binary MUST accept --requirements, --tasks, --source, and --strict flags. Without --strict, print summary and exit 0. With --strict, exit 1 if any requirement is missing or partial, or any orphan exists."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-SELF-001
    title: Self-hosting verification
    description: "Running the CLI with --strict against this repository's own codebase MUST pass: all requirements covered, zero orphans."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: SCS-DOCKER-001
    title: Docker containerization
    description: "Project MUST include a multi-stage Dockerfile producing an Alpine-based image with both server and CLI binaries."
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"
```

#### 1.2 — `tasks.yaml` (project root)

```yaml
tasks:
  - id: TASK-001
    requirementId: SCS-PARSE-001
    title: Implement YAML parser for requirements
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-002
    requirementId: SCS-PARSE-002
    title: Implement YAML parser for tasks
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-003
    requirementId: SCS-SCAN-001
    title: Build annotation scanner with multi-language support
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-004
    requirementId: SCS-SCAN-002
    title: Implement test file detection logic
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-005
    requirementId: SCS-COV-001
    title: Implement coverage calculation
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-006
    requirementId: SCS-COV-002
    title: Implement orphan annotation detection
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-007
    requirementId: SCS-COV-003
    title: Implement orphan task detection
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-008
    requirementId: SCS-COV-004
    title: Implement stats computation
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-009
    requirementId: SCS-API-001
    title: Implement /healthcheck endpoint
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-010
    requirementId: SCS-API-002
    title: Implement /stats endpoint
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-011
    requirementId: SCS-API-003
    title: Implement /requirements endpoint with filters
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-012
    requirementId: SCS-API-004
    title: Implement /requirements/{id} endpoint
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-013
    requirementId: SCS-API-005
    title: Implement /annotations endpoint
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-014
    requirementId: SCS-API-006
    title: Implement /tasks endpoint
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-015
    requirementId: SCS-API-007
    title: Implement /scan endpoints
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-016
    requirementId: SCS-ERR-001
    title: Implement error handling
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-017
    requirementId: SCS-CLI-001
    title: Build CLI binary
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-018
    requirementId: SCS-SELF-001
    title: Self-hosting verification
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"

  - id: TASK-019
    requirementId: SCS-DOCKER-001
    title: Create Dockerfile
    status: open
    createdAt: "2026-04-03T10:00:00Z"
    updatedAt: "2026-04-03T10:00:00Z"
```

#### 1.3 — Cargo Workspace

Root `Cargo.toml`:
```toml
[workspace]
members = ["crates/sdd-core", "crates/sdd-server", "crates/sdd-cli"]
resolver = "2"
```

Create each crate's `Cargo.toml` with dependencies as listed in `CLAUDE.md`. Check latest stable versions on crates.io before writing version numbers.

Create stub `lib.rs` / `main.rs` for each crate so the workspace compiles. Stubs should have minimal content — just enough to pass `cargo check`.

#### 1.4 — Test Fixtures

Create `fixtures/` at workspace root with these subdirectories:

**`fixtures/valid-project/`**: A small complete project with:
- `requirements.yaml` with 3 requirements (FR-TEST-001, FR-TEST-002, FR-TEST-003), each with createdAt and updatedAt timestamps
- `tasks.yaml` with 2 tasks, each with createdAt and updatedAt timestamps
- `src/main.rs` with `// @req FR-TEST-001` and `// @req FR-TEST-002` annotations
- `tests/test_main.rs` with `// @req FR-TEST-001` annotation
- Expected result: FR-TEST-001 = covered, FR-TEST-002 = partial, FR-TEST-003 = missing

**`fixtures/empty-project/`**: Only a `requirements.yaml` with 1 requirement (including timestamps), no source files, no tasks.yaml.

**`fixtures/malformed-yaml/`**: A `requirements.yaml` with invalid YAML syntax.

**`fixtures/mixed-languages/`**: Requirements + source files in .rs, .ts, .py, .js, .go, .dart — each with at least one `@req` annotation using the correct comment syntax.

**`fixtures/orphans-project/`**: Source files with `@req NONEXISTENT-001` annotations and tasks referencing nonexistent requirements.

**`fixtures/nested-dirs/`**: Deeply nested directory structure (`src/a/b/c/d/file.rs`) to test recursive scanning.

#### 1.5 — GitHub Actions Workflow

Create `.github/workflows/ci.yml`:
```yaml
name: CI

on:
  push:
    branches: [main, dev]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo registry & build
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Check formatting
        run: cargo fmt --all --check

      - name: Lint
        run: cargo clippy --workspace -- -D warnings

      - name: Run tests
        run: cargo test --workspace

      - name: Build release
        run: cargo build --workspace --release

      - name: Self-hosting scan
        run: ./scripts/scan.sh
        continue-on-error: ${{ github.ref != 'refs/heads/main' }}
```

#### 1.6 — Local Scan Script

Create `scripts/scan.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail

STRICT=true

for arg in "$@"; do
  case "$arg" in
    --no-strict) STRICT=false ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

if [ "$STRICT" = true ]; then
  ./target/release/sdd-coverage scan \
    --requirements requirements.yaml \
    --tasks tasks.yaml \
    --source . \
    --strict
else
  ./target/release/sdd-coverage scan \
    --requirements requirements.yaml \
    --tasks tasks.yaml \
    --source .
fi
```

Make it executable: `chmod +x scripts/scan.sh`.

The script assumes `cargo build --workspace --release` has already been run. It does NOT build — it only scans.

#### 1.6 — README.md (initial)

Write a README covering:
- Project name and one-line description
- Architecture diagram (text-based: the three crates and their relationship)
- Status: "Phase 1 — project scaffolded, no functionality yet"
- List of requirements (reference requirements.yaml)
- Build instructions: `cargo build --workspace`
- Placeholder sections for: Usage, API, Docker, Testing

#### 1.7 — .gitignore

Standard Rust .gitignore:
```
/target
*.swp
*.swo
.DS_Store
```

### Verification Before Commit
```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo check --workspace
```

### Commit
```
chore(scaffold): project structure, requirements spec, fixtures, CI workflow
```

(This commit defines all requirements; none are implemented yet. No requirement IDs in the message.)

---

## Phase 2: Scanner Core (`sdd-core`)

### Goal
Implement the core library: YAML parsing, annotation scanning, coverage computation, and statistics. All logic lives in `sdd-core`. No HTTP, no CLI — just the engine.

### What to Implement

#### 2.1 — `models.rs`
All shared data types. These structs serve both the scanner internals and the API responses.

Types to define:
- `RawRequirement` — deserialized from YAML (id, title, description, createdAt, updatedAt)
- `RawTask` — deserialized from YAML (id, requirementId, title, status, optional assignee, createdAt, updatedAt)
- `Requirement` — enriched with computed fields (type extracted from id, coverage status). Timestamps come directly from the YAML via `RawRequirement`.
- `RequirementDetail` — Requirement + Vec<Annotation> + Vec<Task>
- `Task` — mapped from RawTask, timestamps passed through from YAML.
- `Annotation` — file, line, reqId, type (impl/test), snippet
- `AnnotationType` — enum: Impl, Test
- `CoverageStatus` — enum: Covered, Partial, Missing
- `TaskStatus` — enum: Open, InProgress, Done
- `ScanResult` — the complete output of a scan (requirements, annotations, tasks, stats, orphan info)
- `Stats`, `RequirementStats`, `AnnotationStats`, `TaskStats` — matching the API spec schemas
- `ScanStatus` — enum: Idle, Scanning, Completed, Failed (with timing fields)

All API-facing structs must derive `Serialize` with `#[serde(rename_all = "camelCase")]`.
`RawRequirement` and `RawTask` derive `Deserialize`.
Enums that appear in query params need `FromStr` / `Display` implementations.

For `AnnotationType` serialization: serialize `Impl` as `"impl"` and `Test` as `"test"`. Use `#[serde(rename = "impl")]` on the Impl variant.

For `TaskStatus` serialization: serialize `InProgress` as `"in_progress"`. Use `#[serde(rename = "in_progress")]` on the InProgress variant.

#### 2.2 — `parser.rs`
YAML file parsing with proper error handling.

Functions:
- `parse_requirements(path: &Path) -> Result<Vec<RawRequirement>>` — reads and validates requirements.yaml. Every entry MUST have id, title, description, createdAt, updatedAt. Validate that createdAt and updatedAt are valid ISO 8601 date-time strings. Return error with line number on malformed YAML.
- `parse_tasks(path: &Path) -> Result<Vec<RawTask>>` — reads and validates tasks.yaml. Every entry MUST have id, requirementId, title, status, createdAt, updatedAt. Missing file → return empty vec with warning log (tasks.yaml is optional).

Error types (in a dedicated `error.rs` or inside `parser.rs`):
- `ParseError` with variants: `IoError`, `YamlError { line: Option<usize>, message: String }`, `ValidationError { id: String, message: String }`

Mark these functions with `/// @req SCS-PARSE-001` and `/// @req SCS-PARSE-002`.

#### 2.3 — `scanner.rs`
File walking and annotation extraction.

Functions:
- `scan_directory(root: &Path) -> Result<Vec<Annotation>>` — recursively walks the directory, skips hidden dirs / `target/` / `node_modules/`, processes files with supported extensions.
- `scan_file(path: &Path, relative_to: &Path) -> Result<Vec<Annotation>>` — reads a single file, finds `@req` annotations, extracts snippet (annotation line + next 2 lines), classifies as impl/test.
- `is_test_file(path: &Path) -> bool` — applies test file detection patterns.
- `supported_extensions() -> &[&str]` — returns `["rs", "ts", "js", "py", "dart", "go"]`.

The regex for annotation detection: `(?://|#)\s*@req\s+([\w-]+)`

Mark with `/// @req SCS-SCAN-001` and `/// @req SCS-SCAN-002`.

#### 2.4 — `coverage.rs`
Coverage computation and stats aggregation.

Functions:
- `compute_coverage(requirements: &[RawRequirement], tasks: &[RawTask], annotations: &[Annotation]) -> ScanResult` — the main function that ties everything together. Computes per-requirement status, detects orphans, builds stats.
- `extract_type_from_id(id: &str) -> String` — extracts type prefix from requirement ID. `SCS-SCAN-001` → `SCS`. Logic: split by `-`, take the first segment.
- `compute_stats(requirements: &[Requirement], annotations: &[Annotation], tasks: &[Task]) -> Stats` — aggregate statistics computation.

Mark with `/// @req SCS-COV-001`, `SCS-COV-002`, `SCS-COV-003`, `SCS-COV-004`.

#### 2.5 — `lib.rs`
Public API of the crate. Re-export all public types and key functions.

### Unit Tests

Write unit tests in each module (inside `#[cfg(test)] mod tests { ... }`).

Every test MUST have a `// @req SCS-XXX-NNN` comment. Examples:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // @req SCS-PARSE-001
    #[test]
    fn test_parse_valid_requirements() {
        // Use fixtures/valid-project/requirements.yaml
    }

    // @req SCS-PARSE-001
    #[test]
    fn test_parse_malformed_yaml_returns_error() {
        // Use fixtures/malformed-yaml/requirements.yaml
    }

    // @req SCS-SCAN-001
    #[test]
    fn test_scan_rust_file() {
        // Use fixtures/valid-project/src/main.rs
    }

    // @req SCS-SCAN-001
    #[test]
    fn test_scan_python_file() {
        // Use fixtures/mixed-languages/ Python files
    }

    // @req SCS-SCAN-002
    #[test]
    fn test_classify_test_file() {
        // Verify is_test_file for various patterns
    }

    // @req SCS-COV-001
    #[test]
    fn test_coverage_calculation() {
        // FR-TEST-001 = covered, FR-TEST-002 = partial, FR-TEST-003 = missing
    }

    // @req SCS-COV-002
    #[test]
    fn test_orphan_annotation_detection() {
        // Use fixtures/orphans-project/
    }

    // @req SCS-COV-003
    #[test]
    fn test_orphan_task_detection() {
        // Use fixtures/orphans-project/
    }

    // @req SCS-COV-004
    #[test]
    fn test_stats_computation() {
        // Verify all stat fields
    }

    // @req SCS-ERR-001
    #[test]
    fn test_missing_file_error() { ... }

    // @req SCS-ERR-001
    #[test]
    fn test_empty_source_directory() { ... }
}
```

### Update tasks.yaml
Change status of TASK-001 through TASK-008 to `done`. Update their `updatedAt` timestamps to the current time.

### Update README.md
- Update status to "Phase 2 — scanner core implemented"
- Add section describing the core library capabilities

### Verification Before Commit
```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

### Commit
```
feat(core): YAML parsing, annotation scanning, coverage computation [SCS-PARSE-001, SCS-PARSE-002, SCS-SCAN-001, SCS-SCAN-002, SCS-COV-001, SCS-COV-002, SCS-COV-003, SCS-COV-004, SCS-ERR-001]
```

---

## Phase 3: REST API (`sdd-server`)

### Goal
Implement the axum HTTP server that serves scan results via the REST API defined in the OpenAPI spec.

### What to Implement

#### 3.1 — `state.rs`
Shared application state using `Arc<RwLock<...>>`:

```rust
pub struct AppState {
    pub scan_result: Option<ScanResult>,
    pub scan_status: ScanStatus,
    pub config: AppConfig,
}

pub struct AppConfig {
    pub project_root: PathBuf,
    pub requirements_path: PathBuf,
    pub tasks_path: PathBuf,
    pub source_path: PathBuf,
}
```

Use `Arc<RwLock<AppState>>` as the axum state type. The `RwLock` allows concurrent reads from handlers while the scan task holds a write lock only when updating results.

#### 3.2 — `errors.rs`
Error types that convert to the API Error schema:

```rust
pub struct ApiError {
    pub status: StatusCode,
    pub error: String,
    pub message: String,
}
```

Implement `IntoResponse` for `ApiError` so it serializes to JSON matching the Error schema. Implement conversions from `sdd_core` error types.

Handle invalid query parameters here — return 400 with descriptive message.

#### 3.3 — `handlers.rs`
One handler function per endpoint. All handlers read from `AppState`.

- `healthcheck()` → returns Healthcheck JSON. Mark with `/// @req SCS-API-001`.
- `get_stats()` → returns Stats JSON. Mark with `/// @req SCS-API-002`.
- `list_requirements(query)` → filtering + sorting logic. Mark with `/// @req SCS-API-003`.
  - Query params: `type`, `status`, `sort` (id|updatedAt), `order` (asc|desc).
  - Validate enum values. Invalid → 400.
- `get_requirement(path)` → single requirement with annotations + tasks. Mark with `/// @req SCS-API-004`.
  - Not found → 404 with Error schema.
- `list_annotations(query)` → filtering. Mark with `/// @req SCS-API-005`.
  - Query params: `type` (impl|test), `orphans` (true|false).
- `list_tasks(query)` → filtering + sorting. Mark with `/// @req SCS-API-006`.
  - Query params: `status`, `orphans`, `sort`, `order`.
- `trigger_scan()` → spawns background scan task, returns 202. Mark with `/// @req SCS-API-007`.
  - If scan already running: cancel (via `CancellationToken` or `AbortHandle`) and restart.
- `get_scan_status()` → returns ScanStatus JSON. Mark with `/// @req SCS-API-007`.

#### 3.4 — `routes.rs`
Route registration:
```rust
pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .route("/healthcheck", get(handlers::healthcheck))
        .route("/stats", get(handlers::get_stats))
        .route("/requirements", get(handlers::list_requirements))
        .route("/requirements/{requirementId}", get(handlers::get_requirement))
        .route("/annotations", get(handlers::list_annotations))
        .route("/tasks", get(handlers::list_tasks))
        .route("/scan", get(handlers::get_scan_status).post(handlers::trigger_scan))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

#### 3.5 — `main.rs`
Entry point:
1. Parse configuration from env vars.
2. Initialize tracing subscriber.
3. Create shared state.
4. Spawn initial background scan.
5. Build router.
6. Bind to configured port.
7. Set up graceful shutdown (tokio signal for SIGTERM + Ctrl+C).
8. Start serving.

Mark startup scan with `/// @req SCS-API-007`.

### Integration Tests

Write API integration tests. These can live in `crates/sdd-server/tests/` or in the module itself.

Use `axum::test` helpers or build a test router with a known fixture project.

```rust
// @req SCS-API-001
#[tokio::test]
async fn test_healthcheck() { ... }

// @req SCS-API-002
#[tokio::test]
async fn test_stats() { ... }

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_filter_by_status() { ... }

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_sort_order() { ... }

// @req SCS-API-003
#[tokio::test]
async fn test_requirements_invalid_param_returns_400() { ... }

// @req SCS-API-004
#[tokio::test]
async fn test_requirement_detail() { ... }

// @req SCS-API-004
#[tokio::test]
async fn test_requirement_not_found_returns_404() { ... }

// @req SCS-API-005
#[tokio::test]
async fn test_annotations_filter_orphans() { ... }

// @req SCS-API-006
#[tokio::test]
async fn test_tasks_filter_by_status() { ... }

// @req SCS-API-007
#[tokio::test]
async fn test_trigger_scan_returns_202() { ... }

// @req SCS-API-007
#[tokio::test]
async fn test_scan_status() { ... }

// @req SCS-ERR-001
#[tokio::test]
async fn test_error_response_format() { ... }
```

### Update tasks.yaml
Change status of TASK-009 through TASK-016 to `done`. Update their `updatedAt` timestamps to the current time.

### Update README.md
- Update status to "Phase 3 — REST API implemented"
- Add API section with endpoint summary table (reference the OpenAPI spec for details)
- Add "Running the server" instructions

### Verification Before Commit
```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

### Commit
```
feat(api): REST API endpoints with filtering, sorting, scan lifecycle [SCS-API-001, SCS-API-002, SCS-API-003, SCS-API-004, SCS-API-005, SCS-API-006, SCS-API-007, SCS-ERR-001]
```

---

## Phase 4: CLI & Self-Hosting (`sdd-cli`)

### Goal
Implement the CLI binary, ensure self-hosting passes (the service scans its own codebase with `--strict` and succeeds), and add all remaining `@req` annotations to reach full coverage.

### What to Implement

#### 4.1 — `sdd-cli/src/main.rs`

Use `clap` with derive macros:

```rust
/// @req SCS-CLI-001
#[derive(Parser)]
#[command(name = "sdd-coverage", about = "SDD coverage scanner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a project for @req annotation coverage
    Scan {
        /// Path to requirements.yaml
        #[arg(long, default_value = "requirements.yaml")]
        requirements: PathBuf,

        /// Path to tasks.yaml
        #[arg(long, default_value = "tasks.yaml")]
        tasks: PathBuf,

        /// Root directory to scan for source files
        #[arg(long, default_value = ".")]
        source: PathBuf,

        /// Strict mode: exit 1 if any requirement is not fully covered or any orphan exists
        #[arg(long)]
        strict: bool,
    },
}
```

Behavior:
1. Parse args.
2. Call `sdd_core` functions to parse YAML and scan.
3. Print a summary table to stdout:
   - Total requirements, covered, partial, missing
   - Total annotations (impl/test), orphans
   - Total tasks, orphans
   - Overall coverage percentage
4. If `--strict`:
   - Exit 0 if all requirements are covered AND zero orphan annotations AND zero orphan tasks.
   - Exit 1 otherwise, printing which requirements are not covered and which orphans exist.

Mark with `/// @req SCS-CLI-001`.

#### 4.2 — Self-Hosting Preparation

This is the critical step. Review all source files and ensure:

1. **Every requirement in `requirements.yaml` has ≥1 impl annotation AND ≥1 test annotation.**
   - Go through each `SCS-*` requirement.
   - Find the function(s) that implement it — ensure they have `/// @req SCS-XXX-NNN`.
   - Find the test(s) that verify it — ensure they have `// @req SCS-XXX-NNN`.

2. **Zero orphan annotations.** Every `@req` annotation references a valid ID from `requirements.yaml`.

3. **Zero orphan tasks.** Every task in `tasks.yaml` references a valid requirement ID.

4. **Run the self-hosting check:**
   ```bash
   cargo build --workspace --release
   ./target/release/sdd-coverage scan --requirements requirements.yaml --tasks tasks.yaml --source . --strict
   ```
   This MUST exit 0.

If any requirement is partial or missing, add the necessary `@req` annotations. If any orphan exists, fix the reference.

#### 4.3 — CLI Integration Tests

```rust
// @req SCS-CLI-001
#[test]
fn test_cli_scan_valid_project() {
    // Run CLI against fixtures/valid-project, verify output
}

// @req SCS-CLI-001
#[test]
fn test_cli_strict_mode_fails_on_missing() {
    // Run CLI with --strict against a project with missing coverage
}

// @req SCS-CLI-001
#[test]
fn test_cli_strict_mode_passes_on_full_coverage() {
    // Run CLI with --strict against a fully covered fixture
}

// @req SCS-SELF-001
#[test]
fn test_self_hosting() {
    // Run CLI against the workspace root with --strict
    // This test verifies self-hosting works
}
```

### Update tasks.yaml
Change status of TASK-017 and TASK-018 to `done`. Update their `updatedAt` timestamps to the current time.

### Update README.md
- Update status to "Phase 4 — CLI and self-hosting complete"
- Add CLI usage section with examples
- Add self-hosting explanation

### Verification Before Commit
```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build --workspace --release
./target/release/sdd-coverage scan --requirements requirements.yaml --tasks tasks.yaml --source . --strict
```

All five checks MUST pass.

### Commit
```
feat(cli): CLI scanner with --strict mode, self-hosting verified [SCS-CLI-001, SCS-SELF-001]
```

---

## Phase 5: Docker & Final Polish

### Goal
Add the Dockerfile, finalize the README, and ensure everything is production-ready.

### What to Implement

#### 5.1 — Dockerfile

```dockerfile
# Build stage
FROM rust:latest AS builder
WORKDIR /build
COPY . .
RUN cargo build --workspace --release

# Runtime stage
FROM alpine:latest
RUN apk add --no-cache libgcc
WORKDIR /app
COPY --from=builder /build/target/release/sdd-server /app/sdd-server
COPY --from=builder /build/target/release/sdd-coverage /app/sdd-coverage
ENV SDD_PORT=4010
ENV SDD_PROJECT_ROOT=/workspace
EXPOSE 4010
ENTRYPOINT ["/app/sdd-server"]
```

Note: If linking issues occur with Alpine/musl, use `rust:latest` builder with `musl-tools` and `--target x86_64-unknown-linux-musl`, or switch runtime to `debian:bookworm-slim`. Test the Docker build and verify the binary runs inside the container.

#### 5.2 — .dockerignore

```
target/
.git/
.github/
fixtures/
*.md
```

#### 5.3 — Final README.md

Complete the README with all sections:
- Project overview and purpose
- Architecture (three crates, their responsibilities)
- Requirements summary (reference requirements.yaml)
- Getting started: prerequisites (Rust toolchain), build, run
- Server usage: `cargo run -p sdd-server`, env vars, example curl commands
- CLI usage: `cargo run -p sdd-cli -- scan --help`, examples, --strict explanation
- Docker: build image, run server, run CLI via Docker
- API reference: endpoint table with brief descriptions, link to OpenAPI spec
- Testing: `cargo test --workspace`, fixture descriptions
- CI: what the GitHub Actions workflow checks
- Self-hosting: explanation of how the project verifies itself
- License (if applicable)

### Update tasks.yaml
Change status of TASK-019 to `done`. Update its `updatedAt` timestamp to the current time.

### Verification Before Commit
```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo build --workspace --release
./target/release/sdd-coverage scan --requirements requirements.yaml --tasks tasks.yaml --source . --strict
docker build -t sdd-coverage .
```

### Commit
```
feat(docker): Dockerfile, final README, production polish [SCS-DOCKER-001]
```

---

## Summary of Commits

| # | Message | Requirements |
|---|---------|-------------|
| 1 | `chore(scaffold): project structure, requirements spec, fixtures, CI workflow` | None (defined, not implemented) |
| 2 | `feat(core): YAML parsing, annotation scanning, coverage computation` | SCS-PARSE-001, SCS-PARSE-002, SCS-SCAN-001, SCS-SCAN-002, SCS-COV-001, SCS-COV-002, SCS-COV-003, SCS-COV-004, SCS-ERR-001 |
| 3 | `feat(api): REST API endpoints with filtering, sorting, scan lifecycle` | SCS-API-001–007, SCS-ERR-001 |
| 4 | `feat(cli): CLI scanner with --strict mode, self-hosting verified` | SCS-CLI-001, SCS-SELF-001 |
| 5 | `feat(docker): Dockerfile, final README, production polish` | SCS-DOCKER-001 |
