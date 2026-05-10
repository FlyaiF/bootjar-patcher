## Why

Discovery and candidate generation are implemented, but users cannot yet apply a
reviewed patch plan. The smallest useful apply slice is parsing reviewed
`patch-plan` YAML and replacing existing outer jar entries, especially
`BOOT-INF/classes` resources.

## What Changes

- Add patch-plan parsing for `kind: patch-plan`, `version: 1`, and
  `replace-entry` operations.
- Reject `kind: candidates` files when passed to `apply`.
- Validate replacement source files and target archive paths before writing.
- Add `bootjar-patcher apply --jar <jar> --plan <plan> --out <jar>`.
- Write a new output jar without mutating the input jar.
- Replace existing outer jar entries, including `BOOT-INF/classes/...`.
- Defer nested jar entry replacement and whole nested jar replacement to later
  changes.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify the first `apply` slice for reviewed patch-plan
  parsing, candidates rejection, output jar creation, and outer-entry
  replacement.

## Impact

- `crates/bootjar-core`: patch-plan parser, validation, outer jar rewrite logic,
  and tests.
- `crates/bootjar-cli`: `apply` argument parsing, error handling, and integration
  tests.
- Adds YAML parsing dependencies.
