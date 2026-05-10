use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use bootjar_core::{
    apply_patch_plan, find_in_jar, inspect_jar, match_in_jar, verify_jar, ApplyError, MatchStatus,
};
use tempfile::tempdir;
use zip::write::FileOptions;
use zip::CompressionMethod;

const APP_JAR: &str = "app/target/bootjar-patcher-fixture-app-0.1.0.jar";
const LIB_ONE_JAR: &str = "BOOT-INF/lib/fixture-lib-one-0.1.0.jar";
const LIB_TWO_JAR: &str = "BOOT-INF/lib/fixture-lib-two-0.1.0.jar";
const APPLICATION_YML: &str = "BOOT-INF/classes/application.yml";
const APP_BANNER: &str = "BOOT-INF/classes/com/acme/app/banner.txt";
const ORDER_CLASS: &str = "com/acme/libone/OrderService.class";
const INVENTORY_CLASS: &str = "com/acme/libtwo/InventoryService.class";
const DUPLICATE_ONE: &str = "com/acme/shared/DuplicateName.class";
const DUPLICATE_TWO: &str = "com/acme/other/DuplicateName.class";

static REAL_SPRING_JAR: OnceLock<PathBuf> = OnceLock::new();

fn real_spring_jar() -> &'static PathBuf {
    REAL_SPRING_JAR.get_or_init(|| {
        let fixture = fixture_dir();
        let wrapper = if cfg!(windows) { "mvnw.cmd" } else { "./mvnw" };
        let status = Command::new(wrapper)
            .args(["-q", "-DskipTests", "package"])
            .current_dir(&fixture)
            .status()
            .expect("failed to run Maven Wrapper");

        assert!(status.success(), "Maven Wrapper build failed: {status}");

        let jar = fixture.join(APP_JAR);
        assert!(
            jar.exists(),
            "Spring Boot jar was not built: {}",
            jar.display()
        );
        jar
    })
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixture")
}

fn read_jar_entry(path: &Path, entry_name: &str) -> Vec<u8> {
    let file = File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name(entry_name).unwrap();
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    bytes
}

fn read_nested_jar_entry(path: &Path, nested_jar: &str, inner_path: &str) -> Vec<u8> {
    let nested_bytes = read_jar_entry(path, nested_jar);
    let cursor = std::io::Cursor::new(nested_bytes);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut entry = archive.by_name(inner_path).unwrap();
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    bytes
}

fn jar_entry_compression(path: &Path, entry_name: &str) -> CompressionMethod {
    let file = File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let entry = archive.by_name(entry_name).unwrap();
    entry.compression()
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}

fn write_patch_plan(path: &Path, operations: &[(&str, &Path)]) {
    let mut yaml = String::from("kind: patch-plan\nversion: 1\noperations:\n");
    for (target, source) in operations {
        yaml.push_str("  - replace-entry:\n");
        yaml.push_str("      target: ");
        yaml.push_str(target);
        yaml.push('\n');
        yaml.push_str("      with: \"");
        yaml.push_str(&source.display().to_string().replace('\\', "\\\\"));
        yaml.push_str("\"\n");
    }
    fs::write(path, yaml).unwrap();
}

fn copy_with_compressed_nested_jar(input: &Path, output: &Path, nested_target: &str) {
    let input_file = File::open(input).unwrap();
    let mut input_archive = zip::ZipArchive::new(input_file).unwrap();
    let output_file = File::create(output).unwrap();
    let mut output_archive = zip::ZipWriter::new(output_file);

    for index in 0..input_archive.len() {
        let mut entry = input_archive.by_index(index).unwrap();
        let name = entry.name().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();

        let compression_method = if name == nested_target {
            CompressionMethod::Deflated
        } else {
            entry.compression()
        };
        let options = FileOptions::default().compression_method(compression_method);

        if entry.is_dir() {
            output_archive.add_directory(name, options).unwrap();
        } else {
            output_archive.start_file(name, options).unwrap();
            output_archive.write_all(&bytes).unwrap();
        }
    }

    output_archive.finish().unwrap();
}

#[test]
#[ignore]
fn inspect_and_verify_real_spring_boot_jar() {
    let jar = real_spring_jar();

    let inspect = inspect_jar(jar).unwrap();
    assert!(inspect.has_boot_inf_classes);
    assert!(inspect.has_boot_inf_lib);
    assert!(inspect.has_boot_loader_entry);
    assert!(inspect
        .nested_jars
        .iter()
        .any(|nested| nested.path == LIB_ONE_JAR && nested.is_stored));
    assert!(inspect.nested_jars.iter().all(|nested| nested.is_stored));

    let verify = verify_jar(jar).unwrap();
    assert!(verify.is_success());
    assert!(verify.non_stored_nested_jars.is_empty());
}

