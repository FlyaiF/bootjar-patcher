## Context

`match` currently produces structured candidates YAML. The next spec requirement
asks for snippets that users can copy into a patch plan. The existing candidate
model already has enough status and target information to render this without
changing matching behavior.

## Goals / Non-Goals

**Goals:**

- Add a snippets formatter for `CandidateFile`.
- Emit uncommented `replace-entry` operations only for `selected` matches.
- Emit ambiguous and no-match information as comments.
- Reuse existing `--out` routing.

**Non-Goals:**

- No patch-plan parser or validator.
- No apply command.
- No automatic conversion of ambiguous candidates into operations.

## Decisions

- Keep snippets as a formatter over `CandidateFile`. Matching remains unchanged,
  and future output formats can follow the same pattern.
- Use the input file path as the `with` value and the selected archive path as
  the `target` value.
- Prefix all non-selected result lines with `#` so commented snippets are visibly
  not executable patch operations.

## Risks / Trade-offs

- Commented YAML is intentionally conservative and may require manual editing ->
  this is the point of the safety boundary for ambiguous matches.
- Snippets are not validated as patch plans yet -> patch-plan validation belongs
  to the upcoming apply/parser changes.
