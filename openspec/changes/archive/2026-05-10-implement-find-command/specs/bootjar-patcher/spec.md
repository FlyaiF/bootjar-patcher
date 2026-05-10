## MODIFIED Requirements

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
