use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use bootjar_core::{
    apply_patch_plan, find_in_jar, inspect_jar, match_in_jar, verify_jar, ApplyError,
    ArchiveLayout, MatchStatus,
};
use tempfile::tempdir;
use zip::write::FileOptions;
use zip::CompressionMethod;

const APP_JAR: &str = "app/target/bootjar-patcher-fixture-app-0.1.0.jar";
const APP_WAR: &str = "war-app/target/bootjar-patcher-fixture-war-0.1.0.war";
const LIB_ONE_JAR: &str = "BOOT-INF/lib/fixture-lib-one-0.1.0.jar";
const LIB_TWO_JAR: &str = "BOOT-INF/lib/fixture-lib-two-0.1.0.jar";
const WAR_LIB_ONE_JAR: &str = "WEB-INF/lib/fixture-lib-one-0.1.0.jar";
const WAR_PROVIDED_JAR: &str = "WEB-INF/lib-provided/fixture-provided-lib-0.1.0.jar";
const APPLICATION_YML: &str = "BOOT-INF/classes/application.yml";
const WAR_APPLICATION_YML: &str = "WEB-INF/classes/application.yml";
const APP_BANNER: &str = "BOOT-INF/classes/com/acme/app/banner.txt";
const ORDER_CLASS: &str = "com/acme/libone/OrderService.class";
const INVENTORY_CLASS: &str = "com/acme/libtwo/InventoryService.class";
const PROVIDED_CLASS: &str = "com/acme/provided/ProvidedService.class";
const DUPLICATE_ONE: &str = "com/acme/shared/DuplicateName.class";
const DUPLICATE_TWO: &str = "com/acme/other/DuplicateName.class";

static FIXTURE_BUILD: OnceLock<()> = OnceLock::new();
static REAL_SPRING_JAR: OnceLock<PathBuf> = OnceLock::new();
static REAL_SPRING_WAR: OnceLock<PathBuf> = OnceLock::new();

fn build_fixture() {
    FIXTURE_BUILD.get_or_init(|| {
        let fixture = fixture_dir();
        let wrapper = if cfg!(windows) { "mvnw.cmd" } else { "./mvnw" };
        let status = Command::new(wrapper)
            .args(["-q", "-DskipTests", "package"])
            .current_dir(&fixture)
            .status()
            .expect("failed to run Maven Wrapper");

        assert!(status.success(), "Maven Wrapper build failed: {status}");
    });
}

fn real_spring_jar() -> &'static PathBuf {
    REAL_SPRING_JAR.get_or_init(|| {
        build_fixture();

        let jar = fixture_dir().join(APP_JAR);
        assert!(
            jar.exists(),
            "Spring Boot jar was not built: {}",
            jar.display()
        );
        jar
    })
}

