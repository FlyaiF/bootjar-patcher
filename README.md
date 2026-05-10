# bootjar-patcher OpenSpec Starter

This archive initializes a spec-driven project for `bootjar-patcher`: a cross-platform CLI plus reusable Rust core library for inspecting, discovering, planning, and applying patches to Spring Boot executable JAR and WAR archives.

## Why this structure

`openspec/config.yaml` is intentionally compact. It only keeps durable project context that should be injected into OpenSpec work repeatedly: stack, architecture boundary, and Spring Boot archive constraints.

Detailed behavior lives in:

- `openspec/project.md` — broader product context
- `openspec/specs/bootjar-patcher/spec.md` — source-of-truth behavioral spec
- `openspec/changes/implement-archive-paths-and-inspect/` — active first implementation change

## Project goal

Patch files at any supported level of a Spring Boot executable archive:

- `BOOT-INF/classes/...`
- whole nested jars under `BOOT-INF/lib/*.jar`
- files inside nested jars using `!` syntax, e.g. `BOOT-INF/lib/a.jar!/com/acme/Foo.class`
- `WEB-INF/classes/...` in executable WARs
- nested WAR libraries under `WEB-INF/lib/*.jar` and `WEB-INF/lib-provided/*.jar`

Then rebuild the archive while preserving Spring Boot loader requirements.

## Suggested next steps

1. Unzip this archive.
2. Review `openspec/config.yaml`.
3. Review `openspec/project.md`.
4. Review the active first implementation change under `openspec/changes/implement-archive-paths-and-inspect/`.
5. Implement from that change's `tasks.md`.

## Key invariant

Outer entries for Spring Boot nested libraries must be STORED/uncompressed in the executable archive:

- `BOOT-INF/lib/*.jar` for executable JARs
- `WEB-INF/lib/*.jar` and `WEB-INF/lib-provided/*.jar` for executable WARs

The nested jar contents themselves may remain compressed.

## Real Spring Boot integration tests

The default Rust test suite stays Java-free:

```bash
cargo test
```

The opt-in real Spring Boot suite builds minimal executable JAR and WAR artifacts with the Maven Wrapper and tests `bootjar-core` against those artifacts:

```bash
cargo test -p bootjar-spring-it -- --ignored
```

Requirements for the opt-in suite:

- Java 21 available on `PATH`
- Network access on first run, unless Maven and dependencies are already cached
- No system Maven installation; tests invoke `crates/bootjar-spring-it/fixture/mvnw`
