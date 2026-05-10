# Tasks: Initialize bootjar-patcher

## 1. Project setup

- [ ] Create Rust workspace.
- [ ] Create `crates/bootjar-core`.
- [ ] Create `crates/bootjar-cli`.
- [ ] Add CI jobs for Linux, Windows, and macOS.
- [ ] Add formatting and linting commands.
- [ ] Add minimal test fixture generation strategy.

## 2. Core archive model

- [ ] Implement `ArchivePath` parser for outer paths.
- [ ] Implement `ArchivePath` parser for nested `outer.jar!/inner/path` syntax.
- [ ] Normalize Windows input separators to jar-style `/`.
- [ ] Add tests for path parsing and invalid path handling.

## 3. Jar indexing

- [ ] Implement outer jar index reader.
- [ ] Detect likely Spring Boot executable jar layout.
- [ ] Detect `BOOT-INF/classes`.
- [ ] Detect `BOOT-INF/lib/*.jar`.
- [ ] Record compression method, size, compressed size, and CRC32 where available.
- [ ] Optionally index entries inside nested jars.
- [ ] Add tests with synthetic fixtures.

## 4. CLI inspect/tree/find

- [ ] Implement `inspect`.
- [ ] Implement `tree`.
- [ ] Implement `find`.
- [ ] Add table output.
- [ ] Add JSON output where useful.
- [ ] Add integration tests for command output basics.

## 5. Candidate matching

- [ ] Walk user input files.
- [ ] Match by exact relative path.
- [ ] Match by filename.
- [ ] Match common Spring resource names.
- [ ] Parse `.class` internal class names when possible.
- [ ] Score candidates with reason strings.
- [ ] Mark exact unambiguous matches as selected.
- [ ] Mark ambiguous matches as needs-selection.
- [ ] Mark unmatched files as no-match.
- [ ] Emit candidates YAML.
- [ ] Emit patch-plan snippet format.
- [ ] Add tests for ambiguous duplicate filenames.

## 6. Patch-plan format

- [ ] Define YAML schema for `kind: patch-plan`.
- [ ] Parse replace-entry operations.
- [ ] Parse add-entry operations.
- [ ] Parse delete-entry operations.
- [ ] Validate source files.
- [ ] Validate target paths.
- [ ] Reject applying `kind: candidates` files directly.
- [ ] Add user-friendly validation errors.

## 7. Patch execution

- [ ] Implement outer entry replacement.
- [ ] Implement replacement inside nested jars.
- [ ] Group nested operations by containing nested jar.
- [ ] Rewrite each affected nested jar once.
- [ ] Rewrite outer jar once.
- [ ] Force `BOOT-INF/lib/*.jar` outer entries to STORED.
- [ ] Compute size and CRC32 for STORED entries before writing.
- [ ] Write to temp file before final output.
- [ ] Add tests for resource replacement under `BOOT-INF/classes`.
- [ ] Add tests for class replacement inside nested jar.
- [ ] Add tests for whole nested jar replacement.

## 8. Verification

- [ ] Implement `verify`.
- [ ] Check output jar can be reopened.
- [ ] Check Spring Boot loader entries are preserved when present.
- [ ] Check all `BOOT-INF/lib/*.jar` entries are STORED.
- [ ] Check patched targets exist.
- [ ] Warn on signed jar metadata.
- [ ] Warn on paths differing only by case.
- [ ] Add verification tests.

## 9. Release readiness

- [ ] Document CLI usage.
- [ ] Document patch-plan YAML.
- [ ] Document candidates YAML.
- [ ] Document Spring Boot constraints.
- [ ] Add examples for simple and complex patch scenarios.
- [ ] Add GitHub release packaging for target platforms.
