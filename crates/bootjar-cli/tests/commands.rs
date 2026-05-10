use std::io::Write;
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

#[test]
fn inspect_reports_spring_boot_layout() {
    let jar = spring_boot_jar();
    let output = command(&["inspect", jar.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("BOOT-INF/classes: present"));
    assert!(stdout.contains("BOOT-INF/lib: present"));
    assert!(stdout.contains("Spring Boot launcher entries: present"));
    assert!(stdout.contains("BOOT-INF/lib/order.jar -> STORED (Stored)"));
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
        "--jar",
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
fn match_writes_candidates_yaml_to_out_file() {
    let jar = spring_boot_jar();
    let dir = tempdir().unwrap();
    write_input_file(&dir.path().join("Missing.class"), b"replacement");
    let out = dir.path().join("candidates.yaml");

    let output = command(&[
        "match",
        "--jar",
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
        "--jar",
        jar.to_str().unwrap(),
        "--inputs",
        missing.to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("match failed: could not read input path"));
}
