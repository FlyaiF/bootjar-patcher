## Context

The apply workflow now writes patched jars. The first verification slice should
check the invariant most likely to break executable Spring Boot jars: nested
dependency jars under `BOOT-INF/lib` must be STORED in the outer jar. The core
index already records nested jar compression status, so verify can build on it.

## Goals / Non-Goals

**Goals:**

- Verify the jar can be opened.
- Report whether all direct `BOOT-INF/lib/*.jar` entries are STORED.
- Warn when signed jar metadata is present.
- Provide CLI output and failure exit codes for failed checks.

**Non-Goals:**

- No patch metadata tracking in this slice.
- No target existence checks from a plan file.
- No signature repair or cryptographic validation.

## Decisions

- Build verification on top of the existing `JarIndex` model. That avoids a
  second zip traversal model and keeps inspect/verify behavior consistent.
- Treat non-STORED nested jars as verification failures. Signed metadata is only
  a warning because the spec says it SHOULD warn.
- Use a compact human-readable CLI report first; structured output can be added
  later if needed.

## Risks / Trade-offs

- Signed metadata detection is shallow and filename-based -> enough to warn
  users that signatures may be invalid after patching.
- Verify does not prove the application boots -> this tool intentionally has no
  JVM runtime requirement.
