# CLAUDE.md — SDD Coverage Service

## Project Overview

Rust HTTP service + CLI tool that scans a project codebase for `@req` annotations, cross-references them with `requirements.yaml` and `tasks.yaml`, computes coverage metrics, and serves results via REST API.

Built following Specification-Driven Development (SDD) principles: traceability, DRY, deterministic enforcement, parsimony.

## Architecture

Cargo workspace with three crates:

```
sdd-coverage/
├── Cargo.toml                  # workspace root
├── requirements.yaml           # this project's own requirements
├── tasks.yaml                  # this project's own tasks
├── sdd-coverage-api.yaml       # OpenAPI spec (source of truth for API)
├── CLAUDE.md                   # this file
├── PLAN.md                     # build plan (5 phases)
├── README.md                   # evolves with each commit
├── Dockerfile                  # multi-stage, alpine runtime
├── .github/workflows/ci.yml    # CI: blocking checks + branch-aware scan
├── scripts/
│   └── scan.sh                 # local scan wrapper (--no-strict option)
├── fixtures/                   # test fixture projects
│   ├── valid-project/          # happy path fixture
│   ├── empty-project/          # edge case: no source files
│   ├── malformed-yaml/         # edge case: invalid YAML
│   ├── mixed-languages/        # .rs, .ts, .py, .js, .go, .dart
│   ├── orphans-project/        # orphan annotations + orphan tasks
│   └── nested-dirs/            # deeply nested source tree
├── crates/
│   ├── sdd-core/               # library: parser, scanner, models, coverage
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models.rs       # shared data types
│   │   │   ├── parser.rs       # YAML parsing (requirements + tasks)
│   │   │   ├── scanner.rs      # annotation scanning + file walking
│   │   │   └── coverage.rs     # coverage computation + stats
│   │   └── Cargo.toml
│   ├── sdd-server/             # binary: axum HTTP service
│   │   ├── src/
│   │   │   ├── main.rs         # entry point, config, startup scan
│   │   │   ├── routes.rs       # route registration
│   │   │   ├── handlers.rs     # endpoint handlers
│   │   │   ├── state.rs        # shared app state (Arc<RwLock<>>)
│   │   │   └── errors.rs       # error types → API Error schema
│   │   └── Cargo.toml
│   └── sdd-cli/                # binary: `sdd-coverage scan`
│       ├── src/
│       │   └── main.rs         # CLI arg parsing, scan, exit code
│       └── Cargo.toml
└── tests/                      # workspace-level integration tests (optional)
```

## Key Design Decisions

### Requirement IDs
- This project uses `SCS-*` prefix (e.g., `SCS-SCAN-001`).
- Requirement type is extracted from the ID prefix (everything before the second hyphen). `SCS-SCAN-001` → type `SCS`.
- The API spec defines `enum: [FR, AR]` but our implementation treats type as a free-form string. No enum restriction.

### Timestamps
- `requirements.yaml` and `tasks.yaml` MUST contain `createdAt` and `updatedAt` fields on every entry (ISO 8601 format).
- `createdAt` is set once when the entry is first written.
- `updatedAt` is set whenever the entry's title, description, or other meaningful fields change.
- The parser MUST validate that both fields are present and are valid ISO 8601 date-time strings.
- The API passes these values through to responses as-is — no derivation or transformation.

### Coverage Status Logic
- **covered**: requirement has ≥1 impl annotation AND ≥1 test annotation
- **partial**: requirement has ≥1 impl annotation but zero test annotations
- **missing**: requirement has zero annotations of any kind

### Orphan Detection
- **Orphan annotation**: `@req SOME-ID` where `SOME-ID` is not in `requirements.yaml`
- **Orphan task**: task with `requirementId` not in `requirements.yaml`

### Test File Detection
A file is a test file if ANY of these match:
- Path contains `/tests/` directory
- Filename starts with `test_` (e.g., `test_parser.py`)
- Filename contains `_test.` (e.g., `parser_test.rs`)
- Filename contains `.test.` (e.g., `parser.test.ts`)

Everything else is an impl file.

### Annotation Scanning — Comment Styles
- `//` line comments: `.rs`, `.ts`, `.js`, `.go`, `.dart`
- `#` line comments: `.py`
- Pattern: comment prefix, optional whitespace, `@req`, whitespace, requirement ID
- Regex: `(?://|#)\s*@req\s+([\w-]+)`

