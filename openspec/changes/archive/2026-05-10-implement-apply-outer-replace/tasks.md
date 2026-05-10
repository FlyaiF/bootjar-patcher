## 1. Patch Plan Parsing

- [x] 1.1 Add YAML parsing dependencies.
- [x] 1.2 Define patch-plan data types for `kind`, `version`, and `replace-entry`.
- [x] 1.3 Reject non-`patch-plan` documents, including `kind: candidates`.
- [x] 1.4 Validate archive path syntax for replace targets.

## 2. Apply Execution

- [x] 2.1 Add a core `apply_patch_plan` API.
- [x] 2.2 Validate replacement source files before writing output.
- [x] 2.3 Validate outer replace targets exist in the input jar.
- [x] 2.4 Reject nested replace targets as unsupported in this slice.
- [x] 2.5 Rewrite the outer jar to a new output path.
- [x] 2.6 Replace matching outer entries with replacement bytes.
- [x] 2.7 Preserve unmodified entries.

## 3. CLI Apply Command

- [x] 3.1 Add `bootjar-patcher apply --jar <jar> --plan <plan> --out <jar>`.
- [x] 3.2 Return success for valid outer-entry replacement.
- [x] 3.3 Return failure for candidates files, missing sources, missing targets, and nested targets.

## 4. Verification

- [x] 4.1 Add unit tests for patch-plan parsing and candidates rejection.
- [x] 4.2 Add unit tests for replacing `BOOT-INF/classes` resources.
- [x] 4.3 Add unit tests for missing source, missing target, and nested target failures.
- [x] 4.4 Add CLI integration tests for successful apply and candidates rejection.
- [x] 4.5 Run `cargo fmt`.
- [x] 4.6 Run `cargo test`.
- [x] 4.7 Run `openspec validate --all`.
