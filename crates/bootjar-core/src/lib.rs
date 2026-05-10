//! Core library for `bootjar-patcher`.
//! Provides archive path parsing and jar inspection primitives.

use std::fmt;
use std::fs::File;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use zip::result::ZipError;
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

fn normalize_entry_name(name: &str) -> String {
    name.replace('\\', "/")
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
