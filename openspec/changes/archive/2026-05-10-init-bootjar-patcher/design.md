# Design: bootjar-patcher initial architecture

## Overview

`bootjar-patcher` is a library-first CLI application.

The core library owns archive indexing, path parsing, matching, patch planning,
patch execution, and verification. The CLI owns argument parsing, terminal formatting,
file input/output selection, and exit codes.

## Workspace Layout

```text
crates/
  bootjar-core/
  bootjar-cli/
openspec/
test-fixtures/
```

## Core Data Model

### ArchivePath

Represents either an outer jar entry or an entry inside a nested jar.

```rust
pub enum ArchivePath {
    Outer {
        path: String,
    },
    Nested {
        outer_jar: String,
        inner_path: String,
    },
}
```

Examples:

```text
BOOT-INF/classes/application.yml
BOOT-INF/lib/order-module-1.4.2.jar!/com/acme/order/OrderCalculator.class
```

### PatchOperation

```rust
pub enum PatchOperation {
    Add {
        target: ArchivePath,
        source: PathBuf,
    },
    Replace {
        target: ArchivePath,
        source: PathBuf,
    },
    Delete {
        target: ArchivePath,
    },
}
```

### PatchPlan

```rust
pub struct PatchPlan {
    pub version: u32,
    pub operations: Vec<PatchOperation>,
    pub options: PatchOptions,
}
```

### CandidateMatch

```rust
pub struct MatchResult {
    pub input: PathBuf,
    pub status: MatchStatus,
    pub candidates: Vec<Candidate>,
}

pub enum MatchStatus {
    Selected,
    NeedsSelection,
    NoMatch,
}

pub struct Candidate {
    pub target: ArchivePath,
    pub score: u8,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}
```

## Commands

### inspect

Reads a jar and reports:

- whether it appears to be a Spring Boot executable jar
- whether `BOOT-INF/classes` exists
- how many `BOOT-INF/lib/*.jar` entries exist
- whether nested jar entries are STORED
- whether signed jar metadata appears

### tree

Displays outer entries and, optionally, nested jar entries.

Default behavior SHOULD summarize nested jars.
`--include-nested` SHOULD print entries inside nested jars.

### find

Searches indexed outer and nested paths by filename, glob, or substring.

### match

Inputs:

```bash
bootjar-patcher match --jar app.jar --inputs ./patch-files --out candidates.yaml
```

Behavior:

1. Index the target fat jar.
2. Index nested jars under `BOOT-INF/lib` when enabled.
3. Walk the replacement input directory.
4. Score possible targets for each input file.
5. Emit candidate results in YAML, JSON, table, or snippets format.

Default policy:

- include nested jars
- max 10 candidates per input
- minimum score 60
- auto-select exact-only
- never auto-apply a candidates file

### apply

Inputs:

```bash
bootjar-patcher apply --jar app.jar --plan patch.yaml --out app-patched.jar
```

Behavior:

1. Parse and validate patch plan.
2. Index the outer jar.
3. Group nested operations by containing nested jar.
4. Rewrite each affected nested jar once.
5. Rewrite the outer jar once.
6. Force outer `BOOT-INF/lib/*.jar` entries to STORED.
7. Verify output.

### verify

Checks the patched jar for structural validity and Spring Boot constraints.

## Matching Design

Candidate scoring is transparent and reasoned.

Suggested scoring:

```text
+100 exact full relative path match
+80  same filename
+20  same Java package path for .class
+15  input parent directory appears to match module or jar name
+10  known Spring config/resource location
-30  ambiguous duplicate basename
-40  Java package mismatch
-50  target under META-INF signature material
```

For `.class` files, the library SHOULD attempt to parse the internal class name from
the class file constant pool. If parsing fails, fallback to filename matching.

For config/resource files, the library SHOULD recognize common Spring resource names
such as:

- application.yml
- application.yaml
- application.properties
- bootstrap.yml
- bootstrap.properties
- logback-spring.xml

## Patch Execution Design

Patch execution MUST avoid repeatedly rewriting the same nested jar.

Execution model:

```text
1. parse plan
2. validate source files and targets
3. group operations:
   - outer operations
   - nested operations by BOOT-INF/lib/*.jar
4. rewrite each affected nested jar into temp bytes/file
5. rewrite the outer jar once
6. ensure BOOT-INF/lib/*.jar entries are STORED
7. verify output
```

## ZIP/JAR Write Rules

For stored entries, the writer must know size and CRC32 before writing the entry.

When replacing a nested jar under `BOOT-INF/lib`, the outer jar entry MUST be written
as STORED. The inner jar's own entries MAY preserve their original compression methods.

## Failure Behavior

The tool MUST fail without writing the final output when:

- the input jar cannot be read
- a replacement file is missing
- a replace target is missing and create-missing is not enabled
- multiple operations target the same path incompatibly
- a nested jar cannot be read
- the output jar fails verification

The tool SHOULD warn, not fail, when:

- signed jar metadata is detected
- a match is ambiguous
- a path differs only by case
- a filename-only candidate has low confidence
