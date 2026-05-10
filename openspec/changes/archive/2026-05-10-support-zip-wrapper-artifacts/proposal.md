## Why

Real release artifacts often wrap the Spring Boot executable JAR or WAR with scripts,
configuration, and templates. Operators need one patch workflow that can update those
wrapper files and still patch the contained Spring Boot archive safely.

## What Changes

- Add support for plain ZIP wrapper artifacts containing at least one Spring Boot executable JAR or WAR.
- Extend archive paths to chained `!` syntax so patch targets can address wrapper entries, contained JAR/WAR entries, and nested libraries inside contained archives.
- Allow `inspect`, `find`, `match`, `apply`, and `verify` to operate on direct JAR/WAR inputs or wrapper ZIP inputs.
- Preserve wrapper entry metadata where practical, including compression method and Unix mode for scripts.
- Keep patch-plan format and CLI commands unchanged apart from accepting chained target paths.

## Capabilities

### New Capabilities

### Modified Capabilities

- `bootjar-patcher`: Add ZIP wrapper artifact behavior to the existing archive inspection, discovery, matching, apply, and verification flow.

## Impact

- `crates/bootjar-core`: archive path parsing, indexing, matching, apply rewrite flow, and verification.
- `crates/bootjar-cli`: reporting for wrapper layout and contained archives.
- `crates/bootjar-spring-it`: ignored Maven-backed integration tests for a real JAR/WAR placed into a ZIP distribution fixture.
- README and OpenSpec specs: document wrapper support and chained paths.