#[test]
#[ignore]
fn find_and_match_real_spring_boot_entries() {
    let jar = real_spring_jar();

    let order_matches = find_in_jar(jar, "OrderService.class").unwrap();
    assert!(order_matches
        .iter()
        .any(|result| { result.archive_path == format!("{LIB_ONE_JAR}!/{ORDER_CLASS}") }));

    let application_matches = find_in_jar(jar, APPLICATION_YML).unwrap();
    assert!(application_matches
        .iter()
        .any(|result| result.archive_path == APPLICATION_YML));

    let missing_matches = find_in_jar(jar, "DefinitelyMissing.class").unwrap();
    assert!(missing_matches.is_empty());

    let dir = tempdir().unwrap();
    let exact = dir.path().join(APPLICATION_YML);
    write_file(&exact, b"fixture:\n  message: patched\n");
    let duplicate = dir.path().join("DuplicateName.class");
    write_file(&duplicate, b"ambiguous");
    let missing = dir.path().join("Missing.class");
    write_file(&missing, b"missing");

    let candidates = match_in_jar(jar, &[dir.path().to_path_buf()]).unwrap();

    let exact_match = candidates
        .matches
        .iter()
        .find(|input| input.input.ends_with(APPLICATION_YML))
        .unwrap();
    assert_eq!(exact_match.status, MatchStatus::Selected);
    assert!(exact_match
        .candidates
        .iter()
        .any(|candidate| candidate.target == APPLICATION_YML));

    let duplicate_match = candidates
        .matches
        .iter()
        .find(|input| input.input.ends_with("DuplicateName.class"))
        .unwrap();
    assert_eq!(duplicate_match.status, MatchStatus::NeedsSelection);
    assert!(duplicate_match
        .candidates
        .iter()
        .any(|candidate| candidate.target == format!("{LIB_ONE_JAR}!/{DUPLICATE_ONE}")));
    assert!(duplicate_match
        .candidates
        .iter()
        .any(|candidate| candidate.target == format!("{LIB_TWO_JAR}!/{DUPLICATE_TWO}")));

    let missing_match = candidates
        .matches
        .iter()
        .find(|input| input.input.ends_with("Missing.class"))
        .unwrap();
    assert_eq!(missing_match.status, MatchStatus::NoMatch);
    assert!(missing_match.candidates.is_empty());
}

#[test]
#[ignore]
fn apply_real_spring_boot_replacements_and_verify_outputs() {
    let jar = real_spring_jar();
    let dir = tempdir().unwrap();

    let application_replacement = dir.path().join("application.yml");
    write_file(&application_replacement, b"fixture:\n  message: patched\n");

    let order_replacement = dir.path().join("OrderService.class");
    write_file(
        &order_replacement,
        &read_nested_jar_entry(jar, LIB_TWO_JAR, INVENTORY_CLASS),
    );

    let duplicate_replacement = dir.path().join("DuplicateName.class");
    write_file(
        &duplicate_replacement,
        &read_nested_jar_entry(jar, LIB_TWO_JAR, DUPLICATE_TWO),
    );

    let whole_nested_replacement = fixture_dir()
        .join("lib-two")
        .join("target")
        .join("fixture-lib-two-0.1.0.jar");

    let order_target = format!("{LIB_ONE_JAR}!/{ORDER_CLASS}");
    let duplicate_target = format!("{LIB_ONE_JAR}!/{DUPLICATE_ONE}");
    let plan = dir.path().join("patch-plan.yaml");
    write_patch_plan(
        &plan,
        &[
            (APPLICATION_YML, &application_replacement),
            (&order_target, &order_replacement),
            (&duplicate_target, &duplicate_replacement),
            (LIB_TWO_JAR, &whole_nested_replacement),
        ],
    );
    let output = dir.path().join("patched.jar");

    apply_patch_plan(jar, &plan, &output).unwrap();

    assert_eq!(
        read_jar_entry(&output, APPLICATION_YML),
        fs::read(&application_replacement).unwrap()
    );
    assert_eq!(
        read_nested_jar_entry(&output, LIB_ONE_JAR, ORDER_CLASS),
        read_nested_jar_entry(jar, LIB_TWO_JAR, INVENTORY_CLASS)
    );
    assert_eq!(
        read_nested_jar_entry(&output, LIB_ONE_JAR, DUPLICATE_ONE),
        read_nested_jar_entry(jar, LIB_TWO_JAR, DUPLICATE_TWO)
    );
    assert_eq!(
        read_jar_entry(&output, LIB_TWO_JAR),
        fs::read(whole_nested_replacement).unwrap()
    );
    assert_eq!(
        jar_entry_compression(&output, LIB_ONE_JAR),
        CompressionMethod::Stored
    );
    assert_eq!(
        jar_entry_compression(&output, LIB_TWO_JAR),
        CompressionMethod::Stored
    );
    assert!(verify_jar(&output).unwrap().is_success());
}

