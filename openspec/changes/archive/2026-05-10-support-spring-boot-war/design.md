## Context

The current implementation recognizes Spring Boot executable JAR layout through hardcoded `BOOT-INF/classes` and `BOOT-INF/lib` checks. Spring Boot executable WARs use `WEB-INF/classes`, `WEB-INF/lib`, and `WEB-INF/lib-provided`, while retaining the same nested archive constraint: nested libraries must be STORED in the outer archive.

## Goals / Non-Goals

**Goals:**

- Support Spring Boot executable WAR archives across inspect, find, match, apply, and verify.
- Keep patch plan archive path syntax unchanged.
- Rename named CLI archive arguments from `--jar` to `--archive`.
- Extend real Spring integration tests with a Maven-built executable WAR.

**Non-Goals:**

- Do not add support for arbitrary traditional servlet WAR verification rules.
- Do not keep a compatibility alias for `--jar`.
- Do not add runtime Java execution checks for produced WARs.

## Decisions

- Add an `ArchiveLayout` model in core with `SpringBootJar`, `SpringBootWar`, and `Unknown`. Layout detection is based on indexed entry roots rather than filename extension.
- Represent nested library roots as data derived from layout: JAR uses `BOOT-INF/lib`; WAR uses `WEB-INF/lib` and `WEB-INF/lib-provided`. This keeps nested indexing, replacement, and verification code shared.
- Preserve public patch paths exactly as archive paths. A WAR nested entry uses `WEB-INF/lib/name.jar!/path/InNested.class`; a WAR outer class/resource uses `WEB-INF/classes/...`.
- Treat whole nested jar replacement under any recognized nested library root as a nested library replacement that must be readable as a jar and written STORED.
- Rename CLI option structs and usage text around archives. `--jar` should fail as an unknown option after the breaking change.

## Risks / Trade-offs

- [Breaking CLI scripts] -> The change intentionally rejects `--jar`; README and usage text must show `--archive`.
- [Unknown archives can still be readable] -> Unknown archives remain inspectable/searchable for outer entries, but nested library storage verification only applies to recognized Spring Boot JAR/WAR roots.
- [WAR `lib-provided` handling can be missed] -> Tests must cover both `WEB-INF/lib` and `WEB-INF/lib-provided` for find, apply, and verify.
