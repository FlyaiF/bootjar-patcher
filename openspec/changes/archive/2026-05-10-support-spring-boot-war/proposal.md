## Why

Some target deployments produce Spring Boot executable WAR artifacts instead of executable JARs, and those archives use `WEB-INF` layout roots that the current implementation does not index, patch, or verify. Supporting WARs lets the same inspect, find, match, apply, and verify workflow work across Spring Boot archive packaging styles.

## What Changes

- Add Spring Boot archive layout detection for executable JARs, executable WARs, and unknown readable archives.
- Support WAR application entries under `WEB-INF/classes`.
- Support WAR nested libraries under `WEB-INF/lib/*.jar` and `WEB-INF/lib-provided/*.jar`.
- Enforce STORED outer entries for WAR nested libraries during apply and verify.
- **BREAKING**: Rename CLI `--jar` options for `match` and `apply` to `--archive`; do not keep `--jar` aliases.
- Update docs, examples, and real Spring integration coverage for executable WAR artifacts.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: extend archive operations from Spring Boot executable JARs to Spring Boot executable WARs and rename named CLI archive options.

## Impact

- Affects `bootjar-core` archive indexing, matching, apply rewriting, and verification.
- Affects `bootjar-cli` option parsing and usage text.
- Updates OpenSpec requirements, README examples, CLI tests, core tests, and ignored Maven Wrapper integration tests.
