## 1. Whole Nested Jar Planning

- [x] 1.1 Detect direct `BOOT-INF/lib/*.jar` outer targets as whole nested jar replacements.
- [x] 1.2 Validate replacement source jars are readable before output creation.
- [x] 1.3 Keep existing nested entry replacement behavior unchanged.

## 2. Apply Execution

- [x] 2.1 Force whole nested jar replacement entries to STORED in the outer jar.
- [x] 2.2 Preserve replacement jar bytes exactly in the output jar entry.
- [x] 2.3 Return clear errors for invalid replacement nested jars.

## 3. Verification

- [x] 3.1 Add unit tests for whole nested jar replacement.
- [x] 3.2 Add unit tests for invalid replacement nested jar failure.
- [x] 3.3 Add CLI integration coverage for whole nested jar replacement.
- [x] 3.4 Run `cargo fmt`.
- [x] 3.5 Run `cargo test`.
- [x] 3.6 Run `openspec validate --all`.
