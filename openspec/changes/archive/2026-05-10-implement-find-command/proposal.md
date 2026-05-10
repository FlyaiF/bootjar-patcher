## Why

`inspect` confirms jar shape, but users still need a direct way to discover exact
archive paths before writing patch plans. `find` is the next discovery primitive
because matching and patch planning depend on reliable search across both outer
entries and nested jars.

## What Changes

- Add indexing of entries inside nested jars under `BOOT-INF/lib/*.jar`.
- Add a core search API that matches outer and nested archive paths by filename
  or path substring.
- Add `bootjar-patcher find <jar> <query>` to print matching archive paths.
- Keep readable non-Spring jars searchable for outer entries.
- Return a failure exit code for unreadable or invalid jars.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify `find` behavior for outer entries, nested jar
  entries, output path syntax, no-match results, and invalid input jars.

## Impact

- `crates/bootjar-core`: nested jar entry indexing, search model, and tests.
- `crates/bootjar-cli`: `find` command parsing, output formatting, and exit
  behavior.
- No archive writing is introduced; Spring Boot STORED nested-jar constraints are
  read and reported but not modified by this change.
