# bootjar-patcher Specification

## Purpose

`bootjar-patcher` is a cross-platform command-line tool and reusable library for
patching Spring Boot executable fat jars safely and repeatably.

It supports direct patch execution and assistive discovery of candidate target paths.

## Definitions

### Outer jar

The Spring Boot executable fat jar passed as input to the tool.

### Nested jar

A jar file stored as an entry under `BOOT-INF/lib` in the outer jar.

### Archive path

A string identifying either an outer jar entry or a nested jar entry.

Outer archive path example:

```text
BOOT-INF/classes/application.yml
```

Nested archive path example:

```text
BOOT-INF/lib/order-module-1.4.2.jar!/com/acme/order/OrderCalculator.class
```

### Candidate file

A generated file containing possible matches between user replacement files and target
archive paths. A candidate file is not directly applied.

### Patch plan

A reviewed YAML file containing deterministic operations to apply to a jar.

## Requirements

### Requirement: Inspect Spring Boot executable jars

The system MUST provide an `inspect` command.

The `inspect` command MUST report whether the input appears to contain:

- `BOOT-INF/classes`
- `BOOT-INF/lib`
- Spring Boot launcher entries, when present

The `inspect` command MUST report whether nested jar entries under `BOOT-INF/lib/*.jar`
are stored uncompressed in the outer jar.

The `inspect` command MUST fail when the input cannot be opened as a jar.

The `inspect` command MUST NOT fail only because the jar does not appear to be a
Spring Boot executable jar.

#### Scenario: Inspect valid executable jar

Given a Spring Boot executable jar with `BOOT-INF/classes` and `BOOT-INF/lib`
When the user runs `bootjar-patcher inspect app.jar`
Then the tool reports the Spring Boot layout
And the tool reports nested jar storage status

#### Scenario: Inspect readable non-Spring jar

Given a readable jar without `BOOT-INF/classes` or `BOOT-INF/lib`
When the user runs `bootjar-patcher inspect library.jar`
Then the tool reports that the Spring Boot layout markers are absent
And the command does not fail only because the layout markers are absent

#### Scenario: Inspect invalid jar

Given a file that cannot be opened as a jar
When the user runs `bootjar-patcher inspect broken.jar`
Then the command fails
And the output explains that the jar could not be read

### Requirement: Address outer and nested entries

The system MUST support archive paths for both outer entries and nested entries.

The system MUST parse `!` as the separator between the nested jar path and the path
inside that nested jar.

The system MUST normalize user-provided filesystem separators to jar-style `/` where safe.

The system MUST reject archive paths with absolute paths, Windows drive prefixes,
empty path segments, `.`, `..`, empty outer paths, empty nested inner paths, or multiple
nested separators.

#### Scenario: Parse nested archive path

Given the path `BOOT-INF/lib/order-module.jar!/com/acme/OrderService.class`
When the path is parsed
Then the outer jar path is `BOOT-INF/lib/order-module.jar`
And the inner path is `com/acme/OrderService.class`

#### Scenario: Normalize safe filesystem separators

Given the path `BOOT-INF\classes\application.yml`
When the path is parsed
Then the normalized outer path is `BOOT-INF/classes/application.yml`

#### Scenario: Reject unsafe archive path

Given the path `../BOOT-INF/classes/application.yml`
When the path is parsed
Then parsing fails
And no archive path is produced

### Requirement: Find entries by filename or path

The system MUST provide a `find` command.

The `find` command MUST search both outer entries and nested entries by default.

The `find` command MUST display matching archive paths using the same path syntax that
patch plans accept.

The `find` command MUST match queries against full archive paths.

The `find` command MUST match queries against entry filenames.

The `find` command MUST return success with no output when no entries match.

The `find` command MUST fail when the input cannot be opened as a jar.

#### Scenario: Find class in nested jar

