# bootjar-patcher OpenSpec Starter

This archive initializes a spec-driven project for `bootjar-patcher`: a cross-platform CLI plus reusable Rust core library for inspecting, discovering, planning, and applying patches to Spring Boot executable fat jars.

## Why this structure

`openspec/config.yaml` is intentionally compact. It only keeps durable project context that should be injected into OpenSpec work repeatedly: stack, architecture boundary, and Spring Boot archive constraints.

Detailed behavior lives in:

- `openspec/project.md` — broader product context
- `openspec/specs/bootjar-patcher/spec.md` — source-of-truth behavioral spec
- `openspec/changes/implement-archive-paths-and-inspect/` — active first implementation change

## Project goal

Patch files at any supported level of a Spring Boot executable jar:

- `BOOT-INF/classes/...`
- whole nested jars under `BOOT-INF/lib/*.jar`
- files inside nested jars using `!` syntax, e.g. `BOOT-INF/lib/a.jar!/com/acme/Foo.class`

Then rebuild the fat jar while preserving Spring Boot loader requirements.

## Suggested next steps

1. Unzip this archive.
2. Review `openspec/config.yaml`.
3. Review `openspec/project.md`.
4. Review the active first implementation change under `openspec/changes/implement-archive-paths-and-inspect/`.
5. Implement from that change's `tasks.md`.

## Key invariant

Outer entries for `BOOT-INF/lib/*.jar` must be STORED/uncompressed in the executable jar. The nested jar contents themselves may remain compressed.
