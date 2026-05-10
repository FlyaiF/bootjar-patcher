## MODIFIED Requirements

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

### Requirement: Group nested operations

The system MUST group operations targeting the same nested jar.

The system MUST rewrite each affected nested jar at most once per apply run.

#### Scenario: Group operations for same nested jar

Given a patch plan with two operations targeting entries inside `BOOT-INF/lib/order.jar`
When the patch plan is applied
Then `BOOT-INF/lib/order.jar` is rewritten once
And both targeted inner entries are replaced
