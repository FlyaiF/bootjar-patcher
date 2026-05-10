## Context

`apply` writes a new jar through the core library and the CLI reports the propagated result. The current writer knows how to force touched nested jars under `BOOT-INF/lib/*.jar` to STORED, but it does not run the same verification pass that the user can run manually with `verify`.

Spring Boot requires nested jars in the outer executable jar to be STORED. A patch can otherwise succeed at the zip-write layer while still producing an invalid executable artifact.

## Goals / Non-Goals

**Goals:**

- Verify every successfully written apply output before returning success.
- Reuse existing verification logic so `apply` and `verify` enforce the same nested-jar storage rule.
- Surface a clear apply error that names the output jar and failing nested jar paths.
- Keep core behavior in `bootjar-core`; the CLI should continue to format propagated errors.

**Non-Goals:**

- Do not add JVM-based validation.
- Do not remove a written output jar after verification failure.
- Do not change candidate generation or patch-plan syntax.

## Decisions

- Run `verify_jar(output_jar)` immediately after `rewrite_outer_jar_with_plan` succeeds. This keeps verification in the core apply API, so all callers get the safety check without duplicating CLI logic.
- Add apply-specific error variants for post-write verification failure and verification read failure. This keeps failure messages contextual instead of exposing a standalone inspect error after a write operation.
- Leave the output file in place on verification failure. The artifact has already been written, and keeping it supports debugging while the non-zero result prevents callers from treating it as a valid patch.

## Risks / Trade-offs

- Previously successful apply runs can now fail when the input already contains an untouched compressed nested jar. This is intended because the resulting artifact violates the Spring Boot storage requirement.
- Verification adds a second read of the output jar. The cost is acceptable because apply already performs a full rewrite and correctness matters more than avoiding one validation pass.
