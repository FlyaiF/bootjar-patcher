## Why

Candidates YAML is useful for review, but users also need a fast way to turn
safe matches into patch-plan operations. Snippet output provides copyable YAML
while preserving the rule that ambiguous matches must not be mistaken for
reviewed operations.

## What Changes

- Add `--format snippets` to `bootjar-patcher match`.
- Render selected candidate matches as copyable `replace-entry` patch-plan
  snippets.
- Render ambiguous and no-match results as comments so they require user review.
- Support snippets output to stdout or `--out`, matching existing candidates
  output routing.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: Clarify snippet output behavior for selected, ambiguous,
  and no-match candidate results.

## Impact

- `crates/bootjar-core`: snippet renderer over the existing candidate model and
  tests.
- `crates/bootjar-cli`: `--format snippets` parsing and integration tests.
- No archive writing is introduced.
