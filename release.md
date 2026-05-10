# Release Runbook

This project publishes the `bootjar-patcher` CLI from the `bootjar-patcher`
Cargo package. Releases are driven by GitHub Actions and include native binaries
for Linux, macOS, and Windows.

## Workflows

### CI

Workflow: `.github/workflows/ci.yml`

Runs on pushes to `main`/`master`, pull requests, and manual dispatch.

Jobs:

- Rust matrix on Ubuntu, macOS, and Windows:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo build -p bootjar-patcher --release`
- Real Spring Boot integration on Ubuntu:
  - installs Java 21
  - uses Maven dependency caching
  - runs `cargo test -p bootjar-spring-it -- --ignored`

### Release

Workflow: `.github/workflows/release.yml`

Runs when a tag matching `v*` is pushed, or by manual dispatch with a tag input.

The release workflow first validates the release ref on Ubuntu:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo test -p bootjar-spring-it -- --ignored`

The release workflow builds:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `x86_64-pc-windows-msvc`

Each artifact contains:

- `bootjar-patcher` or `bootjar-patcher.exe`
- `RUNBOOK.md`
- a `.sha256` checksum file

The publish job creates or updates the GitHub Release and uploads all packaged
artifacts.

## Release Procedure

1. Start from a clean checkout on `master`.

   ```bash
   git status --short
   ```

2. Run local validation.

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cargo test -p bootjar-spring-it -- --ignored
   ```

3. Choose the release version and update crate versions if needed.

   Current crates:

   - `crates/bootjar-cli`
   - `crates/bootjar-core`
   - `crates/bootjar-spring-it` is `publish = false`

4. Commit any version or documentation changes.

5. Create and push an annotated tag.

   ```bash
   git tag -a v0.1.0 -m "v0.1.0"
   git push origin v0.1.0
   ```

6. Watch the Release workflow in GitHub Actions.

7. Download one artifact and confirm build metadata.

   ```bash
   ./bootjar-patcher --version
   ```

   Expected output includes package version, Git commit, tags, branch, dirty state,
   build target, build profile, and `rustc --version`.

8. Confirm checksums are uploaded beside each archive.

## Manual Release Dispatch

Use manual dispatch only when the tag already exists or when rerunning a failed
publish. Pass the exact tag name, for example `v0.1.0`.

## Failure Handling

- If CI fails before tagging, fix the code and rerun CI before creating the tag.
- If release build fails after pushing a tag, push a fix and create a new patch tag.
- If only publishing failed and binaries are correct, rerun the Release workflow
  manually with the same tag.
- If an uploaded artifact is bad, delete the GitHub Release, delete the tag if
  necessary, and publish a corrected patch version.