fn real_spring_war() -> &'static PathBuf {
    REAL_SPRING_WAR.get_or_init(|| {
        build_fixture();

        let war = fixture_dir().join(APP_WAR);
        assert!(
            war.exists(),
            "Spring Boot war was not built: {}",
            war.display()
        );
        war
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

fn read_double_nested_jar_entry(
    path: &Path,
    contained_archive: &str,
    nested_jar: &str,
    inner_path: &str,
) -> Vec<u8> {
    let contained_bytes = read_jar_entry(path, contained_archive);
    let cursor = std::io::Cursor::new(contained_bytes);
    let mut contained = zip::ZipArchive::new(cursor).unwrap();
    let mut nested = contained.by_name(nested_jar).unwrap();
    let mut nested_bytes = Vec::new();
    nested.read_to_end(&mut nested_bytes).unwrap();
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

fn write_zip_wrapper(path: &Path, contained_jar: &Path) {
    let output = File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(output);
    let script_options = FileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755);
    zip.start_file("bin/start.sh", script_options).unwrap();
    zip.write_all(b"#!/bin/sh\njava -jar app/service.jar\n")
        .unwrap();

    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("config/runtime.yml", options).unwrap();
    zip.write_all(b"env: prod\n").unwrap();
    zip.start_file("templates/banner.txt", options).unwrap();
    zip.write_all(b"real-wrapper\n").unwrap();

    let app_options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("app/service.jar", app_options).unwrap();
    zip.write_all(&fs::read(contained_jar).unwrap()).unwrap();
    zip.finish().unwrap();
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
    assert_eq!(inspect.layout, ArchiveLayout::SpringBootJar);
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
fn inspect_and_verify_real_spring_boot_war() {
    let war = real_spring_war();

    let inspect = inspect_jar(war).unwrap();
    assert_eq!(inspect.layout, ArchiveLayout::SpringBootWar);
    assert!(inspect.has_web_inf_classes);
    assert!(inspect.has_web_inf_lib);
    assert!(inspect.has_web_inf_lib_provided);
    assert!(inspect.has_boot_loader_entry);
    assert!(inspect
        .nested_jars
        .iter()
        .any(|nested| nested.path == WAR_LIB_ONE_JAR && nested.is_stored));
    assert!(inspect
        .nested_jars
        .iter()
        .any(|nested| nested.path == WAR_PROVIDED_JAR && nested.is_stored));
    assert!(inspect.nested_jars.iter().all(|nested| nested.is_stored));

    let verify = verify_jar(war).unwrap();
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
fn find_match_apply_and_verify_real_spring_boot_war() {
    let war = real_spring_war();

    let order_matches = find_in_jar(war, "OrderService.class").unwrap();
    assert!(order_matches
        .iter()
        .any(|result| { result.archive_path == format!("{WAR_LIB_ONE_JAR}!/{ORDER_CLASS}") }));

    let provided_matches = find_in_jar(war, "ProvidedService.class").unwrap();
    assert!(provided_matches
        .iter()
        .any(|result| { result.archive_path == format!("{WAR_PROVIDED_JAR}!/{PROVIDED_CLASS}") }));

    let dir = tempdir().unwrap();
    let exact = dir.path().join(WAR_APPLICATION_YML);
    write_file(&exact, b"fixture:\n  mode: patched-war\n");
    let order = dir.path().join("OrderService.class");
    write_file(&order, b"patched-order");

    let candidates = match_in_jar(war, &[dir.path().to_path_buf()]).unwrap();
    let exact_match = candidates
        .matches
        .iter()
        .find(|input| input.input.ends_with(WAR_APPLICATION_YML))
        .unwrap();
    assert_eq!(exact_match.status, MatchStatus::Selected);
    assert!(exact_match
        .candidates
        .iter()
        .any(|candidate| candidate.target == WAR_APPLICATION_YML));

    let order_match = candidates
        .matches
        .iter()
        .find(|input| input.input.ends_with("OrderService.class"))
        .unwrap();
    assert_eq!(order_match.status, MatchStatus::NeedsSelection);
    assert!(order_match
        .candidates
        .iter()
        .any(|candidate| candidate.target == format!("{WAR_LIB_ONE_JAR}!/{ORDER_CLASS}")));

    let application_replacement = dir.path().join("application.yml");
    write_file(&application_replacement, b"fixture:\n  mode: patched-war\n");

    let provided_replacement = dir.path().join("ProvidedService.class");
    write_file(
        &provided_replacement,
        &read_nested_jar_entry(war, WAR_LIB_ONE_JAR, ORDER_CLASS),
    );

    let provided_target = format!("{WAR_PROVIDED_JAR}!/{PROVIDED_CLASS}");
    let plan = dir.path().join("war-patch-plan.yaml");
    write_patch_plan(
        &plan,
        &[
            (WAR_APPLICATION_YML, &application_replacement),
            (&provided_target, &provided_replacement),
        ],
    );
    let output = dir.path().join("patched.war");

    apply_patch_plan(war, &plan, &output).unwrap();

    assert_eq!(
        read_jar_entry(&output, WAR_APPLICATION_YML),
        fs::read(&application_replacement).unwrap()
    );
    assert_eq!(
        read_nested_jar_entry(&output, WAR_PROVIDED_JAR, PROVIDED_CLASS),
        read_nested_jar_entry(war, WAR_LIB_ONE_JAR, ORDER_CLASS)
    );
    assert_eq!(
        jar_entry_compression(&output, WAR_PROVIDED_JAR),
        CompressionMethod::Stored
    );
    assert!(verify_jar(&output).unwrap().is_success());
}

#[test]
#[ignore]
fn find_match_apply_and_verify_real_zip_wrapper() {
    let jar = real_spring_jar();
    let dir = tempdir().unwrap();
    let wrapper = dir.path().join("dist.zip");
    write_zip_wrapper(&wrapper, jar);

    let inspect = inspect_jar(&wrapper).unwrap();
    assert_eq!(inspect.layout, ArchiveLayout::ZipWrapper);
    assert!(inspect
        .contained_archives
        .iter()
        .any(|archive| archive.path == "app/service.jar"));
    assert!(inspect.nested_jars.iter().any(|nested| nested.path
        == format!("app/service.jar!/{LIB_ONE_JAR}")
        && nested.is_stored));

    let runtime_matches = find_in_jar(&wrapper, "runtime.yml").unwrap();
    assert!(runtime_matches
        .iter()
        .any(|result| result.archive_path == "config/runtime.yml"));

    let order_matches = find_in_jar(&wrapper, "OrderService.class").unwrap();
    assert!(order_matches.iter().any(|result| {
        result.archive_path == format!("app/service.jar!/{LIB_ONE_JAR}!/{ORDER_CLASS}")
    }));

    let patch_root = dir.path().join("patch");
    let runtime_input = patch_root.join("config/runtime.yml");
    let app_input = patch_root.join("app/service.jar!/BOOT-INF/classes/application.yml");
    write_file(&runtime_input, b"env: patched\n");
    write_file(&app_input, b"fixture:\n  message: wrapper\n");
    let candidates = match_in_jar(&wrapper, std::slice::from_ref(&patch_root)).unwrap();
    assert!(candidates.matches.iter().any(|input| {
        input.status == MatchStatus::Selected
            && input
                .candidates
                .iter()
                .any(|candidate| candidate.target == "config/runtime.yml")
    }));
    assert!(candidates.matches.iter().any(|input| {
        input.status == MatchStatus::Selected
            && input.candidates.iter().any(|candidate| {
                candidate.target == "app/service.jar!/BOOT-INF/classes/application.yml"
            })
    }));

    let class_replacement = dir.path().join("OrderService.class");
    write_file(
        &class_replacement,
        &read_nested_jar_entry(jar, LIB_TWO_JAR, INVENTORY_CLASS),
    );
    let nested_target = format!("app/service.jar!/{LIB_ONE_JAR}!/{ORDER_CLASS}");
    let plan = dir.path().join("wrapper-plan.yaml");
    write_patch_plan(
        &plan,
        &[
            ("config/runtime.yml", &runtime_input),
            (
                "app/service.jar!/BOOT-INF/classes/application.yml",
                &app_input,
            ),
            (&nested_target, &class_replacement),
        ],
    );
    let output = dir.path().join("patched-dist.zip");

    apply_patch_plan(&wrapper, &plan, &output).unwrap();

    assert_eq!(
        read_jar_entry(&output, "config/runtime.yml"),
        b"env: patched\n"
    );
    assert_eq!(
        read_nested_jar_entry(
            &output,
            "app/service.jar",
            "BOOT-INF/classes/application.yml"
        ),
        b"fixture:\n  message: wrapper\n"
    );
    assert_eq!(
        read_double_nested_jar_entry(&output, "app/service.jar", LIB_ONE_JAR, ORDER_CLASS),
        read_nested_jar_entry(jar, LIB_TWO_JAR, INVENTORY_CLASS)
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
        matches!(err, ApplyError::VerificationFailed { output: failed_output, non_stored_nested_jars, .. }
            if failed_output == output
                && non_stored_nested_jars.iter().any(|nested| nested.path == LIB_ONE_JAR))
    );
    assert!(output.exists());
}

#[test]
#[ignore]
fn verify_and_apply_fail_for_real_spring_war_with_compressed_nested_entry() {
    let war = real_spring_war();
    let dir = tempdir().unwrap();
    let compressed_input = dir.path().join("compressed-input.war");
    copy_with_compressed_nested_jar(war, &compressed_input, WAR_PROVIDED_JAR);

    let verify = verify_jar(&compressed_input).unwrap();
    assert!(!verify.is_success());
    assert!(verify
        .non_stored_nested_jars
        .iter()
        .any(|nested| nested.path == WAR_PROVIDED_JAR));

    let replacement = dir.path().join("application.yml");
    write_file(&replacement, b"fixture:\n  mode: post-write-failure\n");
    let plan = dir.path().join("patch-plan.yaml");
    write_patch_plan(&plan, &[(WAR_APPLICATION_YML, &replacement)]);
    let output = dir.path().join("post-write-failure.war");

    let err = apply_patch_plan(&compressed_input, &plan, &output).unwrap_err();
    assert!(
        matches!(err, ApplyError::VerificationFailed { output: failed_output, non_stored_nested_jars, .. }
            if failed_output == output
                && non_stored_nested_jars.iter().any(|nested| nested.path == WAR_PROVIDED_JAR))
    );
    assert!(output.exists());
}
