use std::path::Path;
use std::path::PathBuf;
use std::process;

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        Some("inspect") => {
            let path = args.next();
            if path.is_none() {
                eprintln!("Usage: bootjar-patcher inspect <jar>");
                process::exit(2);
            }
            let path = Path::new(path.as_ref().unwrap());

            match bootjar_core::inspect_jar(path) {
                Ok(report) => {
                    print_inspect_report(&report);
                }
                Err(err) => {
                    eprintln!("inspect failed: {err}");
                    process::exit(1);
                }
            }
        }
        Some("find") => {
            let path = args.next();
            let query = args.next();
            if path.is_none() || query.is_none() {
                eprintln!("Usage: bootjar-patcher find <jar> <query>");
                process::exit(2);
            }
            let path = Path::new(path.as_ref().unwrap());
            let query = query.as_ref().unwrap();

            match bootjar_core::find_in_jar(path, query) {
                Ok(results) => {
                    for result in results {
                        println!("{}", result.archive_path);
                    }
                }
                Err(err) => {
                    eprintln!("find failed: {err}");
                    process::exit(1);
                }
            }
        }
        Some("match") => {
            let options = parse_match_options(args.collect());
            let options = match options {
                Ok(options) => options,
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!(
                        "Usage: bootjar-patcher match --jar <jar> --inputs <path> [--out <file>]"
                    );
                    process::exit(2);
                }
            };

            match bootjar_core::match_in_jar(&options.jar, &options.inputs) {
                Ok(candidates) => {
                    let output = match options.format {
                        MatchFormat::Candidates => candidates.to_yaml(),
                        MatchFormat::Snippets => candidates.to_snippets(),
                    };
                    if let Some(out) = options.out {
                        if let Err(err) = std::fs::write(&out, output) {
                            eprintln!("match failed: could not write {}: {err}", out.display());
                            process::exit(1);
                        }
                    } else {
                        print!("{output}");
                    }
                }
                Err(err) => {
                    eprintln!("match failed: {err}");
                    process::exit(1);
                }
            }
        }
        Some("apply") => {
            let options = parse_apply_options(args.collect());
            let options = match options {
                Ok(options) => options,
                Err(message) => {
                    eprintln!("{message}");
                    eprintln!("Usage: bootjar-patcher apply --jar <jar> --plan <plan> --out <jar>");
                    process::exit(2);
                }
            };

            match bootjar_core::apply_patch_plan(&options.jar, &options.plan, &options.out) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("apply failed: {err}");
                    process::exit(1);
                }
            }
        }
        Some("help") | Some("-h") | Some("--help") | None => {
            print_usage();
            if command.is_none() {
                process::exit(2);
            }
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            print_usage();
            process::exit(2);
        }
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  bootjar-patcher inspect <jar>");
    eprintln!("  bootjar-patcher find <jar> <query>");
    eprintln!(
        "  bootjar-patcher match --jar <jar> --inputs <path> [--format candidates|snippets] [--out <file>]"
    );
    eprintln!("  bootjar-patcher apply --jar <jar> --plan <plan> --out <jar>");
}

#[derive(Debug, PartialEq, Eq)]
struct MatchOptions {
    jar: PathBuf,
    inputs: Vec<PathBuf>,
    format: MatchFormat,
    out: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchFormat {
    Candidates,
    Snippets,
}

fn parse_match_options(args: Vec<String>) -> Result<MatchOptions, String> {
    let mut jar = None;
    let mut inputs = Vec::new();
    let mut format = MatchFormat::Candidates;
    let mut out = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--jar" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--jar requires a value".to_string())?;
                jar = Some(PathBuf::from(value));
            }
            "--inputs" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--inputs requires a value".to_string())?;
                inputs.push(PathBuf::from(value));
            }
            "--out" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--out requires a value".to_string())?;
                out = Some(PathBuf::from(value));
            }
            "--format" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--format requires a value".to_string())?;
                format = match value.as_str() {
                    "candidates" => MatchFormat::Candidates,
                    "snippets" => MatchFormat::Snippets,
                    other => return Err(format!("unknown match format: {other}")),
                };
            }
            unknown => return Err(format!("unknown match option: {unknown}")),
        }
        index += 1;
    }

    let jar = jar.ok_or_else(|| "match requires --jar".to_string())?;
    if inputs.is_empty() {
        return Err("match requires at least one --inputs path".to_string());
    }

    Ok(MatchOptions {
        jar,
        inputs,
        format,
        out,
    })
}

#[derive(Debug, PartialEq, Eq)]
struct ApplyOptions {
    jar: PathBuf,
    plan: PathBuf,
    out: PathBuf,
}

fn parse_apply_options(args: Vec<String>) -> Result<ApplyOptions, String> {
    let mut jar = None;
    let mut plan = None;
    let mut out = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--jar" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--jar requires a value".to_string())?;
                jar = Some(PathBuf::from(value));
            }
            "--plan" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--plan requires a value".to_string())?;
                plan = Some(PathBuf::from(value));
            }
            "--out" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--out requires a value".to_string())?;
                out = Some(PathBuf::from(value));
            }
            unknown => return Err(format!("unknown apply option: {unknown}")),
        }
        index += 1;
    }

    Ok(ApplyOptions {
        jar: jar.ok_or_else(|| "apply requires --jar".to_string())?,
        plan: plan.ok_or_else(|| "apply requires --plan".to_string())?,
        out: out.ok_or_else(|| "apply requires --out".to_string())?,
    })
}

fn print_inspect_report(report: &bootjar_core::InspectReport) {
    println!("Jar: {}", report.jar_path);
    println!(
        "BOOT-INF/classes: {}",
        format_flag(report.has_boot_inf_classes)
    );
    println!("BOOT-INF/lib: {}", format_flag(report.has_boot_inf_lib));
    println!(
        "Spring Boot launcher entries: {}",
        format_flag(report.has_boot_loader_entry)
    );
    println!("Nested jars:");
    if report.nested_jars.is_empty() {
        println!("  (none)");
        return;
    }

    for nested in &report.nested_jars {
        let status = if nested.is_stored {
            "STORED"
        } else {
            "compressed"
        };
        println!(
            "  {} -> {} ({})",
            nested.path, status, nested.compression_method
        );
    }
}

fn format_flag(value: bool) -> &'static str {
    if value {
        "present"
    } else {
        "absent"
    }
}
