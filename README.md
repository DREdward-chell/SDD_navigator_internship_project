# SDD Coverage Service

A Rust HTTP service and CLI tool that scans a project codebase for `@req` annotations, cross-references them with `requirements.yaml` and `tasks.yaml`, computes coverage metrics, and serves results via REST API.

Built following Specification-Driven Development (SDD) principles: traceability, DRY, deterministic enforcement, parsimony.

**Status: Phase 3 — REST API implemented**

---

## Architecture

Three-crate Cargo workspace:

```
sdd-coverage/
├── crates/
│   ├── sdd-core/     # Library: parser, scanner, models, coverage engine
│   ├── sdd-server/   # Binary: axum HTTP service (REST API)
│   └── sdd-cli/      # Binary: `sdd-coverage scan` CLI tool
```

```
sdd-core ◄── sdd-server (HTTP API)
sdd-core ◄── sdd-cli    (CLI tool)
```

---

## Requirements

All requirements are defined in [`requirements.yaml`](requirements.yaml). Key areas:

- **SCS-PARSE-***: YAML parsing for requirements and tasks
- **SCS-SCAN-***: Annotation scanning across multiple languages
- **SCS-COV-***: Coverage computation, orphan detection, statistics
- **SCS-API-***: REST API endpoints
- **SCS-CLI-***: CLI interface with strict mode
- **SCS-ERR-***: Error handling
- **SCS-SELF-***: Self-hosting verification
- **SCS-DOCKER-***: Docker containerization

---

## Build

Prerequisites: Rust stable toolchain (edition 2021)

```bash
cargo build --workspace
```

---

## Running the server

```bash
cargo run -p sdd-server
# or with options:
SDD_PORT=4010 SDD_SOURCE=./src cargo run -p sdd-server
```

The server scans on startup and serves on `http://localhost:4010`.

---

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/healthcheck` | Service health + version |
| GET | `/stats` | Aggregate coverage statistics |
| GET | `/requirements` | List requirements (`?type`, `?status`, `?sort`, `?order`) |
| GET | `/requirements/{id}` | Requirement detail with annotations + tasks |
| GET | `/annotations` | List annotations (`?type`, `?orphans`) |
| GET | `/tasks` | List tasks (`?status`, `?orphans`, `?sort`, `?order`) |
| POST | `/scan` | Trigger background re-scan (returns 202) |
| GET | `/scan` | Current scan status |

See [`sdd-coverage-api.yaml`](sdd-coverage-api.yaml) for the full OpenAPI spec.

---

## Docker

_Placeholder — multi-stage Dockerfile producing an Alpine-based image._

---

## Core Library (`sdd-core`)

The core library provides:

- **Parser** (`parser.rs`): Reads `requirements.yaml` and `tasks.yaml`, validates all required fields and ISO 8601 timestamps.
- **Scanner** (`scanner.rs`): Recursively walks a directory, finds `@req` annotations in `.rs`, `.ts`, `.js`, `.py`, `.dart`, `.go` files. Classifies annotations as `impl` or `test` by file-path patterns.
- **Coverage** (`coverage.rs`): Computes per-requirement status (covered/partial/missing), detects orphan annotations and tasks, aggregates statistics.
- **Models** (`models.rs`): Shared types used across the library and APIs.

## Testing

```bash
cargo test --workspace
```

---

## CI

GitHub Actions workflow (`.github/workflows/ci.yml`) runs on push to `main`/`dev` and on pull requests:

- `cargo fmt --all --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace --release`
- Self-hosting scan (blocking on `main`, non-blocking on other branches)
