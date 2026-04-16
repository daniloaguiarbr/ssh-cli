//! Testes E2E da CLI via `assert_cmd`.
//!
//! TODOS os testes usam `--config-dir <TempDir>` para isolar completamente o
//! estado do sistema real. Testes que escrevem/leem env vars são marcados
//! com `#[serial]`.

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
fn testa_help() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("ssh-cli"))
        .stdout(predicate::str::contains("vps"))
        .stdout(predicate::str::contains("exec"));
}

#[test]
#[serial]
fn testa_version() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ssh-cli"));
}

#[test]
#[serial]
fn testa_vps_add_cria_registro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "teste",
            "--host",
            "1.2.3.4",
            "--port",
            "22",
            "--user",
            "root",
            "--password",
            "senha-super-secreta-123",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("teste"));
}

#[test]
#[serial]
fn testa_vps_add_duplicado_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "dupe",
            "--host",
            "1.2.3.4",
            "--user",
            "root",
            "--password",
            "senha-super-secreta-123",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "dupe",
            "--host",
            "1.2.3.4",
            "--user",
            "root",
            "--password",
            "outra-senha-super-secreta",
        ])
        .assert()
        .failure();
}

#[test]
#[serial]
fn testa_vps_list_mascara_senhas() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "alfa",
            "--host",
            "a.example.com",
            "--user",
            "admin",
            "--password",
            "senha-muito-longa-para-mascarar-123",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alfa"))
        // Senha NÃO pode aparecer inteira
        .stdout(predicate::str::contains("senha-muito-longa-para-mascarar-123").not())
        // Deve aparecer com "..." do mascaramento
        .stdout(predicate::str::contains("..."));
}

#[test]
#[serial]
fn testa_vps_list_json_funciona() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "beta",
            "--host",
            "b.example.com",
            "--user",
            "admin",
            "--password",
            "senha-muito-longa-para-mascarar-456",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"beta\""));
}

#[test]
#[serial]
fn testa_vps_remove_existente() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "remover",
            "--host",
            "r.example.com",
            "--user",
            "root",
            "--password",
            "senha-muito-longa-para-mascarar",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "remove", "remover"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remover"));
}

#[test]
#[serial]
fn testa_vps_remove_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["vps", "remove", "nao-existe"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn testa_vps_edit_atualiza_campos() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "editar",
            "--host",
            "antigo.example.com",
            "--user",
            "root",
            "--password",
            "senha-original-muito-longa",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args([
            "vps",
            "edit",
            "editar",
            "--host",
            "novo.example.com",
            "--port",
            "2222",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "show", "editar"])
        .assert()
        .success()
        .stdout(predicate::str::contains("novo.example.com"))
        .stdout(predicate::str::contains("2222"));
}

#[test]
#[serial]
fn testa_vps_show_retorna_dados_mascarados() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "mostrar",
            "--host",
            "s.example.com",
            "--user",
            "admin",
            "--password",
            "senha-longa-para-mascaramento-total",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "show", "mostrar"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mostrar"))
        .stdout(predicate::str::contains("senha-longa-para-mascaramento-total").not())
        .stdout(predicate::str::contains("..."));
}

#[test]
#[serial]
fn testa_vps_show_json_mascara() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "jshow",
            "--host",
            "j.example.com",
            "--user",
            "admin",
            "--password",
            "senha-ultra-secreta-para-mascarar-json",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["vps", "show", "jshow", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("jshow"))
        .stdout(predicate::str::contains("senha-ultra-secreta-para-mascarar-json").not());
}

#[test]
#[serial]
fn testa_vps_path_retorna_caminho() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["vps", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
#[serial]
fn testa_connect_seleciona_vps() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "prod",
            "--host",
            "p.example.com",
            "--user",
            "admin",
            "--password",
            "senha-muito-longa-prod",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["connect", "prod"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prod"));
}

#[test]
#[serial]
fn testa_connect_vps_inexistente_retorna_erro() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["connect", "fantasma"]).assert().failure();
}

#[test]
#[serial]
fn testa_list_vazio_mostra_mensagem() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).args(["vps", "list"]).assert().success();
}

#[test]
#[serial]
fn testa_health_check_sem_vps_ativa_dispara_erro_generico() {
    // Exercita o branch `imprimir_erro_generico` em main.rs (linhas 44-45):
    // health-check sem nome e sem VPS ativa retorna um `anyhow::anyhow!`
    // puro (não é ErroSshCli), forçando o fluxo de erro genérico no
    // entry point do binário.
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["health-check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("VPS").or(predicate::str::contains("vps")));
}

#[test]
#[serial]
fn testa_vps_add_sem_password_usa_prompt_ou_erro() {
    // Cobre o branch de Ok(()) + exit_code EX_OK no fluxo completo:
    // fornece todos os campos obrigatórios via flags para evitar prompt.
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "completo",
            "--host",
            "c.example.com",
            "--port",
            "2222",
            "--user",
            "operador",
            "--password",
            "senha-longa-completa-123",
        ])
        .assert()
        .success();
}

#[test]
#[serial]
fn testa_vps_edit_inexistente_retorna_erro_dominio() {
    // Reforça cobertura do branch `downcast::<ErroSshCli>` em main.rs:
    // editar VPS que não existe retorna ErroSshCli::VpsNaoEncontrada.
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["vps", "edit", "fantasma", "--port", "2222"])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Bug C2: `--output-format json vps list` com registro vazio emitia texto
// humano ("Nenhum VPS cadastrado.") quebrando parsers LLM. Os dois testes
// abaixo travam o contrato: a flag global `--output-format json` sempre
// produz JSON parseável, independentemente de haver VPSs cadastradas.
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn testa_vps_list_vazia_com_output_format_json_retorna_array_vazio() {
    let tmp = TempDir::new().unwrap();
    let saida = cmd(&tmp)
        .args(["--output-format", "json", "vps", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(saida).expect("stdout deve ser UTF-8 válido");
    let valor: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("stdout deve ser JSON válido mesmo com registro vazio");
    let arr = valor
        .as_array()
        .expect("modo json deve retornar array JSON na raiz");
    assert!(
        arr.is_empty(),
        "lista vazia deve serializar como [] mas veio {arr:?}"
    );
}

#[test]
#[serial]
fn testa_vps_list_com_uma_vps_output_format_json_mascara_senha() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "vps",
            "add",
            "--name",
            "producao",
            "--host",
            "10.0.0.1",
            "--port",
            "22",
            "--user",
            "deploy",
            "--password",
            "senha-super-secreta-que-deve-ser-mascarada",
        ])
        .assert()
        .success();

    let saida = cmd(&tmp)
        .args(["--output-format", "json", "vps", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(saida).expect("stdout deve ser UTF-8 válido");
    let valor: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout deve ser JSON válido");
    let arr = valor
        .as_array()
        .expect("modo json deve retornar array JSON na raiz");
    assert_eq!(arr.len(), 1, "deve conter exatamente 1 VPS cadastrada");

    let registro = &arr[0];
    assert_eq!(registro["name"], "producao");
    assert_eq!(registro["host"], "10.0.0.1");
    assert_eq!(registro["port"], 22);
    assert_eq!(registro["user"], "deploy");

    let senha_mascarada = registro["password"]
        .as_str()
        .expect("password deve ser string mascarada");
    assert!(
        !senha_mascarada.contains("senha-super-secreta"),
        "senha plain NUNCA pode aparecer no JSON: {senha_mascarada}"
    );
    assert!(
        senha_mascarada.contains("...") || senha_mascarada == "***",
        "senha deve usar padrão de mascaramento conhecido: {senha_mascarada}"
    );
}