Given an executable jar containing `BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
When the user runs `bootjar-patcher find app.jar OrderService.class`
Then the output includes `BOOT-INF/lib/order.jar!/com/acme/OrderService.class`

#### Scenario: Find outer resource by path

Given an executable jar containing `BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher find app.jar BOOT-INF/classes/application.yml`
Then the output includes `BOOT-INF/classes/application.yml`

#### Scenario: Find returns no matches

Given a readable jar without entries matching `Missing.class`
When the user runs `bootjar-patcher find app.jar Missing.class`
Then the command succeeds
And the output contains no archive paths

#### Scenario: Find invalid jar

Given a file that cannot be opened as a jar
When the user runs `bootjar-patcher find broken.jar OrderService.class`
Then the command fails
And the output explains that the jar could not be read

### Requirement: Generate candidate matches

The system MUST provide a `match` command.

The `match` command MUST accept:

- a target jar
- one or more input files or directories
- an output format

The `match` command MUST scan the target jar and produce candidate target paths for
the input files.

The `match` command MUST mark each input as one of:

- `selected`
- `needs-selection`
- `no-match`

The `match` command MUST include reason strings for each candidate.

The `match` command MUST NOT silently convert ambiguous matches into final patch operations.

The `match` command MUST emit a candidates YAML document by default.

The `match` command MUST write the candidates YAML to `--out` when that option is provided.

The `match` command MUST fail when the input jar cannot be opened.

The `match` command MUST fail when an input path does not exist.

#### Scenario: Ambiguous class filename

Given an input file `OrderCalculator.class`
And the target jar contains two entries named `OrderCalculator.class`
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch`
Then the result for `OrderCalculator.class` is `needs-selection`
And the result lists both candidate archive paths
And the result does not auto-select either candidate

#### Scenario: Exact unique match

Given an input file with relative path `BOOT-INF/classes/application.yml`
And the target jar contains `BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch`
Then the result may be marked `selected`
And the selected target is `BOOT-INF/classes/application.yml`

#### Scenario: No matching target

Given an input file `Missing.class`
And the target jar has no matching archive path or filename
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch`
Then the result for `Missing.class` is `no-match`
And the result lists no candidate target paths

#### Scenario: Write candidates YAML to file

Given a target jar and an input file with candidate matches
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch --out candidates.yaml`
Then the command writes candidates YAML to `candidates.yaml`
And the command does not write candidate YAML to standard output

#### Scenario: Reject missing input path

Given a target jar
And the input path `./missing-patch-dir` does not exist
When the user runs `bootjar-patcher match --jar app.jar --inputs ./missing-patch-dir`
Then the command fails
And the output explains that the input path could not be read

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

### Requirement: Apply reviewed patch plans

The system MUST provide an `apply` command.

The `apply` command MUST accept:

- input jar
- patch-plan YAML
- output jar

The `apply` command MUST reject candidate files that have not been converted into
reviewed patch plans.

The `apply` command MUST write a new output jar rather than mutating the input jar
in place.

The `apply` command MUST fail when a replacement source file does not exist.

The `apply` command MUST fail when a replace target does not exist in the input jar.

#### Scenario: Apply reviewed patch plan

Given an input jar and a reviewed patch plan with valid replace operations
When the user runs `bootjar-patcher apply --jar app.jar --plan patch.yaml --out app-patched.jar`
Then the tool writes `app-patched.jar`
And the original `app.jar` is not mutated

#### Scenario: Reject candidates file

Given a candidates YAML file generated by `match`
When the user runs `bootjar-patcher apply --jar app.jar --plan candidates.yaml --out app-patched.jar`
Then the command fails
And the output explains that candidates files are not reviewed patch plans

#### Scenario: Reject missing replacement source

Given a patch plan targeting `BOOT-INF/classes/application.yml`
And the replacement source file does not exist
When the user runs `bootjar-patcher apply --jar app.jar --plan patch.yaml --out app-patched.jar`
Then the command fails
And the output explains that the replacement source file could not be read

### Requirement: Replace entries under BOOT-INF/classes

The system MUST support replacing files directly under `BOOT-INF/classes`.

#### Scenario: Replace classes resource

Given a patch plan targeting `BOOT-INF/classes/application.yml`
And the replacement source file exists
When the patch plan is applied
Then `BOOT-INF/classes/application.yml` in the output jar contains the replacement bytes

#### Scenario: Reject missing outer target

Given a patch plan targeting `BOOT-INF/classes/missing.yml`
And the replacement source file exists
When the patch plan is applied
Then the command fails
And the output explains that the replace target does not exist

### Requirement: Replace entries inside nested jars

