## Context

The current core treats every input as one readable ZIP/JAR/WAR and supports one
optional nested level, such as `BOOT-INF/lib/a.jar!/com/acme/Foo.class`. ZIP
wrapper artifacts add one more container level: top-level scripts/config/templates,
plus one or more contained executable Spring Boot archives.

## Goals / Non-Goals

**Goals:**

- Support plain ZIP wrappers while preserving direct JAR/WAR behavior.
- Address wrapper files and contained archive contents with chained `!` paths.
- Verify Spring Boot nested library STORED rules for every contained executable archive.
- Preserve wrapper file compression and Unix mode when rewriting entries.

**Non-Goals:**

- TAR/TAR.GZ or other package formats.
- Executing or validating wrapper scripts.
- Inferring a contained fatjar from an unqualified `BOOT-INF/...` target.

## Decisions

- Use chained `!` paths instead of new YAML fields. This preserves `replace-entry`
  and naturally extends the current archive path model.
- Detect wrapper ZIPs by scanning readable `.jar` and `.war` entries for Spring Boot
  layouts. Unknown ZIPs remain readable archives but are not wrappers.
- Build one recursive index for public discovery. Direct wrapper entries are indexed
  by path, contained archive entries are indexed as `app/app.jar!/BOOT-INF/...`, and
  dependency entries as `app/app.jar!/BOOT-INF/lib/dep.jar!/inner`.
- Apply wrapper patches in two phases. First rewrite affected contained archives in
  memory using existing JAR/WAR replacement rules, then rewrite the wrapper ZIP with
  top-level replacements and changed contained archives.
- Verify direct JAR/WAR inputs exactly as today. For wrappers, verify the wrapper is
  readable and every contained executable archive passes nested library storage checks;
  the contained archive entry itself does not have to be STORED in the wrapper.

## Risks / Trade-offs

- More recursive indexing increases memory use because contained archives are read
  into memory. The first version accepts this to keep implementation deterministic.
- Multiple contained executable archives can make filename matches ambiguous. The
  matcher must keep those results as `needs-selection`.
- ZIP metadata preservation depends on what the `zip` crate exposes. Compression
  method, modified time, and Unix mode are preserved where available.
