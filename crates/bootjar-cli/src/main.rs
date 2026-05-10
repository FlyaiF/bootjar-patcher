use std::path::Path;
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
    eprintln!("Usage: bootjar-patcher inspect <jar>");
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
