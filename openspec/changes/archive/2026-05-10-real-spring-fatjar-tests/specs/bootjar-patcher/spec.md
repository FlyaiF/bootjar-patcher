## ADDED Requirements

### Requirement: Real Spring Boot fat-jar integration coverage

The project MUST provide opt-in integration tests that build a minimal Spring Boot
executable jar with Maven Wrapper and exercise the core library against that jar.

The integration tests MUST NOT run as part of default `cargo test`.

The integration tests MUST require Java on `PATH` but MUST NOT require a system
Maven installation.

The integration tests MAY download Maven and Maven dependencies when they are run
without a primed local cache.

The integration tests MUST cover inspect, find, match, apply, and verify behavior
against the Maven-built executable jar.

The integration tests MUST verify that nested jars under `BOOT-INF/lib/*.jar` are
STORED in valid Spring Boot outputs.

#### Scenario: Run real Spring Boot integration tests explicitly

Given Java is available on `PATH`
When the user runs `cargo test -p bootjar-spring-it -- --ignored`
Then the test suite builds the fixture with Maven Wrapper
And the tests exercise core behavior against the resulting executable jar

#### Scenario: Keep default Rust tests Java-free

Given the user runs `cargo test`
When the default workspace tests execute
Then the real Spring Boot integration tests are not run
And no Java or Maven Wrapper invocation is required by those ignored tests
