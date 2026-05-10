## Context

`apply` can now replace outer entries and entries inside nested jars. Replacing
an entire nested jar is a simpler but distinct operation: the replacement bytes
come from an external jar file and must be written to the outer executable jar as
a STORED `BOOT-INF/lib/*.jar` entry.

## Goals / Non-Goals

**Goals:**

- Treat `replace-entry` targets for direct `BOOT-INF/lib/*.jar` entries as whole
  nested jar replacement.
- Validate that the replacement source is readable as a jar before writing.
- Force the outer replacement entry to STORED.
- Preserve existing outer and nested-entry replacement behavior.

**Non-Goals:**

- No replacement of arbitrary non-`BOOT-INF/lib` jar files as a special case.
- No Maven/Gradle dependency metadata updates.
- No signed-jar repair.

## Decisions

- Detect whole nested jar replacement by target path shape:
  `BOOT-INF/lib/<name>.jar` without nested `!` syntax.
- Reuse the existing outer jar rewrite path by assigning replacement bytes and a
  forced `Stored` compression method for those targets.
- Open replacement nested jars with the zip reader before output creation. This
  catches invalid replacement jars before writing a partial output file.

## Risks / Trade-offs

- Replacement jars are validated only as readable ZIP/JAR containers -> deeper
  semantic compatibility is outside this tool's scope.
- STORED output may produce larger executable jars -> required by Spring Boot
  loader constraints.
