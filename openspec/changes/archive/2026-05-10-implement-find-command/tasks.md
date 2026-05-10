## 1. Core Indexing

- [x] 1.1 Add a core model for nested jar entries using patch-plan archive path syntax.
- [x] 1.2 Index readable entries inside direct `BOOT-INF/lib/*.jar` nested jars.
- [x] 1.3 Preserve existing outer jar inspection behavior while adding nested indexing.

## 2. Core Find API

- [x] 2.1 Add a core find result type.
- [x] 2.2 Implement search over outer and nested archive paths.
- [x] 2.3 Match queries against full archive paths and entry filenames.

## 3. CLI Find Command

- [x] 3.1 Add `bootjar-patcher find <jar> <query>` argument handling.
- [x] 3.2 Print one matching archive path per line.
- [x] 3.3 Return success with no output for no matches.
- [x] 3.4 Return a failure exit code for unreadable or invalid jars.

## 4. Verification

- [x] 4.1 Add unit tests for nested jar indexing and nested find results.
- [x] 4.2 Add unit tests for outer path matching and no-match behavior.
- [x] 4.3 Run `cargo fmt`.
- [x] 4.4 Run `cargo test`.
- [x] 4.5 Run `openspec validate --all`.
