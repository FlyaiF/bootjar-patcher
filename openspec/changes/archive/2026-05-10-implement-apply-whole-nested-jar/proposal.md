## Why

Nested entry replacement is implemented, but operators also need to replace an
entire dependency jar under `BOOT-INF/lib`. This is a common hotfix path when the
replacement artifact is already built and reviewed.

## What Changes

- Support `replace-entry` operations whose target is a whole
  `BOOT-INF/lib/*.jar` outer entry.
- Write replacement nested jars as STORED entries in the outer executable jar.
- Validate replacement jar readability before writing output.
- Preserve the existing behavior for outer resource replacement and nested entry
  replacement.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify whole nested jar replacement behavior for `apply`.

## Impact

- `crates/bootjar-core`: apply validation and output-entry compression handling.
- `crates/bootjar-cli`: integration coverage for whole nested jar replacement.
