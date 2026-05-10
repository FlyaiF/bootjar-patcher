//! Core library for `bootjar-patcher`.
//! Provides archive path parsing and jar inspection primitives.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use zip::result::ZipError;
use zip::write::FileOptions;
use zip::CompressionMethod;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePath {
    Outer {
        path: String,
    },
    Nested {
        outer_jar: String,
        inner_path: String,
    },
    Chained {
        segments: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePathParseError {
    EmptyInput,
    EmptyOuterPath,
    EmptyInnerPath,
    MultipleNestedSeparators,
    InvalidAbsolutePath,
    InvalidDrivePath,
    EmptySegment,
    DotSegment,
    DotDotSegment,
}

impl fmt::Display for ArchivePathParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "archive path is empty"),
            Self::EmptyOuterPath => write!(f, "outer archive path is empty"),
            Self::EmptyInnerPath => write!(f, "nested inner path is empty"),
            Self::MultipleNestedSeparators => write!(f, "archive path contains empty `!` segment"),
            Self::InvalidAbsolutePath => write!(f, "archive path must not be absolute"),
            Self::InvalidDrivePath => {
                write!(f, "archive path must not include Windows drive prefixes")
            }
            Self::EmptySegment => write!(f, "archive path contains an empty segment"),
            Self::DotSegment => write!(f, "archive path contains a '.' segment"),
            Self::DotDotSegment => write!(f, "archive path contains a '..' segment"),
        }
    }
}

impl std::error::Error for ArchivePathParseError {}

impl ArchivePath {
    /// Parse an archive path with chained nested syntax: `<outer>!/<inner>`.
    ///
    /// For this first slice, both outer and inner paths are normalized to
    /// jar-style `/`, and unsafe path forms are rejected up front.
    pub fn parse(input: &str) -> Result<Self, ArchivePathParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ArchivePathParseError::EmptyInput);
        }

        let separator_parts: Vec<&str> = input.split('!').collect();

        match separator_parts.as_slice() {
            [outer] => {
                let outer = parse_archive_component(outer, true)?;
                Ok(Self::Outer { path: outer })
            }
            [outer, inner] => {
                if outer.is_empty() {
                    return Err(ArchivePathParseError::EmptyOuterPath);
                }
                if inner.is_empty() {
                    return Err(ArchivePathParseError::EmptyInnerPath);
                }
                let outer = parse_archive_component(outer, true)?;
                let inner = inner.strip_prefix('/').unwrap_or(inner);
                if inner.is_empty() {
                    return Err(ArchivePathParseError::EmptyInnerPath);
                }
                let inner = parse_archive_component(inner, true)?;
                Ok(Self::Nested {
                    outer_jar: outer,
                    inner_path: inner,
                })
            }
            parts => {
                let mut segments = Vec::with_capacity(parts.len());
                for (index, part) in parts.iter().enumerate() {
                    if part.is_empty() {
                        return if index == 0 {
                            Err(ArchivePathParseError::EmptyOuterPath)
                        } else if parts.len() > 2 {
                            Err(ArchivePathParseError::MultipleNestedSeparators)
                        } else {
                            Err(ArchivePathParseError::EmptyInnerPath)
                        };
                    }
                    let part = part.strip_prefix('/').unwrap_or(part);
                    if part.is_empty() {
                        return Err(ArchivePathParseError::EmptyInnerPath);
                    }
                    segments.push(parse_archive_component(part, true)?);
                }
                Ok(Self::Chained { segments })
            }
        }
    }

    fn into_segments(self) -> Vec<String> {
        match self {
            Self::Outer { path } => vec![path],
            Self::Nested {
                outer_jar,
                inner_path,
            } => vec![outer_jar, inner_path],
            Self::Chained { segments } => segments,
        }
    }
}

fn parse_archive_component(
    raw: &str,
    reject_drive_prefix: bool,
) -> Result<String, ArchivePathParseError> {
    let normalized = raw.replace('\\', "/");
    if normalized.is_empty() {
        return if reject_drive_prefix {
            Err(ArchivePathParseError::EmptyOuterPath)
        } else {
            Err(ArchivePathParseError::EmptyInnerPath)
        };
    }

    if normalized.starts_with('/') {
        return Err(ArchivePathParseError::InvalidAbsolutePath);
    }

    if reject_drive_prefix && has_windows_drive_prefix(&normalized) {
        return Err(ArchivePathParseError::InvalidDrivePath);
    }

    for segment in normalized.split('/') {
        if segment.is_empty() {
            return Err(ArchivePathParseError::EmptySegment);
        }
        if segment == "." {
            return Err(ArchivePathParseError::DotSegment);
        }
        if segment == ".." {
            return Err(ArchivePathParseError::DotDotSegment);
        }
    }

    Ok(normalized)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

#[derive(Debug)]
pub enum JarInspectError {
    Io(io::Error),
    InvalidJar(ZipError),
}

impl fmt::Display for JarInspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not read jar: {error}"),
            Self::InvalidJar(error) => write!(f, "jar is not readable: {error}"),
        }
    }
}

impl std::error::Error for JarInspectError {}

impl From<io::Error> for JarInspectError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ZipError> for JarInspectError {
    fn from(value: ZipError) -> Self {
        Self::InvalidJar(value)
    }
}

#[derive(Debug)]
pub enum MatchError {
    Jar(JarInspectError),
    InputPath { path: PathBuf, source: io::Error },
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jar(error) => write!(f, "{error}"),
            Self::InputPath { path, source } => {
                write!(f, "could not read input path {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for MatchError {}

impl From<JarInspectError> for MatchError {
    fn from(value: JarInspectError) -> Self {
        Self::Jar(value)
    }
}

#[derive(Debug)]
pub enum ApplyError {
    Io {
        path: PathBuf,
        action: &'static str,
        source: io::Error,
    },
    InvalidJar(ZipError),
    InvalidPlanYaml(serde_yaml::Error),
    UnsupportedPlanKind(String),
    UnsupportedPlanVersion(u32),
    UnsupportedOperation,
    InvalidTarget {
        target: String,
        source: ArchivePathParseError,
    },
    MissingReplacementSource(PathBuf),
    MissingTarget(String),
    MissingNestedJar(String),
    MissingNestedTarget {
        outer_jar: String,
        inner_path: String,
    },
    InvalidNestedJar {
        outer_jar: String,
        source: ZipError,
    },
    InvalidReplacementNestedJar {
        path: PathBuf,
        source: ZipError,
    },
    DuplicateTarget(String),
    VerificationReadFailed {
        output: PathBuf,
        source: JarInspectError,
    },
    VerificationFailed {
        output: PathBuf,
        non_stored_nested_jars: Vec<NestedJarInfo>,
    },
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                path,
                action,
                source,
            } => {
                write!(f, "could not {action} {}: {source}", path.display())
            }
            Self::InvalidJar(error) => write!(f, "jar is not readable: {error}"),
            Self::InvalidPlanYaml(error) => write!(f, "patch plan YAML is invalid: {error}"),
            Self::UnsupportedPlanKind(kind) if kind == "candidates" => {
                write!(f, "candidates files are not reviewed patch plans")
            }
            Self::UnsupportedPlanKind(kind) => {
                write!(f, "unsupported patch plan kind: {kind}")
            }
            Self::UnsupportedPlanVersion(version) => {
                write!(f, "unsupported patch plan version: {version}")
            }
            Self::UnsupportedOperation => write!(f, "unsupported patch plan operation"),
            Self::InvalidTarget { target, source } => {
                write!(f, "invalid replace target {target}: {source}")
            }
            Self::MissingReplacementSource(path) => {
                write!(
                    f,
                    "replacement source file could not be read: {}",
                    path.display()
                )
            }
            Self::MissingTarget(target) => {
                write!(
                    f,
                    "replace target does not exist in input archive: {target}"
                )
            }
            Self::MissingNestedJar(target) => {
                write!(
                    f,
                    "nested jar target does not exist in input archive: {target}"
                )
            }
            Self::MissingNestedTarget {
                outer_jar,
                inner_path,
            } => {
                write!(
                    f,
                    "nested replace target does not exist in {outer_jar}: {inner_path}"
                )
            }
            Self::InvalidNestedJar { outer_jar, source } => {
                write!(f, "nested jar is not readable {outer_jar}: {source}")
            }
            Self::InvalidReplacementNestedJar { path, source } => {
                write!(
                    f,
                    "replacement nested jar could not be read {}: {source}",
                    path.display()
                )
            }
            Self::DuplicateTarget(target) => {
                write!(f, "duplicate replace target in patch plan: {target}")
            }
            Self::VerificationReadFailed { output, source } => {
                write!(
                    f,
                    "could not verify written output {}: {source}",
                    output.display()
                )
            }
            Self::VerificationFailed {
                output,
                non_stored_nested_jars,
            } => {
                write!(
                    f,
                    "verification failed after writing output {}",
                    output.display()
                )?;

                if !non_stored_nested_jars.is_empty() {
                    write!(f, ": non-STORED nested jars")?;
                    for nested_jar in non_stored_nested_jars {
                        write!(f, " {}", nested_jar.path)?;
                    }
                }

                Ok(())
            }
        }
    }
}

impl std::error::Error for ApplyError {}

