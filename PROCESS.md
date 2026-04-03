# PROCESS.md — Development Process Documentation

## 1. Tools Used

### Planning Phase (Claude.ai web interface)
- **Model**: Claude Opus 4.6 via claude.ai
- **Purpose**: Architecture design, requirement specification, decision-making through iterative Q&A. Produced `CLAUDE.md` (project context document) and `PLAN.md` (5-phase build plan).
- **Session**: Extended multi-turn conversation where the developer asked questions, challenged defaults, and refined every architectural decision before any code was written.

### Implementation Phase (Claude Code — desktop client)
- **Model**: Claude Sonnet 4.6 via Claude Code (v2.1.87)
- **IDE integration**: Claude Desktop app with worktree at `.claude/worktrees/naughty-gates`
- **Purpose**: All code generation, file creation, build/test execution, and git operations.
- **Tools used within Claude Code**: `Read`, `Write`, `Edit`, `Bash`, `TodoWrite`, `Agent` (subagent calls)
- **Entrypoint**: `claude-desktop`

### Verification (Claude Code — second session)
- **Model**: Claude Sonnet 4.6 via Claude Code
- **Purpose**: Post-completion SDD pillar evaluation. The developer asked Claude to evaluate its own output against the four SDD pillars, producing a detailed audit with file/line references.

---

## 2. Conversation Log

### Session A — Planning (Claude.ai)

**Duration**: ~2 hours of iterative discussion (this conversation)
**Topic**: Full project specification before any code was written.

| Turn | Developer asked | Outcome |
|------|----------------|---------|
| 1 | Read TASK.txt, SDD.md, produce step-by-step plan with meaningful commits | Claude asked 5 clarifying questions before starting |
| 2 | Answered: API spec provided, GitHub Actions, axum, 5 logical commits, separate CLI binary | Claude asked 6 more questions (CORS, shutdown, version, etc.) |
| 3 | Answered all follow-ups | Claude confirmed full decision list |
| 4 | "Think what else should we consider" | Claude identified CORS, graceful shutdown, version string, assignee field, test validation strategy |
| 5 | Reviewed timestamp approach — **rejected** scan-time derivation | Switched to Option B: timestamps stored in YAML, maintained by developer |
| 6 | **Rejected** Phase 1 commit message listing all requirement IDs | Changed to `chore(scaffold)` with no IDs — only implemented requirements listed |
| 7 | **Rejected** single CI workflow that blocks on self-hosting scan from commit 1 | Redesigned: one workflow file, `continue-on-error` based on branch, `dev` branch for progress tracking |
| 8 | Requested local scan script with `--no-strict` option | Added `scripts/scan.sh` |
| 9 | Requested README.md evolving across commits | Accepted, README updated each phase |

### Session B — Implementation (Claude Code)

**Session ID**: `cb57f80c-73db-4c9a-a41e-0f231ef09926`
**Start**: 2026-04-03T10:39:21Z
**End**: 2026-04-03T12:54:49Z
**Duration**: ~2 hours 15 minutes (active implementation)
**Model**: Claude Sonnet 4.6

| Time | Developer message | What happened |
|------|------------------|---------------|
| 10:39 | "You are working on a project described in CLAUDE.md and PLAN.md..." | Claude read both docs, began Phase 1 |
| 10:48 | *(implicit approval — Claude committed after verification passed)* | Phase 1 committed |
| 10:51 | "Ok, the setup looks good. You may now proceed to phase 2." | Phase 2 began |
| 11:00 | *(Phase 2 committed — 23 tests passing)* | Developer reviewed, gave go-ahead for Phase 3 |
| 11:06 | "Ok, go ahead for phase 3" | Phase 3 began — longest phase |
| 12:27 | "Continue" | Claude had stalled; resumed fixing compilation issues (lib.rs extraction, import fixes) |
| 12:31 | *(Phase 3 committed — 43 tests passing)* | Developer reviewed |
| 12:33 | "Great, lets start phase 4" | Phase 4 began (CLI + self-hosting) |
| 12:45 | *(Phase 4 committed — 65 tests)* | Developer noticed a problem |
| 12:46 | **"I need to clarify one thing..."** — Developer caught regex issue | Course correction (see Section 6) |
| 12:49 | *(Fix committed as separate `fix(scanner)` commit)* | Developer approved |
| 12:52 | "Good. Lets then proceed to phase 5" | Phase 5 began |
| 12:54 | *(Phase 5 committed — final)* | All phases complete |

