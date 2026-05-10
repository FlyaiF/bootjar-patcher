# bootjar-patcher CLI Runbook

This runbook covers the `bootjar-patcher` binary in this crate. The CLI operates on
Spring Boot executable JARs, executable WARs, and plain ZIP wrapper artifacts that
contain Spring Boot archives.

## Quick Workflow

1. Inspect the archive layout and Spring Boot nested jar storage:

   ```bash
   bootjar-patcher inspect dist.zip
   ```

2. Find the target path to patch:

   ```bash
   bootjar-patcher find dist.zip application.yml
   bootjar-patcher find dist.zip OrderService.class
   ```

3. Generate candidate matches from replacement files:

   ```bash
   bootjar-patcher match --archive dist.zip --inputs ./patch --out candidates.yaml
   bootjar-patcher match --archive dist.zip --inputs ./patch --format snippets --out patch-snippets.yaml
   ```

4. Review candidate output and create a `patch-plan` YAML.

5. Apply to a new archive:

   ```bash
   bootjar-patcher apply --archive dist.zip --plan patch.yaml --out dist-patched.zip
   ```

6. Verify the output:

   ```bash
   bootjar-patcher verify dist-patched.zip
   ```

## Supported Archive Paths

Patch targets use archive-style `/` separators and `!` for nested archive levels.

Direct JAR target:

```text
BOOT-INF/classes/application.yml
BOOT-INF/lib/order.jar!/com/acme/OrderService.class
```

Executable WAR target:

```text
WEB-INF/classes/application.yml
WEB-INF/lib/order.jar!/com/acme/OrderService.class
WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class
```

ZIP wrapper target:

```text
config/runtime.yml
bin/start.sh
app/service.jar!/BOOT-INF/classes/application.yml
app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class
```

For ZIP wrappers, command output omits the wrapper file name itself. For example,
when running against `dist.zip`, a contained archive path is shown as
`app/service.jar!/BOOT-INF/classes/application.yml`, not
`dist.zip!/app/service.jar!/BOOT-INF/classes/application.yml`.

## Commands

### inspect

```bash
bootjar-patcher inspect <archive>
```

Reports:

- detected layout: Spring Boot JAR, Spring Boot WAR, ZIP wrapper, or unknown
- Spring Boot layout markers
- nested jar storage status
- contained Spring Boot archives for ZIP wrappers

### find

```bash
bootjar-patcher find <archive> <query>
```

Searches outer entries, contained archive entries, and nested dependency entries.
Matches are printed one path per line. No matches produce success with empty output.

### match

```bash
bootjar-patcher match --archive <archive> --inputs <path> [--format candidates|snippets] [--out <file>]
```

Use `candidates` for review data and `snippets` for copyable patch-plan fragments.
Ambiguous filename matches are marked `needs-selection`; they are not auto-applied.

### apply

```bash
bootjar-patcher apply --archive <archive> --plan <plan> --out <archive>
```

Writes a new archive and verifies it after writing. The input archive is not mutated.
If post-write verification fails, the output archive is left in place for inspection.

### verify

```bash
bootjar-patcher verify <archive>
```

Checks that the archive is readable and that every Spring Boot nested library entry is
STORED/uncompressed:

- `BOOT-INF/lib/*.jar` for executable JARs
- `WEB-INF/lib/*.jar` and `WEB-INF/lib-provided/*.jar` for executable WARs
- the same rules inside every contained Spring Boot archive in a ZIP wrapper

The contained Spring Boot archive entry itself does not have to be STORED in the
wrapper ZIP.

## Patch Plan Example

```yaml
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: config/runtime.yml
      with: ./patch/runtime.yml

  - replace-entry:
      target: bin/start.sh
      with: ./patch/start.sh

  - replace-entry:
      target: app/service.jar!/BOOT-INF/classes/application.yml
      with: ./patch/application.yml

  - replace-entry:
      target: app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class
      with: ./patch/OrderService.class
```

## Exit Codes

- `0`: command succeeded
- `1`: command ran but failed validation, reading, matching, applying, or verification
- `2`: usage or argument error

## Troubleshooting

- `unknown match option: --jar` or `unknown apply option: --jar`: use `--archive`.
- `candidates files are not reviewed patch plans`: convert candidate output into a
  `kind: patch-plan` document before applying.
- `non-STORED nested jars`: rebuild or replace the affected Spring Boot nested jar so
  the outer nested library entry is STORED.
- `missing Spring Boot contained archives`: a ZIP wrapper apply replaced or removed a
  contained Spring Boot JAR/WAR. Replace it with a readable Spring Boot archive with
  the same layout.
- No output from `find`: the command succeeded but no archive path matched the query.
