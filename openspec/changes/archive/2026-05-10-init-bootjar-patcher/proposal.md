# Proposal: Initialize bootjar-patcher

## Summary

Create `bootjar-patcher`, a cross-platform CLI and reusable core library for inspecting,
discovering, planning, and applying patches to Spring Boot executable fat jars.

The tool must support simple replacements as well as complex multi-module patching,
including:

- replacing files under `BOOT-INF/classes`
- replacing whole nested jars under `BOOT-INF/lib`
- replacing classes/resources inside nested jars using `!` archive path syntax
- discovering candidate target paths for a batch of user-provided replacement files
- generating reviewable candidate YAML or patch-plan snippets
- verifying that rebuilt Spring Boot executable jars remain launcher-compatible

## Motivation

Real Spring Boot patching is often not a one-step replacement. In multi-module projects,
the class or resource to replace may live directly under `BOOT-INF/classes`, inside one
of many nested jars under `BOOT-INF/lib`, or in multiple possible locations with the same
filename.

Users need help answering two questions:

1. Where is the file I need to patch?
2. How do I rebuild the fat jar safely after choosing the target?

The tool should therefore provide both deterministic patch execution and assistive
candidate discovery.

## Goals

- Provide a native cross-platform CLI for Windows, Linux, and macOS.
- Provide a reusable core library for future UI integration.
- Support archive paths for both outer and nested entries.
- Support a reviewed YAML patch-plan format.
- Support candidate matching from a target jar plus a directory of replacement files.
- Preserve Spring Boot executable-jar constraints, especially stored nested jar entries.
- Avoid requiring a JVM at runtime.
- Produce clear warnings for ambiguous matches, signed jars, missing targets, and unsafe paths.

## Non-Goals

- Do not decompile or semantically modify Java bytecode.
- Do not repair or re-sign modified signed jars.
- Do not resolve Maven or Gradle dependency graphs.
- Do not provide a graphical UI in the initial change.
- Do not perform unsafe in-place mutation of the input jar.

## Proposed CLI

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

## Proposed Path Syntax

Outer jar entry:

```text
BOOT-INF/classes/application.yml
```

Nested jar entry:

```text
BOOT-INF/lib/order-module-1.4.2.jar!/com/acme/order/OrderCalculator.class
```

## Risks

- Filename-only matching can produce dangerous false positives.
- Modifying signed jars can invalidate signatures.
- Incorrect compression method for `BOOT-INF/lib/*.jar` can break Spring Boot launch.
- Case-insensitive filesystems may hide jar path case issues.
- Rewriting the same nested jar multiple times can introduce inefficiency and bugs.

## Mitigations

- Candidate matching is assistive and review-first.
- Only exact, unambiguous matches may be auto-selected by default.
- Patch application validates target paths before writing.
- Nested jar operations are grouped and each affected nested jar is rewritten once.
- Verification checks that outer `BOOT-INF/lib/*.jar` entries are STORED.
- Signed jar metadata triggers warnings.