### Session C — SDD Evaluation (Claude Code)

**Start**: 2026-04-03T13:54:26Z (first attempt, hit context limit)
**Resumed**: 2026-04-03T21:29:12Z
**End**: 2026-04-03T21:32:52Z

Developer asked Claude to evaluate the repository against all four SDD pillars "as if seeing it for the first time." Claude produced a detailed audit identifying 7 concrete violations (3 DRY, 2 Parsimony, 2 Deterministic Enforcement).

---

## 3. Timeline

| Time (UTC) | Duration | Phase | Activity |
|------------|----------|-------|----------|
| — | ~2h | Planning | Architecture Q&A in Claude.ai, produced CLAUDE.md + PLAN.md |
| 10:39–10:48 | 9 min | Phase 1 | Scaffold: workspace, requirements.yaml, tasks.yaml, fixtures, CI, scan script |
| 10:48–10:51 | 3 min | Review | Developer reviewed Phase 1 output |
| 10:51–11:00 | 9 min | Phase 2 | Core library: models, parser, scanner, coverage, 23 unit tests |
| 11:00–11:06 | 6 min | Review | Developer reviewed Phase 2, approved |
| 11:06–12:31 | 85 min | Phase 3 | REST API: handlers, routes, state, errors, 20 integration tests. Included ~75 min gap (12:27 "Continue" suggests Claude stalled or developer stepped away) |
| 12:31–12:33 | 2 min | Review | Developer reviewed Phase 3 |
| 12:33–12:45 | 12 min | Phase 4 | CLI binary, self-hosting, integration tests, Dockerfile (pulled forward) |
| 12:45–12:49 | 4 min | Fix | Developer caught regex bug, Claude fixed and committed |
| 12:49–12:52 | 3 min | Review | Developer reviewed fix |
| 12:52–12:54 | 2 min | Phase 5 | .dockerignore, final README |
| 13:54 / 21:29 | ~10 min | Evaluation | SDD pillar audit (two attempts due to context limit) |

**Total active AI time**: ~2h 15min implementation + ~2h planning = ~4h 15min
**Total wall-clock time** (planning to final commit): ~4h 15min

---

## 4. Key Decisions

### Decision 1: Cargo workspace with three crates
- **Choice**: `sdd-core` (library), `sdd-server` (axum binary), `sdd-cli` (CLI binary)
- **Alternative considered**: Single crate with two `[[bin]]` targets and a `lib.rs`
- **Rationale**: Developer explicitly wanted the CLI to be portable and evolve independently. Separate crates enforce clean dependency boundaries — the CLI doesn't pull in axum, the server doesn't pull in clap.

### Decision 2: Requirement type as free-form string, not enum
- **Choice**: Extract type prefix from ID (e.g., `SCS-SCAN-001` → type `SCS`), no validation against a fixed enum
- **Alternative considered**: Strict `enum: [FR, AR]` from the OpenAPI spec, or a config-driven allow-list
- **Rationale**: Developer's stated goal was a portable tool. Hardcoding `FR`/`AR` breaks for any project not using those prefixes.

### Decision 3: Timestamps in YAML, not derived
- **Choice**: `createdAt` and `updatedAt` as required fields in `requirements.yaml` and `tasks.yaml`, maintained by the developer
- **Alternatives considered**: (A) Git-derived via `git log`, (B) Scan-time derivation, (C) Hybrid with fallback
- **Rationale**: Developer initially accepted scan-time derivation, then **reversed the decision** after reviewing CLAUDE.md. Git-derived timestamps were fragile (shallow clones, Docker without .git). Scan-time timestamps were meaningless. YAML fields are simple, portable, and the author controls accuracy.

