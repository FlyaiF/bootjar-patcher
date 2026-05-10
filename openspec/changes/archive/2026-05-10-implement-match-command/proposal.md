## Why

Users can now discover archive paths with `find`, but still need an assistive
way to map replacement files to likely targets before writing reviewed patch
plans. `match` is the next step because it converts a patch directory into
structured candidates while preserving the safety rule that ambiguous matches
must not become patch operations automatically.

## What Changes

- Add a core candidate matching model with `selected`, `needs-selection`, and
  `no-match` statuses.
- Match input files against outer and nested archive paths by exact relative
  path and filename.
- Include candidate reason strings so users can understand why each target was
  suggested.
- Add `bootjar-patcher match --jar <jar> --inputs <path> [--out <file>]`.
- Emit candidates YAML for review or later conversion into a patch plan.
- Defer snippets output to a later formatter-specific change.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify the first `match` implementation slice for input
  walking, candidate statuses, candidate reasons, exact matches, ambiguous
  filename matches, no-match behavior, and candidates YAML output.

## Impact

- `crates/bootjar-core`: input file walking, target matching, candidate model,
  YAML rendering, and tests.
- `crates/bootjar-cli`: `match` argument parsing, output path handling, terminal
  output, and exit behavior.
- No archive writing is introduced; Spring Boot STORED nested-jar constraints are
  not modified by this change.
