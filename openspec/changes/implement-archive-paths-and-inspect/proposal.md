## Why

The project has a valid baseline product spec, but implementation needs a smaller first slice than the full patching workflow. Archive path parsing and jar inspection are the foundation for every later feature: matching, patch planning, patch execution, and verification all depend on the same path model and Spring Boot layout index.

## What Changes

- Implement the core `ArchivePath` model for outer and nested jar paths.
- Define concrete path safety behavior for normalized user input.
- Implement basic jar indexing needed by `inspect`.
- Implement the `inspect` CLI command for Spring Boot executable jar layout and nested jar storage reporting.
- Add focused tests for path parsing, invalid paths, Spring Boot layout detection, and nested jar STORED/DEFLATED reporting.
- Defer tree, find, match, apply, patch-plan parsing, and jar rewriting to later changes.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify archive path validation and inspect output behavior for the first implementation slice.

## Impact

- `crates/bootjar-core`: archive path parsing, normalization, jar indexing, inspect report model, and tests.
- `crates/bootjar-cli`: `inspect` command argument parsing, output formatting, and exit behavior.
- Dependencies may include ZIP/JAR reading and error handling crates.