### Snippet Extraction
- Capture the `@req` comment line + the next 2 lines below it.
- If fewer than 2 lines remain in the file, capture whatever is available.
- Newlines in snippet represented as `\n` in the JSON string.

### Scan Behavior
- Server auto-triggers a scan on startup (background task).
- `POST /scan` while a scan is running: cancel the current scan, start a new one.
- `POST /scan` returns 202 immediately with `{ "status": "scanning", "startedAt": "..." }`.
- `GET /scan` returns current scan state: `idle` | `scanning` | `completed` | `failed`.

### Strict Mode (CLI)
`--strict` exit codes:
- Exit 0: zero missing, zero partial, zero orphan annotations, zero orphan tasks.
- Exit 1: any missing, partial, orphan annotation, or orphan task exists.
- CLI prints a summary table before exiting.

### Configuration
Environment variables (defaults) with CLI flag overrides:

| Purpose | Env Var | CLI Flag | Default |
|---------|---------|----------|---------|
| HTTP port | `SDD_PORT` | `--port` | `4010` |
| Project root | `SDD_PROJECT_ROOT` | `--project-root` | `.` |
| Log level | `RUST_LOG` | — | `info` |
| Requirements file | `SDD_REQUIREMENTS` | `--requirements` | `requirements.yaml` |
| Tasks file | `SDD_TASKS` | `--tasks` | `tasks.yaml` |
| Source dirs | `SDD_SOURCE` | `--source` | `./src` |
| Test dirs | `SDD_TESTS` | `--tests` | `./tests` |

### Error Handling
- All error responses use the API spec's `Error` schema: `{ "error": "<code>", "message": "<detail>" }`.
- Invalid query parameters → HTTP 400.
- Requirement not found → HTTP 404.
- Missing YAML files → clear error with expected path.
- Malformed YAML → error with line number if possible.
- Empty source directory → warning log, return empty results (not crash).
- File permission errors → skip file with warning log, continue scan.
- **No panics under any input.** No `unwrap()` in library code. Use `?` and proper error types.

### CORS
- Enabled via `tower-http::CorsLayer`.
- Default: permissive (all origins) for development.
- Configurable via `SDD_CORS_ORIGIN` env var for production.

### Graceful Shutdown
- Server handles SIGTERM (tokio signal handler).
- Finishes in-flight HTTP requests before shutting down.

### Version
- `/healthcheck` returns version from `Cargo.toml` via `env!("CARGO_PKG_VERSION")`.
- All three crates share the same version number.

### Serialization
- `assignee` on Task: `Option<String>`, omitted from JSON when `None` via `#[serde(skip_serializing_if = "Option::is_none")]`.
- Timestamps serialized as ISO 8601 / RFC 3339 strings.
- All JSON field names use camelCase to match the API spec.

## Coding Conventions

### Rust Style
- Edition 2021.
- `cargo fmt` with default rustfmt settings.
- `cargo clippy -- -D warnings` must pass with zero warnings.
- No `unwrap()` or `expect()` in library code (`sdd-core`). Binary crates may use `expect()` only at startup for configuration.
- Use `thiserror` for error type definitions in `sdd-core`.
- Use `anyhow` in binary crates (`sdd-server`, `sdd-cli`) for top-level error handling.
- Prefer `&str` / `&Path` in function signatures over owned types where possible.
- Use `serde` derive macros with `#[serde(rename_all = "camelCase")]` on all API-facing structs.

### Anti-Patterns — DO NOT
- Do NOT write duplicate code. If two handlers share logic, extract it.
- Do NOT leave `todo!()` macros in committed code.
- Do NOT use `println!()` for logging. Use `tracing::info!()`, `tracing::warn!()`, etc.
- Do NOT hardcode paths. Use configuration.
- Do NOT block the async runtime with synchronous file I/O. Use `tokio::fs` or `spawn_blocking`.
- Do NOT store the OpenAPI spec content anywhere other than `sdd-coverage-api.yaml`. Reference it, don't duplicate it.
- Do NOT write the same test logic twice. Use helper functions in test modules.
- Do NOT implement pagination unless explicitly asked — the API spec does not require it.