The system MUST support replacing a class or resource inside a nested jar under
`BOOT-INF/lib`.

When replacing entries inside nested jars, the system MUST rewrite the affected nested
jar and then write the nested jar back into the outer jar.

The system MUST write the outer `BOOT-INF/lib/*.jar` entry as STORED.

The system MUST fail when the nested jar target does not exist in the outer jar.

The system MUST fail when the inner target does not exist in the nested jar.

#### Scenario: Replace nested jar entry

Given a patch plan targeting `BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
And the replacement source file exists
When the patch plan is applied
Then `com/acme/OrderService.class` inside `BOOT-INF/lib/order.jar` contains the replacement bytes
And the outer `BOOT-INF/lib/order.jar` entry is STORED

#### Scenario: Reject missing nested jar

Given a patch plan targeting `BOOT-INF/lib/missing.jar!/com/acme/OrderService.class`
And the replacement source file exists
When the patch plan is applied
Then the command fails
And the output explains that the nested jar target does not exist

#### Scenario: Reject missing nested entry

Given a patch plan targeting `BOOT-INF/lib/order.jar!/com/acme/Missing.class`
And the replacement source file exists
When the patch plan is applied
Then the command fails
And the output explains that the nested replace target does not exist

### Requirement: Replace whole nested jars

The system MUST support replacing a whole nested jar under `BOOT-INF/lib`.

The system MUST write the replacement nested jar as a STORED entry in the outer jar.

#### Scenario: Replace nested jar file

Given a patch plan targeting `BOOT-INF/lib/common-module.jar`
And the replacement source jar exists
When the patch plan is applied
Then `BOOT-INF/lib/common-module.jar` in the output jar contains the replacement jar bytes
And the outer `BOOT-INF/lib/common-module.jar` entry is STORED

### Requirement: Group nested operations

The system MUST group operations targeting the same nested jar.

The system MUST rewrite each affected nested jar at most once per apply run.

#### Scenario: Group operations for same nested jar

Given a patch plan with two operations targeting entries inside `BOOT-INF/lib/order.jar`
When the patch plan is applied
Then `BOOT-INF/lib/order.jar` is rewritten once
And both targeted inner entries are replaced

### Requirement: Verify patched jars

The system MUST provide a `verify` command.

The `verify` command MUST check:

- output jar can be opened
- `BOOT-INF/lib/*.jar` entries are STORED
- patched targets exist when patch metadata is available

The `verify` command SHOULD warn when signed jar metadata is detected.

#### Scenario: Verify stored nested jars

Given a patched jar containing nested jars under `BOOT-INF/lib`
When the user runs `bootjar-patcher verify app-patched.jar`
Then the tool reports whether the jar can be opened
And the tool reports whether all `BOOT-INF/lib/*.jar` entries are STORED

## Patch Plan Format

Example:

```yaml
kind: patch-plan
version: 1

operations:
  - replace-entry:
      target: BOOT-INF/classes/application.yml
      with: ./patch/application.yml

  - replace-entry:
      target: BOOT-INF/lib/order-module-1.4.2.jar!/com/acme/order/OrderCalculator.class
      with: ./patch/OrderCalculator.class

  - replace-entry:
      target: BOOT-INF/lib/common-module-2.7.0.jar
      with: ./patch/common-module-2.7.0-hotfix.jar
```

## Candidate File Format

Example:

```yaml
kind: candidates
version: 1
source: app.jar

matches:
  - input: ./patch/OrderCalculator.class
    status: needs-selection
    candidates:
      - target: BOOT-INF/classes/com/acme/order/OrderCalculator.class
        score: 96
        reason:
          - same filename
          - class path match
      - target: BOOT-INF/lib/order-module-1.4.2.jar!/com/acme/order/OrderCalculator.class
        score: 91
        reason:
          - same filename
          - found inside nested jar
```

## Safety Rules

The system MUST fail when:

- the input jar does not exist
- a replacement source file does not exist
- a replace target does not exist
- the patch plan contains duplicate incompatible operations
- the output jar cannot be written
- verification fails after writing

The system SHOULD warn when:

- signed jar metadata is detected
- matching is ambiguous
- a path differs only by case
- a candidate is based only on filename matching
