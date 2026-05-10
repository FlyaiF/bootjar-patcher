//! Core library for `bootjar-patcher`.
//! Provides archive path parsing and jar inspection primitives.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::{self, Cursor, Read, Write};
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
            Self::EmptyOuterPath => write!(f, "outer jar path is empty"),
            Self::EmptyInnerPath => write!(f, "nested inner path is empty"),
            Self::MultipleNestedSeparators => {
                write!(f, "archive path contains multiple `!` separators")
            }
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
    /// Parse an archive path with optional nested syntax: `<outer>!/<inner>`.
    ///
    /// For this first slice, both outer and inner paths are normalized to
    /// jar-style `/`, and unsafe path forms are rejected up front.
    pub fn parse(input: &str) -> Result<Self, ArchivePathParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ArchivePathParseError::EmptyInput);
        }

        let separator_parts: Vec<&str> = input.split('!').collect();
        if separator_parts.len() > 2 {
            return Err(ArchivePathParseError::MultipleNestedSeparators);
        }

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
            _ => Err(ArchivePathParseError::MultipleNestedSeparators),
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
    UnsupportedNestedTarget(String),
    MissingReplacementSource(PathBuf),
    MissingTarget(String),
    DuplicateTarget(String),
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
            Self::UnsupportedNestedTarget(target) => {
                write!(f, "nested replace target is not supported yet: {target}")
            }
            Self::MissingReplacementSource(path) => {
                write!(
                    f,
                    "replacement source file could not be read: {}",
                    path.display()
                )
            }
            Self::MissingTarget(target) => {
                write!(f, "replace target does not exist in input jar: {target}")
            }
            Self::DuplicateTarget(target) => {
                write!(f, "duplicate replace target in patch plan: {target}")
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JarIndex {
    pub entries: Vec<JarEntry>,
    pub has_boot_inf_classes: bool,
    pub has_boot_inf_lib: bool,
    pub has_boot_loader_entry: bool,
    pub nested_jars: Vec<NestedJarInfo>,
    pub nested_entries: Vec<NestedJarEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectReport {
    pub jar_path: String,
    pub has_boot_inf_classes: bool,
    pub has_boot_inf_lib: bool,
    pub has_boot_loader_entry: bool,
    pub nested_jars: Vec<NestedJarInfo>,
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

pub fn build_jar_index(path: impl Into<PathBuf>) -> Result<JarIndex, JarInspectError> {
    let path = path.into();
    let file = File::open(&path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut entries = Vec::with_capacity(archive.len());
    let mut has_boot_inf_classes = false;
    let mut has_boot_inf_lib = false;
    let mut has_boot_loader_entry = false;
    let mut nested_jars = Vec::new();
    let mut nested_entries = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let path = normalize_entry_name(entry.name());
        let is_dir = entry.is_dir();

        let compression = entry.compression();
        let info = JarEntry {
            path: path.to_string(),
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
        if is_boot_loader_entry(&path) {
            has_boot_loader_entry = true;
        }
        if let Some(nested_name) = nested_jar_entry(&path) {
            let nested_name = nested_name.to_string();
            nested_jars.push(NestedJarInfo {
                path: nested_name.clone(),
                compression_method: compression.to_string(),
                is_stored: compression == CompressionMethod::Stored,
            });
            index_nested_jar_entries(&mut entry, &nested_name, &mut nested_entries);
        }
    }

    Ok(JarIndex {
        entries,
        has_boot_inf_classes,
        has_boot_inf_lib,
        has_boot_loader_entry,
        nested_jars,
        nested_entries,
    })
}

impl JarIndex {
    pub fn inspect_report(&self, path: &Path) -> InspectReport {
        InspectReport {
            jar_path: path.display().to_string(),
            has_boot_inf_classes: self.has_boot_inf_classes,
            has_boot_inf_lib: self.has_boot_inf_lib,
            has_boot_loader_entry: self.has_boot_loader_entry,
            nested_jars: self.nested_jars.clone(),
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
        let archive_path = ArchivePath::parse(&replace_entry.target).map_err(|source| {
            ApplyError::InvalidTarget {
                target: replace_entry.target.clone(),
                source,
            }
        })?;
        if matches!(archive_path, ArchivePath::Nested { .. }) {
            return Err(ApplyError::UnsupportedNestedTarget(replace_entry.target));
        }
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
    let replacements = read_replacements(&plan)?;
    rewrite_outer_jar(input_jar, output_jar, replacements)
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

fn read_replacements(plan: &PatchPlan) -> Result<BTreeMap<String, Vec<u8>>, ApplyError> {
    let mut replacements = BTreeMap::new();
    for operation in &plan.operations {
        let bytes = std::fs::read(&operation.source)
            .map_err(|_| ApplyError::MissingReplacementSource(operation.source.clone()))?;
        replacements.insert(operation.target.clone(), bytes);
    }
    Ok(replacements)
}

fn rewrite_outer_jar(
    input_jar: &Path,
    output_jar: &Path,
    mut replacements: BTreeMap<String, Vec<u8>>,
) -> Result<(), ApplyError> {
    let input = File::open(input_jar).map_err(|source| ApplyError::Io {
        path: input_jar.to_path_buf(),
        action: "read input jar",
        source,
    })?;
    let mut archive = zip::ZipArchive::new(input)?;
    let mut existing_paths = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if !entry.is_dir() {
            existing_paths.insert(normalize_entry_name(entry.name()));
        }
    }
    for target in replacements.keys() {
        if !existing_paths.contains(target) {
            return Err(ApplyError::MissingTarget(target.clone()));
        }
    }

    let output = File::create(output_jar).map_err(|source| ApplyError::Io {
        path: output_jar.to_path_buf(),
        action: "write output jar",
        source,
    })?;
    let mut writer = zip::ZipWriter::new(output);

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

        writer.start_file(&path, options)?;
        if let Some(bytes) = replacements.remove(&path) {
            writer.write_all(&bytes).map_err(|source| ApplyError::Io {
                path: output_jar.to_path_buf(),
                action: "write output jar",
                source,
            })?;
        } else {
            io::copy(&mut entry, &mut writer).map_err(|source| ApplyError::Io {
                path: output_jar.to_path_buf(),
                action: "write output jar",
                source,
            })?;
        }
    }

    writer.finish()?;
    Ok(())
}

fn is_boot_inf_classes_entry(path: &str) -> bool {
    path == "BOOT-INF/classes"
        || path == "BOOT-INF/classes/"
        || path.starts_with("BOOT-INF/classes/")
}

fn is_boot_inf_lib_entry(path: &str) -> bool {
    path == "BOOT-INF/lib" || path == "BOOT-INF/lib/" || path.starts_with("BOOT-INF/lib/")
}

fn is_boot_loader_entry(path: &str) -> bool {
    path.ends_with(".class")
        && path.starts_with("org/springframework/boot/loader/")
        && (path.contains("Launcher.class") || path.contains("PropertiesLauncher.class"))
}

fn nested_jar_entry(path: &str) -> Option<&str> {
    let lib_prefix = "BOOT-INF/lib/";
    if !path.starts_with(lib_prefix) {
        return None;
    }

    let rest = &path[lib_prefix.len()..];
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
        let dir = tempdir().unwrap();
        let path = dir.path().join("fixture.jar");
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        for (name, method, bytes) in entries {
            let options = FileOptions::default().compression_method(*method);
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
    fn apply_rejects_nested_targets_for_this_slice() {
        let dir = tempdir().unwrap();
        let replacement = dir.path().join("OrderService.class");
        write_input_file(&replacement, b"class-bytes");
        let err = parse_patch_plan(&format!(
            r#"
kind: patch-plan
version: 1
operations:
  - replace-entry:
      target: BOOT-INF/lib/order.jar!/com/acme/OrderService.class
      with: "{}"
"#,
            replacement.display()
        ))
        .unwrap_err();

        assert!(matches!(
            err,
            ApplyError::UnsupportedNestedTarget(target)
                if target == "BOOT-INF/lib/order.jar!/com/acme/OrderService.class"
        ));
    }

    #[test]
    fn inspect_reports_non_spring_jar_as_success() {
        let jar = non_spring_fixture();
        let report = inspect_jar(&jar).unwrap();

        assert!(!report.has_boot_inf_classes);
        assert!(!report.has_boot_inf_lib);
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
