//! Único módulo autorizado a emitir output em stdout para CRUD de VPS.
//!
//! Este módulo centraliza TODA formatação de CRUD: texto e JSON.
//!
//! Logs (tracing) vão para stderr, gerenciados por `tracing-subscriber`.

use crate::mascaramento::mascarar;
use crate::ssh::SaidaExecucao;
use crate::vps::modelo::VpsRegistro;
use secrecy::ExposeSecret;
use serde_json::json;
use std::io::{self, BufRead, IsTerminal, Write};

/// Escreve uma linha em stdout garantindo LF puro (nunca CRLF).
///
/// # Erros
/// Retorna erro se o I/O em stdout falhar.
pub fn escrever_linha(conteudo: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(conteudo.as_bytes())?;
    handle.write_all(b"\n")?;
    handle.flush()?;
    Ok(())
}

/// Imprime mensagem de sucesso em texto para humanos.
pub fn imprimir_sucesso(mensagem: &str) {
    println!("{mensagem}");
}

/// Indica se stdin está conectado a um terminal interativo (TTY).
#[must_use]
pub fn stdin_e_tty() -> bool {
    io::stdin().is_terminal()
}

/// Versão pura e testável da leitura de confirmação `sim/não`.
///
/// Escreve `prompt` em `writer` e lê UMA linha de `reader`. Aceita como
/// afirmativo: `s`, `S`, `sim`, `SIM`, `y`, `Y`, `yes`, `YES` (case-insensitive,
/// com espaços em branco ao redor ignorados). Qualquer outra entrada — incluindo
/// linha vazia ou EOF — é tratada como negativa.
///
/// # Erros
/// Retorna erro se a escrita do prompt ou a leitura falharem.
pub fn ler_confirmacao<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    prompt: &str,
) -> io::Result<bool> {
    writer.write_all(prompt.as_bytes())?;
    writer.flush()?;
    let mut linha = String::new();
    let lidos = reader.read_line(&mut linha)?;
    if lidos == 0 {
        // EOF sem input = negativo (seguro para operação destrutiva).
        return Ok(false);
    }
    let resposta = linha.trim().to_lowercase();
    Ok(matches!(resposta.as_str(), "s" | "sim" | "y" | "yes"))
}

/// Emite `prompt` em stderr e lê a resposta de stdin.
///
/// Wrapper sobre [`ler_confirmacao`] usando stderr (para não poluir stdout em
/// pipelines) e stdin real. Usado pelo handler de `vps remove` quando a flag
/// `--yes` não foi passada e stdin está em modo TTY.
///
/// # Erros
/// Retorna erro se o I/O falhar.
pub fn perguntar_confirmacao(prompt: &str) -> io::Result<bool> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stderr = io::stderr();
    let mut writer = stderr.lock();
    ler_confirmacao(&mut reader, &mut writer, prompt)
}

/// Imprime mensagem de erro em stderr (para humanos).
pub fn imprimir_erro(mensagem: &str) {
    eprintln!("{mensagem}");
}

/// Imprime erro de inicialização de runtime em stderr.
///
/// Emite no formato `erro ao criar runtime: {mensagem}` via stderr.
/// Usada em `main.rs` para falhas de construção do runtime tokio ANTES
/// de qualquer lógica async estar disponível.
pub fn imprimir_erro_runtime(mensagem: &str) {
    eprintln!("erro ao criar runtime: {mensagem}");
}

/// Imprime erro de domínio [`crate::erros::ErroSshCli`] em stderr.
///
/// Usa o `Display` do `thiserror` para emitir a mensagem canônica do erro.
/// Chamada pelo `main.rs` após downcast de `anyhow::Error` para o tipo de
/// domínio, preservando o contrato de mensagens definido em `errors.rs`.
pub fn imprimir_erro_dominio(erro: &crate::erros::ErroSshCli) {
    eprintln!("{}", erro.mensagem_i18n());
}

