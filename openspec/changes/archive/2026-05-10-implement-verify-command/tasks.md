## 1. Core Verification

- [x] 1.1 Add a verification report model.
- [x] 1.2 Check that the input jar can be opened.
- [x] 1.3 Check all direct `BOOT-INF/lib/*.jar` entries are STORED.
- [x] 1.4 Detect signed jar metadata under `META-INF`.

## 2. CLI Verify Command

- [x] 2.1 Add `bootjar-patcher verify <jar>`.
- [x] 2.2 Print readable jar and nested jar storage status.
- [x] 2.3 Print signed metadata warnings.
- [x] 2.4 Return failure for invalid jars or non-STORED nested jars.

## 3. Verification

- [x] 3.1 Add core tests for stored nested jars.
- [x] 3.2 Add core tests for compressed nested jar failure.
- [x] 3.3 Add core tests for signed metadata warnings.
- [x] 3.4 Add CLI integration tests for success and failure cases.
- [x] 3.5 Run `cargo fmt`.
- [x] 3.6 Run `cargo test`.
- [x] 3.7 Run `openspec validate --all`.
