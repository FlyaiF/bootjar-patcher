## 1. OpenSpec Setup

- [x] 1.1 Validate support-spring-boot-war artifacts before implementation

## 2. Core Archive Layout Support

- [x] 2.1 Add archive layout detection for Spring Boot JAR, Spring Boot WAR, and unknown archives
- [x] 2.2 Generalize nested library indexing and verification across JAR and WAR roots
- [x] 2.3 Generalize apply replacement handling for WAR classes and nested libraries

## 3. CLI Rename

- [x] 3.1 Rename `match --jar` to `match --archive` and reject `--jar`
- [x] 3.2 Rename `apply --jar` to `apply --archive` and reject `--jar`
- [x] 3.3 Update CLI usage text to use archive wording

## 4. Tests And Real Fixture

- [x] 4.1 Add core and CLI coverage for Spring Boot WAR inspect, find, match, apply, and verify behavior
- [x] 4.2 Extend the Maven Wrapper integration fixture with an executable WAR module
- [x] 4.3 Add ignored real Spring WAR integration tests

## 5. Docs And Verification

- [x] 5.1 Update README and canonical OpenSpec spec for archive/WAR wording
- [x] 5.2 Run formatting, default tests, ignored Spring integration tests, and OpenSpec validation
