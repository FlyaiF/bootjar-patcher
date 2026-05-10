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
