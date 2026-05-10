## Why

`apply` can replace outer jar entries, but the main Spring Boot patching use case
often targets classes inside nested dependency jars. This change adds nested
entry replacement while preserving Spring Boot's requirement that outer
`BOOT-INF/lib/*.jar` entries remain STORED.

## What Changes

- Support `replace-entry` targets using nested archive path syntax.
- Group operations targeting the same nested jar.
- Rewrite each affected nested jar once.
- Write affected nested jars back into the outer jar as STORED entries.
- Validate missing nested jars, missing inner entries, and invalid nested jar
  inputs with explicit errors.
- Defer whole nested jar replacement to the next slice.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify nested entry replacement and grouped nested rewrite
  behavior for `apply`.

## Impact

- `crates/bootjar-core`: nested operation planning, nested jar rewrite logic,
  STORED outer entry handling, and tests.
- `crates/bootjar-cli`: integration coverage for nested replacement behavior.
