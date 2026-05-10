use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::tempdir;
use zip::write::FileOptions;
use zip::CompressionMethod;

fn command(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bootjar-patcher"))
        .args(args)
        .output()
        .unwrap()
}

fn write_jar(entries: &[(&str, CompressionMethod, &[u8])]) -> PathBuf {
    let dir = tempdir().unwrap();
    let path = dir.path().join("fixture.jar");
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    for (name, method, bytes) in entries {
        let options = FileOptions::default().compression_method(*method);
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();

    std::mem::forget(dir);
    path
}

fn nested_jar_bytes(entries: &[(&str, CompressionMethod, &[u8])]) -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);

    for (name, method, bytes) in entries {
        let options = FileOptions::default().compression_method(*method);
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }

    zip.finish().unwrap().into_inner()
}

fn spring_boot_jar() -> PathBuf {
    let nested = nested_jar_bytes(&[(
        "com/acme/OrderService.class",
        CompressionMethod::Deflated,
        b"class-bytes",
    )]);

    write_jar(&[
        (
            "BOOT-INF/classes/application.yml",
            CompressionMethod::Stored,
            b"server.port: 8080",
        ),
        ("BOOT-INF/lib/order.jar", CompressionMethod::Stored, &nested),
        (
            "org/springframework/boot/loader/Launcher.class",
            CompressionMethod::Stored,
            b"boot-loader",
        ),
    ])
}

fn spring_boot_jar_bytes(compressed_nested: bool) -> Vec<u8> {
    let nested_method = if compressed_nested {
        CompressionMethod::Deflated
    } else {
        CompressionMethod::Stored
    };
    let nested = nested_jar_bytes(&[(
        "com/acme/OrderService.class",
        CompressionMethod::Deflated,
        b"class-bytes",
    )]);

    nested_jar_bytes(&[
        (
            "BOOT-INF/classes/application.yml",
            CompressionMethod::Stored,
            b"server.port: 8080",
        ),
        ("BOOT-INF/lib/order.jar", nested_method, &nested),
    ])
}

fn zip_wrapper() -> PathBuf {
    let app = spring_boot_jar_bytes(false);
    write_jar(&[
        ("bin/start.sh", CompressionMethod::Stored, b"#!/bin/sh\n"),
        (
            "config/runtime.yml",
            CompressionMethod::Deflated,
            b"env: prod\n",
        ),
        ("app/service.jar", CompressionMethod::Deflated, &app),
    ])
}

fn spring_boot_war() -> PathBuf {
    let nested = nested_jar_bytes(&[(
        "com/acme/OrderService.class",
        CompressionMethod::Deflated,
        b"class-bytes",
    )]);
    let provided = nested_jar_bytes(&[(
        "com/acme/ProvidedService.class",
        CompressionMethod::Deflated,
        b"provided-class-bytes",
    )]);

    write_jar(&[
        (
            "WEB-INF/classes/application.yml",
            CompressionMethod::Stored,
            b"server.port: 8080",
        ),
        ("WEB-INF/lib/order.jar", CompressionMethod::Stored, &nested),
        (
            "WEB-INF/lib-provided/container.jar",
            CompressionMethod::Stored,
            &provided,
        ),
        (
            "org/springframework/boot/loader/launch/WarLauncher.class",
            CompressionMethod::Stored,
            b"boot-loader",
        ),
    ])
}

fn non_spring_jar() -> PathBuf {
    write_jar(&[("com/example/App.class", CompressionMethod::Stored, b"")])
}

fn invalid_jar() -> PathBuf {
    let dir = tempdir().unwrap();
    let path = dir.path().join("invalid.jar");
    std::fs::write(&path, b"not a jar file").unwrap();
    std::mem::forget(dir);
    path
}

fn write_input_file(path: &std::path::Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, bytes).unwrap();
}

