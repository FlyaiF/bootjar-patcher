# bootjar-patcher Specification

## Purpose

`bootjar-patcher` is a cross-platform command-line tool and reusable library for
patching Spring Boot executable archives safely and repeatably.

It supports direct patch execution and assistive discovery of candidate target paths.

## Definitions

### Input archive

The Spring Boot executable JAR or WAR passed as input to the tool.

### Nested jar

A jar file stored as an entry under a Spring Boot nested library root:
`BOOT-INF/lib` for executable JARs, or `WEB-INF/lib` and `WEB-INF/lib-provided`
for executable WARs.

### Archive path

A string identifying either an outer archive entry or a nested jar entry.

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

A reviewed YAML file containing deterministic operations to apply to an archive.

## Requirements

### Requirement: Inspect Spring Boot executable archives

The system MUST provide an `inspect` command.

The `inspect` command MUST report the detected archive layout:

- Spring Boot executable JAR
- Spring Boot executable WAR
- unknown readable archive

The `inspect` command MUST report whether the input appears to contain JAR layout
markers:

- `BOOT-INF/classes`
- `BOOT-INF/lib`

The `inspect` command MUST report whether the input appears to contain WAR layout
markers:

- `WEB-INF/classes`
- `WEB-INF/lib`
- `WEB-INF/lib-provided`

The `inspect` command MUST report whether the input appears to contain:

- Spring Boot launcher entries, when present

The `inspect` command MUST report whether nested jar entries under supported nested
library roots are stored uncompressed in the outer archive.

The `inspect` command MUST fail when the input cannot be opened as an archive.

The `inspect` command MUST NOT fail only because the archive does not appear to be a
Spring Boot executable archive.

#### Scenario: Inspect valid executable jar

Given a Spring Boot executable jar with `BOOT-INF/classes` and `BOOT-INF/lib`
When the user runs `bootjar-patcher inspect app.jar`
Then the tool reports the Spring Boot JAR layout
And the tool reports nested jar storage status

#### Scenario: Inspect valid executable WAR

Given a Spring Boot executable WAR with `WEB-INF/classes`, `WEB-INF/lib`, and `WEB-INF/lib-provided`
When the user runs `bootjar-patcher inspect app.war`
Then the tool reports the Spring Boot WAR layout
And the tool reports nested jar storage status for both WAR library roots

#### Scenario: Inspect readable non-Spring jar

Given a readable jar without `BOOT-INF/classes` or `BOOT-INF/lib`
When the user runs `bootjar-patcher inspect library.jar`
Then the tool reports an unknown archive layout
And the command does not fail only because the layout markers are absent

#### Scenario: Inspect invalid jar

Given a file that cannot be opened as an archive
When the user runs `bootjar-patcher inspect broken.jar`
Then the command fails
And the output explains that the archive could not be read

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
Then the outer archive path is `BOOT-INF/lib/order-module.jar`
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

The `find` command MUST fail when the input cannot be opened as an archive.

#### Scenario: Find class in nested jar

Given an executable jar containing `BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
When the user runs `bootjar-patcher find app.jar OrderService.class`
Then the output includes `BOOT-INF/lib/order.jar!/com/acme/OrderService.class`

#### Scenario: Find class in WAR nested jar

Given an executable WAR containing `WEB-INF/lib/order.jar!/com/acme/OrderService.class`
When the user runs `bootjar-patcher find app.war OrderService.class`
Then the output includes `WEB-INF/lib/order.jar!/com/acme/OrderService.class`

#### Scenario: Find class in WAR provided nested jar

Given an executable WAR containing `WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class`
When the user runs `bootjar-patcher find app.war ProvidedService.class`
Then the output includes `WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class`

#### Scenario: Find outer resource by path

Given an executable jar containing `BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher find app.jar BOOT-INF/classes/application.yml`
Then the output includes `BOOT-INF/classes/application.yml`

#### Scenario: Find WAR outer resource by path

Given an executable WAR containing `WEB-INF/classes/application.yml`
When the user runs `bootjar-patcher find app.war WEB-INF/classes/application.yml`
Then the output includes `WEB-INF/classes/application.yml`

#### Scenario: Find returns no matches

Given a readable jar without entries matching `Missing.class`
When the user runs `bootjar-patcher find app.jar Missing.class`
Then the command succeeds
And the output contains no archive paths

#### Scenario: Find invalid jar

Given a file that cannot be opened as an archive
When the user runs `bootjar-patcher find broken.jar OrderService.class`
Then the command fails
And the output explains that the archive could not be read

### Requirement: Generate candidate matches

The system MUST provide a `match` command.

The `match` command MUST accept:

- a target archive provided as `--archive`
- one or more input files or directories
- an output format

The `match` command MUST scan the target archive and produce candidate target paths for
the input files.

The `match` command MUST mark each input as one of:

- `selected`
- `needs-selection`
- `no-match`

The `match` command MUST include reason strings for each candidate.

The `match` command MUST NOT silently convert ambiguous matches into final patch operations.

The `match` command MUST emit a candidates YAML document by default.

The `match` command MUST write the candidates YAML to `--out` when that option is provided.

The `match` command MUST fail when the input archive cannot be opened.

The `match` command MUST fail when an input path does not exist.

The `match` command MUST reject the removed `--jar` option as unknown.

#### Scenario: Ambiguous class filename

Given an input file `OrderCalculator.class`
And the target archive contains two entries named `OrderCalculator.class`
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch`
Then the result for `OrderCalculator.class` is `needs-selection`
And the result lists both candidate archive paths
And the result does not auto-select either candidate

