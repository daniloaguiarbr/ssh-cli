//! Build script para ssh-cli.
//!
//! Embute o commit hash do git na variável de ambiente SSH_CLI_COMMIT_HASH.

fn main() {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let hash = match output {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout)
            .unwrap_or_default()
            .trim()
            .to_string(),
        _ => "unknown".to_string(),
    };

    println!("cargo:rustc-env=SSH_CLI_COMMIT_HASH={hash}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
}
