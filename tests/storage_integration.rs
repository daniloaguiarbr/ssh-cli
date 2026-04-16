//! Testes de integração do CRUD de VPS via CLI.
//!
//! Testa as operações de carregar, salvar, buscar, adicionar e remover
//! registros de VPS usando `--config-dir` com TempDir para isolamento.

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
fn carregar_retorna_vazio_quando_nao_existe() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["vps", "list"]).assert().success();
}

#[test]
#[serial]
fn salvar_cria_arquivo_config() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "teste-salvar",
            "--host",
            "1.2.3.4",
            "--user",
            "root",
            "--password",
            "senha-longa-para-testar-save",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("teste-salvar"));

    let config_path = tmp.path().join("config.toml");
    assert!(config_path.exists(), "arquivo config.toml deve existir");
}

#[test]
#[serial]
fn salvar_define_schema_version() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "schema-test",
            "--host",
            "5.6.7.8",
            "--user",
            "admin",
            "--password",
            "senha-longa-schema-test",
        ])
        .assert()
        .success();

    let config_path = tmp.path().join("config.toml");
    let conteudo = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        conteudo.contains("schema_version"),
        "deve conter schema_version"
    );
}

#[test]
#[serial]
fn buscar_por_nome_encontra_vps() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "buscar-teste",
            "--host",
            "9.9.9.9",
            "--user",
            "root",
            "--password",
            "senha-longa-para-busca",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "show", "buscar-teste"])
        .assert()
        .success()
        .stdout(predicate::str::contains("buscar-teste"))
        .stdout(predicate::str::contains("9.9.9.9"));
}

#[test]
#[serial]
fn buscar_por_nome_nao_encontra_inexistente() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["vps", "show", "fantasma-xyz"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn adicionar_duplicado_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "duplicado",
            "--host",
            "1.1.1.1",
            "--user",
            "root",
            "--password",
            "senha-primeira",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "duplicado",
            "--host",
            "2.2.2.2",
            "--user",
            "admin",
            "--password",
            "senha-segunda",
        ])
        .assert()
        .failure();
}

#[test]
#[serial]
fn remover_vps_existente() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "remover-teste",
            "--host",
            "4.5.6.7",
            "--user",
            "root",
            "--password",
            "senha-para-remover",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "remove", "remover-teste", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removida"));

    cmd(&tmp)
        .args(["vps", "show", "remover-teste"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn remover_vps_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["vps", "remove", "nao-existe-123", "--yes"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn list_vazio_retorna_sucesso_sem_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["vps", "list"]).assert().success();
}

#[test]
#[serial]
fn list_com_vps_retorna_todos() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "vps-um",
            "--host",
            "1.0.0.1",
            "--user",
            "root",
            "--password",
            "senha-vps-um-longa",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "vps-dois",
            "--host",
            "2.0.0.2",
            "--user",
            "admin",
            "--password",
            "senha-vps-dois-longa",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("vps-um"))
        .stdout(predicate::str::contains("vps-dois"));
}

#[test]
#[serial]
fn editar_atualiza_campos() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "editar-teste",
            "--host",
            "antigo-host.example.com",
            "--port",
            "22",
            "--user",
            "root",
            "--password",
            "senha-original-longa",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args([
            "vps",
            "edit",
            "editar-teste",
            "--host",
            "novo-host.example.com",
            "--port",
            "2222",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("editada"));

    cmd(&tmp)
        .args(["vps", "show", "editar-teste"])
        .assert()
        .success()
        .stdout(predicate::str::contains("novo-host.example.com"))
        .stdout(predicate::str::contains("2222"));
}

#[test]
#[serial]
fn path_retorna_caminho_config() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["vps", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
#[serial]
fn add_com_porta_personalizada() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "porta-custom",
            "--host",
            "custom.example.com",
            "--port",
            "2222",
            "--user",
            "admin",
            "--password",
            "senha-porta-custom-longa",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "show", "porta-custom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2222"));
}
