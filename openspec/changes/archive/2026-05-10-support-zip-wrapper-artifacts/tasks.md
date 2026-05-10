## 1. OpenSpec Setup

- [x] 1.1 Validate support-zip-wrapper-artifacts artifacts before implementation

## 2. Core Path And Indexing

- [x] 2.1 Generalize archive path parsing to chained `!` segments while preserving existing one-level API behavior where practical
- [x] 2.2 Add ZIP wrapper layout detection and contained Spring Boot archive indexing
- [x] 2.3 Include wrapper entries, contained archive entries, and dependency entries in find and match targets

## 3. Core Apply And Verify

- [x] 3.1 Rewrite wrapper-level entries while preserving compression, modified time, and Unix mode where available
- [x] 3.2 Rewrite entries inside contained JAR/WAR archives using chained paths
- [x] 3.3 Rewrite dependency entries inside contained archives and preserve Spring Boot nested library STORED rules
- [x] 3.4 Verify direct JAR/WAR inputs and every contained Spring Boot archive inside ZIP wrappers

## 4. CLI And Tests

- [x] 4.1 Update CLI inspect and verify output to report ZIP wrapper layout and contained archives
- [x] 4.2 Add core tests for wrapper detection, find, match, apply, failure modes, and verify
- [x] 4.3 Add CLI tests for wrapper inspect, find, match, apply, verify, and chained paths
- [x] 4.4 Extend real Spring integration tests with a ZIP wrapper distribution fixture

## 5. Docs And Validation

- [x] 5.1 Update README and canonical OpenSpec spec for ZIP wrapper support
- [x] 5.2 Run formatting, default tests, ignored Spring integration tests, and OpenSpec validation
- [x] 5.3 Archive support-zip-wrapper-artifacts after implementation
