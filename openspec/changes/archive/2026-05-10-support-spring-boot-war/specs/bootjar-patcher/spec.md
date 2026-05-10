## ADDED Requirements

### Requirement: Support Spring Boot executable WAR archives

The system MUST support Spring Boot executable WAR archives in the same inspect,
find, match, apply, and verify workflows as Spring Boot executable JAR archives.

The system MUST identify Spring Boot executable WAR application entries under
`WEB-INF/classes`.

The system MUST identify Spring Boot executable WAR nested libraries under
`WEB-INF/lib/*.jar` and `WEB-INF/lib-provided/*.jar`.

The system MUST index readable entries inside WAR nested libraries.

The system MUST write changed WAR nested libraries as STORED entries in the outer
WAR archive.

The system MUST fail verification when any WAR nested library under `WEB-INF/lib`
or `WEB-INF/lib-provided` is not STORED.

#### Scenario: Inspect executable WAR layout

Given a Spring Boot executable WAR with `WEB-INF/classes`, `WEB-INF/lib`, and
`WEB-INF/lib-provided`
When the user runs `bootjar-patcher inspect app.war`
Then the tool reports the Spring Boot WAR layout
And the tool reports nested library storage status for both WAR library roots

#### Scenario: Find nested entry in WAR dependency

Given an executable WAR containing `WEB-INF/lib/order.jar!/com/acme/OrderService.class`
When the user runs `bootjar-patcher find app.war OrderService.class`
Then the output includes `WEB-INF/lib/order.jar!/com/acme/OrderService.class`

#### Scenario: Apply WAR patch plan

Given an input WAR and a reviewed patch plan targeting `WEB-INF/classes/application.yml`
When the user runs `bootjar-patcher apply --archive app.war --plan patch.yaml --out app-patched.war`
Then the tool writes `app-patched.war`
And the original `app.war` is not mutated

#### Scenario: Reject compressed WAR nested library

Given a WAR containing a compressed `WEB-INF/lib/order.jar` outer entry
When the user runs `bootjar-patcher verify app.war`
Then the command fails
And the output identifies `WEB-INF/lib/order.jar` as not STORED

## MODIFIED Requirements

### Requirement: Generate candidate matches

The system MUST provide a `match` command.

The `match` command MUST accept:

- a target archive
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

The `match` command MUST accept `--archive` as the target archive option.

The `match` command MUST reject `--jar`.

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

#### Scenario: Reject legacy jar option

Given a target archive
When the user runs `bootjar-patcher match --jar app.jar --inputs ./patch`
Then the command fails
And the output explains that `--jar` is unknown

### Requirement: Apply reviewed patch plans

The system MUST provide an `apply` command.

The `apply` command MUST accept:

- input archive
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

The `apply` command MUST accept `--archive` as the input archive option.

The `apply` command MUST reject `--jar`.

#### Scenario: Apply reviewed patch plan

Given an input archive and a reviewed patch plan with valid replace operations
When the user runs `bootjar-patcher apply --archive app.jar --plan patch.yaml --out app-patched.jar`
Then the tool writes `app-patched.jar`
And the original `app.jar` is not mutated

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

#### Scenario: Reject legacy jar option

Given an input archive
When the user runs `bootjar-patcher apply --jar app.jar --plan patch.yaml --out app-patched.jar`
Then the command fails
And the output explains that `--jar` is unknown