### Traceability
- Every test function MUST have a `// @req SCS-XXX-NNN` comment on the line above `#[test]` or `#[tokio::test]`.
- Every implementation function that directly satisfies a requirement SHOULD have a `/// @req SCS-XXX-NNN` doc comment.
- Commit messages for implementation: `feat(scope): description [SCS-XXX-NNN, SCS-YYY-NNN]`
- Commit messages for scaffolding/config (no requirements implemented): `chore(scope): description`
- Only list requirement IDs in commit messages when the commit actually implements them.

## Dependencies — Use Latest Stable

**IMPORTANT**: Before writing `Cargo.toml` files, check crates.io or docs.rs for the latest stable version of each dependency. Do NOT guess version numbers. The following are the crates to use (check latest versions):

### sdd-core
- `serde` + `serde_yaml` — YAML parsing and serialization
- `serde_json` — JSON serialization
- `regex` — annotation pattern matching
- `walkdir` — recursive directory traversal
- `thiserror` — error type definitions
- `chrono` — timestamp handling (with `serde` feature)
- `tracing` — structured logging

### sdd-server
- `axum` — HTTP framework
- `tokio` (full features) — async runtime
- `tower-http` — CORS, tracing middleware
- `serde_json` — JSON responses
- `tracing` + `tracing-subscriber` — logging
- `sdd-core` (path dependency)

### sdd-cli
- `clap` (derive feature) — CLI argument parsing
- `sdd-core` (path dependency)
- `tracing` + `tracing-subscriber` — logging
- `tokio` (minimal, for potential async usage)

### dev-dependencies (workspace-wide or per-crate)
- `axum-test` or `reqwest` — HTTP integration testing
- `tempfile` — temporary directories for test fixtures
- `assert_cmd` — CLI integration testing (for sdd-cli)

## Docker

Single multi-stage Dockerfile:
1. **Builder stage**: `rust:latest` (or specific stable version), build both binaries in release mode.
2. **Runtime stage**: `alpine:latest`, copy binaries from builder, minimal image.
3. Default entrypoint: `sdd-server`. CLI available via `docker run --entrypoint sdd-coverage ...`.
4. Project to scan mounted at `/workspace`: `docker run -v /path/to/project:/workspace -p 4010:4010 sdd-coverage`.
5. Env vars for configuration.

## GitHub Actions Workflow

File: `.github/workflows/ci.yml`

Triggers: push to `main` and `dev`, pull requests.

Single workflow, two behaviors:
- **Blocking steps** (all branches): `fmt`, `clippy`, `test`, `build --release`. These always fail the workflow if they fail.
- **Self-hosting scan** (branch-dependent): runs `./scripts/scan.sh`. On `main`, the scan is blocking (failure = red build). On all other branches (`dev`, PRs), the scan runs with `continue-on-error: true` — failures show as warnings but don't block the build. This lets you track coverage progress on `dev` without blocking commits.

Key line:
```yaml
continue-on-error: ${{ github.ref != 'refs/heads/main' }}
```

## Local Scan Script

File: `scripts/scan.sh` (must be executable: `chmod +x`)

Usage:
- `./scripts/scan.sh` — strict mode (default). Exits 1 on any coverage gap or orphan.
- `./scripts/scan.sh --no-strict` — summary only. Prints results, always exits 0.

The script does NOT build. Run `cargo build --workspace --release` first.

## Git Branching

- `dev` branch: active development. All phase commits go here. CI runs full checks, scan is non-blocking.
- `main` branch: merge from `dev` when scan passes. CI scan is blocking on `main`.

## CLI Interface

CLI accepts `--source <dir>` which is the root to scan recursively. Test detection is by file-path pattern, not by a separate `--tests` flag.

The `--source .` scans everything recursively from the given directory, skipping non-supported file extensions and hidden directories / `target/` / `node_modules/`.

### Directories to Skip During Scan
- Hidden directories (starting with `.`)
- `target/`
- `node_modules/`
- `.git/`

## OpenAPI Specification (Source of Truth)

The file `sdd-coverage-api.yaml` in the repository root is the authoritative API contract. Below is the full content for reference. When implementing handlers, match this spec exactly for response schemas, status codes, and query parameter names.