impl From<ZipError> for ApplyError {
    fn from(value: ZipError) -> Self {
        Self::InvalidJar(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JarEntry {
    pub path: String,
    pub compression_method: String,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub crc32: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedJarInfo {
    pub path: String,
    pub compression_method: String,
    pub is_stored: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedJarEntry {
    pub outer_jar: String,
    pub inner_path: String,
    pub archive_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveLayout {
    SpringBootJar,
    SpringBootWar,
    ZipWrapper,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JarIndex {
    pub entries: Vec<JarEntry>,
    pub layout: ArchiveLayout,
    pub has_boot_inf_classes: bool,
    pub has_boot_inf_lib: bool,
    pub has_web_inf_classes: bool,
    pub has_web_inf_lib: bool,
    pub has_web_inf_lib_provided: bool,
    pub has_boot_loader_entry: bool,
    pub nested_jars: Vec<NestedJarInfo>,
    pub nested_entries: Vec<NestedJarEntry>,
    pub contained_archives: Vec<ContainedArchiveInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectReport {
    pub jar_path: String,
    pub layout: ArchiveLayout,
    pub has_boot_inf_classes: bool,
    pub has_boot_inf_lib: bool,
    pub has_web_inf_classes: bool,
    pub has_web_inf_lib: bool,
    pub has_web_inf_lib_provided: bool,
    pub has_boot_loader_entry: bool,
    pub nested_jars: Vec<NestedJarInfo>,
    pub contained_archives: Vec<ContainedArchiveInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    pub jar_path: String,
    pub readable: bool,
    pub nested_jars: Vec<NestedJarInfo>,
    pub non_stored_nested_jars: Vec<NestedJarInfo>,
    pub signed_metadata: Vec<String>,
    pub contained_archives: Vec<ContainedArchiveInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainedArchiveInfo {
    pub path: String,
    pub layout: ArchiveLayout,
    pub nested_jars: Vec<NestedJarInfo>,
}

impl VerifyReport {
    pub fn is_success(&self) -> bool {
        self.readable && self.non_stored_nested_jars.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindResult {
    pub archive_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFile {
    pub source: String,
    pub matches: Vec<InputMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputMatch {
    pub input: String,
    pub status: MatchStatus,
    pub candidates: Vec<CandidateTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchStatus {
    Selected,
    NeedsSelection,
    NoMatch,
}

impl MatchStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::NeedsSelection => "needs-selection",
            Self::NoMatch => "no-match",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateTarget {
    pub target: String,
    pub score: u16,
    pub reason: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPlan {
    pub operations: Vec<ReplaceOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceOperation {
    pub target: String,
    pub source: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputFile {
    display_path: String,
    relative_path: String,
    file_name: String,
}

#[derive(Debug, Deserialize)]
struct RawDocumentKind {
    kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPatchPlan {
    #[serde(rename = "kind")]
    _kind: String,
    version: u32,
    operations: Vec<RawOperation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOperation {
    #[serde(rename = "replace-entry")]
    replace_entry: Option<RawReplaceEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReplaceEntry {
    target: String,
    #[serde(rename = "with")]
    source: PathBuf,
}

#[derive(Debug)]
struct ResolvedReplacement {
    target: ArchivePath,
    source: PathBuf,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct ReplacementBytes {
    source: PathBuf,
    bytes: Vec<u8>,
}

pub fn build_jar_index(path: impl Into<PathBuf>) -> Result<JarIndex, JarInspectError> {
    let path = path.into();
    let bytes = std::fs::read(&path)?;
    build_index_from_bytes(&bytes, None, true)
}

fn build_index_from_bytes(
    bytes: &[u8],
    prefix: Option<&str>,
    recurse_contained: bool,
) -> Result<JarIndex, JarInspectError> {
    let cursor = Cursor::new(bytes);
    let archive = zip::ZipArchive::new(cursor)?;
    build_index_from_archive(archive, prefix, recurse_contained)
}

fn build_index_from_archive<R: Read + Seek>(
    mut archive: zip::ZipArchive<R>,
    prefix: Option<&str>,
    recurse_contained: bool,
) -> Result<JarIndex, JarInspectError> {
    let prefix_path = |path: &str| -> String {
        if let Some(prefix) = prefix {
            format!("{prefix}!/{path}")
        } else {
            path.to_string()
        }
    };

    let mut entries = Vec::with_capacity(archive.len());
    let mut has_boot_inf_classes = false;
    let mut has_boot_inf_lib = false;
    let mut has_web_inf_classes = false;
    let mut has_web_inf_lib = false;
    let mut has_web_inf_lib_provided = false;
    let mut has_boot_loader_entry = false;
    let mut nested_jars = Vec::new();
    let mut nested_entries = Vec::new();
    let mut contained_archives = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let path = normalize_entry_name(entry.name());
        let is_dir = entry.is_dir();

        let compression = entry.compression();
        let info = JarEntry {
            path: prefix_path(&path),
            compression_method: compression.to_string(),
            uncompressed_size: entry.size(),
            compressed_size: entry.compressed_size(),
            crc32: Some(entry.crc32()),
        };
        entries.push(info);

        if is_dir {
            continue;
        }

        if is_boot_inf_classes_entry(&path) {
            has_boot_inf_classes = true;
        }
        if is_boot_inf_lib_entry(&path) {
            has_boot_inf_lib = true;
        }
        if is_web_inf_classes_entry(&path) {
            has_web_inf_classes = true;
        }
        if is_web_inf_lib_entry(&path) {
            has_web_inf_lib = true;
        }
        if is_web_inf_lib_provided_entry(&path) {
            has_web_inf_lib_provided = true;
        }
        if is_boot_loader_entry(&path) {
            has_boot_loader_entry = true;
        }
        if let Some(nested_name) = nested_library_entry(&path) {
            let nested_name = prefix_path(nested_name);
            nested_jars.push(NestedJarInfo {
                path: nested_name.clone(),
                compression_method: compression.to_string(),
                is_stored: compression == CompressionMethod::Stored,
            });
            index_nested_jar_entries(&mut entry, &nested_name, &mut nested_entries);
        } else if recurse_contained && is_supported_archive_file(&path) {
            let mut bytes = Vec::new();
            if entry.read_to_end(&mut bytes).is_ok() {
                if let Ok(child) = build_index_from_bytes(&bytes, Some(&prefix_path(&path)), false)
                {
                    if matches!(
                        child.layout,
                        ArchiveLayout::SpringBootJar | ArchiveLayout::SpringBootWar
                    ) {
                        contained_archives.push(ContainedArchiveInfo {
                            path: prefix_path(&path),
                            layout: child.layout,
                            nested_jars: child.nested_jars.clone(),
                        });
                        nested_jars.extend(child.nested_jars);
                        nested_entries.extend(child.entries.into_iter().filter_map(|entry| {
                            if entry.path.ends_with('/') {
                                None
                            } else {
                                let inner_path = entry
                                    .path
                                    .split_once("!/")
                                    .map(|(_, inner)| inner.to_string())
                                    .unwrap_or_else(|| entry.path.clone());
                                Some(NestedJarEntry {
                                    outer_jar: prefix_path(&path),
                                    inner_path,
                                    archive_path: entry.path,
                                })
                            }
                        }));
                        nested_entries.extend(child.nested_entries);
                    }
                }
            }
        }
    }

    let mut layout = detect_archive_layout(
        has_boot_inf_classes,
        has_boot_inf_lib,
        has_web_inf_classes,
        has_web_inf_lib,
        has_web_inf_lib_provided,
    );
    if layout == ArchiveLayout::Unknown && !contained_archives.is_empty() {
        layout = ArchiveLayout::ZipWrapper;
    }

    Ok(JarIndex {
        entries,
        layout,
        has_boot_inf_classes,
        has_boot_inf_lib,
        has_web_inf_classes,
        has_web_inf_lib,
        has_web_inf_lib_provided,
        has_boot_loader_entry,
        nested_jars,
        nested_entries,
        contained_archives,
    })
}

impl JarIndex {
    pub fn inspect_report(&self, path: &Path) -> InspectReport {
        InspectReport {
            jar_path: path.display().to_string(),
            layout: self.layout,
            has_boot_inf_classes: self.has_boot_inf_classes,
            has_boot_inf_lib: self.has_boot_inf_lib,
            has_web_inf_classes: self.has_web_inf_classes,
            has_web_inf_lib: self.has_web_inf_lib,
            has_web_inf_lib_provided: self.has_web_inf_lib_provided,
            has_boot_loader_entry: self.has_boot_loader_entry,
            nested_jars: self.nested_jars.clone(),
            contained_archives: self.contained_archives.clone(),
        }
    }

    pub fn find(&self, query: &str) -> Vec<FindResult> {
        let query = normalize_entry_name(query.trim());
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        for entry in &self.entries {
            if path_matches_query(&entry.path, &query) {
                results.push(FindResult {
                    archive_path: entry.path.clone(),
                });
            }
        }

        for entry in &self.nested_entries {
            if path_matches_query(&entry.archive_path, &query)
                || path_matches_query(&entry.inner_path, &query)
            {
                results.push(FindResult {
                    archive_path: entry.archive_path.clone(),
                });
            }
        }

        results
    }

    pub fn verify_report(&self, path: &Path) -> VerifyReport {
        let non_stored_nested_jars = self
            .nested_jars
            .iter()
            .filter(|entry| !entry.is_stored)
            .cloned()
            .collect();
        let signed_metadata = self
            .entries
            .iter()
            .filter(|entry| is_signed_metadata_entry(&entry.path))
            .map(|entry| entry.path.clone())
            .collect();

        VerifyReport {
            jar_path: path.display().to_string(),
            readable: true,
            nested_jars: self.nested_jars.clone(),
            non_stored_nested_jars,
            signed_metadata,
            contained_archives: self.contained_archives.clone(),
        }
    }
}

pub fn inspect_jar(path: impl AsRef<Path>) -> Result<InspectReport, JarInspectError> {
    let path_ref = path.as_ref();
    Ok(build_jar_index(path_ref)?.inspect_report(path_ref))
}

pub fn find_in_jar(
    path: impl AsRef<Path>,
    query: impl AsRef<str>,
) -> Result<Vec<FindResult>, JarInspectError> {
    Ok(build_jar_index(path.as_ref().to_path_buf())?.find(query.as_ref()))
}

pub fn verify_jar(path: impl AsRef<Path>) -> Result<VerifyReport, JarInspectError> {
    let path_ref = path.as_ref();
    Ok(build_jar_index(path_ref)?.verify_report(path_ref))
}

pub fn match_in_jar(
    jar_path: impl AsRef<Path>,
    input_roots: &[PathBuf],
) -> Result<CandidateFile, MatchError> {
    let jar_path = jar_path.as_ref();
    let index = build_jar_index(jar_path.to_path_buf())?;
    let inputs = collect_input_files(input_roots)?;
    Ok(index.match_inputs(jar_path, &inputs))
}

pub fn parse_patch_plan(yaml: &str) -> Result<PatchPlan, ApplyError> {
    let raw_kind: RawDocumentKind =
        serde_yaml::from_str(yaml).map_err(ApplyError::InvalidPlanYaml)?;
    if raw_kind.kind != "patch-plan" {
        return Err(ApplyError::UnsupportedPlanKind(raw_kind.kind));
    }

    let raw: RawPatchPlan = serde_yaml::from_str(yaml).map_err(ApplyError::InvalidPlanYaml)?;
    if raw.version != 1 {
        return Err(ApplyError::UnsupportedPlanVersion(raw.version));
    }

    let mut seen_targets = BTreeSet::new();
    let mut operations = Vec::with_capacity(raw.operations.len());
    for raw_operation in raw.operations {
        let replace_entry = raw_operation
            .replace_entry
            .ok_or(ApplyError::UnsupportedOperation)?;
        ArchivePath::parse(&replace_entry.target).map_err(|source| ApplyError::InvalidTarget {
            target: replace_entry.target.clone(),
            source,
        })?;
        if !seen_targets.insert(replace_entry.target.clone()) {
            return Err(ApplyError::DuplicateTarget(replace_entry.target));
        }

        operations.push(ReplaceOperation {
            target: replace_entry.target,
            source: replace_entry.source,
        });
    }

    Ok(PatchPlan { operations })
}

pub fn apply_patch_plan(
    input_jar: impl AsRef<Path>,
    plan_path: impl AsRef<Path>,
    output_jar: impl AsRef<Path>,
) -> Result<(), ApplyError> {
    let input_jar = input_jar.as_ref();
    let plan_path = plan_path.as_ref();
    let output_jar = output_jar.as_ref();

    let plan_yaml = std::fs::read_to_string(plan_path).map_err(|source| ApplyError::Io {
        path: plan_path.to_path_buf(),
        action: "read patch plan",
        source,
    })?;
    let plan = parse_patch_plan(&plan_yaml)?;
    let replacements = resolve_replacements(&plan)?;
    rewrite_outer_jar_with_plan(input_jar, output_jar, replacements)?;

    let report = verify_jar(output_jar).map_err(|source| ApplyError::VerificationReadFailed {
        output: output_jar.to_path_buf(),
        source,
    })?;

    if !report.is_success() {
        return Err(ApplyError::VerificationFailed {
            output: output_jar.to_path_buf(),
            non_stored_nested_jars: report.non_stored_nested_jars,
        });
    }

    Ok(())
}

impl CandidateFile {
    pub fn to_yaml(&self) -> String {
        let mut yaml = String::new();
        yaml.push_str("kind: candidates\n");
        yaml.push_str("version: 1\n");
        yaml.push_str("source: ");
        yaml.push_str(&yaml_string(&self.source));
        yaml.push_str("\n\nmatches:\n");

        if self.matches.is_empty() {
            yaml.push_str("  []\n");
            return yaml;
        }

        for input_match in &self.matches {
            yaml.push_str("  - input: ");
            yaml.push_str(&yaml_string(&input_match.input));
            yaml.push('\n');
            yaml.push_str("    status: ");
            yaml.push_str(input_match.status.as_str());
            yaml.push('\n');
            yaml.push_str("    candidates:\n");
            if input_match.candidates.is_empty() {
                yaml.push_str("      []\n");
                continue;
            }

            for candidate in &input_match.candidates {
                yaml.push_str("      - target: ");
                yaml.push_str(&yaml_string(&candidate.target));
                yaml.push('\n');
                yaml.push_str("        score: ");
                yaml.push_str(&candidate.score.to_string());
                yaml.push('\n');
                yaml.push_str("        reason:\n");
                for reason in &candidate.reason {
                    yaml.push_str("          - ");
                    yaml.push_str(&yaml_string(reason));
                    yaml.push('\n');
                }
            }
        }

        yaml
    }

    pub fn to_snippets(&self) -> String {
        let mut snippets = String::new();
        snippets.push_str("# kind: patch-plan\n");
        snippets.push_str("# version: 1\n");
        snippets.push_str("operations:\n");

        if self.matches.is_empty() {
            snippets.push_str("#  []\n");
            return snippets;
        }

        for input_match in &self.matches {
            match input_match.status {
                MatchStatus::Selected => {
                    if let Some(candidate) = input_match.candidates.first() {
                        snippets.push_str("  - replace-entry:\n");
                        snippets.push_str("      target: ");
                        snippets.push_str(&yaml_string(&candidate.target));
                        snippets.push('\n');
                        snippets.push_str("      with: ");
                        snippets.push_str(&yaml_string(&input_match.input));
                        snippets.push('\n');
                    }
                }
                MatchStatus::NeedsSelection => {
                    snippets.push_str("# needs-selection: ");
                    snippets.push_str(&yaml_string(&input_match.input));
                    snippets.push('\n');
                    for candidate in &input_match.candidates {
                        snippets.push_str("#  - replace-entry:\n");
                        snippets.push_str("#      target: ");
                        snippets.push_str(&yaml_string(&candidate.target));
                        snippets.push('\n');
                        snippets.push_str("#      with: ");
                        snippets.push_str(&yaml_string(&input_match.input));
                        snippets.push('\n');
                    }
                }
                MatchStatus::NoMatch => {
                    snippets.push_str("# no-match: ");
                    snippets.push_str(&yaml_string(&input_match.input));
                    snippets.push('\n');
                }
            }
        }

        snippets
    }
}

impl JarIndex {
    fn match_inputs(&self, jar_path: &Path, inputs: &[InputFile]) -> CandidateFile {
        let targets = self.match_targets();
        let matches = inputs
            .iter()
            .map(|input| match_input(input, &targets))
            .collect();

        CandidateFile {
            source: jar_path.display().to_string(),
            matches,
        }
    }

    fn match_targets(&self) -> Vec<MatchTarget> {
        let outer_targets = self
            .entries
            .iter()
            .filter(|entry| !entry.path.ends_with('/'))
            .map(|entry| MatchTarget::new(entry.path.clone()));
        let nested_targets = self
            .nested_entries
            .iter()
            .map(|entry| MatchTarget::new(entry.archive_path.clone()));

        outer_targets.chain(nested_targets).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MatchTarget {
    archive_path: String,
    file_name: String,
}

impl MatchTarget {
    fn new(archive_path: String) -> Self {
        let file_name = path_file_name(&archive_path).unwrap_or("").to_string();
        Self {
            archive_path,
            file_name,
        }
    }
}

fn normalize_entry_name(name: &str) -> String {
    name.replace('\\', "/")
}

fn collect_input_files(input_roots: &[PathBuf]) -> Result<Vec<InputFile>, MatchError> {
    let mut files = Vec::new();
    for root in input_roots {
        collect_input_root(root, root, &mut files)?;
    }
    files.sort_by(|left, right| left.display_path.cmp(&right.display_path));
    Ok(files)
}

fn collect_input_root(
    base: &Path,
    current: &Path,
    files: &mut Vec<InputFile>,
) -> Result<(), MatchError> {
    let metadata = std::fs::metadata(current).map_err(|source| MatchError::InputPath {
        path: current.to_path_buf(),
        source,
    })?;

    if metadata.is_file() {
        push_input_file(base, current, files);
        return Ok(());
    }

    let entries = std::fs::read_dir(current).map_err(|source| MatchError::InputPath {
        path: current.to_path_buf(),
        source,
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| MatchError::InputPath {
            path: current.to_path_buf(),
            source,
        })?;
        paths.push(entry.path());
    }
    paths.sort();

    for path in paths {
        let metadata = std::fs::metadata(&path).map_err(|source| MatchError::InputPath {
            path: path.clone(),
            source,
        })?;
        if metadata.is_dir() {
            collect_input_root(base, &path, files)?;
        } else if metadata.is_file() {
            push_input_file(base, &path, files);
        }
    }

    Ok(())
}

fn push_input_file(root: &Path, path: &Path, files: &mut Vec<InputFile>) {
    let relative = if root == path {
        path.file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.strip_prefix(root).unwrap_or(path).to_path_buf()
    };
    let relative_path = normalize_entry_name(&relative.to_string_lossy());
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| relative_path.clone());

    files.push(InputFile {
        display_path: path.display().to_string(),
        relative_path,
        file_name,
    });
}

fn match_input(input: &InputFile, targets: &[MatchTarget]) -> InputMatch {
    let mut candidates = Vec::new();

    for target in targets {
        if target.archive_path == input.relative_path {
            candidates.push(CandidateTarget {
                target: target.archive_path.clone(),
                score: 100,
                reason: vec!["exact relative path".to_string()],
            });
        }
    }

    let exact_count = candidates.len();
    for target in targets {
        if target.file_name == input.file_name
            && !candidates
                .iter()
                .any(|candidate| candidate.target == target.archive_path)
        {
            candidates.push(CandidateTarget {
                target: target.archive_path.clone(),
                score: 80,
                reason: vec!["same filename".to_string()],
            });
        }
    }

    let status = if exact_count == 1 {
        MatchStatus::Selected
    } else if candidates.is_empty() {
        MatchStatus::NoMatch
    } else {
        MatchStatus::NeedsSelection
    };

    InputMatch {
        input: input.display_path.clone(),
        status,
        candidates,
    }
}

fn yaml_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn resolve_replacements(plan: &PatchPlan) -> Result<Vec<ResolvedReplacement>, ApplyError> {
    let mut replacements = Vec::with_capacity(plan.operations.len());
    for operation in &plan.operations {
        let bytes = std::fs::read(&operation.source)
            .map_err(|_| ApplyError::MissingReplacementSource(operation.source.clone()))?;
        let target =
            ArchivePath::parse(&operation.target).map_err(|source| ApplyError::InvalidTarget {
                target: operation.target.clone(),
                source,
            })?;
        replacements.push(ResolvedReplacement {
            target,
            source: operation.source.clone(),
            bytes,
        });
    }
    Ok(replacements)
}

fn rewrite_outer_jar_with_plan(
    input_jar: &Path,
    output_jar: &Path,
    resolved: Vec<ResolvedReplacement>,
) -> Result<(), ApplyError> {
    let input_bytes = std::fs::read(input_jar).map_err(|source| ApplyError::Io {
        path: input_jar.to_path_buf(),
        action: "read input archive",
        source,
    })?;

    let mut replacements = BTreeMap::new();
    for replacement in resolved {
        replacements.insert(
            replacement.target.into_segments(),
            ReplacementBytes {
                source: replacement.source,
                bytes: replacement.bytes,
            },
        );
    }

    let output_bytes = rewrite_zip_bytes(None, input_bytes, replacements)?;

    let mut output = File::create(output_jar).map_err(|source| ApplyError::Io {
        path: output_jar.to_path_buf(),
        action: "write output archive",
        source,
    })?;
    output
        .write_all(&output_bytes)
        .map_err(|source| ApplyError::Io {
            path: output_jar.to_path_buf(),
            action: "write output archive",
            source,
        })?;

    Ok(())
}

fn rewrite_zip_bytes(
    archive_label: Option<&str>,
    input_bytes: Vec<u8>,
    replacements: BTreeMap<Vec<String>, ReplacementBytes>,
) -> Result<Vec<u8>, ApplyError> {
    let cursor = Cursor::new(input_bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|source| ApplyError::InvalidNestedJar {
            outer_jar: archive_label.unwrap_or("input archive").to_string(),
            source,
        })?;

    let mut existing_paths = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if !entry.is_dir() {
            existing_paths.insert(normalize_entry_name(entry.name()));
        }
    }

    for segments in replacements.keys() {
        let Some(first) = segments.first() else {
            continue;
        };
        if !existing_paths.contains(first) {
            if segments.len() == 1 {
                if let Some(label) = archive_label {
                    return Err(ApplyError::MissingNestedTarget {
                        outer_jar: label.to_string(),
                        inner_path: first.clone(),
                    });
                }
                return Err(ApplyError::MissingTarget(first.clone()));
            }
            return Err(ApplyError::MissingNestedJar(qualify_archive_path(
                archive_label,
                first,
            )));
        }
    }

    let output = Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(output);
    let mut remaining = replacements;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let path = normalize_entry_name(entry.name());
        let mut options = FileOptions::default()
            .compression_method(entry.compression())
            .last_modified_time(entry.last_modified());
        if let Some(mode) = entry.unix_mode() {
            options = options.unix_permissions(mode);
        }

        if entry.is_dir() {
            writer.add_directory(&path, options)?;
            continue;
        }

        let direct_key = vec![path.clone()];
        let direct_replacement = remaining.remove(&direct_key);
        let child_replacements = take_child_replacements(&mut remaining, &path);

        if let Some(replacement) = direct_replacement {
            if nested_library_entry(&path).is_some() {
                validate_replacement_nested_jar(&replacement.source, &replacement.bytes)?;
                options = options.compression_method(CompressionMethod::Stored);
            }
            writer.start_file(&path, options)?;
            writer
                .write_all(&replacement.bytes)
                .map_err(|source| ApplyError::Io {
                    path: PathBuf::from(qualify_archive_path(archive_label, &path)),
                    action: "write archive entry",
                    source,
                })?;
        } else if !child_replacements.is_empty() {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|source| ApplyError::Io {
                    path: PathBuf::from(qualify_archive_path(archive_label, &path)),
                    action: "read archive entry",
                    source,
                })?;
            let child_label = qualify_archive_path(archive_label, &path);
            let rewritten = rewrite_zip_bytes(Some(&child_label), bytes, child_replacements)?;
            if nested_library_entry(&path).is_some() {
                options = options.compression_method(CompressionMethod::Stored);
            }
            writer.start_file(&path, options)?;
            writer
                .write_all(&rewritten)
                .map_err(|source| ApplyError::Io {
                    path: PathBuf::from(child_label),
                    action: "write archive entry",
                    source,
                })?;
        } else {
            writer.start_file(&path, options)?;
            io::copy(&mut entry, &mut writer).map_err(|source| ApplyError::Io {
                path: PathBuf::from(qualify_archive_path(archive_label, &path)),
                action: "write archive entry",
                source,
            })?;
        }
    }

    let output = writer.finish()?;
    Ok(output.into_inner())
}

fn take_child_replacements(
    replacements: &mut BTreeMap<Vec<String>, ReplacementBytes>,
    path: &str,
) -> BTreeMap<Vec<String>, ReplacementBytes> {
    let keys: Vec<Vec<String>> = replacements
        .keys()
        .filter(|segments| segments.len() > 1 && segments[0] == path)
        .cloned()
        .collect();
    let mut child_replacements = BTreeMap::new();
    for key in keys {
        if let Some(value) = replacements.remove(&key) {
            child_replacements.insert(key[1..].to_vec(), value);
        }
    }
    child_replacements
}

fn qualify_archive_path(label: Option<&str>, path: &str) -> String {
    if let Some(label) = label {
        format!("{label}!/{path}")
    } else {
        path.to_string()
    }
}

fn validate_replacement_nested_jar(path: &Path, bytes: &[u8]) -> Result<(), ApplyError> {
    let cursor = Cursor::new(bytes);
    zip::ZipArchive::new(cursor).map(|_| ()).map_err(|source| {
        ApplyError::InvalidReplacementNestedJar {
            path: path.to_path_buf(),
            source,
        }
    })
}

fn is_boot_inf_classes_entry(path: &str) -> bool {
    path == "BOOT-INF/classes"
        || path == "BOOT-INF/classes/"
        || path.starts_with("BOOT-INF/classes/")
}

fn is_boot_inf_lib_entry(path: &str) -> bool {
    path == "BOOT-INF/lib" || path == "BOOT-INF/lib/" || path.starts_with("BOOT-INF/lib/")
}

fn is_web_inf_classes_entry(path: &str) -> bool {
    path == "WEB-INF/classes" || path == "WEB-INF/classes/" || path.starts_with("WEB-INF/classes/")
}

fn is_web_inf_lib_entry(path: &str) -> bool {
    path == "WEB-INF/lib" || path == "WEB-INF/lib/" || path.starts_with("WEB-INF/lib/")
}

fn is_web_inf_lib_provided_entry(path: &str) -> bool {
    path == "WEB-INF/lib-provided"
        || path == "WEB-INF/lib-provided/"
        || path.starts_with("WEB-INF/lib-provided/")
}

fn detect_archive_layout(
    has_boot_inf_classes: bool,
    has_boot_inf_lib: bool,
    has_web_inf_classes: bool,
    has_web_inf_lib: bool,
    has_web_inf_lib_provided: bool,
) -> ArchiveLayout {
    if has_web_inf_classes && (has_web_inf_lib || has_web_inf_lib_provided) {
        ArchiveLayout::SpringBootWar
    } else if has_boot_inf_classes && has_boot_inf_lib {
        ArchiveLayout::SpringBootJar
    } else {
        ArchiveLayout::Unknown
    }
}

fn is_boot_loader_entry(path: &str) -> bool {
    path.ends_with(".class")
        && path.starts_with("org/springframework/boot/loader/")
        && (path.contains("Launcher.class") || path.contains("PropertiesLauncher.class"))
}

fn nested_library_entry(path: &str) -> Option<&str> {
    let rest = path
        .strip_prefix("BOOT-INF/lib/")
        .or_else(|| path.strip_prefix("WEB-INF/lib/"))
        .or_else(|| path.strip_prefix("WEB-INF/lib-provided/"))?;
    if rest.is_empty() {
        return None;
    }
    if !rest.ends_with(".jar") {
        return None;
    }
    if rest.contains('/') {
        return None;
    }
    Some(path)
}

fn is_supported_archive_file(path: &str) -> bool {
    path.ends_with(".jar") || path.ends_with(".war")
}

fn is_signed_metadata_entry(path: &str) -> bool {
    if !path.starts_with("META-INF/") {
        return false;
    }
    let Some(file_name) = path_file_name(path) else {
        return false;
    };
    let upper = file_name.to_ascii_uppercase();
    upper.ends_with(".SF")
        || upper.ends_with(".RSA")
        || upper.ends_with(".DSA")
        || upper.ends_with(".EC")
}

fn index_nested_jar_entries<R: Read>(
    nested_reader: &mut R,
    outer_jar: &str,
    nested_entries: &mut Vec<NestedJarEntry>,
) {
    let mut bytes = Vec::new();
    if nested_reader.read_to_end(&mut bytes).is_err() {
        return;
    }

    let cursor = Cursor::new(bytes);
    let Ok(mut nested_archive) = zip::ZipArchive::new(cursor) else {
        return;
    };

    for index in 0..nested_archive.len() {
        let Ok(entry) = nested_archive.by_index(index) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }

        let inner_path = normalize_entry_name(entry.name());
        nested_entries.push(NestedJarEntry {
            outer_jar: outer_jar.to_string(),
            archive_path: format!("{outer_jar}!/{inner_path}"),
            inner_path,
        });
    }
}

fn path_matches_query(path: &str, query: &str) -> bool {
    path.contains(query) || path_file_name(path).is_some_and(|file_name| file_name.contains(query))
}

fn path_file_name(path: &str) -> Option<&str> {
    path.rsplit('/')
        .next()
        .filter(|file_name| !file_name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::FileOptions;
    use zip::CompressionMethod;

    fn write_jar(entries: &[(&str, CompressionMethod, &[u8])]) -> PathBuf {
        write_jar_with_modes(
            &entries
                .iter()
                .map(|(name, method, bytes)| (*name, *method, *bytes, None))
                .collect::<Vec<_>>(),
        )
    }

    fn write_jar_with_modes(entries: &[(&str, CompressionMethod, &[u8], Option<u32>)]) -> PathBuf {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fixture.jar");
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        for (name, method, bytes, mode) in entries {
            let mut options = FileOptions::default().compression_method(*method);
            if let Some(mode) = mode {
                options = options.unix_permissions(*mode);
            }
            zip.start_file(*name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();

        // Intentionally leak tempdir to keep path valid for returned fixture.
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

    fn spring_boot_fixture_with_nested_entries() -> PathBuf {
        let nested = nested_jar_bytes(&[
            (
                "com/acme/OrderService.class",
                CompressionMethod::Deflated,
                b"class-bytes",
            ),
            (
                "com/acme/config/order.yml",
                CompressionMethod::Stored,
                b"enabled: true",
            ),
        ]);

        write_jar(&[
            (
                "BOOT-INF/classes/application.yml",
                CompressionMethod::Stored,
                b"server.port: 8080",
            ),
            ("BOOT-INF/lib/order.jar", CompressionMethod::Stored, &nested),
        ])
    }

    fn spring_boot_jar_bytes_with_nested_entries(compressed_nested: bool) -> Vec<u8> {
        let nested_method = if compressed_nested {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        };
        let nested = nested_jar_bytes(&[
            (
                "com/acme/OrderService.class",
                CompressionMethod::Deflated,
                b"class-bytes",
            ),
            (
                "com/acme/config/order.yml",
                CompressionMethod::Stored,
                b"enabled: true",
            ),
        ]);

        nested_jar_bytes(&[
            (
                "BOOT-INF/classes/application.yml",
                CompressionMethod::Stored,
                b"server.port: 8080",
            ),
            ("BOOT-INF/lib/order.jar", nested_method, &nested),
        ])
    }

    fn zip_wrapper_fixture_with_nested_entries() -> PathBuf {
        let app = spring_boot_jar_bytes_with_nested_entries(false);
        write_jar_with_modes(&[
            (
                "bin/start.sh",
                CompressionMethod::Stored,
                b"#!/bin/sh\njava -jar app/service.jar\n",
                Some(0o755),
            ),
            (
                "config/runtime.yml",
                CompressionMethod::Deflated,
                b"env: prod\n",
                None,
            ),
            (
                "templates/banner.txt",
                CompressionMethod::Stored,
                b"banner\n",
                None,
            ),
            ("app/service.jar", CompressionMethod::Deflated, &app, None),
        ])
    }

    fn spring_boot_war_fixture_with_nested_entries() -> PathBuf {
        let lib_nested = nested_jar_bytes(&[
            (
                "com/acme/OrderService.class",
                CompressionMethod::Deflated,
                b"class-bytes",
            ),
            (
                "com/acme/DuplicateName.class",
                CompressionMethod::Stored,
                b"lib-duplicate",
            ),
        ]);
        let provided_nested = nested_jar_bytes(&[
            (
                "com/acme/ProvidedService.class",
                CompressionMethod::Deflated,
                b"provided-class-bytes",
            ),
            (
                "com/acme/provided/DuplicateName.class",
                CompressionMethod::Stored,
                b"provided-duplicate",
            ),
        ]);

        write_jar(&[
            (
                "WEB-INF/classes/application.yml",
                CompressionMethod::Stored,
                b"server.port: 8080",
            ),
            (
                "WEB-INF/lib/order.jar",
                CompressionMethod::Stored,
                &lib_nested,
            ),
            (
                "WEB-INF/lib-provided/container.jar",
                CompressionMethod::Stored,
                &provided_nested,
            ),
            (
                "org/springframework/boot/loader/launch/WarLauncher.class",
                CompressionMethod::Stored,
                b"boot-loader",
            ),
        ])
    }

    fn spring_boot_fixture() -> PathBuf {
        write_jar(&[
            ("BOOT-INF/classes", CompressionMethod::Stored, b""),
            (
                "BOOT-INF/classes/Marker.class",
                CompressionMethod::Stored,
                b"",
            ),
            ("BOOT-INF/lib/dep.jar", CompressionMethod::Stored, b"nested"),
            (
                "BOOT-INF/lib/compressed.jar",
                CompressionMethod::Deflated,
                b"nested-compressed",
            ),
            (
                "org/springframework/boot/loader/Launcher.class",
                CompressionMethod::Stored,
                b"boot-loader",
            ),
        ])
    }

    fn non_spring_fixture() -> PathBuf {
        write_jar(&[
            ("com/example/App.class", CompressionMethod::Stored, b""),
            ("README.txt", CompressionMethod::Stored, b"hello"),
        ])
    }

    fn invalid_jar_fixture() -> PathBuf {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.jar");
        std::fs::write(&path, b"not a jar file").unwrap();
        std::mem::forget(dir);
        path
    }

    fn write_input_file(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, bytes).unwrap();
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
        read_entry_bytes_by_name(&mut archive, inner_path)
    }

    fn read_entry_bytes_by_name<R: Read + Seek>(
        archive: &mut zip::ZipArchive<R>,
        entry_name: &str,
    ) -> Vec<u8> {
        let mut entry = archive.by_name(entry_name).unwrap();
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

    fn jar_entry_unix_mode(path: &Path, entry_name: &str) -> Option<u32> {
        let file = File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let entry = archive.by_name(entry_name).unwrap();
        entry.unix_mode()
    }

    #[test]
    fn parses_outer_archive_paths() {
        let path = ArchivePath::parse("BOOT-INF/lib/order-module.jar").unwrap();
        assert_eq!(
            path,
            ArchivePath::Outer {
                path: "BOOT-INF/lib/order-module.jar".to_string()
            }
        );
    }

    #[test]
    fn parses_nested_archive_paths() {
        let path = ArchivePath::parse("BOOT-INF/lib/order-module.jar!/com/acme/OrderService.class")
            .unwrap();
        assert_eq!(
            path,
            ArchivePath::Nested {
                outer_jar: "BOOT-INF/lib/order-module.jar".to_string(),
                inner_path: "com/acme/OrderService.class".to_string(),
            }
        );
    }

    #[test]
    fn parses_nested_archive_paths_without_slash_after_separator() {
        let path = ArchivePath::parse("BOOT-INF/lib/order-module.jar!com/acme/OrderService.class")
            .unwrap();
        assert_eq!(
            path,
            ArchivePath::Nested {
                outer_jar: "BOOT-INF/lib/order-module.jar".to_string(),
                inner_path: "com/acme/OrderService.class".to_string(),
            }
        );
    }

    #[test]
    fn parses_chained_archive_paths() {
        let path = ArchivePath::parse(
            "app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class",
        )
        .unwrap();
        assert_eq!(
            path,
            ArchivePath::Chained {
                segments: vec![
                    "app/service.jar".to_string(),
                    "BOOT-INF/lib/order.jar".to_string(),
                    "com/acme/OrderService.class".to_string(),
                ]
            }
        );
    }

    #[test]
    fn normalizes_backslashes() {
        let path = ArchivePath::parse(r"BOOT-INF\classes\application.yml").unwrap();
        assert_eq!(
            path,
            ArchivePath::Outer {
                path: "BOOT-INF/classes/application.yml".to_string()
            }
        );
    }

    #[test]
    fn rejects_unsafe_archive_paths() {
        assert!(matches!(
            ArchivePath::parse("../BOOT-INF/classes/application.yml"),
            Err(ArchivePathParseError::DotDotSegment)
        ));
        assert!(matches!(
            ArchivePath::parse("/tmp/app.jar"),
            Err(ArchivePathParseError::InvalidAbsolutePath)
        ));
        assert!(matches!(
            ArchivePath::parse("C:/temp/app.jar"),
            Err(ArchivePathParseError::InvalidDrivePath)
        ));
        assert!(matches!(
            ArchivePath::parse("D:temp.jar"),
            Err(ArchivePathParseError::InvalidDrivePath)
        ));
        assert!(matches!(
            ArchivePath::parse("BOOT-INF/lib/order.jar!"),
            Err(ArchivePathParseError::EmptyInnerPath)
        ));
        assert!(matches!(
            ArchivePath::parse("BOOT-INF/lib/order.jar!!/a.class"),
            Err(ArchivePathParseError::MultipleNestedSeparators)
        ));
        assert!(matches!(
            ArchivePath::parse("BOOT-INF//classes"),
            Err(ArchivePathParseError::EmptySegment)
        ));
        assert!(matches!(
            ArchivePath::parse("./BOOT-INF/classes"),
            Err(ArchivePathParseError::DotSegment)
        ));
    }

    #[test]
    fn indexes_spring_boot_markers_and_nested_storage() {
        let jar = spring_boot_fixture();
        let index = build_jar_index(&jar).unwrap();

        assert!(index.has_boot_inf_classes);
        assert!(index.has_boot_inf_lib);
        assert_eq!(index.layout, ArchiveLayout::SpringBootJar);
        assert!(index.has_boot_loader_entry);

        assert_eq!(index.entries.len(), 5);
        assert_eq!(index.nested_jars.len(), 2);

        let stored = index
            .nested_jars
            .iter()
            .find(|entry| entry.path.ends_with("dep.jar"))
            .expect("dep.jar must be present");
        assert!(stored.is_stored);
        assert_eq!(stored.compression_method, "Stored");

        let compressed = index
            .nested_jars
            .iter()
            .find(|entry| entry.path.ends_with("compressed.jar"))
            .expect("compressed.jar must be present");
        assert!(!compressed.is_stored);
        assert_eq!(compressed.compression_method, "Deflated");
    }

    #[test]
    fn indexes_spring_boot_war_markers_and_nested_storage() {
        let war = spring_boot_war_fixture_with_nested_entries();
        let index = build_jar_index(&war).unwrap();

        assert_eq!(index.layout, ArchiveLayout::SpringBootWar);
        assert!(index.has_web_inf_classes);
        assert!(index.has_web_inf_lib);
        assert!(index.has_web_inf_lib_provided);
        assert!(index.has_boot_loader_entry);
        assert_eq!(index.nested_jars.len(), 2);
        assert!(index
            .nested_jars
            .iter()
            .any(|entry| entry.path == "WEB-INF/lib/order.jar" && entry.is_stored));
        assert!(index.nested_jars.iter().any(|entry| {
            entry.path == "WEB-INF/lib-provided/container.jar" && entry.is_stored
        }));
        assert!(index.nested_entries.iter().any(|entry| {
            entry.archive_path == "WEB-INF/lib/order.jar!/com/acme/OrderService.class"
        }));
        assert!(index.nested_entries.iter().any(|entry| {
            entry.archive_path
                == "WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class"
        }));
    }

    #[test]
    fn indexes_readable_nested_jar_entries() {
        let jar = spring_boot_fixture_with_nested_entries();
        let index = build_jar_index(&jar).unwrap();

        assert_eq!(index.nested_entries.len(), 2);
        assert!(index.nested_entries.iter().any(|entry| {
            entry.archive_path == "BOOT-INF/lib/order.jar!/com/acme/OrderService.class"
        }));
        assert!(index.nested_entries.iter().any(|entry| {
            entry.archive_path == "BOOT-INF/lib/order.jar!/com/acme/config/order.yml"
        }));
    }

    #[test]
    fn indexes_zip_wrapper_with_contained_spring_boot_archive() {
        let wrapper = zip_wrapper_fixture_with_nested_entries();
        let index = build_jar_index(&wrapper).unwrap();

        assert_eq!(index.layout, ArchiveLayout::ZipWrapper);
        assert!(index
            .entries
            .iter()
            .any(|entry| entry.path == "config/runtime.yml"));
        assert_eq!(index.contained_archives.len(), 1);
        assert_eq!(index.contained_archives[0].path, "app/service.jar");
        assert_eq!(
            index.contained_archives[0].layout,
            ArchiveLayout::SpringBootJar
        );
        assert!(index.nested_jars.iter().any(|entry| entry.path
            == "app/service.jar!/BOOT-INF/lib/order.jar"
            && entry.is_stored));
        assert!(index.nested_entries.iter().any(|entry| {
            entry.archive_path
                == "app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class"
        }));
    }

    #[test]
    fn finds_nested_entries_by_filename() {
        let jar = spring_boot_fixture_with_nested_entries();
        let results = find_in_jar(&jar, "OrderService.class").unwrap();

        assert_eq!(
            results,
            vec![FindResult {
                archive_path: "BOOT-INF/lib/order.jar!/com/acme/OrderService.class".to_string()
            }]
        );
    }

    #[test]
    fn finds_outer_entries_by_path() {
        let jar = spring_boot_fixture_with_nested_entries();
        let results = find_in_jar(&jar, "BOOT-INF/classes/application.yml").unwrap();

        assert_eq!(
            results,
            vec![FindResult {
                archive_path: "BOOT-INF/classes/application.yml".to_string()
            }]
        );
    }

    #[test]
    fn finds_wrapper_entries_and_chained_nested_entries() {
        let wrapper = zip_wrapper_fixture_with_nested_entries();

        let config_results = find_in_jar(&wrapper, "runtime.yml").unwrap();
        assert_eq!(
            config_results,
            vec![FindResult {
                archive_path: "config/runtime.yml".to_string()
            }]
        );

        let app_results = find_in_jar(&wrapper, "application.yml").unwrap();
        assert!(app_results.iter().any(|result| {
            result.archive_path == "app/service.jar!/BOOT-INF/classes/application.yml"
        }));

        let class_results = find_in_jar(&wrapper, "OrderService.class").unwrap();
        assert!(class_results.iter().any(|result| {
            result.archive_path
                == "app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class"
        }));
    }

    #[test]
    fn finds_war_outer_and_nested_entries() {
        let war = spring_boot_war_fixture_with_nested_entries();

        let outer = find_in_jar(&war, "WEB-INF/classes/application.yml").unwrap();
        assert_eq!(
            outer,
            vec![FindResult {
                archive_path: "WEB-INF/classes/application.yml".to_string()
            }]
        );

        let nested = find_in_jar(&war, "ProvidedService.class").unwrap();
        assert_eq!(
            nested,
            vec![FindResult {
                archive_path: "WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class"
                    .to_string()
            }]
        );
    }

    #[test]
    fn find_returns_empty_results_for_no_match() {
        let jar = spring_boot_fixture_with_nested_entries();
        let results = find_in_jar(&jar, "Missing.class").unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn invalid_jars_fail_find() {
        let jar = invalid_jar_fixture();
        let err = find_in_jar(&jar, "OrderService.class").unwrap_err();
        assert!(matches!(err, JarInspectError::InvalidJar(_)));
    }

    #[test]
    fn match_selects_unique_exact_relative_path() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let input = dir.path().join("BOOT-INF/classes/application.yml");
        write_input_file(&input, b"server.port: 9090");

        let candidates = match_in_jar(&jar, &[dir.path().to_path_buf()]).unwrap();

        assert_eq!(candidates.matches.len(), 1);
        assert_eq!(candidates.matches[0].status, MatchStatus::Selected);
        assert_eq!(candidates.matches[0].candidates.len(), 1);
        assert_eq!(
            candidates.matches[0].candidates[0].target,
            "BOOT-INF/classes/application.yml"
        );
        assert_eq!(
            candidates.matches[0].candidates[0].reason,
            vec!["exact relative path".to_string()]
        );
    }

    #[test]
    fn match_selects_wrapper_and_chained_paths_without_unqualified_inference() {
        let wrapper = zip_wrapper_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let wrapper_input = dir.path().join("config/runtime.yml");
        let chained_input = dir
            .path()
            .join("app/service.jar!/BOOT-INF/classes/application.yml");
        let unqualified_input = dir.path().join("BOOT-INF/classes/application.yml");
        write_input_file(&wrapper_input, b"env: test\n");
        write_input_file(&chained_input, b"server.port: 9090");
        write_input_file(&unqualified_input, b"server.port: 7070");

        let candidates = match_in_jar(&wrapper, &[dir.path().to_path_buf()]).unwrap();

        let wrapper_match = candidates
            .matches
            .iter()
            .find(|input| input.input.ends_with("config/runtime.yml"))
            .unwrap();
        assert_eq!(wrapper_match.status, MatchStatus::Selected);
        assert_eq!(wrapper_match.candidates[0].target, "config/runtime.yml");

        let chained_match = candidates
            .matches
            .iter()
            .find(|input| {
                input
                    .input
                    .ends_with("app/service.jar!/BOOT-INF/classes/application.yml")
            })
            .unwrap();
        assert_eq!(chained_match.status, MatchStatus::Selected);
        assert_eq!(
            chained_match.candidates[0].target,
            "app/service.jar!/BOOT-INF/classes/application.yml"
        );

        let unqualified_match = candidates
            .matches
            .iter()
            .find(|input| {
                input.input.ends_with("BOOT-INF/classes/application.yml")
                    && !input.input.contains("service.jar!")
            })
            .unwrap();
        assert_ne!(unqualified_match.status, MatchStatus::Selected);
    }

    #[test]
    fn match_marks_ambiguous_filename_matches_for_selection() {
        let nested = nested_jar_bytes(&[(
            "com/acme/OrderCalculator.class",
            CompressionMethod::Stored,
            b"nested",
        )]);
        let jar = write_jar(&[
            (
                "BOOT-INF/classes/com/acme/OrderCalculator.class",
                CompressionMethod::Stored,
                b"outer",
            ),
            ("BOOT-INF/lib/order.jar", CompressionMethod::Stored, &nested),
        ]);
        let dir = tempdir().unwrap();
        let input = dir.path().join("OrderCalculator.class");
        write_input_file(&input, b"replacement");

        let candidates = match_in_jar(&jar, &[dir.path().to_path_buf()]).unwrap();

        assert_eq!(candidates.matches.len(), 1);
        assert_eq!(candidates.matches[0].status, MatchStatus::NeedsSelection);
        let targets: Vec<&str> = candidates.matches[0]
            .candidates
            .iter()
            .map(|candidate| candidate.target.as_str())
            .collect();
        assert_eq!(
            targets,
            vec![
                "BOOT-INF/classes/com/acme/OrderCalculator.class",
                "BOOT-INF/lib/order.jar!/com/acme/OrderCalculator.class",
            ]
        );
        assert!(candidates.matches[0]
            .candidates
            .iter()
            .all(|candidate| { candidate.reason == vec!["same filename".to_string()] }));
    }

    #[test]
    fn match_selects_war_paths_and_marks_ambiguous_nested_filenames() {
        let war = spring_boot_war_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let exact = dir.path().join("WEB-INF/classes/application.yml");
        write_input_file(&exact, b"server.port: 9090");
        let ambiguous = dir.path().join("DuplicateName.class");
        write_input_file(&ambiguous, b"replacement");

        let candidates = match_in_jar(&war, &[dir.path().to_path_buf()]).unwrap();

        let exact_match = candidates
            .matches
            .iter()
            .find(|input| input.input.ends_with("WEB-INF/classes/application.yml"))
            .unwrap();
        assert_eq!(exact_match.status, MatchStatus::Selected);
        assert_eq!(
            exact_match.candidates[0].target,
            "WEB-INF/classes/application.yml"
        );

        let ambiguous_match = candidates
            .matches
            .iter()
            .find(|input| input.input.ends_with("DuplicateName.class"))
            .unwrap();
        assert_eq!(ambiguous_match.status, MatchStatus::NeedsSelection);
        let targets: Vec<&str> = ambiguous_match
            .candidates
            .iter()
            .map(|candidate| candidate.target.as_str())
            .collect();
        assert_eq!(
            targets,
            vec![
                "WEB-INF/lib/order.jar!/com/acme/DuplicateName.class",
                "WEB-INF/lib-provided/container.jar!/com/acme/provided/DuplicateName.class",
            ]
        );
    }

    #[test]
    fn match_records_no_match_results() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let input = dir.path().join("Missing.class");
        write_input_file(&input, b"replacement");

        let candidates = match_in_jar(&jar, &[dir.path().to_path_buf()]).unwrap();

        assert_eq!(candidates.matches.len(), 1);
        assert_eq!(candidates.matches[0].status, MatchStatus::NoMatch);
        assert!(candidates.matches[0].candidates.is_empty());
    }

    #[test]
    fn match_fails_for_missing_input_path() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing");

        let err = match_in_jar(&jar, &[missing.clone()]).unwrap_err();

        assert!(matches!(err, MatchError::InputPath { path, .. } if path == missing));
    }

    #[test]
    fn renders_candidates_yaml() {
        let candidates = CandidateFile {
            source: "app.jar".to_string(),
            matches: vec![InputMatch {
                input: "./patch/application.yml".to_string(),
                status: MatchStatus::Selected,
                candidates: vec![CandidateTarget {
                    target: "BOOT-INF/classes/application.yml".to_string(),
                    score: 100,
                    reason: vec!["exact relative path".to_string()],
                }],
            }],
        };

        assert_eq!(
            candidates.to_yaml(),
            concat!(
                "kind: candidates\n",
                "version: 1\n",
                "source: \"app.jar\"\n",
                "\n",
                "matches:\n",
                "  - input: \"./patch/application.yml\"\n",
                "    status: selected\n",
                "    candidates:\n",
                "      - target: \"BOOT-INF/classes/application.yml\"\n",
                "        score: 100\n",
                "        reason:\n",
                "          - \"exact relative path\"\n",
            )
        );
    }

    #[test]
    fn renders_selected_match_as_patch_snippet() {
        let candidates = CandidateFile {
            source: "app.jar".to_string(),
            matches: vec![InputMatch {
                input: "./patch/BOOT-INF/classes/application.yml".to_string(),
                status: MatchStatus::Selected,
                candidates: vec![CandidateTarget {
                    target: "BOOT-INF/classes/application.yml".to_string(),
                    score: 100,
                    reason: vec!["exact relative path".to_string()],
                }],
            }],
        };

        assert_eq!(
            candidates.to_snippets(),
            concat!(
                "# kind: patch-plan\n",
                "# version: 1\n",
                "operations:\n",
                "  - replace-entry:\n",
                "      target: \"BOOT-INF/classes/application.yml\"\n",
                "      with: \"./patch/BOOT-INF/classes/application.yml\"\n",
            )
        );
    }

    #[test]
    fn renders_ambiguous_and_no_match_snippets_as_comments() {
        let candidates = CandidateFile {
            source: "app.jar".to_string(),
            matches: vec![
                InputMatch {
                    input: "./patch/OrderCalculator.class".to_string(),
                    status: MatchStatus::NeedsSelection,
                    candidates: vec![
                        CandidateTarget {
                            target: "BOOT-INF/classes/com/acme/OrderCalculator.class".to_string(),
                            score: 80,
                            reason: vec!["same filename".to_string()],
                        },
                        CandidateTarget {
                            target: "BOOT-INF/lib/order.jar!/com/acme/OrderCalculator.class"
                                .to_string(),
                            score: 80,
                            reason: vec!["same filename".to_string()],
                        },
                    ],
                },
                InputMatch {
                    input: "./patch/Missing.class".to_string(),
                    status: MatchStatus::NoMatch,
                    candidates: Vec::new(),
                },
            ],
        };

        let snippets = candidates.to_snippets();
        assert!(snippets.contains("# needs-selection: \"./patch/OrderCalculator.class\"\n"));
        assert!(snippets.contains(
            "#      target: \"BOOT-INF/lib/order.jar!/com/acme/OrderCalculator.class\"\n"
        ));
        assert!(snippets.contains("# no-match: \"./patch/Missing.class\"\n"));
        assert!(!snippets.lines().any(|line| line == "  - replace-entry:"));
    }

    #[test]
    fn parses_patch_plan_replace_operations() {
        let plan = parse_patch_plan(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/classes/application.yml
      with: ./patch/application.yml
"#,
        )
        .unwrap();

        assert_eq!(
            plan,
            PatchPlan {
                operations: vec![ReplaceOperation {
                    target: "BOOT-INF/classes/application.yml".to_string(),
                    source: PathBuf::from("./patch/application.yml"),
                }]
            }
        );
    }

    #[test]
    fn parses_patch_plan_nested_replace_operations() {
        let plan = parse_patch_plan(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/lib/order.jar!/com/acme/OrderService.class
      with: ./patch/OrderService.class
"#,
        )
        .unwrap();

        assert_eq!(
            plan.operations[0],
            ReplaceOperation {
                target: "BOOT-INF/lib/order.jar!/com/acme/OrderService.class".to_string(),
                source: PathBuf::from("./patch/OrderService.class"),
            }
        );
    }

    #[test]
    fn rejects_candidates_as_patch_plan() {
        let err = parse_patch_plan(
            r#"
kind: candidates
version: 1
matches: []
"#,
        )
        .unwrap_err();

        assert!(matches!(err, ApplyError::UnsupportedPlanKind(kind) if kind == "candidates"));
    }

    #[test]
    fn apply_replaces_boot_inf_classes_resource() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("application.yml");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
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

        apply_patch_plan(&jar, &plan, &output).unwrap();

        assert_eq!(
            read_jar_entry(&output, "BOOT-INF/classes/application.yml"),
            b"server.port: 9090"
        );
        assert_eq!(
            read_jar_entry(&jar, "BOOT-INF/classes/application.yml"),
            b"server.port: 8080"
        );
        assert_eq!(
            read_jar_entry(&output, "BOOT-INF/lib/order.jar"),
            read_jar_entry(&jar, "BOOT-INF/lib/order.jar")
        );
    }

    #[test]
    fn apply_replaces_web_inf_classes_resource() {
        let war = spring_boot_war_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("application.yml");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.war");
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

        apply_patch_plan(&war, &plan, &output).unwrap();

        assert_eq!(
            read_jar_entry(&output, "WEB-INF/classes/application.yml"),
            b"server.port: 9090"
        );
        assert!(verify_jar(&output).unwrap().is_success());
    }

    #[test]
    fn apply_replaces_wrapper_entries_and_contained_archive_entries() {
        let wrapper = zip_wrapper_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let config_replacement = dir.path().join("runtime.yml");
        let app_replacement = dir.path().join("application.yml");
        let class_replacement = dir.path().join("OrderService.class");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.zip");
        write_input_file(&config_replacement, b"env: patched\n");
        write_input_file(&app_replacement, b"server.port: 9090");
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
      target: app/service.jar!/BOOT-INF/classes/application.yml
      with: "{}"
  - replace-entry:
      target: app/service.jar!/BOOT-INF/lib/order.jar!/com/acme/OrderService.class
      with: "{}"
"#,
                config_replacement.display(),
                app_replacement.display(),
                class_replacement.display()
            ),
        )
        .unwrap();

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
            b"server.port: 9090"
        );
        let service_bytes = read_jar_entry(&output, "app/service.jar");
        let cursor = std::io::Cursor::new(service_bytes);
        let mut service = zip::ZipArchive::new(cursor).unwrap();
        let order_bytes = read_entry_bytes_by_name(&mut service, "BOOT-INF/lib/order.jar");
        let cursor = std::io::Cursor::new(order_bytes);
        let mut order = zip::ZipArchive::new(cursor).unwrap();
        assert_eq!(
            read_entry_bytes_by_name(&mut order, "com/acme/OrderService.class"),
            b"patched-class"
        );
        assert!(verify_jar(&output).unwrap().is_success());
    }

    #[test]
    fn apply_preserves_wrapper_script_unix_mode() {
        let wrapper = zip_wrapper_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let script_replacement = dir.path().join("start.sh");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.zip");
        write_input_file(&script_replacement, b"#!/bin/sh\necho patched\n");
        std::fs::write(
            &plan,
            format!(
                r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: bin/start.sh
      with: "{}"
"#,
                script_replacement.display()
            ),
        )
        .unwrap();

        apply_patch_plan(&wrapper, &plan, &output).unwrap();

        assert_eq!(
            read_jar_entry(&output, "bin/start.sh"),
            b"#!/bin/sh\necho patched\n"
        );
        assert_eq!(
            jar_entry_unix_mode(&output, "bin/start.sh").map(|mode| mode & 0o777),
            Some(0o755)
        );
    }

    #[test]
    fn apply_fails_for_missing_replacement_source() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing.yml");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
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
                missing.display()
            ),
        )
        .unwrap();

        let err = apply_patch_plan(&jar, &plan, &output).unwrap_err();

        assert!(matches!(err, ApplyError::MissingReplacementSource(path) if path == missing));
        assert!(!output.exists());
    }

    #[test]
    fn apply_fails_for_missing_outer_target() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("application.yml");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
        write_input_file(&replacement, b"server.port: 9090");
        std::fs::write(
            &plan,
            format!(
                r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/classes/missing.yml
      with: "{}"
"#,
                replacement.display()
            ),
        )
        .unwrap();

        let err = apply_patch_plan(&jar, &plan, &output).unwrap_err();

        assert!(
            matches!(err, ApplyError::MissingTarget(target) if target == "BOOT-INF/classes/missing.yml")
        );
        assert!(!output.exists());
    }

    #[test]
    fn apply_replaces_nested_jar_entry_and_stores_outer_entry() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("OrderService.class");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
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

        apply_patch_plan(&jar, &plan, &output).unwrap();

        assert_eq!(
            read_nested_jar_entry(
                &output,
                "BOOT-INF/lib/order.jar",
                "com/acme/OrderService.class"
            ),
            b"patched-class-bytes"
        );
        assert_eq!(
            read_nested_jar_entry(
                &output,
                "BOOT-INF/lib/order.jar",
                "com/acme/config/order.yml"
            ),
            b"enabled: true"
        );
        assert_eq!(
            jar_entry_compression(&output, "BOOT-INF/lib/order.jar"),
            CompressionMethod::Stored
        );
    }

    #[test]
    fn apply_replaces_war_nested_entries_and_stores_outer_entries() {
        let war = spring_boot_war_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let lib_replacement = dir.path().join("OrderService.class");
        let provided_replacement = dir.path().join("ProvidedService.class");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.war");
        write_input_file(&lib_replacement, b"patched-lib-class");
        write_input_file(&provided_replacement, b"patched-provided-class");
        std::fs::write(
            &plan,
            format!(
                r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: WEB-INF/lib/order.jar!/com/acme/OrderService.class
      with: "{}"
  - replace-entry:
      target: WEB-INF/lib-provided/container.jar!/com/acme/ProvidedService.class
      with: "{}"
"#,
                lib_replacement.display(),
                provided_replacement.display()
            ),
        )
        .unwrap();

        apply_patch_plan(&war, &plan, &output).unwrap();

        assert_eq!(
            read_nested_jar_entry(
                &output,
                "WEB-INF/lib/order.jar",
                "com/acme/OrderService.class"
            ),
            b"patched-lib-class"
        );
        assert_eq!(
            read_nested_jar_entry(
                &output,
                "WEB-INF/lib-provided/container.jar",
                "com/acme/ProvidedService.class"
            ),
            b"patched-provided-class"
        );
        assert_eq!(
            jar_entry_compression(&output, "WEB-INF/lib/order.jar"),
            CompressionMethod::Stored
        );
        assert_eq!(
            jar_entry_compression(&output, "WEB-INF/lib-provided/container.jar"),
            CompressionMethod::Stored
        );
    }

    #[test]
    fn apply_groups_multiple_operations_in_same_nested_jar() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let class_replacement = dir.path().join("OrderService.class");
        let config_replacement = dir.path().join("order.yml");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
        write_input_file(&class_replacement, b"patched-class-bytes");
        write_input_file(&config_replacement, b"enabled: false");
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
  - replace-entry:
      target: BOOT-INF/lib/order.jar!/com/acme/config/order.yml
      with: "{}"
"#,
                class_replacement.display(),
                config_replacement.display()
            ),
        )
        .unwrap();

        apply_patch_plan(&jar, &plan, &output).unwrap();

        assert_eq!(
            read_nested_jar_entry(
                &output,
                "BOOT-INF/lib/order.jar",
                "com/acme/OrderService.class"
            ),
            b"patched-class-bytes"
        );
        assert_eq!(
            read_nested_jar_entry(
                &output,
                "BOOT-INF/lib/order.jar",
                "com/acme/config/order.yml"
            ),
            b"enabled: false"
        );
    }

    #[test]
    fn apply_replaces_whole_nested_jar_and_stores_outer_entry() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("order-replacement.jar");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
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

        apply_patch_plan(&jar, &plan, &output).unwrap();

        assert_eq!(
            read_jar_entry(&output, "BOOT-INF/lib/order.jar"),
            replacement_bytes
        );
        assert_eq!(
            jar_entry_compression(&output, "BOOT-INF/lib/order.jar"),
            CompressionMethod::Stored
        );
        assert_eq!(
            read_nested_jar_entry(
                &output,
                "BOOT-INF/lib/order.jar",
                "com/acme/NewOrderService.class"
            ),
            b"new-class-bytes"
        );
    }

    #[test]
    fn apply_replaces_whole_war_nested_jar_and_stores_outer_entry() {
        let war = spring_boot_war_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("container-replacement.jar");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.war");
        let replacement_bytes = nested_jar_bytes(&[(
            "com/acme/NewProvidedService.class",
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
      target: WEB-INF/lib-provided/container.jar
      with: "{}"
"#,
                replacement.display()
            ),
        )
        .unwrap();

        apply_patch_plan(&war, &plan, &output).unwrap();

        assert_eq!(
            read_jar_entry(&output, "WEB-INF/lib-provided/container.jar"),
            replacement_bytes
        );
        assert_eq!(
            jar_entry_compression(&output, "WEB-INF/lib-provided/container.jar"),
            CompressionMethod::Stored
        );
    }

    #[test]
    fn apply_rejects_invalid_whole_nested_jar_replacement() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("not-a-jar.bin");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
        write_input_file(&replacement, b"not a jar");
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

        let err = apply_patch_plan(&jar, &plan, &output).unwrap_err();

        assert!(
            matches!(err, ApplyError::InvalidReplacementNestedJar { path, .. } if path == replacement)
        );
        assert!(!output.exists());
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
        let output = dir.path().join("patched.jar");
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

        let err = apply_patch_plan(&jar, &plan, &output).unwrap_err();

        assert!(
            matches!(err, ApplyError::VerificationFailed { output: failed_output, non_stored_nested_jars }
                if failed_output == output
                    && non_stored_nested_jars.len() == 1
                    && non_stored_nested_jars[0].path == "BOOT-INF/lib/order.jar")
        );
        assert!(output.exists());
        assert_eq!(
            read_jar_entry(&output, "BOOT-INF/classes/application.yml"),
            b"server.port: 9090"
        );
        assert_eq!(
            jar_entry_compression(&output, "BOOT-INF/lib/order.jar"),
            CompressionMethod::Deflated
        );
    }

    #[test]
    fn verify_succeeds_for_stored_nested_jars() {
        let jar = spring_boot_fixture_with_nested_entries();
        let report = verify_jar(&jar).unwrap();

        assert!(report.is_success());
        assert_eq!(report.nested_jars.len(), 1);
        assert!(report.non_stored_nested_jars.is_empty());
        assert!(report.signed_metadata.is_empty());
    }

    #[test]
    fn verify_succeeds_for_zip_wrapper_with_stored_contained_nested_jars() {
        let wrapper = zip_wrapper_fixture_with_nested_entries();
        let report = verify_jar(&wrapper).unwrap();

        assert!(report.is_success());
        assert_eq!(report.contained_archives.len(), 1);
        assert!(report
            .nested_jars
            .iter()
            .any(|entry| entry.path == "app/service.jar!/BOOT-INF/lib/order.jar"));
        assert!(report.non_stored_nested_jars.is_empty());
    }

    #[test]
    fn verify_fails_for_compressed_nested_jars() {
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

        let report = verify_jar(&jar).unwrap();

        assert!(!report.is_success());
        assert_eq!(report.non_stored_nested_jars.len(), 1);
        assert_eq!(
            report.non_stored_nested_jars[0].path,
            "BOOT-INF/lib/order.jar"
        );
    }

    #[test]
    fn verify_fails_for_zip_wrapper_with_compressed_contained_nested_jar() {
        let app = spring_boot_jar_bytes_with_nested_entries(true);
        let wrapper = write_jar(&[("app/service.jar", CompressionMethod::Deflated, &app)]);

        let report = verify_jar(&wrapper).unwrap();

        assert!(!report.is_success());
        assert_eq!(report.non_stored_nested_jars.len(), 1);
        assert_eq!(
            report.non_stored_nested_jars[0].path,
            "app/service.jar!/BOOT-INF/lib/order.jar"
        );
    }

    #[test]
    fn verify_fails_for_compressed_war_nested_jars() {
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

        let report = verify_jar(&war).unwrap();

        assert!(!report.is_success());
        assert_eq!(report.non_stored_nested_jars.len(), 1);
        assert_eq!(
            report.non_stored_nested_jars[0].path,
            "WEB-INF/lib/order.jar"
        );
    }

    #[test]
    fn verify_warns_on_signed_metadata() {
        let jar = write_jar(&[
            (
                "META-INF/APP.SF",
                CompressionMethod::Stored,
                b"Signature-Version: 1.0",
            ),
            ("META-INF/APP.RSA", CompressionMethod::Stored, b"signature"),
            (
                "BOOT-INF/lib/order.jar",
                CompressionMethod::Stored,
                &nested_jar_bytes(&[(
                    "com/acme/OrderService.class",
                    CompressionMethod::Stored,
                    b"",
                )]),
            ),
        ]);

        let report = verify_jar(&jar).unwrap();

        assert!(report.is_success());
        assert_eq!(
            report.signed_metadata,
            vec![
                "META-INF/APP.SF".to_string(),
                "META-INF/APP.RSA".to_string()
            ]
        );
    }

    #[test]
    fn apply_fails_for_missing_nested_jar() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("OrderService.class");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
        write_input_file(&replacement, b"patched-class-bytes");
        std::fs::write(
            &plan,
            format!(
                r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/lib/missing.jar!/com/acme/OrderService.class
      with: "{}"
"#,
                replacement.display()
            ),
        )
        .unwrap();

        let err = apply_patch_plan(&jar, &plan, &output).unwrap_err();

        assert!(
            matches!(err, ApplyError::MissingNestedJar(target) if target == "BOOT-INF/lib/missing.jar")
        );
        assert!(!output.exists());
    }

    #[test]
    fn apply_fails_for_missing_nested_target() {
        let jar = spring_boot_fixture_with_nested_entries();
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("Missing.class");
        let plan = dir.path().join("patch-plan.yaml");
        let output = dir.path().join("patched.jar");
        write_input_file(&replacement, b"patched-class-bytes");
        std::fs::write(
            &plan,
            format!(
                r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/lib/order.jar!/com/acme/Missing.class
      with: "{}"
"#,
                replacement.display()
            ),
        )
        .unwrap();

        let err = apply_patch_plan(&jar, &plan, &output).unwrap_err();

        assert!(matches!(
            err,
            ApplyError::MissingNestedTarget {
                outer_jar,
                inner_path
            } if outer_jar == "BOOT-INF/lib/order.jar" && inner_path == "com/acme/Missing.class"
        ));
        assert!(!output.exists());
    }

    #[test]
    fn inspect_reports_non_spring_jar_as_success() {
        let jar = non_spring_fixture();
        let report = inspect_jar(&jar).unwrap();

        assert!(!report.has_boot_inf_classes);
        assert!(!report.has_boot_inf_lib);
        assert_eq!(report.layout, ArchiveLayout::Unknown);
        assert!(!report.has_boot_loader_entry);
        assert!(report.nested_jars.is_empty());
    }

    #[test]
    fn invalid_jars_fail_inspect() {
        let jar = invalid_jar_fixture();
        let err = inspect_jar(&jar).unwrap_err();
        assert!(matches!(err, JarInspectError::InvalidJar(_)));
    }
}