### Decision 4: Single CI workflow with branch-aware scan
- **Choice**: One `.github/workflows/ci.yml` with `continue-on-error: ${{ github.ref != 'refs/heads/main' }}`
- **Alternative considered**: Two separate workflow files (main.yml, dev.yml), or adding the scan step only in Phase 4
- **Rationale**: Developer wanted to see coverage progress on `dev` from commit 1 without blocking builds. Single file avoids DRY violation in CI config.

### Decision 5: Anchor regex to line start (post-implementation fix)
- **Choice**: Changed annotation regex from `(?://|#)\s*@req` to `^\s*(?:/{2,}|#)\s*@req`
- **Alternative considered by Claude**: Runtime string construction to hide `@req` patterns in test literals
- **Rationale**: Developer rejected the hack and asked for a proper regex fix. Anchoring to line start correctly excludes `@req` inside string literals.

### Decision 6: Commit message conventions
- **Choice**: `feat(scope): description [SCS-XXX-NNN]` for implementation commits, `chore(scope): description` for scaffolding
- **Developer correction**: Initially Claude listed all 19 requirement IDs in the Phase 1 commit. Developer rejected this — only implemented requirements should be listed.

---

## 5. What the Developer Controlled

### Pre-implementation (planning phase)
The developer drove every architectural decision through direct questioning. Specific areas where the developer overrode Claude's initial suggestion or probed deeper:

- **Timestamps**: Rejected scan-time derivation after reviewing first draft of CLAUDE.md. Forced reconsideration and chose YAML-stored timestamps.
- **Commit messages**: Rejected listing all requirement IDs in scaffold commit.
- **CI strategy**: Rejected a single blocking scan step. Designed the branch-aware approach.
- **Requirement types**: Guided the discussion from "which option" to choosing relaxed strings.
- **Docker base image**: Chose Alpine over debian-slim.
- **README strategy**: Chose evolving README over write-once-at-the-end.

### During implementation (Claude Code session)
The developer operated in a review-and-approve loop:

1. **Phase gate control**: Claude was explicitly told it could only proceed to the next phase with developer approval. Each phase required the developer to type "proceed" / "go ahead" / "lets start phase N."
2. **Build verification**: Claude ran `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` before every commit. The developer reviewed pass/fail output.
3. **Regex bug catch** (12:46 UTC): The developer read Claude's output "I need to construct those strings at runtime so the regex doesn't match" and challenged it directly: "Is it really a proper solution? Maybe regex has to be changed, so it would not capture the text in string literals." This forced a correct fix.
4. **Post-completion audit**: The developer asked Claude to evaluate the entire project against SDD pillars, producing a list of 7 violations that the developer now has as a backlog.

### Files the developer did NOT write
All code was generated by Claude Sonnet 4.6. The developer did not manually edit any source file. The developer's role was:
- Specification (what to build and how)
- Quality gate (when to proceed)
- Bug detection (regex issue)
- Process direction (commit strategy, CI design)

---

## 6. Course Corrections

### Correction 1: Timestamp strategy reversal
- **When**: During planning phase (Claude.ai conversation)
- **Issue**: Developer initially agreed to derive timestamps at scan time. After reviewing the first draft of CLAUDE.md, realized scan-time timestamps are meaningless — they tell you when the tool ran, not when the requirement was created.
- **How caught**: Developer re-read CLAUDE.md and said "I didn't really like the thing about timestamps. Lets rethink."
- **Resolution**: Switched to YAML-stored timestamps maintained by the developer. Both CLAUDE.md and PLAN.md were updated.