```yaml
openapi: 3.0.3
info:
  title: SDD Navigator API
  description: |
    REST API for browsing a project's Specification-Driven Development structure.
    Single-project scope: requirements, code annotations, tasks — with filters and sorting.

    Three artifact types linked to requirements:
    - **Annotations** (`@req` markers in code) — impl and test
    - **Tasks** (work items from `tasks.yaml`) — open, in_progress, done

    Orphan detection: each artifact type can have orphans — references to non-existent requirements.

    See: https://blog.rezvov.com/posts/specification-driven-development-four-pillars
  version: 3.0.0
  contact:
    name: ForEach Partners
    url: https://foreachpartners.com/

servers:
  - url: https://api.pdd.foreachpartners.com
    description: Production server
  - url: http://localhost:4010
    description: Local development

paths:
  /healthcheck:
    get:
      summary: Service health
      description: Returns service status and version. Use to verify the API is running.
      operationId: healthcheck
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Healthcheck"
              example:
                status: healthy
                version: "3.0.0"
                timestamp: "2026-03-01T10:15:00Z"

  /stats:
    get:
      summary: Project-wide statistics
      description: |
        Aggregate numbers for the dashboard: requirement totals broken down by type and coverage status,
        annotation counts by kind (impl/test) with orphan count, task counts by status with orphan count,
        and overall coverage percentage.
      operationId: getStats
      responses:
        "200":
          description: Aggregate stats across all artifact types
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Stats"
              example:
                requirements:
                  total: 8
                  byType: { FR: 6, AR: 2 }
                  byStatus: { covered: 5, partial: 1, missing: 2 }
                annotations:
                  total: 16
                  impl: 10
                  test: 6
                  orphans: 2
                tasks:
                  total: 6
                  byStatus: { open: 3, in_progress: 1, done: 2 }
                  orphans: 1
                coverage: 62.5
                lastScanAt: "2026-03-01T10:15:00Z"

  /requirements:
    get:
      summary: List requirements
      description: |
        Returns all requirements from `requirements.yaml`. Each requirement has a coverage status
        computed by the scanner: **covered** (has both impl and test annotations), **partial** (impl only),
        or **missing** (no annotations). Filter by type or status. Sort by id or updatedAt.
      operationId: listRequirements
      parameters:
        - name: type
          in: query
          description: "Filter by requirement type"
          schema:
            $ref: "#/components/schemas/RequirementType"
        - name: status
          in: query
          description: "Filter by coverage status"
          schema:
            $ref: "#/components/schemas/CoverageStatus"
        - name: sort
          in: query
          schema:
            type: string
            enum: [id, updatedAt]
            default: id
        - name: order
          in: query
          schema:
            type: string
            enum: [asc, desc]
            default: asc
      responses:
        "200":
          description: Requirements list
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: "#/components/schemas/Requirement"

  /requirements/{requirementId}:
    get:
      summary: Requirement detail with all linked artifacts
      description: |
        Returns a single requirement with its full traceability chain:
        all code annotations and all tasks that reference this requirement.
      operationId: getRequirement
      parameters:
        - $ref: "#/components/parameters/requirementId"
      responses:
        "200":
          description: Requirement with annotations and tasks
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/RequirementDetail"
        "404":
          $ref: "#/components/responses/NotFound"

  /annotations:
    get:
      summary: "List code annotations (sorted by file + line)"
      description: |
        Returns @req annotations found in source code by the scanner.
        Use orphans=true to find annotations referencing non-existent requirements.
      operationId: listAnnotations
      parameters:
        - name: type
          in: query
          description: "Filter by annotation kind"
          schema:
            $ref: "#/components/schemas/AnnotationType"
        - name: orphans
          in: query
          description: "If true, return only orphan annotations"
          schema:
            type: boolean
            default: false
      responses:
        "200":
          description: Annotations list
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: "#/components/schemas/Annotation"

  /tasks:
    get:
      summary: List tasks
      description: |
        Returns work items from tasks.yaml.
        Use orphans=true to find tasks referencing non-existent requirements.
      operationId: listTasks
      parameters:
        - name: status
          in: query
          description: "Filter by task status"
          schema:
            $ref: "#/components/schemas/TaskStatus"
        - name: orphans
          in: query
          description: "If true, return only orphan tasks"
          schema:
            type: boolean
            default: false
        - name: sort
          in: query
          schema:
            type: string
            enum: [id, updatedAt]
            default: id
        - name: order
          in: query
          schema:
            type: string
            enum: [asc, desc]
            default: asc
      responses:
        "200":
          description: Tasks list
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: "#/components/schemas/Task"

  /scan:
    post:
      summary: Trigger codebase re-scan
      description: |
        Starts a new scan. Returns immediately with status scanning.
        Poll GET /scan for completion.
      operationId: triggerScan
      responses:
        "202":
          description: Scan started
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ScanStatus"
              example:
                status: scanning
                startedAt: "2026-03-01T10:14:59Z"
    get:
      summary: Current scan status
      description: Returns the state of the most recent scan.
      operationId: getScanStatus
      responses:
        "200":
          description: Scan state
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ScanStatus"

components:
  parameters:
    requirementId:
      name: requirementId
      in: path
      required: true
      schema:
        type: string
      example: FR-SCAN-001

  responses:
    NotFound:
      description: Resource not found
      content:
        application/json:
          schema:
            $ref: "#/components/schemas/Error"
          example:
            error: not_found
            message: "Requirement 'FR-UNKNOWN-999' not found"

  schemas:
    Healthcheck:
      type: object
      required: [status, version, timestamp]
      properties:
        status:
          type: string
          enum: [healthy, degraded]
        version:
          type: string
        timestamp:
          type: string
          format: date-time

    Stats:
      type: object
      required: [requirements, annotations, tasks, coverage, lastScanAt]
      properties:
        requirements:
          $ref: "#/components/schemas/RequirementStats"
        annotations:
          $ref: "#/components/schemas/AnnotationStats"
        tasks:
          $ref: "#/components/schemas/TaskStats"
        coverage:
          type: number
          format: float
          description: "Fully covered / total * 100"
        lastScanAt:
          type: string
          format: date-time

    RequirementStats:
      type: object
      required: [total, byType, byStatus]
      properties:
        total:
          type: integer
        byType:
          type: object
          additionalProperties:
            type: integer
        byStatus:
          type: object
          additionalProperties:
            type: integer

    AnnotationStats:
      type: object
      required: [total, impl, test, orphans]
      properties:
        total:
          type: integer
        impl:
          type: integer
        test:
          type: integer
        orphans:
          type: integer

    TaskStats:
      type: object
      required: [total, byStatus, orphans]
      properties:
        total:
          type: integer
        byStatus:
          type: object
          additionalProperties:
            type: integer
        orphans:
          type: integer

    RequirementType:
      type: string
      description: "Free-form string extracted from requirement ID prefix. Common values: FR, AR."

    CoverageStatus:
      type: string
      enum: [covered, partial, missing]

    TaskStatus:
      type: string
      enum: [open, in_progress, done]

    Requirement:
      type: object
      required: [id, type, title, description, status, createdAt, updatedAt]
      properties:
        id:
          type: string
        type:
          $ref: "#/components/schemas/RequirementType"
        title:
          type: string
        description:
          type: string
        status:
          $ref: "#/components/schemas/CoverageStatus"
        createdAt:
          type: string
          format: date-time
        updatedAt:
          type: string
          format: date-time

    RequirementDetail:
      allOf:
        - $ref: "#/components/schemas/Requirement"
        - type: object
          required: [annotations, tasks]
          properties:
            annotations:
              type: array
              items:
                $ref: "#/components/schemas/Annotation"
            tasks:
              type: array
              items:
                $ref: "#/components/schemas/Task"

    AnnotationType:
      type: string
      enum: [impl, test]

    Annotation:
      type: object
      required: [file, line, reqId, type, snippet]
      properties:
        file:
          type: string
        line:
          type: integer
        reqId:
          type: string
        type:
          $ref: "#/components/schemas/AnnotationType"
        snippet:
          type: string

    Task:
      type: object
      required: [id, requirementId, title, status, createdAt, updatedAt]
      properties:
        id:
          type: string
        requirementId:
          type: string
        title:
          type: string
        status:
          $ref: "#/components/schemas/TaskStatus"
        assignee:
          type: string
        createdAt:
          type: string
          format: date-time
        updatedAt:
          type: string
          format: date-time

    ScanStatus:
      type: object
      required: [status, startedAt]
      properties:
        status:
          type: string
          enum: [idle, scanning, completed, failed]
        startedAt:
          type: string
          format: date-time
        completedAt:
          type: string
          format: date-time
        duration:
          type: integer

    Error:
      type: object
      required: [error, message]
      properties:
        error:
          type: string
        message:
          type: string
```
