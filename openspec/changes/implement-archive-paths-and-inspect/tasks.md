## 1. Core Archive Paths

- [x] 1.1 Define `ArchivePath` and parse errors in `bootjar-core`.
- [x] 1.2 Implement parsing for outer archive paths.
- [x] 1.3 Implement parsing for nested `outer.jar!/inner/path` archive paths.
- [x] 1.4 Normalize safe backslash separators to `/`.
- [x] 1.5 Reject absolute paths, drive-prefixed paths, empty segments, `.`, `..`, empty nested components, and multiple `!` separators.
- [x] 1.6 Add unit tests for valid outer paths, valid nested paths, normalization, and invalid path cases.

## 2. Jar Indexing

- [x] 2.1 Add a jar reader/index model in `bootjar-core`.
- [x] 2.2 Record entry path, compression method, uncompressed size, compressed size, and CRC32 when available.
- [x] 2.3 Detect `BOOT-INF/classes` layout markers.
- [x] 2.4 Detect `BOOT-INF/lib/*.jar` entries.
- [x] 2.5 Detect whether each nested jar entry is STORED or compressed in the outer jar.
- [x] 2.6 Detect likely Spring Boot launcher entries.
- [x] 2.7 Add synthetic fixture helpers for valid Spring Boot jars, non-Spring jars, and invalid jar input.

## 3. Inspect Command

- [x] 3.1 Define an inspect report type in `bootjar-core`.
- [x] 3.2 Implement inspect report generation from the jar index.
- [x] 3.3 Add `bootjar-patcher inspect <jar>` in `bootjar-cli`.
- [x] 3.4 Format human-readable inspect output with Spring Boot layout and nested jar storage status.
- [x] 3.5 Return a failure exit code for unreadable or invalid jars.
- [x] 3.6 Keep readable non-Spring jars as successful inspections that report absent markers.

## 4. Verification

- [x] 4.1 Run `cargo fmt`.
- [x] 4.2 Run `cargo test`.
- [x] 4.3 Manually inspect command output for valid Spring Boot, non-Spring, and invalid jar fixtures.
