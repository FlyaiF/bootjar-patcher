## MODIFIED Requirements

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
