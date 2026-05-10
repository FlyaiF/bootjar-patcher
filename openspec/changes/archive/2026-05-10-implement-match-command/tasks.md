## 1. Core Matching

- [x] 1.1 Add input file discovery for file and directory roots.
- [x] 1.2 Add candidate match data types and statuses.
- [x] 1.3 Build match targets from outer and nested jar index entries.
- [x] 1.4 Implement exact relative-path matching.
- [x] 1.5 Implement filename matching with reason strings.
- [x] 1.6 Mark unique exact matches as `selected`, ambiguous matches as `needs-selection`, and misses as `no-match`.

## 2. Candidates YAML

- [x] 2.1 Add candidates YAML rendering for match results.
- [x] 2.2 Include `kind`, `version`, `source`, and per-input matches.
- [x] 2.3 Ensure candidate YAML is not patch-plan YAML.

## 3. CLI Match Command

- [x] 3.1 Add `bootjar-patcher match --jar <jar> --inputs <path> [--out <file>]`.
- [x] 3.2 Print candidates YAML to standard output when `--out` is absent.
- [x] 3.3 Write candidates YAML to `--out` when provided.
- [x] 3.4 Return failure for invalid jars and missing input paths.

## 4. Verification

- [x] 4.1 Add unit tests for exact unique match selection.
- [x] 4.2 Add unit tests for ambiguous filename matches.
- [x] 4.3 Add unit tests for no-match results and missing input paths.
- [x] 4.4 Add CLI integration tests for stdout and `--out` behavior.
- [x] 4.5 Run `cargo fmt`.
- [x] 4.6 Run `cargo test`.
- [x] 4.7 Run `openspec validate --all`.