#[test]
#[ignore]
fn apply_real_spring_boot_rejection_cases() {
    let jar = real_spring_jar();
    let dir = tempdir().unwrap();
    let replacement = dir.path().join("application.yml");
    write_file(&replacement, b"fixture:\n  message: patched\n");

    let candidates = dir.path().join("candidates.yaml");
    fs::write(&candidates, "kind: candidates\nversion: 1\nmatches: []\n").unwrap();
    let err =
        apply_patch_plan(jar, &candidates, dir.path().join("candidates-output.jar")).unwrap_err();
    assert!(matches!(err, ApplyError::UnsupportedPlanKind(kind) if kind == "candidates"));

    let missing_source = dir.path().join("missing.yml");
    let missing_source_plan = dir.path().join("missing-source.yaml");
    write_patch_plan(&missing_source_plan, &[(APPLICATION_YML, &missing_source)]);
    let err = apply_patch_plan(
        jar,
        &missing_source_plan,
        dir.path().join("missing-source.jar"),
    )
    .unwrap_err();
    assert!(matches!(err, ApplyError::MissingReplacementSource(path) if path == missing_source));

    let missing_outer_plan = dir.path().join("missing-outer.yaml");
    write_patch_plan(
        &missing_outer_plan,
        &[("BOOT-INF/classes/missing.yml", &replacement)],
    );
    let err = apply_patch_plan(
        jar,
        &missing_outer_plan,
        dir.path().join("missing-outer.jar"),
    )
    .unwrap_err();
    assert!(
        matches!(err, ApplyError::MissingTarget(target) if target == "BOOT-INF/classes/missing.yml")
    );

    let missing_nested_jar_plan = dir.path().join("missing-nested-jar.yaml");
    write_patch_plan(
        &missing_nested_jar_plan,
        &[(
            "BOOT-INF/lib/missing.jar!/com/acme/Missing.class",
            &replacement,
        )],
    );
    let err = apply_patch_plan(
        jar,
        &missing_nested_jar_plan,
        dir.path().join("missing-nested-jar.jar"),
    )
    .unwrap_err();
    assert!(
        matches!(err, ApplyError::MissingNestedJar(target) if target == "BOOT-INF/lib/missing.jar")
    );

    let missing_nested_entry_plan = dir.path().join("missing-nested-entry.yaml");
    let missing_nested_entry_target = format!("{LIB_ONE_JAR}!/com/acme/Missing.class");
    write_patch_plan(
        &missing_nested_entry_plan,
        &[(&missing_nested_entry_target, &replacement)],
    );
    let err = apply_patch_plan(
        jar,
        &missing_nested_entry_plan,
        dir.path().join("missing-nested-entry.jar"),
    )
    .unwrap_err();
    assert!(
        matches!(err, ApplyError::MissingNestedTarget { outer_jar, inner_path }
            if outer_jar == LIB_ONE_JAR && inner_path == "com/acme/Missing.class")
    );
}

#[test]
#[ignore]
fn verify_and_apply_fail_for_real_spring_jar_with_compressed_nested_entry() {
    let jar = real_spring_jar();
    let dir = tempdir().unwrap();
    let compressed_input = dir.path().join("compressed-input.jar");
    copy_with_compressed_nested_jar(jar, &compressed_input, LIB_ONE_JAR);

    let verify = verify_jar(&compressed_input).unwrap();
    assert!(!verify.is_success());
    assert!(verify
        .non_stored_nested_jars
        .iter()
        .any(|nested| nested.path == LIB_ONE_JAR));

    let replacement = dir.path().join("banner.txt");
    write_file(&replacement, b"patched-banner");
    let plan = dir.path().join("patch-plan.yaml");
    write_patch_plan(&plan, &[(APP_BANNER, &replacement)]);
    let output = dir.path().join("post-write-failure.jar");

    let err = apply_patch_plan(&compressed_input, &plan, &output).unwrap_err();
    assert!(
        matches!(err, ApplyError::VerificationFailed { output: failed_output, non_stored_nested_jars }
            if failed_output == output
                && non_stored_nested_jars.iter().any(|nested| nested.path == LIB_ONE_JAR))
    );
    assert!(output.exists());
}
