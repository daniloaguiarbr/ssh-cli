//! Testes de integração do módulo SCP.
//!
//! Testa o subcomando `scp` via CLI, validando help e parâmetros obrigatórios.

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use tempfile::TempDir;

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
fn scp_help_exibe_usage() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["scp", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scp"))
        .stdout(predicate::str::contains("upload"))
        .stdout(predicate::str::contains("download"));
}

#[test]
#[serial]
fn scp_upload_help_exibe_parametros() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["scp", "upload", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("VPS_NOME"))
        .stdout(predicate::str::contains("LOCAL"))
        .stdout(predicate::str::contains("REMOTE"));
}

#[test]
#[serial]
fn scp_download_help_exibe_parametros() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["scp", "download", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("VPS_NOME"))
        .stdout(predicate::str::contains("REMOTE"))
        .stdout(predicate::str::contains("LOCAL"));
}

#[test]
#[serial]
fn scp_upload_sem_parametros_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["scp", "upload"]).assert().failure();
}

#[test]
#[serial]
fn scp_download_sem_parametros_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["scp", "download"]).assert().failure();
}

#[test]
#[serial]
fn scp_upload_com_vps_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "scp",
            "upload",
            "fantasma-scp",
            "/tmp/arquivo_local.txt",
            "/tmp/arquivo_remoto.txt",
        ])
        .assert()
        .failure();
}

#[test]
#[serial]
fn scp_download_com_vps_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "scp",
            "download",
            "fantasma-scp",
            "/tmp/arquivo_remoto.txt",
            "/tmp/arquivo_local.txt",
        ])
        .assert()
        .failure();
}

#[test]
#[serial]
fn scp_subcomando_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["scp", "comando-inexistente"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn scp_sem_subcomando_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["scp"]).assert().failure();
}
