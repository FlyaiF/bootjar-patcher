## MODIFIED Requirements

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