fn read_jar_entry(path: &std::path::Path, entry_name: &str) -> Vec<u8> {
    let file = std::fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name(entry_name).unwrap();
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    bytes
}

fn read_nested_jar_entry(path: &std::path::Path, nested_jar: &str, inner_path: &str) -> Vec<u8> {
    let nested_bytes = read_jar_entry(path, nested_jar);
    let cursor = std::io::Cursor::new(nested_bytes);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut entry = archive.by_name(inner_path).unwrap();
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    bytes
}

fn read_double_nested_jar_entry(
    path: &std::path::Path,
    contained: &str,
    nested_jar: &str,
    inner_path: &str,
) -> Vec<u8> {
    let contained_bytes = read_jar_entry(path, contained);
    let cursor = std::io::Cursor::new(contained_bytes);
    let mut contained_archive = zip::ZipArchive::new(cursor).unwrap();
    let mut nested = contained_archive.by_name(nested_jar).unwrap();
    let mut nested_bytes = Vec::new();
    nested.read_to_end(&mut nested_bytes).unwrap();
    let cursor = std::io::Cursor::new(nested_bytes);
    let mut nested_archive = zip::ZipArchive::new(cursor).unwrap();
    let mut entry = nested_archive.by_name(inner_path).unwrap();
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    bytes
}

fn jar_entry_compression(path: &std::path::Path, entry_name: &str) -> CompressionMethod {
    let file = std::fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let entry = archive.by_name(entry_name).unwrap();
    entry.compression()
}

