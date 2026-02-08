---
name: scriv-cli-agent
description: Operate Scrivener 3 projects via the scriv CLI with safe binder-path handling, metadata updates, sync/conflict workflows, compile, and Git-aware automation. Use when an agent must read or mutate .scriv projects from the terminal (Codex or Claude Code), especially for tree operations, chapter/document edits, notes/synopsis maintenance, conflict resolution, and formatting-safe workflows.
---

# Scriv CLI Agent

Follow this workflow when working on a Scrivener project.

## 1. Resolve Target Project
- Discover the project path first (`*.scriv` directory).
- Prefer absolute paths in automation and scripts.
- Confirm project health before mutating:
  - `scriv --project <path> project validate --strict`
  - `scriv --project <path> sync status`

## 2. Use Canonical Binder Paths
- Treat root `Draft` prefix as optional for command input.
- Use one style consistently per command batch.
- For machine-safe targeting, prefer UUID ids from `tree ls --recursive`.

## 3. Prefer Safe Mutation Sequence
- For structure changes:
  - `tree mkdir`, `tree mkdoc`, `tree mv`, `tree reorder`, `tree rm`
- For document/meta changes:
  - `doc cat`, `doc write`, `doc append`, `doc prepend`, `doc edit`
  - `meta notes get|set`, `meta synopsis get|set`
- After large mutation batches, run:
  - `sync status`
  - `project validate --strict`

## 4. Preserve Rich Formatting
- Treat `content.rtf` as authoritative for rich-text formatting.
- Prefer `doc prepend` for additive intro text on rich chapters.
- If `doc write` or full rewrite is rejected for formatting safety, switch to:
  - metadata-only updates in CLI, or
  - direct editing in Scrivener UI.

## 5. Handle Conflicts Deterministically
- On `sync push` conflict (exit 5):
  - `conflict status`
  - `conflict resolve --id <uuid> --use mirror|project|manual`
  - rerun `sync push`
- Verify terminal state with `sync status`.

## 6. Keep Package Hygiene
- Avoid creating raw `Files/Data/<UUID>` entries without binder updates.
- If recovery warnings appear in Scrivener, inspect binder and data UUID parity.
- Ensure `docs.checksum` and binder UUIDs are in sync after writes.

## 7. Automation-Friendly Output
- Use `--json` for agent parsing.
- Handle non-zero exit codes explicitly:
  - `0` success
  - `5` conflict
  - `6` compile unavailable/failure

## 8. Recommended Command Snippets
- Project info: `scriv --project <path> project info --json`
- Chapter list: `scriv --project <path> tree ls --recursive`
- Read scene: `scriv --project <path> doc cat --id <uuid>`
- Set synopsis: `scriv --project <path> meta synopsis set --id <uuid> --text "..."`
- Set notes from stdin:
  - `scriv --project <path> meta notes set --id <uuid> --stdin`