#### Scenario: Exact unique match

Given an input file with relative path `BOOT-INF/classes/application.yml`
And the target archive contains `BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch`
Then the result may be marked `selected`
And the selected target is `BOOT-INF/classes/application.yml`

#### Scenario: Exact unique WAR match

Given an input file with relative path `WEB-INF/classes/application.yml`
And the target WAR contains `WEB-INF/classes/application.yml`
When the user runs `bootjar-patcher match --archive app.war --inputs ./patch`
Then the result may be marked `selected`
And the selected target is `WEB-INF/classes/application.yml`

#### Scenario: Reject legacy jar option

Given a target archive and an input file
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch`
Then the command fails
And the output explains that `--jar` is unknown

#### Scenario: No matching target

Given an input file `Missing.class`
And the target archive has no matching archive path or filename
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch`
Then the result for `Missing.class` is `no-match`
And the result lists no candidate target paths

#### Scenario: Write candidates YAML to file

Given a target archive and an input file with candidate matches
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch --out candidates.yaml`
Then the command writes candidates YAML to `candidates.yaml`
And the command does not write candidate YAML to standard output

#### Scenario: Reject missing input path

Given a target archive
And the input path `./missing-patch-dir` does not exist
When the user runs `bootjar-patcher match --archive app.jar --inputs ./missing-patch-dir`
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

Given a target archive and an input file with candidate matches
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch --format snippets`
Then the output contains YAML patch operation snippets
And ambiguous candidate snippets are commented or clearly marked

#### Scenario: Emit selected replacement snippet

Given an input file with relative path `BOOT-INF/classes/application.yml`
And the target archive contains `BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch --format snippets`
Then the output contains an uncommented `replace-entry` operation
And the operation target is `BOOT-INF/classes/application.yml`
And the operation source is the input file path

#### Scenario: Comment no-match snippet result

