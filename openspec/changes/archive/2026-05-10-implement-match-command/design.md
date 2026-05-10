## Context

The current code can inspect jars and search archive entries, including direct
nested jars under `BOOT-INF/lib`. Users still have to manually translate a local
patch directory into reviewed patch-plan targets. `match` bridges that gap by
producing candidate YAML while keeping patch execution out of scope.

## Goals / Non-Goals

**Goals:**

- Walk input files from one or more file or directory roots.
- Compare input files to indexed outer and nested archive paths.
- Select exactly one exact relative-path match automatically.
- Mark filename-only ambiguity as `needs-selection`.
- Preserve `no-match` entries in the output.
- Emit stable candidates YAML without introducing patch operations.

**Non-Goals:**

- No snippet output in this slice.
- No patch-plan parsing or apply behavior.
- No bytecode parsing or semantic class-name matching.
- No matching against `WEB-INF/lib` nested jars yet.

## Decisions

- Keep matching in `bootjar-core` and output routing in `bootjar-cli`. This keeps
  candidate generation reusable for later snippets and UI formats.
- Use deterministic scoring: exact relative-path matches score higher than
  filename matches. A unique exact relative-path match becomes `selected`; all
  other one-or-more candidate cases become `needs-selection`.
- Render YAML manually from structured data for this first slice. The output
  schema is small, and avoiding a new serialization dependency keeps the change
  narrow. Structured parsing can be added when patch-plan parsing lands.
- Normalize input relative paths to jar-style `/` before matching.

## Risks / Trade-offs

- Manual YAML rendering can grow brittle as the schema expands -> keep rendering
  isolated and covered by tests.
- Filename-only matching is intentionally conservative -> it may require user
  review for cases that appear obvious, but it avoids unsafe auto-selection.
- Directory walking is recursive and eager -> acceptable for patch directories;
  very large inputs can be optimized later.
