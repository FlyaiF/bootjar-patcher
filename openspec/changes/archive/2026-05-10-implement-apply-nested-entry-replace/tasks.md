## 1. Operation Planning

- [x] 1.1 Allow parsed `replace-entry` operations to keep nested archive paths.
- [x] 1.2 Partition apply operations into outer replacements and nested replacements.
- [x] 1.3 Group nested replacements by containing `BOOT-INF/lib/*.jar`.

## 2. Nested Jar Rewrite

- [x] 2.1 Validate affected nested jar entries exist in the outer jar.
- [x] 2.2 Validate inner replace targets exist in each nested jar.
- [x] 2.3 Rewrite each affected nested jar once.
- [x] 2.4 Replace inner entries with replacement bytes.
- [x] 2.5 Preserve unmodified inner entries.
- [x] 2.6 Write affected outer nested jar entries as STORED.

## 3. Verification

- [x] 3.1 Add unit tests for replacing a nested class/resource.
- [x] 3.2 Add unit tests for grouping multiple operations in the same nested jar.
- [x] 3.3 Add unit tests for missing nested jar and missing inner target failures.
- [x] 3.4 Add CLI integration coverage for nested replacement.
- [x] 3.5 Run `cargo fmt`.
- [x] 3.6 Run `cargo test`.
- [x] 3.7 Run `openspec validate --all`.