Given an input file `Missing.class`
And the target archive has no matching archive path or filename
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch --format snippets`
Then the output does not contain an uncommented `replace-entry` operation for `Missing.class`
And the output comments that no match was found

#### Scenario: Write snippets to file

Given a target archive and an input file with candidate matches
When the user runs `bootjar-patcher match --archive app.jar --inputs ./patch --format snippets --out patch-snippets.yaml`
Then the command writes snippets to `patch-snippets.yaml`
And the command does not write snippets to standard output

### Requirement: Apply reviewed patch plans

The system MUST provide an `apply` command.

The `apply` command MUST accept:

- input archive provided as `--archive`
- patch-plan YAML
- output archive

The `apply` command MUST reject candidate files that have not been converted into
reviewed patch plans.

The `apply` command MUST write a new output archive rather than mutating the input
archive in place.

The `apply` command MUST verify the output archive after writing.

The `apply` command MUST fail when post-write verification fails.

The `apply` command MUST leave the written output archive in place when post-write
verification fails.

The `apply` command MUST fail when a replacement source file does not exist.

The `apply` command MUST fail when a replace target does not exist in the input archive.

The `apply` command MUST reject the removed `--jar` option as unknown.

#### Scenario: Apply reviewed patch plan

Given an input archive and a reviewed patch plan with valid replace operations
When the user runs `bootjar-patcher apply --archive app.jar --plan patch.yaml --out app-patched.jar`
Then the tool writes `app-patched.jar`
And the original `app.jar` is not mutated

#### Scenario: Reject legacy apply jar option

Given an input archive and a reviewed patch plan
When the user runs `bootjar-patcher apply --jar app.jar --plan patch.yaml --out app-patched.jar`
Then the command fails
And the output explains that `--jar` is unknown

#### Scenario: Reject output that fails verification

Given an input archive containing a compressed `BOOT-INF/lib/order.jar` outer entry
And a reviewed patch plan with otherwise valid replace operations
When the user runs `bootjar-patcher apply --archive app.jar --plan patch.yaml --out app-patched.jar`
Then the command fails after writing `app-patched.jar`
And the output explains that post-write verification failed for `BOOT-INF/lib/order.jar`

#### Scenario: Reject candidates file

Given a candidates YAML file generated by `match`
When the user runs `bootjar-patcher apply --archive app.jar --plan candidates.yaml --out app-patched.jar`
Then the command fails
And the output explains that candidates files are not reviewed patch plans

#### Scenario: Reject missing replacement source

Given a patch plan targeting `BOOT-INF/classes/application.yml`
And the replacement source file does not exist
When the user runs `bootjar-patcher apply --archive app.jar --plan patch.yaml --out app-patched.jar`
Then the command fails
And the output explains that the replacement source file could not be read

### Requirement: Replace entries under BOOT-INF/classes

The system MUST support replacing files directly under `BOOT-INF/classes`.

#### Scenario: Replace classes resource

Given a patch plan targeting `BOOT-INF/classes/application.yml`
And the replacement source file exists
When the patch plan is applied
Then `BOOT-INF/classes/application.yml` in the output archive contains the replacement bytes

#### Scenario: Reject missing outer target

Given a patch plan targeting `BOOT-INF/classes/missing.yml`
And the replacement source file exists
When the patch plan is applied
Then the command fails
And the output explains that the replace target does not exist

### Requirement: Replace entries under WEB-INF/classes

The system MUST support replacing files directly under `WEB-INF/classes` in executable WARs.

#### Scenario: Replace WAR classes resource

Given a patch plan targeting `WEB-INF/classes/application.yml`
And the replacement source file exists
When the patch plan is applied
Then `WEB-INF/classes/application.yml` in the output WAR contains the replacement bytes

### Requirement: Replace entries inside nested jars

The system MUST support replacing a class or resource inside a nested jar under supported
nested library roots.

When replacing entries inside nested jars, the system MUST rewrite the affected nested
jar and then write the nested jar back into the outer archive.

The system MUST write changed nested library entries as STORED.

The system MUST fail when the nested jar target does not exist in the outer archive.

The system MUST fail when the inner target does not exist in the nested jar.

#### Scenario: Replace nested jar entry

Given a patch plan targeting `BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
And the replacement source file exists
When the patch plan is applied
Then `com/acme/OrderService.class` inside `BOOT-INF/lib/order.jar` contains the replacement bytes
And the outer `BOOT-INF/lib/order.jar` entry is STORED

#### Scenario: Replace WAR nested jar entry

Given a patch plan targeting `WEB-INF/lib/order.jar!/com/acme/OrderService.class`
And the replacement source file exists
When the patch plan is applied
Then `com/acme/OrderService.class` inside `WEB-INF/lib/order.jar` contains the replacement bytes
And the outer `WEB-INF/lib/order.jar` entry is STORED

#### Scenario: Replace WAR provided nested jar entry

Given a patch plan targeting `WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class`
And the replacement source file exists
When the patch plan is applied
Then `com/acme/ProvidedService.class` inside `WEB-INF/lib-provided/container.jar` contains the replacement bytes
And the outer `WEB-INF/lib-provided/container.jar` entry is STORED

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

The system MUST support replacing a whole nested jar under supported nested library roots.

The system MUST write the replacement nested jar as a STORED entry in the outer archive.

The system MUST fail when the replacement source is not readable as a jar.

#### Scenario: Replace nested jar file

