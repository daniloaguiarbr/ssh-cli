//! Testes de integração do módulo tunnel.
//!
//! Testa o subcomando `tunnel` via CLI, validando help e parâmetros obrigatórios.

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
fn tunnel_help_exibe_usage() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["tunnel", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tunnel"))
        .stdout(predicate::str::contains("VPS_NOME"))
        .stdout(predicate::str::contains("PORTA_LOCAL"));
}

#[test]
#[serial]
fn tunnel_sem_parametros_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["tunnel"]).assert().failure();
}

#[test]
#[serial]
fn tunnel_com_apenas_vps_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["tunnel", "minha-vps"]).assert().failure();
}

#[test]
#[serial]
fn tunnel_com_parametros_invalidos_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["tunnel", "vps-teste", "abc", "host-remoto", "8080"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn tunnel_com_vps_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["tunnel", "fantasma-tunnel", "8080", "localhost", "3000"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn tunnel_com_porta_local_fora_range_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["tunnel", "vps-inexistente", "999999", "localhost", "8080"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn tunnel_comando_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["tunnel", "comando-inexistente"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn tunnel_help_exibe_descricao_port_forward() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["tunnel", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("forward"))
        .stdout(predicate::str::contains("SSH"));
}
