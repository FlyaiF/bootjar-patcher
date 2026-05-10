## MODIFIED Requirements

### Requirement: Address outer and nested entries

The system MUST support archive paths for outer entries and chained nested entries.

The system MUST parse `!` as the separator between archive path segments.

The system MUST support direct entry paths such as `config/app.yml`.

The system MUST support contained archive paths such as `app/service.jar!/BOOT-INF/classes/application.yml`.

The system MUST support dependency archive paths such as `app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class`.

The system MUST normalize user-provided filesystem separators to archive-style `/` where safe.

The system MUST reject archive paths with absolute paths, Windows drive prefixes,
empty path segments, `.`, `..`, empty outer paths, empty nested inner paths, or empty
archive segments.

#### Scenario: Parse chained archive path

Given the path `app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
When the path is parsed
Then the archive path contains three segments
And the first segment is `app/service.jar`
And the last segment is `com/acme/OrderService.class`

#### Scenario: Parse direct archive path

Given the path `config/application.yml`
When the path is parsed
Then the archive path contains one segment
And the segment is `config/application.yml`

#### Scenario: Reject unsafe chained archive path

Given the path `app/service.jar!/../BOOT-INF/classes/application.yml`
When the path is parsed
Then parsing fails
And no archive path is produced

### Requirement: Inspect Spring Boot executable archives

The system MUST provide an `inspect` command.

The `inspect` command MUST report the detected archive layout:

- Spring Boot executable JAR
- Spring Boot executable WAR
- ZIP wrapper
- unknown readable archive

The `inspect` command MUST report whether direct JAR/WAR layout markers are present.

The `inspect` command MUST report contained Spring Boot archives for ZIP wrappers.

The `inspect` command MUST report whether nested jar entries under supported nested
library roots are stored uncompressed in each Spring Boot archive.

The `inspect` command MUST fail when the input cannot be opened as an archive.

The `inspect` command MUST NOT fail only because the archive does not appear to be a
Spring Boot executable archive or wrapper.

#### Scenario: Inspect ZIP wrapper

Given a readable ZIP containing `app/service.jar` with a Spring Boot JAR layout
When the user runs `bootjar-patcher inspect dist.zip`
Then the tool reports the ZIP wrapper layout
And the tool reports `app/service.jar` as a contained Spring Boot JAR
And the tool reports nested jar storage status for `app/service.jar`

### Requirement: Find entries by filename or path

The system MUST provide a `find` command.

The `find` command MUST search wrapper entries, contained Spring Boot archive entries,
and nested dependency entries by default.

The `find` command MUST display matching archive paths using the same chained path
syntax that patch plans accept.

The `find` command MUST match queries against full archive paths.

The `find` command MUST match queries against entry filenames.

The `find` command MUST return success with no output when no entries match.

The `find` command MUST fail when the input cannot be opened as an archive.

#### Scenario: Find wrapper configuration file

Given a ZIP wrapper containing `config/runtime.yml`
When the user runs `bootjar-patcher find dist.zip runtime.yml`
Then the output includes `config/runtime.yml`

#### Scenario: Find contained archive resource

Given a ZIP wrapper containing `app/service.jar!/BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher find dist.zip application.yml`
Then the output includes `app/service.jar!/BOOT-INF/classes/application.yml`

#### Scenario: Find dependency entry inside contained archive

Given a ZIP wrapper containing `app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
When the user runs `bootjar-patcher find dist.zip OrderService.class`
Then the output includes `app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class`

### Requirement: Generate candidate matches

The system MUST provide a `match` command.

The `match` command MUST scan the target archive and produce candidate target paths for
the input files.

The `match` command MUST include wrapper entries and chained contained archive entries
as candidate targets.

The `match` command MUST NOT infer a contained archive target from an unqualified
`BOOT-INF/...` input path when the target archive is a ZIP wrapper.

The `match` command MUST mark each input as one of `selected`, `needs-selection`, or `no-match`.

The `match` command MUST NOT silently convert ambiguous matches into final patch operations.

#### Scenario: Match exact wrapper path