#[test]
fn inspect_reports_spring_boot_layout() {
    let jar = spring_boot_jar();
    let output = command(&["inspect", jar.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Layout: Spring Boot JAR"));
    assert!(stdout.contains("BOOT-INF/classes: present"));
    assert!(stdout.contains("BOOT-INF/lib: present"));
    assert!(stdout.contains("Spring Boot launcher entries: present"));
    assert!(stdout.contains("BOOT-INF/lib/order.jar -> STORED (Stored)"));
}

#[test]
fn help_prints_build_information() {
    let output = command(&["--help"]);

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("bootjar-patcher version"));
    assert!(stderr.contains("Build:"));
    assert!(stderr.contains("Version:"));
    assert!(stderr.contains("Git commit:"));
    assert!(stderr.contains("Git tags:"));
    assert!(stderr.contains("Git branch:"));
    assert!(stderr.contains("Git dirty:"));
    assert!(stderr.contains("Build target:"));
    assert!(stderr.contains("Build profile:"));
    assert!(stderr.contains("Rustc:"));
}

#[test]
fn version_prints_build_information() {
    let output = command(&["--version"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Version:"));
    assert!(stdout.contains("Git commit:"));
    assert!(stdout.contains("Git tags:"));
    assert!(stdout.contains("Git branch:"));
    assert!(stdout.contains("Git dirty:"));
    assert!(stdout.contains("Build target:"));
    assert!(stdout.contains("Build profile:"));
    assert!(stdout.contains("Rustc:"));
}

#[test]
fn inspect_reports_spring_boot_war_layout() {
    let war = spring_boot_war();
    let output = command(&["inspect", war.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Layout: Spring Boot WAR"));
    assert!(stdout.contains("WEB-INF/classes: present"));
    assert!(stdout.contains("WEB-INF/lib: present"));
    assert!(stdout.contains("WEB-INF/lib-provided: present"));
    assert!(stdout.contains("WEB-INF/lib/order.jar -> STORED (Stored)"));
    assert!(stdout.contains("WEB-INF/lib-provided/container.jar -> STORED (Stored)"));
}

#[test]
fn inspect_reports_zip_wrapper_layout() {
    let wrapper = zip_wrapper();
    let output = command(&["inspect", wrapper.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Layout: ZIP wrapper"));
    assert!(stdout.contains("app/service.jar!/BOOT-INF/lib/order.jar -> STORED (Stored)"));
    assert!(stdout.contains("Contained archives:"));
    assert!(stdout.contains("app/service.jar -> Spring Boot JAR"));
}

#[test]
fn inspect_reports_non_spring_jar_without_failing() {
    let jar = non_spring_jar();
    let output = command(&["inspect", jar.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("BOOT-INF/classes: absent"));
    assert!(stdout.contains("BOOT-INF/lib: absent"));
    assert!(stdout.contains("Spring Boot launcher entries: absent"));
}

#[test]
fn inspect_fails_for_invalid_jar() {
    let jar = invalid_jar();
    let output = command(&["inspect", jar.to_str().unwrap()]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("inspect failed: jar is not readable"));
}

#[test]
fn find_prints_nested_match() {
    let jar = spring_boot_jar();
    let output = command(&["find", jar.to_str().unwrap(), "OrderService.class"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "BOOT-INF/lib/order.jar!/com/acme/OrderService.class\n"
    );
}

#[test]
fn find_prints_outer_path_match() {
    let jar = spring_boot_jar();
    let output = command(&[
        "find",
        jar.to_str().unwrap(),
        "BOOT-INF/classes/application.yml",
    ]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "BOOT-INF/classes/application.yml\n"
    );
}

#[test]
fn find_prints_wrapper_and_chained_matches() {
    let wrapper = zip_wrapper();
    let output = command(&["find", wrapper.to_str().unwrap(), "runtime.yml"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "config/runtime.yml\n"
    );

    let output = command(&["find", wrapper.to_str().unwrap(), "OrderService.class"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class\n"
    );
}

#[test]
fn find_succeeds_with_empty_output_for_no_match() {
    let jar = spring_boot_jar();
    let output = command(&["find", jar.to_str().unwrap(), "Missing.class"]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn find_fails_for_invalid_jar() {
    let jar = invalid_jar();
    let output = command(&["find", jar.to_str().unwrap(), "OrderService.class"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("find failed: jar is not readable"));
}

#[test]
fn match_prints_candidates_yaml_to_stdout() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    write_input_file(
        &dir.path().join("BOOT-INF/classes/application.yml"),
        b"server.port: 9090",
    );

    let output = command(&[
        "match",
        "--archive",
        jar.to_str().unwrap(),
        "--inputs",
        dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("kind: candidates\n"));
    assert!(stdout.contains("status: selected\n"));
    assert!(stdout.contains("target: \"BOOT-INF/classes/application.yml\""));
    assert!(stdout.contains("- \"exact relative path\""));
}

#[test]
fn match_prints_wrapper_chained_candidates() {
    let wrapper = zip_wrapper();
    let dir = tempdir().unwrap();
    write_input_file(&dir.path().join("config/runtime.yml"), b"env: test\n");
    write_input_file(
        &dir.path()
            .join("app/service.jar!/BOOT-INF/classes/application.yml"),
        b"server.port: 9090",
    );

    let output = command(&[
        "match",
        "--archive",
        wrapper.to_str().unwrap(),
        "--inputs",
        dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("target: \"config/runtime.yml\""));
    assert!(stdout.contains("target: \"app/service.jar!/BOOT-INF/classes/application.yml\""));
}

#[test]
fn match_writes_candidates_yaml_to_out_file() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    write_input_file(&dir.path().join("Missing.class"), b"replacement");
    let out = dir.path().join("candidates.yaml");

    let output = command(&[
        "match",
        "--archive",
        jar.to_str().unwrap(),
        "--inputs",
        dir.path().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let yaml = std::fs::read_to_string(out).unwrap();
    assert!(yaml.contains("kind: candidates\n"));
    assert!(yaml.contains("status: no-match\n"));
    assert!(yaml.contains("candidates:\n      []\n"));
}

#[test]
fn match_fails_for_missing_input_path() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing-patch-dir");

    let output = command(&[
        "match",
        "--archive",
        jar.to_str().unwrap(),
        "--inputs",
        missing.to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("match failed: could not read input path"));
}

#[test]
fn match_prints_snippets_to_stdout() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    write_input_file(
        &dir.path().join("BOOT-INF/classes/application.yml"),
        b"server.port: 9090",
    );

    let output = command(&[
        "match",
        "--archive",
        jar.to_str().unwrap(),
        "--inputs",
        dir.path().to_str().unwrap(),
        "--format",
        "snippets",
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("operations:\n"));
    assert!(stdout.contains("  - replace-entry:\n"));
    assert!(stdout.contains("      target: \"BOOT-INF/classes/application.yml\"\n"));
    assert!(stdout.contains("      with: "));
}

#[test]
fn match_writes_snippets_to_out_file() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    write_input_file(
        &dir.path().join("BOOT-INF/classes/application.yml"),
        b"server.port: 9090",
    );
    let out = dir.path().join("patch-snippets.yaml");

    let output = command(&[
        "match",
        "--archive",
        jar.to_str().unwrap(),
        "--inputs",
        dir.path().to_str().unwrap(),
        "--format",
        "snippets",
        "--out",
        out.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let snippets = std::fs::read_to_string(out).unwrap();
    assert!(snippets.contains("operations:\n"));
    assert!(snippets.contains("  - replace-entry:\n"));
}

#[test]
fn match_rejects_unknown_format() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    write_input_file(&dir.path().join("Missing.class"), b"replacement");

    let output = command(&[
        "match",
        "--archive",
        jar.to_str().unwrap(),
        "--inputs",
        dir.path().to_str().unwrap(),
        "--format",
        "xml",
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown match format: xml"));
}

#[test]
fn match_rejects_legacy_jar_option() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    write_input_file(&dir.path().join("Missing.class"), b"replacement");

    let output = command(&[
        "match",
        "--jar",
        jar.to_str().unwrap(),
        "--inputs",
        dir.path().to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown match option: --jar"));
}

#[test]
fn apply_replaces_outer_entry() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    let replacement = dir.path().join("application.yml");
    let plan = dir.path().join("patch-plan.yaml");
    let output_jar = dir.path().join("app-patched.jar");
    write_input_file(&replacement, b"server.port: 9090");
    std::fs::write(
        &plan,
        format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/classes/application.yml
      with: "{}"
"#,
            replacement.display()
        ),
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        jar.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_jar.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        read_jar_entry(&output_jar, "BOOT-INF/classes/application.yml"),
        b"server.port: 9090"
    );
    assert_eq!(
        read_jar_entry(&jar, "BOOT-INF/classes/application.yml"),
        b"server.port: 8080"
    );
}

#[test]
fn apply_replaces_nested_entry() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    let replacement = dir.path().join("OrderService.class");
    let plan = dir.path().join("patch-plan.yaml");
    let output_jar = dir.path().join("app-patched.jar");
    write_input_file(&replacement, b"patched-class-bytes");
    std::fs::write(
        &plan,
        format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/lib/order.jar!/com/acme/OrderService.class
      with: "{}"
"#,
            replacement.display()
        ),
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        jar.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_jar.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        read_nested_jar_entry(
            &output_jar,
            "BOOT-INF/lib/order.jar",
            "com/acme/OrderService.class"
        ),
        b"patched-class-bytes"
    );
}

#[test]
fn apply_replaces_whole_nested_jar() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    let replacement = dir.path().join("order-replacement.jar");
    let plan = dir.path().join("patch-plan.yaml");
    let output_jar = dir.path().join("app-patched.jar");
    let replacement_bytes = nested_jar_bytes(&[(
        "com/acme/NewOrderService.class",
        CompressionMethod::Deflated,
        b"new-class-bytes",
    )]);
    write_input_file(&replacement, &replacement_bytes);
    std::fs::write(
        &plan,
        format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/lib/order.jar
      with: "{}"
"#,
            replacement.display()
        ),
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        jar.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_jar.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        read_jar_entry(&output_jar, "BOOT-INF/lib/order.jar"),
        replacement_bytes
    );
    assert_eq!(
        jar_entry_compression(&output_jar, "BOOT-INF/lib/order.jar"),
        CompressionMethod::Stored
    );
}

#[test]
fn apply_rejects_candidates_file() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    let plan = dir.path().join("candidates.yaml");
    let output_jar = dir.path().join("app-patched.jar");
    std::fs::write(
        &plan,
        r#"
kind: candidates
version: 1
matches: []
"#,
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        jar.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_jar.to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("apply failed: candidates files are not reviewed patch plans"));
    assert!(!output_jar.exists());
}

#[test]
fn apply_replaces_war_outer_entry() {
    let war = spring_boot_war();
    let dir = tempdir().unwrap();
    let replacement = dir.path().join("application.yml");
    let plan = dir.path().join("patch-plan.yaml");
    let output_war = dir.path().join("app-patched.war");
    write_input_file(&replacement, b"server.port: 9090");
    std::fs::write(
        &plan,
        format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: WEB-INF/classes/application.yml
      with: "{}"
"#,
            replacement.display()
        ),
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        war.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_war.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        read_jar_entry(&output_war, "WEB-INF/classes/application.yml"),
        b"server.port: 9090"
    );
}

#[test]
fn apply_replaces_war_nested_entry() {
    let war = spring_boot_war();
    let dir = tempdir().unwrap();
    let replacement = dir.path().join("ProvidedService.class");
    let plan = dir.path().join("patch-plan.yaml");
    let output_war = dir.path().join("app-patched.war");
    write_input_file(&replacement, b"patched-provided-class");
    std::fs::write(
        &plan,
        format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class
      with: "{}"
"#,
            replacement.display()
        ),
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        war.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_war.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        read_nested_jar_entry(
            &output_war,
            "WEB-INF/lib-provided/container.jar",
            "com/acme/ProvidedService.class"
        ),
        b"patched-provided-class"
    );
    assert_eq!(
        jar_entry_compression(&output_war, "WEB-INF/lib-provided/container.jar"),
        CompressionMethod::Stored
    );
}

#[test]
fn apply_replaces_wrapper_and_chained_entries() {
    let wrapper = zip_wrapper();
    let dir = tempdir().unwrap();
    let config_replacement = dir.path().join("runtime.yml");
    let class_replacement = dir.path().join("OrderService.class");
    let plan = dir.path().join("patch-plan.yaml");
    let output_zip = dir.path().join("dist-patched.zip");
    write_input_file(&config_replacement, b"env: patched\n");
    write_input_file(&class_replacement, b"patched-class");
    std::fs::write(
        &plan,
        format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: config/runtime.yml
      with: "{}"
  - replace-entry:
      target: app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class
      with: "{}"
"#,
            config_replacement.display(),
            class_replacement.display()
        ),
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        wrapper.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_zip.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        read_jar_entry(&output_zip, "config/runtime.yml"),
        b"env: patched\n"
    );
    assert_eq!(
        read_double_nested_jar_entry(
            &output_zip,
            "app/service.jar",
            "BOOT-INF/lib/order.jar",
            "com/acme/OrderService.class"
        ),
        b"patched-class"
    );
}

#[test]
fn apply_rejects_legacy_jar_option() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    let plan = dir.path().join("patch-plan.yaml");
    let output_jar = dir.path().join("app-patched.jar");
    std::fs::write(&plan, "kind: patch-plan\nversion: 1\noperations: []\n").unwrap();

    let output = command(&[
        "apply",
        "--jar",
        jar.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_jar.to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown apply option: --jar"));
}

#[test]
fn apply_fails_after_writing_output_that_does_not_verify() {
    let nested = nested_jar_bytes(&[(
        "com/acme/OrderService.class",
        CompressionMethod::Stored,
        b"class-bytes",
    )]);
    let jar = write_jar(&[
        (
            "BOOT-INF/classes/application.yml",
            CompressionMethod::Stored,
            b"server.port: 8080",
        ),
        (
            "BOOT-INF/lib/order.jar",
            CompressionMethod::Deflated,
            &nested,
        ),
    ]);
    let dir = tempdir().unwrap();
    let replacement = dir.path().join("application.yml");
    let plan = dir.path().join("patch-plan.yaml");
    let output_jar = dir.path().join("app-patched.jar");
    write_input_file(&replacement, b"server.port: 9090");
    std::fs::write(
        &plan,
        format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/classes/application.yml
      with: "{}"
"#,
            replacement.display()
        ),
    )
    .unwrap();

    let output = command(&[
        "apply",
        "--archive",
        jar.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        output_jar.to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("apply failed: verification failed after writing output"));
    assert!(stderr.contains("BOOT-INF/lib/order.jar"));
    assert!(output_jar.exists());
    assert_eq!(
        read_jar_entry(&output_jar, "BOOT-INF/classes/application.yml"),
        b"server.port: 9090"
    );
    assert_eq!(
        jar_entry_compression(&output_jar, "BOOT-INF/lib/order.jar"),
        CompressionMethod::Deflated
    );
}

#[test]
fn verify_succeeds_for_stored_nested_jars() {
    let jar = spring_boot_jar();
    let output = command(&["verify", jar.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Readable: present"));
    assert!(stdout.contains("BOOT-INF/lib/order.jar -> STORED (Stored)"));
    assert!(stdout.contains("Nested jar storage: ok"));
}

#[test]
fn verify_succeeds_for_zip_wrapper() {
    let wrapper = zip_wrapper();
    let output = command(&["verify", wrapper.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app/service.jar!/BOOT-INF/lib/order.jar -> STORED (Stored)"));
    assert!(stdout.contains("Nested jar storage: ok"));
    assert!(stdout.contains("app/service.jar -> Spring Boot JAR"));
}

#[test]
fn verify_fails_for_compressed_nested_jar() {
    let nested = nested_jar_bytes(&[(
        "com/acme/OrderService.class",
        CompressionMethod::Stored,
        b"",
    )]);
    let jar = write_jar(&[(
        "BOOT-INF/lib/order.jar",
        CompressionMethod::Deflated,
        &nested,
    )]);

    let output = command(&["verify", jar.to_str().unwrap()]);

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("BOOT-INF/lib/order.jar -> not STORED (Deflated)"));
    assert!(stdout.contains("Nested jar storage: failed"));
}

#[test]
fn verify_fails_for_compressed_war_nested_jar() {
    let nested = nested_jar_bytes(&[(
        "com/acme/OrderService.class",
        CompressionMethod::Stored,
        b"",
    )]);
    let war = write_jar(&[
        (
            "WEB-INF/classes/application.yml",
            CompressionMethod::Stored,
            b"server.port: 8080",
        ),
        (
            "WEB-INF/lib/order.jar",
            CompressionMethod::Deflated,
            &nested,
        ),
    ]);

    let output = command(&["verify", war.to_str().unwrap()]);

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("WEB-INF/lib/order.jar -> not STORED (Deflated)"));
    assert!(stdout.contains("Nested jar storage: failed"));
}

#[test]
fn verify_fails_for_zip_wrapper_with_compressed_contained_nested_jar() {
    let app = spring_boot_jar_bytes(true);
    let wrapper = write_jar(&[("app/service.jar", CompressionMethod::Deflated, &app)]);

    let output = command(&["verify", wrapper.to_str().unwrap()]);

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app/service.jar!/BOOT-INF/lib/order.jar -> not STORED (Deflated)"));
    assert!(stdout.contains("Nested jar storage: failed"));
}
