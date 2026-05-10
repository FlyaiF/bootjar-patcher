## Why

`apply` now rewrites executable jars, so users need a direct check that the
result is readable and still satisfies Spring Boot's nested jar storage rule.
`verify` closes the basic patch loop before adding broader release polish.

## What Changes

- Add a core verification report for readable jars.
- Check every direct `BOOT-INF/lib/*.jar` outer entry is STORED.
- Report signed jar metadata when present.
- Add `bootjar-patcher verify <jar>`.
- Return failure when the jar cannot be opened or any nested jar outer entry is
  not STORED.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify verify command behavior for readable jars, nested
  jar STORED checks, invalid jars, and signed metadata warnings.

## Impact

- `crates/bootjar-core`: verification report API and tests.
- `crates/bootjar-cli`: `verify` command output, exit codes, and integration
  tests.