### Correction 2: Phase 1 commit message
- **When**: During planning phase
- **Issue**: PLAN.md initially had the Phase 1 commit as `feat(scaffold): ... [SCS-PARSE-001, SCS-PARSE-002, ... all 19 IDs]`. This violates traceability — the commit doesn't implement any requirements.
- **How caught**: Developer reviewed the PLAN.md and said "I believe all commits should only mention those requirements that have been implemented."
- **Resolution**: Changed to `chore(scaffold): project structure, requirements spec, fixtures, CI workflow` with no requirement IDs.

### Correction 3: CI workflow blocking on self-hosting scan
- **When**: During planning phase
- **Issue**: The CI workflow had the self-hosting scan as a blocking step. This would fail on every commit from Phase 1 through Phase 3.
- **How caught**: Developer pointed out "the CI workflow requires strict scan, so it would only work with the last commit... all the commits before would be labeled as failing."
- **Resolution**: Single workflow with `continue-on-error` based on branch. Added `dev` branch strategy and local scan script.

### Correction 4: Regex matching string literals (the "runtime construction" hack)
- **When**: 2026-04-03T12:46:56Z (during Phase 4)
- **Issue**: Claude's annotation regex `(?://|#)\s*@req` matched `@req` inside string literals in test files (e.g., `"// @req FR-FULL-001\n"`). Claude's initial fix was to construct those strings at runtime using `format!()` to hide the pattern from the scanner.
- **How caught**: Developer read Claude's explanation and challenged it: "Is it really a proper solution? Maybe regex has to be changed, so it would not capture the text in string literals."
- **Resolution**: Claude anchored the regex to line start: `^\s*(?:/{2,}|#)\s*@req`. This correctly excludes `@req` inside string literals (which never start at position 0 on a line). Also fixed `//` to `/{2,}` to handle doc comments (`///`). Committed as a separate `fix(scanner)` commit with `[SCS-SCAN-001]`.

### Correction 5: Scanner skipping root directory
- **When**: 2026-04-03T12:40:38Z (during Phase 4, self-hosting attempt)
- **Issue**: The scanner found 0 annotations when scanning `.` because `is_skipped_dir` treated the root directory `.` as hidden (starts with `.`).
- **How caught**: Claude ran the self-hosting scan and got 0 results, then debugged it.
- **Resolution**: Skip depth-0 entries from the hidden-directory check. This was caught and fixed by Claude autonomously, not by the developer.

---

## 7. Self-Assessment

### Traceability — Well Covered ✅
- All 19 requirements defined in `requirements.yaml` with MUST/SHOULD directives
- Every implementation function carries `/// @req SCS-XXX-NNN`
- Every test carries `// @req SCS-XXX-NNN`
- All behavioral commits reference implemented requirement IDs
- Self-hosting scan enforced in CI (blocking on `main`)
- The scaffold commit correctly uses `chore()` with no requirement IDs

### DRY — Needs Improvement ⚠️
- Core types defined once in `sdd-core`, shared by server and CLI — good
- Regex pattern compiled once via `OnceLock` — good
- OpenAPI spec lives in one file, README summarizes without duplicating — good
- **But**: Three sets of duplicated logic in `handlers.rs` (orphan parsing, sort/order validation, scan-result guard). These were identified in the post-completion audit but not fixed. A helper module or middleware extractors would eliminate the duplication.

### Deterministic Enforcement — Needs Improvement ⚠️
- `cargo fmt`, `clippy`, `test`, `build --release` all automated in CI — good
- Self-hosting scan with branch-aware strictness — good
- Local `scripts/scan.sh` with `--no-strict` option — good
- **But**: No OpenAPI schema validation against actual responses. The API spec file exists but is not machine-verified. No `cargo audit` for dependency vulnerabilities.

### Parsimony — Needs Improvement ⚠️
- Three-crate workspace is minimal and justified — good
- No dead code, no premature abstractions — good
- CLAUDE.md uses directive vocabulary throughout — good
- **But**: Unused `tokio` dependency in `sdd-cli` (CLI is synchronous). Unused `serde_json` in `sdd-core` (JSON serialization happens in the server crate). These are small but violate the principle.
