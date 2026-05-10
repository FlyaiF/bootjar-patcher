## Context

The repository already contains a Rust workspace with `bootjar-core` and `bootjar-cli`, but the implementation is still placeholder-level. The baseline spec covers the full product, so this change deliberately implements the lowest-level slice first: archive paths and `inspect`.

This slice should establish the contracts that later commands reuse:

- one parser for outer and nested archive paths
- one normalized jar-path representation
- one jar index that records enough metadata for Spring Boot layout checks
- one CLI command that exercises the index without modifying archives

## Goals / Non-Goals

**Goals:**

- Keep archive parsing and jar inspection in `bootjar-core`.
- Keep argument parsing, terminal output, and process exit codes in `bootjar-cli`.
- Normalize safe filesystem separators to jar-style `/`.
- Reject unsafe or ambiguous archive paths before later write operations depend on them.
- Report whether `BOOT-INF/classes`, `BOOT-INF/lib`, launcher entries, and nested jar storage constraints are present.

**Non-Goals:**

- No patch-plan parsing.
- No candidate matching.
- No archive rewriting.
- No `tree`, `find`, `match`, `apply`, or `verify` commands.
- No signed-jar repair or signature policy beyond detection in the inspect model if cheap to record.

## Decisions

1. Represent archive paths as owned normalized strings.

   `ArchivePath::Outer { path }` and `ArchivePath::Nested { outer_jar, inner_path }` are enough for v1 and match the baseline design. Later code can add typed wrappers if validation grows more complex.

2. Treat `!` as the only nested separator.

   The parser accepts exactly one nested separator. Empty outer paths, empty inner paths, and paths with multiple `!` separators are invalid. This avoids silently accepting paths later write code cannot address deterministically.

3. Normalize only path separators, not semantic path components.

   Backslashes become `/`. Absolute paths, Windows drive prefixes, empty path segments, `.`, and `..` are rejected. The tool should not turn local filesystem paths into archive destinations by guessing intent.

4. Build a lightweight jar index before command-specific reports.

   The index should record entry name, compression method, size, compressed size, CRC32 when available, and whether an entry is a candidate nested jar under `BOOT-INF/lib/*.jar`. `inspect` can derive its report from this index, and future `tree`/`find` work can extend it.

5. Keep `inspect` read-only and non-fatal for non-Spring jars.

   A readable jar that does not look like Spring Boot should produce a report showing missing layout markers rather than failing. I/O errors and invalid ZIP/JAR structure should fail.

## Risks / Trade-offs

- Strict path validation may reject unusual but technically valid ZIP entry names. That is acceptable for a patching tool because unsafe ambiguity is worse than supporting every possible archive edge case.
- A lightweight index that does not read nested jar contents is sufficient for `inspect`, but later `find` and `match` will need optional nested indexing.
- Spring Boot launcher detection can vary by version. The initial check should look for known launcher package prefixes and report presence rather than trying to classify every Spring Boot generation exactly.
