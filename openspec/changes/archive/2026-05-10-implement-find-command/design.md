## Context

The current implementation can parse archive paths and inspect outer jar layout.
`JarIndex` only records outer entries, so discovery cannot yet return paths for
classes or resources inside nested `BOOT-INF/lib/*.jar` archives. The `find`
command needs nested entry visibility while preserving the boundary where
`bootjar-core` owns archive traversal and `bootjar-cli` owns terminal behavior.

## Goals / Non-Goals

**Goals:**

- Index readable entries inside nested jars under `BOOT-INF/lib/*.jar`.
- Represent search results as copyable archive paths accepted by patch plans.
- Search by filename or path substring across both outer and nested entries.
- Add a small CLI command with clear success, no-match, and invalid-jar behavior.

**Non-Goals:**

- No candidate scoring, ambiguity handling, or YAML output.
- No patch-plan parsing or jar rewriting.
- No recursive jar-in-jar traversal beyond Spring Boot's outer jar plus direct
  nested jars under `BOOT-INF/lib`.

## Decisions

- Extend the core index with nested entries instead of creating a separate find
  scanner. This keeps one archive traversal model for future `match` and
  `verify` work.
- Use simple case-sensitive substring matching against full archive paths and
  basenames. This is deterministic, easy to explain, and sufficient for the
  first discovery slice. Case-insensitive matching can be added later if the
  spec calls for it.
- Treat unreadable nested jars as absent from nested search results for this
  change. The outer jar can still be inspected, and later verification can make
  nested jar readability stricter where needed.
- Return no matches as a successful command with empty output. Absence of a
  match is a valid search result, not a tool failure.

## Risks / Trade-offs

- Nested jar indexing reads nested jar bytes into memory -> acceptable for the
  first implementation; future large-jar optimization can stream or cap reads.
- Case-sensitive matching may miss user intent on case-insensitive filesystems
  -> documented behavior keeps results deterministic.
- Skipping malformed nested jars can hide entries inside bad dependencies ->
  acceptable for discovery; invalid outer jars still fail.