Given an input file with relative path `config/runtime.yml`
And the target ZIP wrapper contains `config/runtime.yml`
When the user runs `bootjar-patcher match --archive dist.zip --inputs ./patch`
Then the result may be marked `selected`
And the selected target is `config/runtime.yml`

#### Scenario: Match exact chained path

Given an input file with relative path `app/service.jar!/BOOT-INF/classes/application.yml`
And the target ZIP wrapper contains `app/service.jar!/BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher match --archive dist.zip --inputs ./patch`
Then the result may be marked `selected`
And the selected target is `app/service.jar!/BOOT-INF/classes/application.yml`

#### Scenario: Do not infer unqualified fatjar target

Given an input file with relative path `BOOT-INF/classes/application.yml`
And the target ZIP wrapper contains `app/service.jar!/BOOT-INF/classes/application.yml`
When the user runs `bootjar-patcher match --archive dist.zip --inputs ./patch`
Then the result is not selected by exact relative path

### Requirement: Apply reviewed patch plans

The system MUST provide an `apply` command.

The `apply` command MUST support `replace-entry` operations targeting wrapper entries.

The `apply` command MUST support `replace-entry` operations targeting entries inside
contained Spring Boot JAR/WAR archives using chained paths.

The `apply` command MUST support `replace-entry` operations targeting nested dependency
entries inside contained Spring Boot archives using chained paths.

The `apply` command MUST preserve wrapper entry compression method, modified time, and
Unix mode where available.

The `apply` command MUST verify the output archive after writing.

The `apply` command MUST fail when post-write verification fails.

The `apply` command MUST leave the written output archive in place when post-write
verification fails.

#### Scenario: Replace wrapper script

Given a ZIP wrapper containing executable `bin/start.sh`
And a patch plan targeting `bin/start.sh`
When the patch plan is applied
Then `bin/start.sh` in the output ZIP contains the replacement bytes
And the output entry preserves executable Unix mode where available

#### Scenario: Replace contained archive resource

Given a ZIP wrapper containing `app/service.jar!/BOOT-INF/classes/application.yml`
And a patch plan targeting `app/service.jar!/BOOT-INF/classes/application.yml`
When the patch plan is applied
Then the output ZIP contains a rewritten `app/service.jar`
And `BOOT-INF/classes/application.yml` inside that JAR contains the replacement bytes

#### Scenario: Replace dependency entry inside contained archive

Given a ZIP wrapper containing `app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
And a patch plan targeting `app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class`
When the patch plan is applied
Then the output ZIP contains a rewritten `app/service.jar`
And `BOOT-INF/lib/order.jar` inside that JAR is STORED
And `com/acme/OrderService.class` inside the dependency jar contains the replacement bytes

#### Scenario: Reject missing contained archive

Given a patch plan targeting `missing/service.jar!/BOOT-INF/classes/application.yml`
When the patch plan is applied
Then the command fails
And the output explains that the contained archive target does not exist

### Requirement: Verify patched archives

The system MUST provide a `verify` command.

The `verify` command MUST check that the input archive can be opened.

The `verify` command MUST verify direct Spring Boot JAR/WAR inputs using existing nested
library STORED rules.

The `verify` command MUST verify every contained Spring Boot JAR/WAR inside a ZIP
wrapper using existing nested library STORED rules.

The `verify` command MUST NOT require contained Spring Boot archive entries to be
STORED in the wrapper ZIP.

#### Scenario: Verify valid ZIP wrapper

Given a ZIP wrapper containing a Spring Boot JAR with STORED nested libraries
When the user runs `bootjar-patcher verify dist.zip`
Then the command succeeds
And the output reports the contained Spring Boot archive nested libraries

#### Scenario: Reject ZIP wrapper with invalid contained archive

Given a ZIP wrapper containing a Spring Boot JAR with compressed `BOOT-INF/lib/order.jar`
When the user runs `bootjar-patcher verify dist.zip`
Then the command fails
And the output identifies `app/service.jar!/BOOT-INF/lib/order.jar` as not STORED
