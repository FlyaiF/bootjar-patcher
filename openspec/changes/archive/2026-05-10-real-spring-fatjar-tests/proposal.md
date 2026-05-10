## Why

The current tests use synthetic zip fixtures, which cover core behavior but cannot prove that the implementation handles jars produced by the real Spring Boot build tooling. A Maven-built executable jar gives confidence that `BOOT-INF/classes`, `BOOT-INF/lib`, launcher entries, and STORED nested jar requirements match production artifacts.

## What Changes

- Add an opt-in Rust integration-test crate that builds a minimal Spring Boot executable jar with Maven Wrapper.
- Check in the Spring Boot fixture and Maven Wrapper files so CI does not need system Maven.
- Exercise `bootjar-core` inspect, find, match, apply, and verify behavior against the real fat jar.
- Keep the real Spring suite ignored by default so normal `cargo test` remains Java-free.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bootjar-patcher`: add opt-in real Spring Boot fat-jar integration coverage for the existing behavior contract.

## Impact

- Adds a workspace test crate with Java/Maven fixture sources.
- Adds tracked Maven Wrapper files for the fixture.
- Adds documentation for running the ignored integration suite.
- Does not change public Rust APIs or CLI behavior.
