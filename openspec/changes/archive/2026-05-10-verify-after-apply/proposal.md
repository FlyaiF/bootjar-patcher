## Why

`apply` already rewrites jars, but the safety contract says verification must fail the run when the written output violates Spring Boot nested-jar storage rules. Without an automatic post-write check, callers can produce an unusable fat jar unless they remember to run `verify` separately.

## What Changes

- Run verification after a successful `apply` write.
- Fail `apply` when the written output contains compressed `BOOT-INF/lib/*.jar` entries or otherwise cannot be verified.
- Report the verification failure with the output jar path and failing nested jar paths.
- Leave the written output jar on disk when post-write verification fails so callers can inspect the artifact.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: `apply` enforces the existing post-write verification safety rule.

## Impact

- Affects `crates/bootjar-core` apply flow and error reporting.
- Affects `crates/bootjar-cli` apply exit behavior through propagated core errors.
- Adds focused core and CLI coverage for post-write verification failure.
