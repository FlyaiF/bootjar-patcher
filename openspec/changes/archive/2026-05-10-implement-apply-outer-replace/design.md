## Context

The codebase already understands archive paths, jar indexing, matching, and
candidate/snippet output. `apply` needs a stricter boundary: only reviewed
`kind: patch-plan` YAML can be executed, and candidate files must fail early.
This slice should produce a real patched jar while avoiding nested jar rewrite
complexity.

## Goals / Non-Goals

**Goals:**

- Parse patch-plan YAML with `replace-entry` operations.
- Validate source file existence and target archive path safety.
- Reject candidates YAML.
- Replace existing outer entries in a new output jar.
- Preserve unmodified entries and common zip metadata where practical.

**Non-Goals:**

- No replacement inside nested jars.
- No whole nested jar replacement.
- No add/delete operations.
- No post-write verify command.

## Decisions

- Use `serde` and `serde_yaml` for plan parsing. Patch-plan parsing needs
  structured YAML handling, and ad hoc string parsing would make validation
  brittle.
- Reject nested archive paths in this slice with a clear unsupported-operation
  error. The parser can recognize them, but execution is intentionally scoped to
  outer entries.
- Rewrite the entire outer jar to a temporary output path supplied by the user.
  This avoids in-place mutation and establishes the pattern later nested rewrite
  work can reuse.
- Preserve each copied entry's compression method and use the original method
  for replaced entries. STORED entries are written from full replacement bytes so
  the zip crate can compute size and CRC correctly.

## Risks / Trade-offs

- Rewriting the whole jar is not the fastest path -> it is the safest baseline
  for deterministic output and future nested changes.
- Metadata preservation is best effort in this slice -> permissions and comments
  can be expanded later if real fixtures require them.
- Nested targets fail as unsupported -> this is explicit scope control, not a
  silent partial apply.
