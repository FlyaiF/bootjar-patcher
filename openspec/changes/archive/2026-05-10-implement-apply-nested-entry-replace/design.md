## Context

The current apply implementation parses patch plans and rewrites the outer jar,
but rejects nested archive targets. The jar index and archive path parser already
recognize `BOOT-INF/lib/a.jar!/path` syntax, so the next implementation step is
execution: rewrite the affected nested jar bytes and write them back to the
outer jar as STORED.

## Goals / Non-Goals

**Goals:**

- Accept nested `replace-entry` targets under direct `BOOT-INF/lib/*.jar`.
- Group nested operations by containing jar.
- Rewrite each affected nested jar once.
- Preserve unmodified nested jar entries.
- Force affected outer nested jar entries to STORED.

**Non-Goals:**

- No whole nested jar replacement in this slice.
- No add/delete operations.
- No recursive jar-in-jar behavior.
- No verify command.

## Decisions

- Partition replacement operations into outer replacements and nested
  replacements after plan parsing. This keeps the outer rewrite loop as the
  single place that writes final entries.
- Read each affected nested jar into memory, rewrite it to a byte buffer, and
  use that buffer as the replacement bytes for its outer `BOOT-INF/lib/*.jar`
  entry. This matches the existing in-memory approach used by nested indexing
  and keeps correctness simple.
- Validate all affected nested jars and inner targets before creating the output
  jar. This avoids writing a partial output when a nested target is missing.
- Use STORED compression for affected outer nested jar entries regardless of the
  original method.

## Risks / Trade-offs

- In-memory nested rewrites are simple but can be expensive for very large
  nested jars -> acceptable for this slice and can be optimized later.
- Metadata preservation inside rewritten nested jars is best effort -> content,
  compression method, modification time, and Unix mode are preserved where the
  zip crate exposes them.
