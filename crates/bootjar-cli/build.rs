use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");

    set_env(
        "BUILD_GIT_COMMIT",
        git(&["rev-parse", "--short=12", "HEAD"]),
    );
    set_env(
        "BUILD_GIT_BRANCH",
        git(&["rev-parse", "--abbrev-ref", "HEAD"]),
    );
    set_env("BUILD_GIT_TAGS", git(&["tag", "--points-at", "HEAD"]));
    set_env("BUILD_GIT_DIRTY", git_dirty());
    set_env("BUILD_TARGET", std::env::var("TARGET").ok());
    set_env("BUILD_PROFILE", std::env::var("PROFILE").ok());
    set_env(
        "BUILD_RUSTC_VERSION",
        command_output("rustc", &["--version"]),
    );
}

fn set_env(key: &str, value: Option<String>) {
    let value = value
        .map(|value| {
            let value = value.trim();
            if value.is_empty() {
                "none".to_string()
            } else {
                value.replace('\n', ",")
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env={key}={value}");
}

fn git(args: &[&str]) -> Option<String> {
    command_output("git", args)
}

fn git_dirty() -> Option<String> {
    let output = command_output("git", &["status", "--porcelain"])?;
    if output.trim().is_empty() {
        Some("false".to_string())
    } else {
        Some("true".to_string())
    }
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
