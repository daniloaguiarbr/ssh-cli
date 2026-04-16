//! Snapshot tests para outputs estáveis do ssh-cli.
//!
//! Usa `insta` para capturar e verificar outputs do binário.
//! Na primeira execução os snapshots são criados como `.snap.new`.
//! Para aceitar: execute `cargo insta review` ou `cargo insta test --accept`.

use assert_cmd::Command;
use serial_test::serial;
use tempfile::TempDir;

/// Helper para criar Command com isolamento completo.
fn cmd(tmp: &TempDir) -> Command {
    let llvm_profile_file = std::env::var_os("LLVM_PROFILE_FILE");
    let mut c = Command::new(env!("CARGO_BIN_EXE_ssh-cli"));
    c.env_clear();
    c.env("PATH", std::env::var_os("PATH").unwrap_or_default());
    if let Some(valor) = llvm_profile_file {
        c.env("LLVM_PROFILE_FILE", valor);
    }
    c.env("HOME", tmp.path());
    c.env("XDG_CONFIG_HOME", tmp.path());
    c.arg("--config-dir").arg(tmp.path());
    c
}

#[test]
#[serial]
fn snapshot_help_output() {
    let tmp = TempDir::new().unwrap();
    let output = cmd(&tmp).arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("help_output", stdout);
}

#[test]
#[serial]
fn snapshot_vps_list_empty() {
    let tmp = TempDir::new().unwrap();
    let output = cmd(&tmp).args(["vps", "list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("vps_list_empty", stdout);
}

#[test]
#[serial]
fn snapshot_vps_path_format() {
    let tmp = TempDir::new().unwrap();
    let output = cmd(&tmp).args(["vps", "path"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    // Substitui o caminho temporário variável por um placeholder estável.
    let redacted = stdout.replace(tmp.path().to_str().unwrap_or(""), "[CONFIG_DIR]");
    insta::assert_snapshot!("vps_path_format", redacted);
}

#[test]
#[serial]
fn snapshot_version_format() {
    let tmp = TempDir::new().unwrap();
    let output = cmd(&tmp).arg("--version").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("version_format", stdout);
}

#[test]
#[serial]
fn snapshot_completions_bash_header() {
    let tmp = TempDir::new().unwrap();
    let output = cmd(&tmp).args(["completions", "bash"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    // Captura apenas as primeiras 10 linhas para estabilidade.
    let header: String = stdout.lines().take(10).collect::<Vec<_>>().join("\n");
    insta::assert_snapshot!("completions_bash_header", header);
}

#[test]
#[serial]
fn snapshot_error_vps_not_found() {
    let tmp = TempDir::new().unwrap();
    let output = cmd(&tmp)
        .args(["vps", "show", "nao-existe"])
        .output()
        .unwrap();
    // O erro vai para stdout neste CLI (não para stderr).
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let saida = if stdout.is_empty() { stderr } else { stdout };
    insta::assert_snapshot!("error_vps_not_found", saida);
}