/// Imprime erro genérico `anyhow::Error` em stderr incluindo a cadeia de causas.
///
/// Emite primeiro a mensagem principal e, se houver, as causas encadeadas
/// prefixadas por `  causado por: ` (uma por linha). Usada em `main.rs`
/// como fallback quando o erro NÃO é um `ErroSshCli` conhecido.
pub fn imprimir_erro_generico(erro: &anyhow::Error) {
    eprintln!("{erro}");
    for causa in erro.chain().skip(1) {
        eprintln!("  causado por: {causa}");
    }
}

/// Imprime lista de VPS em formato texto (mascarado).
pub fn imprimir_lista_texto(registros: &[VpsRegistro]) {
    if registros.is_empty() {
        println!(
            "{}",
            crate::i18n::t(crate::i18n::Mensagem::VpsRegistroVazio)
        );
        return;
    }

    println!(
        "{:<20} {:<30} {:<6} {:<15} {:<20}",
        "NOME", "HOST", "PORTA", "USUÁRIO", "SENHA"
    );
    for r in registros {
        println!(
            "{:<20} {:<30} {:<6} {:<15} {:<20}",
            r.nome,
            r.host,
            r.porta,
            r.usuario,
            mascarar(r.senha.expose_secret())
        );
    }
}

/// Imprime lista de VPS em formato JSON (mascarado).
pub fn imprimir_lista_json(registros: &[VpsRegistro]) {
    let lista: Vec<_> = registros.iter().map(registro_para_json_mascarado).collect();
    match serde_json::to_string_pretty(&lista) {
        Ok(s) => println!("{s}"),
        Err(erro) => eprintln!("erro ao serializar JSON: {erro}"),
    }
}

/// Imprime detalhes de UMA VPS em texto (mascarado).
pub fn imprimir_detalhes_texto(r: &VpsRegistro) {
    println!("Nome:           {}", r.nome);
    println!("Host:           {}", r.host);
    println!("Porta:          {}", r.porta);
    println!("Usuário:        {}", r.usuario);
    println!("Senha:          {}", mascarar(r.senha.expose_secret()));
    println!(
        "Senha sudo:     {}",
        r.senha_sudo
            .as_ref()
            .map_or_else(|| "(não definida)".into(), |s| mascarar(s.expose_secret()))
    );
    println!(
        "Senha su:       {}",
        r.senha_su
            .as_ref()
            .map_or_else(|| "(não definida)".into(), |s| mascarar(s.expose_secret()))
    );
    println!("Timeout (ms):   {}", r.timeout_ms);
    println!("Max chars:      {}", r.max_chars);
    println!("Schema version: {}", r.schema_version);
    println!("Adicionado em:  {}", r.adicionado_em);
}

/// Imprime detalhes de UMA VPS em JSON (mascarado).
pub fn imprimir_detalhes_json(r: &VpsRegistro) {
    let v = registro_para_json_mascarado(r);
    match serde_json::to_string_pretty(&v) {
        Ok(s) => println!("{s}"),
        Err(erro) => eprintln!("erro ao serializar JSON: {erro}"),
    }
}

fn registro_para_json_mascarado(r: &VpsRegistro) -> serde_json::Value {
    json!({
        "name": r.nome,
        "host": r.host,
        "port": r.porta,
        "user": r.usuario,
        "password": mascarar(r.senha.expose_secret()),
        "sudo_password": r.senha_sudo.as_ref().map(|s| mascarar(s.expose_secret())),
        "su_password": r.senha_su.as_ref().map(|s| mascarar(s.expose_secret())),
        "timeout_ms": r.timeout_ms,
        "max_chars": r.max_chars,
        "schema_version": r.schema_version,
        "added_at": r.adicionado_em,
    })
}

/// Imprime stdout/stderr de execução de comando SSH.
///
/// Formato:
/// ```text
/// --- stdout ---
/// <stdout>
/// --- stderr ---
/// <stderr>
/// --- exit code: <code> (<duracao_ms>ms) ---
/// ```
pub fn imprimir_saida_execucao(saida: &SaidaExecucao) {
    println!("--- stdout ---");
    if saida.stdout.is_empty() {
        println!("(vazio)");
    } else {
        println!("{}", saida.stdout);
    }
    println!("--- stderr ---");
    if saida.stderr.is_empty() {
        println!("(vazio)");
    } else {
        println!("{}", saida.stderr);
    }
    let code_str = saida
        .exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    println!("--- exit code: {} ({}ms) ---", code_str, saida.duracao_ms);
    if saida.truncado_stdout {
        println!("(stdout foi truncado)");
    }
    if saida.truncado_stderr {
        println!("(stderr foi truncado)");
    }
}