Given a patch plan targeting `BOOT-INF/lib/common-module.jar`
And the replacement source jar exists
When the patch plan is applied
Then `BOOT-INF/lib/common-module.jar` in the output archive contains the replacement jar bytes
And the outer `BOOT-INF/lib/common-module.jar` entry is STORED

#### Scenario: Replace whole WAR nested jar file

Given a patch plan targeting `WEB-INF/lib-provided/common-module.jar`
And the replacement source jar exists
When the patch plan is applied
Then `WEB-INF/lib-provided/common-module.jar` in the output WAR contains the replacement jar bytes
And the outer `WEB-INF/lib-provided/common-module.jar` entry is STORED

#### Scenario: Reject invalid replacement nested jar

Given a patch plan targeting `BOOT-INF/lib/common-module.jar`
And the replacement source file is not readable as a jar
When the patch plan is applied
Then the command fails
And the output explains that the replacement nested jar could not be read

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

- output archive can be opened
- supported nested library entries are STORED
- patched targets exist when patch metadata is available

The `verify` command SHOULD warn when signed jar metadata is detected.

The `verify` command MUST fail when the archive cannot be opened.

The `verify` command MUST fail when any supported nested library entry is not STORED.

#### Scenario: Verify stored nested jars

Given a patched jar containing nested jars under `BOOT-INF/lib`
When the user runs `bootjar-patcher verify app-patched.jar`
Then the tool reports whether the jar can be opened
And the tool reports whether all `BOOT-INF/lib/*.jar` entries are STORED

#### Scenario: Verify stored WAR nested jars

Given a patched WAR containing nested jars under `WEB-INF/lib` and `WEB-INF/lib-provided`
When the user runs `bootjar-patcher verify app-patched.war`
Then the tool reports whether the WAR can be opened
And the tool reports whether all WAR nested library entries are STORED

#### Scenario: Reject compressed nested jar

Given a jar containing a compressed `BOOT-INF/lib/order.jar` outer entry
When the user runs `bootjar-patcher verify app.jar`
Then the command fails
And the output identifies `BOOT-INF/lib/order.jar` as not STORED

#### Scenario: Reject compressed WAR nested jar

Given a WAR containing a compressed `WEB-INF/lib/order.jar` outer entry
When the user runs `bootjar-patcher verify app.war`
Then the command fails
And the output identifies `WEB-INF/lib/order.jar` as not STORED

#### Scenario: Warn on signed metadata

Given a jar containing signed jar metadata under `META-INF`
When the user runs `bootjar-patcher verify app.jar`
Then the command succeeds if required checks pass
And the output warns that signed jar metadata was detected

### Requirement: Real Spring Boot integration coverage

The project MUST provide opt-in integration tests that build minimal Spring Boot
executable JAR and WAR artifacts with Maven Wrapper and exercise the core library
against those artifacts.

The integration tests MUST NOT run as part of default `cargo test`.

The integration tests MUST require Java on `PATH` but MUST NOT require a system
Maven installation.

The integration tests MAY download Maven and Maven dependencies when they are run
without a primed local cache.

The integration tests MUST cover inspect, find, match, apply, and verify behavior
against the Maven-built executable archives.

The integration tests MUST verify that nested jars under `BOOT-INF/lib/*.jar` are
STORED in valid Spring Boot outputs.

The integration tests MUST verify that nested jars under `WEB-INF/lib/*.jar` and
`WEB-INF/lib-provided/*.jar` are STORED in valid Spring Boot WAR outputs.

#### Scenario: Run real Spring Boot integration tests explicitly

Given Java is available on `PATH`
When the user runs `cargo test -p bootjar-spring-it -- --ignored`
Then the test suite builds the fixture with Maven Wrapper
And the tests exercise core behavior against the resulting executable JAR and WAR

#### Scenario: Keep default Rust tests Java-free

Given the user runs `cargo test`
When the default workspace tests execute
Then the real Spring Boot integration tests are not run
And no Java or Maven Wrapper invocation is required by those ignored tests

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

- the input archive does not exist
- a replacement source file does not exist
- a replace target does not exist
- the patch plan contains duplicate incompatible operations
- the output archive cannot be written
- verification fails after writing

The system SHOULD warn when:

- signed jar metadata is detected
- matching is ambiguous
- a path differs only by case
- a candidate is based only on filename matching
