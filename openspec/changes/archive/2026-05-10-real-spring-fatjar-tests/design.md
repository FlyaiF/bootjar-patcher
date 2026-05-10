## Context

The existing core and CLI tests build synthetic zip files that mimic Spring Boot executable jars. Those fixtures are fast and deterministic, but they do not prove that `bootjar-core` handles jars produced by Spring Boot's real Maven packaging, especially nested jar storage, launcher entries, and real dependency jar layout.

## Goals / Non-Goals

**Goals:**

- Add opt-in integration coverage against a real Maven-built Spring Boot executable jar.
- Use Maven Wrapper from the fixture so CI only needs Java on `PATH`.
- Allow first-run Maven and dependency downloads for CI jobs that opt into this suite.
- Exercise core library behavior directly without adding Java requirements to normal `cargo test`.
- Keep fixture build outputs under ignored `target/` directories.

**Non-Goals:**

- Do not change CLI behavior or public core APIs.
- Do not require Maven to be installed on CI machines.
- Do not make Java/Maven integration tests run by default.
- Do not use the fixture to test Spring application runtime behavior.

## Decisions

- Create `crates/bootjar-spring-it` as a dedicated ignored-test crate. This keeps slow Java/Maven work separate from `bootjar-core` and `bootjar-cli` while still participating in the workspace.
- Place the Maven fixture under the integration crate and invoke `./mvnw -q -DskipTests package` from Rust tests. This makes the fixture self-contained and CI-friendly.
- Use a minimal multi-module Maven fixture: one Spring Boot application module plus two small dependency jar modules. The dependency modules provide stable nested jar entries and duplicate filenames for match ambiguity tests without pulling in large application code.
- Cache fixture build state by relying on Maven's normal `target/` output. Tests locate the packaged boot jar after invoking the wrapper rather than checking generated jars into the repository.
- Mark every real Spring integration test with `#[ignore]`. The explicit command is `cargo test -p bootjar-spring-it -- --ignored`.

## Risks / Trade-offs

- [First-run downloads are slower] -> Maven Wrapper and dependencies may download on a clean CI machine; subsequent runs can use Maven cache.
- [Network may be unavailable] -> The suite is opt-in and does not affect normal `cargo test`; CI jobs that enable it must provide network access or a primed Maven cache.
- [Fixture class names can drift] -> Tests should assert exact archive paths from fixture source names and fail loudly if the fixture changes.
- [Spring Boot packaging evolves] -> Pin Spring Boot and Maven Wrapper versions in the fixture to keep test artifacts stable.