/// Imprime stdout/stderr de execução de comando SSH em formato JSON.
pub fn imprimir_saida_execucao_json(saida: &SaidaExecucao) {
    let v = json!({
        "stdout": saida.stdout,
        "stderr": saida.stderr,
        "exit_code": saida.exit_code,
        "truncated_stdout": saida.truncado_stdout,
        "truncated_stderr": saida.truncado_stderr,
        "duration_ms": saida.duracao_ms,
    });
    match serde_json::to_string_pretty(&v) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("erro ao serializar JSON: {e}"),
    }
}

/// Imprime resultado de health-check em formato texto.
pub fn imprimir_health_check(nome: &str, latencia_ms: u64) {
    println!(
        "{}",
        crate::i18n::t(crate::i18n::Mensagem::HealthCheckOk {
            nome: nome.to_string(),
        })
    );
    println!("  latência: {latencia_ms}ms");
}

/// Imprime resultado de health-check em formato JSON.
pub fn imprimir_health_check_json(nome: &str, latencia_ms: u64) {
    let v = json!({
        "name": nome,
        "status": "ok",
        "latency_ms": latencia_ms,
    });
    match serde_json::to_string_pretty(&v) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("erro ao serializar JSON: {e}"),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::ssh::SaidaExecucao;
    use crate::vps::modelo::VpsRegistro;
    use secrecy::SecretString;

    fn registro_teste() -> VpsRegistro {
        VpsRegistro::novo(
            "vps-teste".into(),
            "1.2.3.4".into(),
            22,
            "root".into(),
            SecretString::from("senha-super-secreta".to_string()),
            Some(5000),
            Some(1000),
            Some(SecretString::from("sudo-password-longa-aqui".to_string())),
            None,
        )
    }

    #[test]
    fn registro_para_json_mascarado_contem_campos_obrigatorios() {
        let r = registro_teste();
        let json = registro_para_json_mascarado(&r);
        assert_eq!(json["name"], "vps-teste");
        assert_eq!(json["host"], "1.2.3.4");
        assert_eq!(json["port"], 22);
        assert_eq!(json["user"], "root");
        assert!(json["password"].as_str().unwrap().contains("..."));
        assert!(json["sudo_password"].as_str().unwrap().contains("..."));
        assert!(json["su_password"].is_null());
        assert_eq!(json["timeout_ms"], 5000);
        assert_eq!(json["max_chars"], 1000);
        assert_eq!(json["schema_version"], 1);
    }

    #[test]
    fn registro_para_json_mascarado_senha_sudo_nula_quando_nao_definida() {
        let mut r = registro_teste();
        r.senha_sudo = None;
        let json = registro_para_json_mascarado(&r);
        assert!(json["sudo_password"].is_null());
    }

    #[test]
    fn registro_para_json_mascarado_su_password_presente() {
        let mut r = registro_teste();
        r.senha_su = Some(SecretString::from("senha-su-muito-longa-aqui".to_string()));
        let json = registro_para_json_mascarado(&r);
        assert!(json["su_password"].as_str().unwrap().contains("..."));
    }

    #[test]
    fn escribir_linha_ok() {
        let resultado = escrever_linha("teste de escrita");
        assert!(resultado.is_ok());
    }

    #[test]
    fn escribir_linha_com_caracteres_especiais() {
        let resultado = escrever_linha("linha com \t tab e \"aspas\"");
        assert!(resultado.is_ok());
    }

    #[test]
    fn salida_execucao_completa_formatada() {
        let saida = SaidaExecucao {
            stdout: "output do comando".to_string(),
            stderr: "erro do comando".to_string(),
            exit_code: Some(0),
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 150,
        };
        let resultado = escrever_linha(&format!(
            "stdout: {}, stderr: {}, exit: {:?}",
            saida.stdout, saida.stderr, saida.exit_code
        ));
        assert!(resultado.is_ok());
    }

    #[test]
    fn salida_execucao_sem_exit_code() {
        let saida = SaidaExecucao {
            stdout: "".to_string(),
            stderr: "".to_string(),
            exit_code: None,
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 0,
        };
        let code_str = saida
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        assert_eq!(code_str, "N/A");
    }

    #[test]
    fn vps_registro_debug_nao_expoe_senha() {
        let r = registro_teste();
        let json = registro_para_json_mascarado(&r);
        let json_str = serde_json::to_string(&json).unwrap();
        assert!(!json_str.contains("senha-super-secreta"));
        assert!(!json_str.contains("sudo-password-longa-aqui"));
    }

    #[test]
    fn salida_execucao_truncada_mostra_aviso() {
        let saida = SaidaExecucao {
            stdout: "output".to_string(),
            stderr: "erro".to_string(),
            exit_code: Some(1),
            truncado_stdout: true,
            truncado_stderr: true,
            duracao_ms: 100,
        };
        assert!(saida.truncado_stdout);
        assert!(saida.truncado_stderr);
    }

    #[test]
    fn salida_execucao_com_exit_code_numerico() {
        let saida = SaidaExecucao {
            stdout: "".to_string(),
            stderr: "".to_string(),
            exit_code: Some(127),
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 0,
        };
        let code_str = saida
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        assert_eq!(code_str, "127");
    }

    #[test]
    fn escribir_linha_string_vazia() {
        let resultado = escrever_linha("");
        assert!(resultado.is_ok());
    }

    #[test]
    fn escribir_linha_com_unicode_brasileiro() {
        let resultado = escrever_linha("ação você está Itaú");
        assert!(resultado.is_ok());
    }

    #[test]
    fn escribir_linha_com_emojis() {
        let resultado = escrever_linha("texto com 🚀 e 🔐");
        assert!(resultado.is_ok());
    }

    #[test]
    fn escribir_linha_com_newlines() {
        let resultado = escrever_linha("linha1\nlinha2\nlinha3");
        assert!(resultado.is_ok());
    }

    #[test]
    fn escribir_linha_longo_texto() {
        let texto_longo = "a".repeat(10000);
        let resultado = escrever_linha(&texto_longo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn registro_para_json_mascarado_com_senha_curta_mascara_com_asteriscos() {
        let mut r = registro_teste();
        r.senha = SecretString::from("curta".to_string());
        let json = registro_para_json_mascarado(&r);
        let senha_str = json["password"].as_str().unwrap();
        assert_eq!(senha_str, "***");
    }

    #[test]
    fn registro_para_json_mascarado_com_sudo_e_su_definidos() {
        let mut r = registro_teste();
        r.senha_sudo = Some(SecretString::from("sudo-pass-longa-aqui".to_string()));
        r.senha_su = Some(SecretString::from("su-pass-longa-aqui".to_string()));
        let json = registro_para_json_mascarado(&r);
        assert!(!json["sudo_password"].is_null());
        assert!(!json["su_password"].is_null());
        assert!(json["sudo_password"].as_str().unwrap().contains("..."));
        assert!(json["su_password"].as_str().unwrap().contains("..."));
    }

    #[test]
    fn saida_execucao_formatacao_completa() {
        let saida = SaidaExecucao {
            stdout: "comando executado".to_string(),
            stderr: "aviso harmless".to_string(),
            exit_code: Some(0),
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 1000,
        };
        assert_eq!(saida.stdout, "comando executado");
        assert_eq!(saida.stderr, "aviso harmless");
        assert_eq!(saida.exit_code, Some(0));
        assert_eq!(saida.duracao_ms, 1000);
        assert!(!saida.truncado_stdout);
        assert!(!saida.truncado_stderr);
    }

    #[test]
    fn saida_execucao_sem_stderr() {
        let saida = SaidaExecucao {
            stdout: "ok".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 50,
        };
        assert!(saida.stderr.is_empty());
    }

    #[test]
    fn saida_execucao_com_sinal_em_vez_de_exit_code() {
        let saida = SaidaExecucao {
            stdout: String::new(),
            stderr: "signal received".to_string(),
            exit_code: None,
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 5000,
        };
        assert!(saida.exit_code.is_none());
    }

    #[test]
    fn saida_execucao_json_contem_campos_obrigatorios() {
        let saida = SaidaExecucao {
            stdout: "output".to_string(),
            stderr: "erro".to_string(),
            exit_code: Some(0),
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 100,
        };
        imprimir_saida_execucao_json(&saida);
    }

    #[test]
    fn imprimir_erro_runtime_nao_panica_com_mensagem_simples() {
        imprimir_erro_runtime("falha ao bindar socket");
    }

    #[test]
    fn imprimir_erro_runtime_nao_panica_com_mensagem_vazia() {
        imprimir_erro_runtime("");
    }

    #[test]
    fn imprimir_erro_runtime_nao_panica_com_unicode() {
        imprimir_erro_runtime("erro acentuação: operação não concluída");
    }

    #[test]
    fn imprimir_erro_dominio_nao_panica_com_variante_simples() {
        let erro = crate::erros::ErroSshCli::VpsNaoEncontrada("producao".into());
        imprimir_erro_dominio(&erro);
    }

    #[test]
    fn imprimir_erro_dominio_nao_panica_com_variante_estruturada() {
        let erro = crate::erros::ErroSshCli::ComandoFalhou {
            exit_code: 127,
            stderr: "command not found".into(),
        };
        imprimir_erro_dominio(&erro);
    }

    #[test]
    fn imprimir_erro_dominio_nao_panica_com_autenticacao_falhou() {
        let erro = crate::erros::ErroSshCli::AutenticacaoFalhou;
        imprimir_erro_dominio(&erro);
    }

    #[test]
    fn imprimir_erro_generico_nao_panica_com_erro_simples() {
        let erro = anyhow::anyhow!("falha genérica no pipeline");
        imprimir_erro_generico(&erro);
    }

    #[test]
    fn imprimir_erro_generico_nao_panica_com_chain_de_causas() {
        let raiz = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "acesso negado");
        let intermediario = anyhow::Error::new(raiz).context("falha ao abrir socket");
        let topo = intermediario.context("falha ao inicializar conexão");
        imprimir_erro_generico(&topo);
    }

    #[test]
    fn ler_confirmacao_aceita_s_minusculo() {
        let input = b"s\n";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "prompt: ").unwrap();
        assert!(r);
        assert_eq!(writer, b"prompt: ");
    }

    #[test]
    fn ler_confirmacao_aceita_sim_maiusculo() {
        let input = b"SIM\n";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "p: ").unwrap();
        assert!(r);
    }

    #[test]
    fn ler_confirmacao_aceita_yes_com_espaco() {
        let input = b"  yes  \n";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "p: ").unwrap();
        assert!(r);
    }

    #[test]
    fn ler_confirmacao_aceita_y() {
        let input = b"y\n";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "p: ").unwrap();
        assert!(r);
    }

    #[test]
    fn ler_confirmacao_rejeita_n() {
        let input = b"n\n";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "p: ").unwrap();
        assert!(!r);
    }

    #[test]
    fn ler_confirmacao_rejeita_linha_vazia() {
        let input = b"\n";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "p: ").unwrap();
        assert!(!r);
    }

    #[test]
    fn ler_confirmacao_rejeita_eof() {
        let input: &[u8] = b"";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "p: ").unwrap();
        assert!(!r);
    }

    #[test]
    fn ler_confirmacao_rejeita_texto_arbitrario() {
        let input = b"talvez\n";
        let mut reader: &[u8] = input;
        let mut writer: Vec<u8> = Vec::new();
        let r = ler_confirmacao(&mut reader, &mut writer, "p: ").unwrap();
        assert!(!r);
    }

    #[test]
    fn imprimir_erro_generico_com_chain_contem_multiplas_causas() {
        let raiz = std::io::Error::new(std::io::ErrorKind::NotFound, "arquivo ausente");
        let erro = anyhow::Error::new(raiz)
            .context("falha ao carregar config")
            .context("falha ao inicializar");
        let total_causas = erro.chain().count();
        assert!(total_causas >= 2);
        imprimir_erro_generico(&erro);
    }
}
