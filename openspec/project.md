# bootjar-patcher Project Context

## Product

`bootjar-patcher` is a cross-platform CLI and reusable Rust library for patching Spring Boot executable fat jars.

It supports:

- inspecting executable jar structure
- listing and finding classes/resources in `BOOT-INF/classes` and `BOOT-INF/lib` nested jars
- matching user-provided replacement files to possible target paths
- generating candidate YAML/snippets
- applying reviewed patch plans
- verifying Spring Boot nested-jar constraints

## Key domain rule

Entries under `BOOT-INF/lib/*.jar` must be STORED in the outer executable jar. The nested jar's own internal entries may remain compressed.

## Architecture

- `bootjar-core` owns archive indexing, matching, patch planning, patch execution, and verification.
- `bootjar-cli` owns argument parsing, terminal formatting, and exit codes.

## Safety model

Candidate matching is assistive. Ambiguous matches must require user selection.
Patch execution only applies reviewed patch plans.

## Initial CLI shape

```bash
bootjar-patcher inspect app.jar
bootjar-patcher tree app.jar
bootjar-patcher find app.jar "OrderCalculator.class"

bootjar-patcher match \
  --jar app.jar \
  --inputs ./patch-files \
  --out candidates.yaml

bootjar-patcher match \
  --jar app.jar \
  --inputs ./patch-files \
  --format snippets

bootjar-patcher apply \
  --jar app.jar \
  --plan patch.yaml \
  --out app-patched.jar

bootjar-patcher verify app-patched.jar
```

## Path syntax

Outer entry:

```text
BOOT-INF/classes/application.yml
```

Nested entry:

```text
BOOT-INF/lib/order-module-1.4.2.jar!/com/acme/order/OrderCalculator.class
```

## Non-goals

- No bytecode semantic modification or decompilation.
- No Maven/Gradle dependency resolution.
- No signed-jar repair in initial versions.
- No JVM runtime requirement.
- No unsafe in-place mutation in initial versions.
