## MODIFIED Requirements

### Requirement: Verify patched jars

The system MUST provide a `verify` command.

The `verify` command MUST check:

- output jar can be opened
- `BOOT-INF/lib/*.jar` entries are STORED
- patched targets exist when patch metadata is available

The `verify` command SHOULD warn when signed jar metadata is detected.

The `verify` command MUST fail when the jar cannot be opened.

The `verify` command MUST fail when any `BOOT-INF/lib/*.jar` entry is not STORED.

#### Scenario: Verify stored nested jars

Given a patched jar containing nested jars under `BOOT-INF/lib`
When the user runs `bootjar-patcher verify app-patched.jar`
Then the tool reports whether the jar can be opened
And the tool reports whether all `BOOT-INF/lib/*.jar` entries are STORED

#### Scenario: Reject compressed nested jar

Given a jar containing a compressed `BOOT-INF/lib/order.jar` outer entry
When the user runs `bootjar-patcher verify app.jar`
Then the command fails
And the output identifies `BOOT-INF/lib/order.jar` as not STORED

#### Scenario: Warn on signed metadata

Given a jar containing signed jar metadata under `META-INF`
When the user runs `bootjar-patcher verify app.jar`
Then the command succeeds if required checks pass
And the output warns that signed jar metadata was detected
