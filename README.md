# scriv

`scriv` is a non-interactive CLI for working with Scrivener 3 projects (`.scriv`) in terminal-first workflows.

It is designed for automation, scripting, and agent usage (Codex / Claude Code), while preserving Scrivener package integrity and avoiding formatting-damaging writes.

## Status

This project is actively evolving.

Current implementation includes:
- Scrivener project discovery and binder import
- Binder/tree manipulation (folders, docs, moves, reorder, delete)
- Document and metadata operations (content, notes, synopsis)
- Mirror sync model with conflict handling
- Compile to Markdown/TXT (built-in), app compile stub on macOS
- Git passthrough wrappers
- Safety guards for rich-text formatting

## Core Model

`scriv` uses a mirror-first model:

1. Read/load project state from `.scrivx` + `Files/Data`
2. Materialize to a mirror directory (`<Project>.scriv-mirror`)
3. Apply command mutations
4. Sync/push with conflict detection
5. Persist metadata and safe content updates to native Scrivener package files

### Why this model
- Shell-friendly operations (`grep`, `sed`, `cat`, `jq`)
- Deterministic automation behavior
- Structured conflict flow
- Better guardrails around rich formatting

## Installation

### Prerequisites
- Rust toolchain (stable)
- macOS or Linux

### Build

```bash
cargo build --release
```

Binary:

```bash
./target/release/scriv
```

## Quick Start

### 1. Project info + validation

```bash
scriv --project "/path/to/Book.scriv" project info
scriv --project "/path/to/Book.scriv" project validate --strict
```

### 2. Inspect binder tree

```bash
scriv --project "/path/to/Book.scriv" tree ls --recursive
```

### 3. Create structure and documents

```bash
scriv --project "/path/to/Book.scriv" tree mkdir --path "Draft/Act II/New Chapter"
scriv --project "/path/to/Book.scriv" tree mkdoc --path "Draft/Act II/New Chapter/Scene 1"
```

### 4. Edit content and metadata

```bash
scriv --project "/path/to/Book.scriv" doc write --path "Draft/Act II/New Chapter/Scene 1" --stdin
scriv --project "/path/to/Book.scriv" meta synopsis set --path "Draft/Act II/New Chapter/Scene 1" --text "Scene synopsis"
scriv --project "/path/to/Book.scriv" meta notes set --path "Draft/Act II/New Chapter/Scene 1" --stdin
```

### 5. Sync status

```bash
scriv --project "/path/to/Book.scriv" sync status
```

## Command Surface

## Project
- `project create <name> [--dir <path>] [--template <name>]`
- `project info`
- `project validate [--strict]`
- `project doctor --check`

## Tree
- `tree ls [--path <binder-path>] [--recursive]`
- `tree mkdir --path <binder-path>`
- `tree mkdoc --path <binder-path>`
- `tree mv --from <path> --to <path>`
- `tree reorder --path <path> --before <path>`
- `tree reorder --path <path> --after <path>`
- `tree rm --path <path> [--force]`

## Documents
- `doc cat --id <uuid>|--path <binder-path>`
- `doc write --id|--path --from-file <file>|--stdin`
- `doc append --id|--path --from-file <file>|--stdin`
- `doc prepend --id|--path --from-file <file>|--stdin`
- `doc edit --id|--path [--set-title <title>] [--set-text <text>|--stdin]`

## Metadata
- `meta notes get --id|--path`
- `meta notes set --id|--path --from-file <file>|--stdin`
- `meta synopsis get --id|--path`
- `meta synopsis set --id|--path --text <text>|--stdin`

## Sync and Conflicts
- `sync pull`
- `sync push`
- `sync status`
- `conflict status`
- `conflict resolve --id|--path --use mirror|project|manual [--manual-file <path>]`

## Compile
- `compile run --format md --output <path>`
- `compile run --format txt --output <path>`
- `compile run --format app --output <path> [--preset <name>]` (macOS path, currently limited)

## Git wrappers
- `git status`
- `git diff`
- `git add`
- `git commit`
- `git log`
- `git restore`

## JSON Mode and Exit Codes

Use `--json` for machine-readable output:

```bash
scriv --project "/path/to/Book.scriv" --json project info
```

Exit codes:
- `0` success
- `1` runtime/general failure
- `2` invalid arguments
- `3` not found
- `4` validation failure
- `5` conflict detected
- `6` compile unavailable/failure

## Rich Text Safety

Scrivener stores rich text in RTF (`Files/Data/<UUID>/content.rtf`).

Safety rules in this CLI:
- Metadata updates (`notes`, `synopsis`) are safe and native.
- Prepend operations are formatting-aware for rich docs.
- Full rewrites that risk flattening rich formatting are blocked with an explicit error.
- If full content replacement is needed for styled docs, edit in Scrivener UI.

## Recovery Warning Prevention

This project includes safeguards to reduce Scrivener “Recovered Files” warnings:
- Orphan `Files/Data/<UUID>` pruning for unbound binder nodes
- `docs.checksum` regeneration during native writes
- Conflict cleanup and mirror/manifest rebinding on resolution

If you still see a recovery warning:
1. Close and reopen Scrivener project
2. Run `scriv ... project validate --strict`
3. Verify binder/data UUID parity before continuing

## Development

### Run checks

```bash
cargo check
cargo test
```

### Test coverage
- Unit tests in source modules
- Integration tests in `tests/integration_flow.rs`
- Regression tests in `tests/regression_fixes.rs`

Regression tests cover:
- Path normalization (`Draft/...` handling)
- Conflict resolution stability
- Orphan/checksum hygiene
- Metadata-only save preserving rich content files

## Agent Skill

A ready-to-use skill for Codex/Claude-style agents is included:

- `/Users/eoghancollins/Personal Tools/scrivenerCLI/skills/scriv-cli-agent/SKILL.md`
- `/Users/eoghancollins/Personal Tools/scrivenerCLI/skills/scriv-cli-agent/agents/openai.yaml`
- `/Users/eoghancollins/Personal Tools/scrivenerCLI/skills/scriv-cli-agent/references/commands.md`

Use this skill when an agent needs to safely automate `.scriv` projects via terminal workflows.

## Known Limitations

- macOS app compile bridge is still minimal.
- Full rich-text rewrite/edit support is intentionally conservative to avoid formatting loss.
- Extremely large projects can benefit from additional indexing/perf optimizations.

## License

No license file is currently included in this repository.
