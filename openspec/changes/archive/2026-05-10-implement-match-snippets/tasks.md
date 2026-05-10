## 1. Core Snippet Rendering

- [x] 1.1 Add a snippets renderer over `CandidateFile`.
- [x] 1.2 Render selected matches as uncommented `replace-entry` operations.
- [x] 1.3 Render ambiguous matches as commented candidate choices.
- [x] 1.4 Render no-match results as comments.

## 2. CLI Format Selection

- [x] 2.1 Add `--format candidates|snippets` parsing to `match`.
- [x] 2.2 Keep candidates YAML as the default format.
- [x] 2.3 Route snippets output to stdout or `--out`.
- [x] 2.4 Reject unknown formats with a usage error.

## 3. Verification

- [x] 3.1 Add core tests for selected snippet rendering.
- [x] 3.2 Add core tests for ambiguous and no-match snippet comments.
- [x] 3.3 Add CLI integration tests for `--format snippets` stdout and `--out`.
- [x] 3.4 Run `cargo fmt`.
- [x] 3.5 Run `cargo test`.
- [x] 3.6 Run `openspec validate --all`.
