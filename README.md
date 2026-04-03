# SDD Coverage Service

A Rust HTTP service and CLI tool that scans a project codebase for `@req` annotations,
cross-references them with `requirements.yaml` and `tasks.yaml`, computes coverage metrics,
and serves results via REST API.

Built following [Specification-Driven Development](https://blog.rezvov.com/posts/specification-driven-development-four-pillars)
principles: traceability, DRY, deterministic enforcement, parsimony.

---

## Architecture

Three-crate Cargo workspace:

```
sdd-coverage/
├── crates/
│   ├── sdd-core/     # Library: parser, scanner, models, coverage engine
│   ├── sdd-server/   # Binary: axum HTTP service (REST API on :4010)
│   └── sdd-cli/      # Binary: `sdd-coverage scan` CLI tool
├── requirements.yaml # This project's own requirements
├── tasks.yaml        # This project's own tasks
├── sdd-coverage-api.yaml  # OpenAPI spec (source of truth)
└── fixtures/         # Test fixture projects (6 scenarios)
```

```
sdd-core ◄── sdd-server   (HTTP API)
sdd-core ◄── sdd-cli      (CLI tool)
```

---

## Requirements

All requirements are defined in [`requirements.yaml`](requirements.yaml). Key groups:

| Prefix | Area |
|--------|------|
| `SCS-PARSE-*` | YAML parsing for requirements and tasks |
| `SCS-SCAN-*` | Annotation scanning across 6 languages |
| `SCS-COV-*` | Coverage computation, orphan detection, statistics |
| `SCS-API-*` | REST API endpoints |
| `SCS-CLI-*` | CLI interface with strict mode |
| `SCS-ERR-*` | Error handling (no panics under any input) |
| `SCS-SELF-*` | Self-hosting verification |
| `SCS-DOCKER-*` | Docker containerization |

---

## Getting Started

**Prerequisites:** Rust stable toolchain (edition 2021)

```bash
# Clone and build
git clone <repo-url>
cd sdd-coverage
cargo build --workspace
```

---

## Running the Server

```bash
# Default: port 4010, scans ./src
cargo run -p sdd-server

# With configuration
SDD_PORT=8080 \
SDD_PROJECT_ROOT=/path/to/project \
SDD_REQUIREMENTS=requirements.yaml \
SDD_TASKS=tasks.yaml \
SDD_SOURCE=./src \
cargo run -p sdd-server
```

The server triggers a background scan on startup. All endpoints return `503` until
the first scan completes.

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `SDD_PORT` | `4010` | HTTP listen port |
| `SDD_PROJECT_ROOT` | `.` | Base path for relative file resolution |
| `SDD_REQUIREMENTS` | `requirements.yaml` | Path to requirements file |
| `SDD_TASKS` | `tasks.yaml` | Path to tasks file |
| `SDD_SOURCE` | `./src` | Root directory to scan |
| `RUST_LOG` | `info` | Log level |

### Example Requests

```bash
# Health check
curl http://localhost:4010/healthcheck

# Coverage statistics
curl http://localhost:4010/stats

# Requirements filtered and sorted
curl "http://localhost:4010/requirements?status=partial&sort=id&order=asc"

# Requirement detail with annotations + tasks
curl http://localhost:4010/requirements/SCS-SCAN-001

# Annotations — orphans only
curl "http://localhost:4010/annotations?orphans=true"

# Trigger a re-scan
curl -X POST http://localhost:4010/scan

# Poll scan status
curl http://localhost:4010/scan
```

---

## CLI

```bash
# Print summary and exit 0
cargo run -p sdd-cli -- scan \
  --requirements requirements.yaml \
  --tasks tasks.yaml \
  --source .

# Strict mode: exit 1 on any gap or orphan
cargo run -p sdd-cli -- scan --strict

# Release binary
./target/release/sdd-coverage scan --strict
```

### Strict Mode Exit Codes

| Exit code | Meaning |
|-----------|---------|
| `0` | All requirements covered, zero orphans |
| `1` | Any partial/missing requirement or any orphan exists |
| `2` | Configuration error (missing file, malformed YAML) |

---

## API Reference

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/healthcheck` | Service health + version |
| `GET` | `/stats` | Aggregate coverage statistics |
| `GET` | `/requirements` | List requirements (`?type`, `?status`, `?sort`, `?order`) |
| `GET` | `/requirements/{id}` | Requirement detail with linked annotations + tasks |
| `GET` | `/annotations` | List annotations (`?type=impl\|test`, `?orphans=true`) |
| `GET` | `/tasks` | List tasks (`?status`, `?orphans`, `?sort`, `?order`) |
| `POST` | `/scan` | Trigger background re-scan (202 Accepted) |
| `GET` | `/scan` | Current scan state: `idle\|scanning\|completed\|failed` |

Invalid query parameters return `400`. Missing resources return `404`. Error bodies
follow `{ "error": "<code>", "message": "<detail>" }`.

Full contract: [`sdd-coverage-api.yaml`](sdd-coverage-api.yaml)

---

## Docker

```bash
# Build image
docker build -t sdd-coverage .

# Run the HTTP server (mount your project at /workspace)
docker run -p 4010:4010 -v /path/to/project:/workspace sdd-coverage

# Run the CLI via Docker
docker run --entrypoint /app/sdd-coverage \
  -v /path/to/project:/workspace \
  sdd-coverage scan \
    --requirements /workspace/requirements.yaml \
    --tasks /workspace/tasks.yaml \
    --source /workspace \
    --strict
```

Multi-stage build: `rust:latest` builder → `alpine:latest` runtime. Both
`sdd-server` and `sdd-coverage` binaries are present in the image.

---

## Core Library (`sdd-core`)

| Module | Responsibility |
|--------|----------------|
| `models.rs` | Shared types: `Requirement`, `Task`, `Annotation`, `ScanResult`, `Stats`, enums |
| `parser.rs` | Reads and validates `requirements.yaml` / `tasks.yaml`; ISO 8601 timestamp validation |
| `scanner.rs` | Recursive directory walk; `@req` annotation extraction; impl/test classification |
| `coverage.rs` | Per-requirement status, orphan detection, aggregate statistics |
| `error.rs` | `CoreError` with `Io`, `Yaml { line }`, `Validation` variants |

**Annotation pattern:** `^\s*(?:/{2,}|#)\s*@req\s+([\w-]+)` — matches `//`, `///`, and `#` comment styles at the start of a line. Supported extensions: `.rs .ts .js .py .dart .go`.

**Coverage status logic:**
- `covered` — ≥1 impl annotation AND ≥1 test annotation
- `partial` — ≥1 impl annotation, zero test annotations
- `missing` — zero annotations of any kind

---

## Testing

```bash
cargo test --workspace
```

65 tests across three crates:

| Suite | Count | What it covers |
|-------|-------|----------------|
| `sdd-core` unit tests | 23 | Parser, scanner, coverage per-module |
| `sdd-core` integration tests | 15 | End-to-end against all 6 fixtures |
| `sdd-server` API tests | 20 | All endpoints, filtering, error responses |
| `sdd-cli` CLI tests | 7 | Summary output, strict mode, self-hosting |

### Fixtures

| Directory | Purpose |
|-----------|---------|
| `valid-project/` | Happy path: 3 reqs (covered/partial/missing), 2 tasks |
| `empty-project/` | Edge case: no source files |
| `malformed-yaml/` | Edge case: invalid YAML (error-handling test) |
| `mixed-languages/` | `.rs .ts .py .js .go .dart` annotation detection |
| `orphans-project/` | Orphan annotation + orphan task detection |
| `nested-dirs/` | Deep directory recursion (`src/a/b/c/d/`) |

---

## CI

GitHub Actions (`.github/workflows/ci.yml`) — triggers on push to `main`/`dev` and PRs:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`
4. `cargo build --workspace --release`
5. `./scripts/scan.sh` — self-hosting scan

Step 5 is **blocking on `main`**, non-blocking (`continue-on-error: true`) on all other branches.

---

## Self-Hosting

The project scans its own source code with `--strict` and must pass. This is enforced
in CI on `main` and verified by the `test_self_hosting` integration test.

```bash
cargo build --workspace --release
./target/release/sdd-coverage scan \
  --requirements requirements.yaml \
  --tasks tasks.yaml \
  --source . \
  --strict
# → Coverage: 100.00% | STRICT MODE: PASSED
```

Every implementation function that satisfies a requirement carries a `/// @req SCS-XXX-NNN`
doc comment. Every test that verifies a requirement carries a `// @req SCS-XXX-NNN` comment
above the `#[test]` attribute.
