## MODIFIED Requirements

### Requirement: Replace whole nested jars

The system MUST support replacing a whole nested jar under `BOOT-INF/lib`.

The system MUST write the replacement nested jar as a STORED entry in the outer jar.

The system MUST fail when the replacement source is not readable as a jar.

#### Scenario: Replace nested jar file

Given a patch plan targeting `BOOT-INF/lib/common-module.jar`
And the replacement source jar exists
When the patch plan is applied
Then `BOOT-INF/lib/common-module.jar` in the output jar contains the replacement jar bytes
And the outer `BOOT-INF/lib/common-module.jar` entry is STORED

#### Scenario: Reject invalid replacement nested jar

Given a patch plan targeting `BOOT-INF/lib/common-module.jar`
And the replacement source file is not readable as a jar
When the patch plan is applied
Then the command fails
And the output explains that the replacement nested jar could not be read
