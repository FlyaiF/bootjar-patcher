## 1. OpenSpec And Workspace Setup

- [x] 1.1 Validate the OpenSpec change artifacts before implementation
- [x] 1.2 Add a `bootjar-spring-it` workspace crate for ignored Java/Maven integration tests

## 2. Maven Wrapper Spring Fixture

- [x] 2.1 Add a minimal Maven Wrapper Spring Boot multi-module fixture
- [x] 2.2 Add fixture app resources and dependency classes that expose stable outer, nested, and ambiguous archive paths

## 3. Real Fat-Jar Integration Tests

- [x] 3.1 Add ignored tests for inspect, find, match, apply, and verify behavior against the Maven-built jar
- [x] 3.2 Add ignored tests for apply rejection and verification failure cases against real-jar-derived artifacts

## 4. Documentation And Verification

- [x] 4.1 Document Java prerequisite and the ignored test command
- [x] 4.2 Run formatting, default Rust tests, OpenSpec validation, and the ignored Spring integration suite when toolchain access allows
