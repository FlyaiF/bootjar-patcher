## MODIFIED Requirements

### Requirement: Emit copyable patch snippets

The system MUST support a snippets output format for `match`.

Snippet output MUST emit YAML `replace-entry` operations that the user can copy into a
patch plan for selected matches.

Ambiguous snippets MUST be commented or clearly marked so they are not mistaken for
safe automatic selections.

No-match results MUST be commented or clearly marked so they are not mistaken for
patch operations.

The `match` command MUST write snippets to `--out` when that option is provided.

#### Scenario: Emit snippet output

Given a target jar and an input file with candidate matches
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch --format snippets`
Then the output contains YAML patch operation snippets
And ambiguous candidate snippets are commented or clearly marked

#### Scenario: Emit selected replacement snippet

Given an input file with relative path `BOOT-INF/classes/application.yml`
And the target jar contains `BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch --format snippets`
Then the output contains an uncommented `replace-entry` operation
And the operation target is `BOOT-INF/classes/application.yml`
And the operation source is the input file path

#### Scenario: Comment no-match snippet result

Given an input file `Missing.class`
And the target jar has no matching archive path or filename
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch --format snippets`
Then the output does not contain an uncommented `replace-entry` operation for `Missing.class`
And the output comments that no match was found

#### Scenario: Write snippets to file

Given a target jar and an input file with candidate matches
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch --format snippets --out patch-snippets.yaml`
Then the command writes snippets to `patch-snippets.yaml`
And the command does not write snippets to standard output
