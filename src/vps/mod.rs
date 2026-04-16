//! CRUD e persistência de registros de VPS.
//!
//! Cada VPS é armazenada em `$CONFIG_DIR/ssh-cli/config.toml` com permissões
//! 0o600 no Unix. Toda a gestão acontece via comandos CLI — ZERO arquivo `.env`.
//!
//! O modelo [`modelo::VpsRegistro`] usa `SecretString` para senhas, garantindo
//! Zeroize on Drop automático.

pub mod modelo;

use crate::cli::{AcaoVps, FormatoSaida};
use crate::erros::{ErroSshCli, ResultadoSshCli};
use crate::output;
use crate::ssh::cliente::{ClienteSsh, ClienteSshTrait, ConfiguracaoConexao};
use anyhow::Result;
use modelo::VpsRegistro;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Arquivo de configuração completo.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ArquivoConfig {
    /// Versão do schema.
    #[serde(default)]
    pub schema_version: u32,
    /// Mapa de VPSs por nome.
    #[serde(default)]
    pub hosts: BTreeMap<String, VpsRegistro>,
}

/// Resolve o caminho do arquivo de config a partir de um override opcional.
///
/// - Se `override_path` for `Some` e apontar para um diretório (existente ou não),
///   retorna `<dir>/config.toml`.
/// - Se `override_path` for `Some` e apontar para um arquivo (terminando em `.toml`
///   ou não sendo diretório existente), retorna o caminho como é.
/// - Se `override_path` for `None`, usa [`caminho_config_padrao`].
pub fn resolver_caminho_config(override_path: Option<PathBuf>) -> ResultadoSshCli<PathBuf> {
    match override_path {
        Some(p) => {
            // Se já existe e é diretório, ou se o nome não tem extensão, trata como dir.
            if p.is_dir() {
                return Ok(p.join("config.toml"));
            }
            // Se terminar em .toml explicitamente, é arquivo.
            if p.extension().and_then(|e| e.to_str()) == Some("toml") {
                return Ok(p);
            }
            // Caso contrário, assume dir e adiciona config.toml.
            Ok(p.join("config.toml"))
        }
        None => caminho_config_padrao(),
    }
}

/// Retorna o caminho do arquivo de config respeitando `SSH_CLI_HOME`.
pub fn caminho_config_padrao() -> ResultadoSshCli<PathBuf> {
    if let Ok(home) = std::env::var("SSH_CLI_HOME") {
        if home.contains("..") {
            return Err(ErroSshCli::ArgumentoInvalido(
                "SSH_CLI_HOME não pode conter '..'".to_string(),
            ));
        }
        return Ok(PathBuf::from(home).join("config.toml"));
    }

    let dirs = directories::ProjectDirs::from("", "", "ssh-cli").ok_or_else(|| {
        ErroSshCli::Generico("não foi possível resolver diretório de config".to_string())
    })?;
    Ok(dirs.config_dir().join("config.toml"))
}

/// Carrega o arquivo de configuração (retorna vazio se não existir).
pub fn carregar(caminho: &PathBuf) -> ResultadoSshCli<ArquivoConfig> {
    if !caminho.exists() {
        return Ok(ArquivoConfig {
            schema_version: modelo::SCHEMA_VERSION_ATUAL,
            hosts: BTreeMap::new(),
        });
    }
    let conteudo = std::fs::read_to_string(caminho)?;
    let arquivo: ArquivoConfig = toml::from_str(&conteudo)?;
    Ok(arquivo)
}

/// Salva o arquivo de configuração e aplica permissões 0o600 no Unix.
pub fn salvar(caminho: &PathBuf, arquivo: &ArquivoConfig) -> ResultadoSshCli<()> {
    if let Some(pai) = caminho.parent() {
        std::fs::create_dir_all(pai)?;
    }
    let texto = toml::to_string_pretty(arquivo)
        .map_err(|e| ErroSshCli::Generico(format!("falha serializando TOML: {e}")))?;
    std::fs::write(caminho, texto)?;
    aplicar_permissoes_600(caminho)?;
    Ok(())
}

#[cfg(unix)]
fn aplicar_permissoes_600(caminho: &PathBuf) -> ResultadoSshCli<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissoes = std::fs::metadata(caminho)?.permissions();
    permissoes.set_mode(0o600);
    std::fs::set_permissions(caminho, permissoes)?;
    Ok(())
}

#[cfg(not(unix))]
fn aplicar_permissoes_600(_caminho: &PathBuf) -> ResultadoSshCli<()> {
    Ok(())
}

/// Escapa uma string para uso seguro dentro de single quotes no shell.
///
/// Estratégia: envolve em single quotes e escapa single quotes internas
/// com a sequência `'\''` (fecha quote, backslash-quote, abre quote).
fn escapar_senha_shell(valor: &str) -> String {
    let mut resultado = String::with_capacity(valor.len() + 2);
    resultado.push('\'');
    for ch in valor.chars() {
        if ch == '\'' {
            resultado.push_str("'\\''");
        } else {
            resultado.push(ch);
        }
    }
    resultado.push('\'');
    resultado
}

/// Aplica overrides de runtime sobre um VpsRegistro clonado.
///
/// Campos fornecidos pelo CLI em runtime PREVALECEM sobre valores armazenados.
fn aplicar_overrides(
    vps: &mut VpsRegistro,
    password_override: Option<String>,
    sudo_password_override: Option<String>,
    timeout_override: Option<u64>,
) {
    if let Some(pwd) = password_override {
        vps.senha = secrecy::SecretString::from(pwd);
    }
    if let Some(spwd) = sudo_password_override {
        vps.senha_sudo = Some(secrecy::SecretString::from(spwd));
    }
    if let Some(t) = timeout_override {
        vps.timeout_ms = t;
    }
}

/// Dispatcher dos subcomandos `vps`.
pub async fn executar_comando_vps(
    acao: AcaoVps,
    config_override: Option<PathBuf>,
    formato: FormatoSaida,
) -> Result<()> {
    let caminho = resolver_caminho_config(config_override)?;

    match acao {
        AcaoVps::Add {
            name,
            host,
            port,
            user,
            password,
            timeout,
            max_chars,
            sudo_password,
            su_password,
        } => {
            let name = crate::paths::normalizar_nfc(&name);
            let mut arquivo = carregar(&caminho)?;
            if arquivo.hosts.contains_key(&name) {
                return Err(ErroSshCli::VpsDuplicada(name).into());
            }
            let senha = SecretString::from(password.unwrap_or_default());
            let max_chars_num: usize = parse_max_chars(&max_chars);
            let registro = VpsRegistro::novo(
                name.clone(),
                host,
                port,
                user,
                senha,
                Some(timeout),
                Some(max_chars_num),
                sudo_password.map(SecretString::from),
                su_password.map(SecretString::from),
            );
            arquivo.hosts.insert(name.clone(), registro);
            arquivo.schema_version = modelo::SCHEMA_VERSION_ATUAL;
            salvar(&caminho, &arquivo)?;
            crate::output::imprimir_sucesso(&format!("VPS '{name}' adicionada ao registro"));
        }
        AcaoVps::List { json } => {
            let arquivo = carregar(&caminho)?;
            let registros: Vec<_> = arquivo.hosts.values().cloned().collect();
            if formato == FormatoSaida::Json || json {
                crate::output::imprimir_lista_json(&registros);
            } else {
                crate::output::imprimir_lista_texto(&registros);
            }
        }
        AcaoVps::Remove { nome } => {
            let mut arquivo = carregar(&caminho)?;
            if arquivo.hosts.remove(&nome).is_none() {
                return Err(ErroSshCli::VpsNaoEncontrada(nome).into());
            }
            salvar(&caminho, &arquivo)?;
            crate::output::imprimir_sucesso(&format!("VPS '{nome}' removida"));
        }
        AcaoVps::Edit {
            nome,
            host,
            port,
            user,
            password,
            timeout,
            max_chars,
            sudo_password,
            su_password,
        } => {
            let mut arquivo = carregar(&caminho)?;
            let registro = arquivo
                .hosts
                .get_mut(&nome)
                .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(nome.clone()))?;
            if let Some(h) = host {
                registro.host = h;
            }
            if let Some(p) = port {
                registro.porta = p;
            }
            if let Some(u) = user {
                registro.usuario = u;
            }
            if let Some(pw) = password {
                registro.senha = SecretString::from(pw);
            }
            if let Some(t) = timeout {
                registro.timeout_ms = t;
            }
            if let Some(m) = max_chars {
                registro.max_chars = parse_max_chars(&m);
            }
            if let Some(sp) = sudo_password {
                registro.senha_sudo = Some(SecretString::from(sp));
            }
            if let Some(sp) = su_password {
                registro.senha_su = Some(SecretString::from(sp));
            }
            salvar(&caminho, &arquivo)?;
            crate::output::imprimir_sucesso(&format!("VPS '{nome}' editada"));
        }
        AcaoVps::Show { nome, json } => {
            let arquivo = carregar(&caminho)?;
            let registro = arquivo
                .hosts
                .get(&nome)
                .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(nome.clone()))?;
            if formato == FormatoSaida::Json || json {
                crate::output::imprimir_detalhes_json(registro);
            } else {
                crate::output::imprimir_detalhes_texto(registro);
            }
        }
        AcaoVps::Path => {
            crate::output::escrever_linha(&caminho.display().to_string())?;
        }
    }
    Ok(())
}

/// Define a VPS ativa gravando seu nome em `<config_dir>/active`.
///
/// Esta função é chamada pelo subcomando `connect <nome>` e valida que a VPS
/// existe no registro antes de gravar.
pub async fn executar_connect(nome: &str, config_override: Option<PathBuf>) -> Result<()> {
    let caminho = resolver_caminho_config(config_override)?;
    let arquivo = carregar(&caminho)?;
    if !arquivo.hosts.contains_key(nome) {
        return Err(ErroSshCli::VpsNaoEncontrada(nome.to_string()).into());
    }

    let arquivo_ativo = caminho
        .parent()
        .map(|p| p.join("active"))
        .unwrap_or_else(|| PathBuf::from("active"));
    if let Some(pai) = arquivo_ativo.parent() {
        std::fs::create_dir_all(pai)?;
    }
    std::fs::write(&arquivo_ativo, nome)?;
    crate::output::imprimir_sucesso(&format!("VPS ativa definida: '{nome}'"));
    Ok(())
}

/// Busca um registro de VPS por nome.
///
/// Retorna `Ok(None)` se a VPS não existir (para que o caller decida o tratamento).
pub fn buscar_por_nome(
    config_override: Option<PathBuf>,
    nome: &str,
) -> ResultadoSshCli<Option<VpsRegistro>> {
    let caminho = resolver_caminho_config(config_override)?;
    let arquivo = carregar(&caminho)?;
    Ok(arquivo.hosts.get(nome).cloned())
}

/// Lê o nome da VPS ativa.
pub fn ler_vps_ativa(config_override: Option<PathBuf>) -> ResultadoSshCli<Option<String>> {
    let caminho = resolver_caminho_config(config_override)?;
    let arquivo_ativo = caminho
        .parent()
        .map(|p| p.join("active"))
        .unwrap_or_else(|| PathBuf::from("active"));
    if !arquivo_ativo.exists() {
        return Ok(None);
    }
    let nome = std::fs::read_to_string(&arquivo_ativo)?;
    Ok(Some(nome.trim().to_string()))
}

fn parse_max_chars(s: &str) -> usize {
    if s == "none" || s == "0" {
        usize::MAX
    } else {
        s.parse().unwrap_or(modelo::MAX_CHARS_PADRAO)
    }
}

/// Constrói `ConfiguracaoConexao` a partir de um `VpsRegistro`.
pub fn construir_configuracao(vps: &VpsRegistro) -> ConfiguracaoConexao {
    ConfiguracaoConexao {
        host: vps.host.clone(),
        porta: vps.porta,
        usuario: vps.usuario.clone(),
        senha: vps.senha.clone(),
        timeout_ms: vps.timeout_ms,
    }
}

/// Executa um comando em uma VPS via SSH.
pub async fn executar_exec(
    vps_nome: &str,
    comando: &str,
    config_override: Option<PathBuf>,
    formato: FormatoSaida,
    json: bool,
    password_override: Option<String>,
    timeout_override: Option<u64>,
) -> Result<()> {
    if crate::signals::cancelado() || crate::signals::terminado() {
        return Err(anyhow::anyhow!(crate::i18n::t(
            crate::i18n::Mensagem::OperacaoCancelada
        )));
    }
    let caminho = resolver_caminho_config(config_override)?;
    let arquivo = carregar(&caminho)?;
    let vps_base = arquivo
        .hosts
        .get(vps_nome)
        .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(vps_nome.to_string()))?;

    let mut vps = vps_base.clone();
    aplicar_overrides(&mut vps, password_override, None, timeout_override);
    let cfg = construir_configuracao(&vps);
    let cliente: Box<dyn ClienteSshTrait> = <ClienteSsh as ClienteSshTrait>::conectar(cfg).await?;
    executar_exec_with_client(&vps, comando, cliente, formato, json).await
}

/// Versão testável de executar_exec que aceita o cliente como parâmetro.
pub async fn executar_exec_with_client(
    vps: &VpsRegistro,
    comando: &str,
    mut cliente: Box<dyn ClienteSshTrait>,
    formato: FormatoSaida,
    json: bool,
) -> Result<()> {
    if crate::signals::cancelado() || crate::signals::terminado() {
        return Err(anyhow::anyhow!(crate::i18n::t(
            crate::i18n::Mensagem::OperacaoCancelada
        )));
    }
    let saida = cliente.executar_comando(comando, vps.max_chars).await?;
    cliente.desconectar().await?;
    if formato == FormatoSaida::Json || json {
        output::imprimir_saida_execucao_json(&saida);
    } else {
        output::imprimir_saida_execucao(&saida);
    }
    if let Some(code) = saida.exit_code {
        if code != 0 {
            return Err(ErroSshCli::ComandoFalhou {
                exit_code: code,
                stderr: saida.stderr.clone(),
            }
            .into());
        }
    }
    Ok(())
}

/// Executa um comando com `sudo` em uma VPS via SSH.
///
/// Se a VPS tiver `senha_sudo` definida (ou `sudo_password_override` for fornecido),
/// o comando é executado via `printf '%s\n' <senha> | sudo -S -p '' <cmd>`,
/// que injeta a senha no stdin do sudo sem expô-la nos argumentos do processo.
/// Caso contrário, usa `sudo <cmd>` assumindo NOPASSWD configurado.
#[allow(clippy::too_many_arguments)]
pub async fn executar_sudo_exec(
    vps_nome: &str,
    comando: &str,
    config_override: Option<PathBuf>,
    formato: FormatoSaida,
    json: bool,
    password_override: Option<String>,
    sudo_password_override: Option<String>,
    timeout_override: Option<u64>,
) -> Result<()> {
    if crate::signals::cancelado() || crate::signals::terminado() {
        return Err(anyhow::anyhow!(crate::i18n::t(
            crate::i18n::Mensagem::OperacaoCancelada
        )));
    }
    let caminho = resolver_caminho_config(config_override)?;
    let arquivo = carregar(&caminho)?;
    let vps_base = arquivo
        .hosts
        .get(vps_nome)
        .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(vps_nome.to_string()))?;

    let mut vps = vps_base.clone();
    aplicar_overrides(
        &mut vps,
        password_override,
        sudo_password_override,
        timeout_override,
    );
    let cfg = construir_configuracao(&vps);
    let cliente: Box<dyn ClienteSshTrait> = <ClienteSsh as ClienteSshTrait>::conectar(cfg).await?;
    executar_sudo_exec_with_client(&vps, comando, cliente, formato, json).await
}

/// Versão testável de executar_sudo_exec que aceita o cliente como parâmetro.
pub async fn executar_sudo_exec_with_client(
    vps: &VpsRegistro,
    comando: &str,
    mut cliente: Box<dyn ClienteSshTrait>,
    formato: FormatoSaida,
    json: bool,
) -> Result<()> {
    if crate::signals::cancelado() || crate::signals::terminado() {
        return Err(anyhow::anyhow!(crate::i18n::t(
            crate::i18n::Mensagem::OperacaoCancelada
        )));
    }
    let sudo_cmd = if let Some(ref senha) = vps.senha_sudo {
        use secrecy::ExposeSecret;
        let escaped = escapar_senha_shell(senha.expose_secret());
        format!("printf '%s\\n' {} | sudo -S -p '' {}", escaped, comando)
    } else {
        format!("sudo {}", comando)
    };

    let saida = cliente.executar_comando(&sudo_cmd, vps.max_chars).await?;
    cliente.desconectar().await?;
    if formato == FormatoSaida::Json || json {
        output::imprimir_saida_execucao_json(&saida);
    } else {
        output::imprimir_saida_execucao(&saida);
    }
    if let Some(code) = saida.exit_code {
        if code != 0 {
            return Err(ErroSshCli::ComandoFalhou {
                exit_code: code,
                stderr: saida.stderr.clone(),
            }
            .into());
        }
    }
    Ok(())
}

/// Executa um health-check (ping SSH) em uma VPS e imprime a latência.
///
/// Se `vps_nome` for `None`, usa a VPS ativa registrada.
pub async fn executar_health_check(
    vps_nome: Option<&str>,
    config_override: Option<PathBuf>,
    formato: FormatoSaida,
    password_override: Option<String>,
) -> Result<()> {
    if crate::signals::cancelado() || crate::signals::terminado() {
        return Err(anyhow::anyhow!(crate::i18n::t(
            crate::i18n::Mensagem::OperacaoCancelada
        )));
    }
    let nome_resolvido: String = match vps_nome {
        Some(n) => n.to_string(),
        None => {
            let ativa = ler_vps_ativa(config_override.clone())?;
            ativa.ok_or_else(|| {
                anyhow::anyhow!(crate::i18n::t(crate::i18n::Mensagem::HealthCheckSemVps))
            })?
        }
    };
    let caminho = resolver_caminho_config(config_override)?;
    let arquivo = carregar(&caminho)?;
    let vps_base = arquivo
        .hosts
        .get(&nome_resolvido)
        .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(nome_resolvido.clone()))?;

    let mut vps = vps_base.clone();
    aplicar_overrides(&mut vps, password_override, None, None);
    let cfg = construir_configuracao(&vps);
    let inicio = std::time::Instant::now();
    let cliente: Box<dyn ClienteSshTrait> = <ClienteSsh as ClienteSshTrait>::conectar(cfg).await?;
    let latencia_ms = inicio.elapsed().as_millis() as u64;
    cliente.desconectar().await?;

    if formato == FormatoSaida::Json {
        output::imprimir_health_check_json(&nome_resolvido, latencia_ms);
    } else {
        output::imprimir_health_check(&nome_resolvido, latencia_ms);
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use serial_test::serial;

    #[test]
    fn arquivo_vazio_serializa_com_schema() {
        let arq = ArquivoConfig {
            schema_version: modelo::SCHEMA_VERSION_ATUAL,
            hosts: BTreeMap::new(),
        };
        let texto = toml::to_string(&arq).unwrap();
        assert!(texto.contains("schema_version = 1"));
    }

    #[test]
    fn parse_max_chars_none_retorna_usize_max() {
        assert_eq!(parse_max_chars("none"), usize::MAX);
        assert_eq!(parse_max_chars("0"), usize::MAX);
        assert_eq!(parse_max_chars("1000"), 1000);
    }

    #[test]
    fn parse_max_chars_valor_invalido() {
        assert_eq!(parse_max_chars("abc"), modelo::MAX_CHARS_PADRAO);
        assert_eq!(parse_max_chars("invalido"), modelo::MAX_CHARS_PADRAO);
    }

    #[test]
    fn construir_configuracao_copia_campos_corretamente() {
        let registro = modelo::VpsRegistro::novo(
            "srv".into(),
            "host.example.com".into(),
            2222,
            "admin".into(),
            SecretString::from("pass".to_string()),
            Some(60_000),
            Some(50_000),
            None,
            None,
        );
        let cfg = construir_configuracao(&registro);
        assert_eq!(cfg.host, "host.example.com");
        assert_eq!(cfg.porta, 2222);
        assert_eq!(cfg.usuario, "admin");
        assert_eq!(cfg.timeout_ms, 60_000);
    }

    #[test]
    fn arquivo_config_vazio_tem_schema_correto() {
        let arq = ArquivoConfig {
            schema_version: modelo::SCHEMA_VERSION_ATUAL,
            hosts: BTreeMap::new(),
        };
        let toml_str = toml::to_string(&arq).unwrap();
        assert!(toml_str.contains("schema_version"));
        assert!(toml_str.contains("hosts"));
    }

    #[test]
    fn arquivo_config_com_hosts_serializa_para_toml() {
        let mut hosts = BTreeMap::new();
        hosts.insert(
            "teste".to_string(),
            modelo::VpsRegistro::novo(
                "teste".into(),
                "1.2.3.4".into(),
                22,
                "root".into(),
                SecretString::from("senha".to_string()),
                None,
                None,
                None,
                None,
            ),
        );
        let arq = ArquivoConfig {
            schema_version: modelo::SCHEMA_VERSION_ATUAL,
            hosts,
        };
        let toml_str = toml::to_string(&arq).unwrap();
        assert!(toml_str.contains("teste"));
        assert!(toml_str.contains("1.2.3.4"));
    }

    #[test]
    fn resolver_caminho_config_com_override_diretorio() {
        let resultado = resolver_caminho_config(Some(PathBuf::from("/tmp/test-dir")));
        assert!(resultado.is_ok());
        assert_eq!(
            resultado.unwrap(),
            PathBuf::from("/tmp/test-dir/config.toml")
        );
    }

    #[test]
    fn resolver_caminho_config_com_override_arquivo_explicito() {
        let resultado = resolver_caminho_config(Some(PathBuf::from("/tmp/test.toml")));
        assert!(resultado.is_ok());
        assert_eq!(resultado.unwrap(), PathBuf::from("/tmp/test.toml"));
    }

    #[test]
    fn resolver_caminho_config_sem_extensao_trata_como_diretorio() {
        let resultado = resolver_caminho_config(Some(PathBuf::from("/tmp/test")));
        assert!(resultado.is_ok());
        assert_eq!(resultado.unwrap(), PathBuf::from("/tmp/test/config.toml"));
    }

    #[test]
    fn carregar_retorna_config_vazio_quando_arquivo_nao_existe() {
        let tmp = tempfile::TempDir::new().unwrap();
        let caminho = tmp.path().join("nao-existe.toml");
        let resultado = carregar(&caminho);
        assert!(resultado.is_ok());
        let arq = resultado.unwrap();
        assert_eq!(arq.schema_version, modelo::SCHEMA_VERSION_ATUAL);
        assert!(arq.hosts.is_empty());
    }

    #[test]
    fn carregar_faz_parse_de_toml_existente() {
        let tmp = tempfile::TempDir::new().unwrap();
        let caminho = tmp.path().join("config.toml");
        let conteudo = r#"
schema_version = 1
[hosts.minha-vps]
nome = "minha-vps"
host = "1.2.3.4"
porta = 22
usuario = "root"
senha = "senhateste"
timeout_ms = 30000
max_chars = 100000
schema_version = 1
adicionado_em = "2024-01-01T00:00:00Z"
"#;
        std::fs::write(&caminho, conteudo).unwrap();
        let resultado = carregar(&caminho);
        assert!(resultado.is_ok());
        let arq = resultado.unwrap();
        assert!(arq.hosts.contains_key("minha-vps"));
    }

    #[test]
    fn ler_vps_ativa_retorna_none_quando_arquivo_nao_existe() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("ssh-cli");
        std::fs::create_dir_all(&config_dir).unwrap();
        let caminho_config = config_dir.join("config.toml");
        std::fs::write(&caminho_config, "").unwrap();
        let resultado = ler_vps_ativa(Some(config_dir.clone()));
        assert!(resultado.is_ok());
        assert!(resultado.unwrap().is_none());
    }

    #[test]
    fn ler_vps_ativa_retorna_nome_quando_arquivo_existe() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("ssh-cli");
        std::fs::create_dir_all(&config_dir).unwrap();
        let caminho_config = config_dir.join("config.toml");
        let caminho_ativo = config_dir.join("active");
        std::fs::write(&caminho_config, "").unwrap();
        std::fs::write(&caminho_ativo, "minha-vps\n").unwrap();
        let resultado = ler_vps_ativa(Some(config_dir));
        assert!(resultado.is_ok());
        assert_eq!(resultado.unwrap(), Some("minha-vps".to_string()));
    }

    #[test]
    fn ler_vps_ativa_com_override_diretorio() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("minha-config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let caminho_config = config_dir.join("config.toml");
        let caminho_ativo = config_dir.join("active");
        std::fs::write(&caminho_config, "").unwrap();
        std::fs::write(&caminho_ativo, "vps-teste\n").unwrap();
        let resultado = ler_vps_ativa(Some(config_dir));
        assert!(resultado.is_ok());
        assert_eq!(resultado.unwrap(), Some("vps-teste".to_string()));
    }

    #[test]
    fn buscar_por_nome_retorna_none_quando_nao_existe() {
        let tmp = tempfile::TempDir::new().unwrap();
        let caminho = tmp.path().join("config.toml");
        std::fs::write(&caminho, "").unwrap();
        let resultado = buscar_por_nome(Some(caminho.clone()), "inexistente");
        assert!(resultado.is_ok());
        assert!(resultado.unwrap().is_none());
    }

    #[test]
    fn buscar_por_nome_retorna_registro_quando_existe() {
        let tmp = tempfile::TempDir::new().unwrap();
        let caminho = tmp.path().join("config.toml");
        let conteudo = r#"
schema_version = 1
[hosts.minha-vps]
nome = "minha-vps"
host = "1.2.3.4"
porta = 22
usuario = "root"
senha = "senhateste"
timeout_ms = 30000
max_chars = 100000
schema_version = 1
adicionado_em = "2024-01-01T00:00:00Z"
"#;
        std::fs::write(&caminho, conteudo).unwrap();
        let resultado = buscar_por_nome(Some(caminho), "minha-vps");
        assert!(resultado.is_ok());
        let vps = resultado.unwrap();
        assert!(vps.is_some());
        assert_eq!(vps.unwrap().nome, "minha-vps");
    }

    #[cfg(unix)]
    #[test]
    fn salvar_aplica_permissoes_600_no_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::TempDir::new().unwrap();
        let caminho = tmp.path().join("config.toml");
        let arquivo = ArquivoConfig {
            schema_version: modelo::SCHEMA_VERSION_ATUAL,
            hosts: BTreeMap::new(),
        };
        let resultado = salvar(&caminho, &arquivo);
        assert!(resultado.is_ok());
        let metadados = std::fs::metadata(&caminho).unwrap();
        let permissoes = metadados.permissions();
        assert_eq!(permissoes.mode() & 0o777, 0o600);
    }

    #[test]
    fn salvar_cria_diretorio_pai_se_nao_existir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let caminho = tmp
            .path()
            .join("subdir1")
            .join("subdir2")
            .join("config.toml");
        let arquivo = ArquivoConfig {
            schema_version: modelo::SCHEMA_VERSION_ATUAL,
            hosts: BTreeMap::new(),
        };
        let resultado = salvar(&caminho, &arquivo);
        assert!(resultado.is_ok());
        assert!(caminho.exists());
    }

    #[test]
    fn arquivo_config_parsing_com_campos_parciais() {
        let tmp = tempfile::TempDir::new().unwrap();
        let caminho = tmp.path().join("config.toml");
        let conteudo = r#"
schema_version = 1
[hosts.vps-minima]
nome = "vps-minima"
host = "5.6.7.8"
porta = 2222
usuario = "admin"
senha = "senha123"
timeout_ms = 30000
max_chars = 100000
schema_version = 1
adicionado_em = "2024-01-01T00:00:00Z"
"#;
        std::fs::write(&caminho, conteudo).unwrap();
        let resultado = carregar(&caminho);
        assert!(resultado.is_ok());
        let arq = resultado.unwrap();
        assert!(arq.hosts.contains_key("vps-minima"));
        let vps = arq.hosts.get("vps-minima").unwrap();
        assert_eq!(vps.host, "5.6.7.8");
        assert_eq!(vps.porta, 2222);
    }

    #[tokio::test]
    #[serial]
    async fn executar_exec_with_client_retorna_ok_quando_mock_sucesso() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::{ClienteSshTrait, SaidaExecucao};

        let mut mock = MockClienteSsh::new();
        mock.expect_executar_comando()
            .returning(|_cmd, _max_chars| {
                Ok(SaidaExecucao {
                    stdout: "output test".to_string(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    truncado_stdout: false,
                    truncado_stderr: false,
                    duracao_ms: 100,
                })
            });
        mock.expect_desconectar().returning(|| Ok(()));

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado =
            executar_exec_with_client(&registro, "echo test", cliente, FormatoSaida::Text, false)
                .await;
        assert!(resultado.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn executar_sudo_exec_with_client_retorna_ok_quando_mock_sucesso() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::{ClienteSshTrait, SaidaExecucao};

        let mut mock = MockClienteSsh::new();
        mock.expect_executar_comando()
            .returning(|_cmd, _max_chars| {
                Ok(SaidaExecucao {
                    stdout: "sudo output".to_string(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    truncado_stdout: false,
                    truncado_stderr: false,
                    duracao_ms: 100,
                })
            });
        mock.expect_desconectar().returning(|| Ok(()));

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let mut registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );
        registro.senha_sudo = Some(SecretString::from("sudo_pass".to_string()));

        let resultado = executar_sudo_exec_with_client(
            &registro,
            "echo sudo",
            cliente,
            FormatoSaida::Text,
            false,
        )
        .await;
        assert!(resultado.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn executar_sudo_exec_with_client_retorna_ok_quando_sem_senha_sudo() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::{ClienteSshTrait, SaidaExecucao};

        let mut mock = MockClienteSsh::new();
        mock.expect_executar_comando()
            .returning(|_cmd, _max_chars| {
                Ok(SaidaExecucao {
                    stdout: "output".to_string(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    truncado_stdout: false,
                    truncado_stderr: false,
                    duracao_ms: 100,
                })
            });
        mock.expect_desconectar().returning(|| Ok(()));

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado = executar_sudo_exec_with_client(
            &registro,
            "echo test",
            cliente,
            FormatoSaida::Text,
            false,
        )
        .await;
        assert!(resultado.is_ok());
    }

    #[tokio::test]
    async fn executar_sudo_exec_with_client_retorna_erro_quando_executar_comando_falha() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::ClienteSshTrait;

        let mut mock = MockClienteSsh::new();
        mock.expect_executar_comando()
            .returning(|_cmd, _max_chars| {
                Err(crate::erros::ErroSshCli::CanalFalhou(
                    "mock error".to_string(),
                ))
            });

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado =
            executar_exec_with_client(&registro, "echo test", cliente, FormatoSaida::Text, false)
                .await;
        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_scp_upload_with_client_retorna_ok_quando_mock_sucesso() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::{ClienteSshTrait, TransferenciaResultado};

        let mut mock = MockClienteSsh::new();
        mock.expect_upload().returning(|_local, _remote| {
            Ok(TransferenciaResultado {
                bytes_transferidos: 1024,
                duracao_ms: 50,
            })
        });
        mock.expect_desconectar().returning(|| Ok(()));

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado = crate::scp::executar_scp_upload_with_client(
            &registro,
            std::path::Path::new("/local/file.txt"),
            std::path::Path::new("/remote/file.txt"),
            cliente,
        )
        .await;
        assert!(resultado.is_ok());
    }

    #[tokio::test]
    async fn executar_scp_download_with_client_retorna_ok_quando_mock_sucesso() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::{ClienteSshTrait, TransferenciaResultado};

        let mut mock = MockClienteSsh::new();
        mock.expect_download().returning(|_remote, _local| {
            Ok(TransferenciaResultado {
                bytes_transferidos: 2048,
                duracao_ms: 75,
            })
        });
        mock.expect_desconectar().returning(|| Ok(()));

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado = crate::scp::executar_scp_download_with_client(
            &registro,
            std::path::Path::new("/remote/file.txt"),
            std::path::Path::new("/local/file.txt"),
            cliente,
        )
        .await;
        assert!(resultado.is_ok());
    }

    #[tokio::test]
    async fn executar_scp_upload_with_client_retorna_erro_quando_upload_falha() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::ClienteSshTrait;

        let mut mock = MockClienteSsh::new();
        mock.expect_upload().returning(|_local, _remote| {
            Err(crate::erros::ErroSshCli::Generico(
                "falha no upload".to_string(),
            ))
        });

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado = crate::scp::executar_scp_upload_with_client(
            &registro,
            std::path::Path::new("/local/file.txt"),
            std::path::Path::new("/remote/file.txt"),
            cliente,
        )
        .await;
        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_scp_download_with_client_retorna_erro_quando_download_falha() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::ClienteSshTrait;

        let mut mock = MockClienteSsh::new();
        mock.expect_download().returning(|_remote, _local| {
            Err(crate::erros::ErroSshCli::Generico(
                "falha no download".to_string(),
            ))
        });

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado = crate::scp::executar_scp_download_with_client(
            &registro,
            std::path::Path::new("/remote/file.txt"),
            std::path::Path::new("/local/file.txt"),
            cliente,
        )
        .await;
        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_sudo_exec_with_client_retorna_erro_quando_desconectar_falha() {
        use crate::ssh::cliente::mocks::MockClienteSsh;
        use crate::ssh::cliente::{ClienteSshTrait, SaidaExecucao};

        let mut mock = MockClienteSsh::new();
        mock.expect_executar_comando()
            .returning(|_cmd, _max_chars| {
                Ok(SaidaExecucao {
                    stdout: "output".to_string(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    truncado_stdout: false,
                    truncado_stderr: false,
                    duracao_ms: 100,
                })
            });
        mock.expect_desconectar().returning(|| {
            Err(crate::erros::ErroSshCli::CanalFalhou(
                "erro desconexão".to_string(),
            ))
        });

        let cliente = Box::new(mock) as Box<dyn ClienteSshTrait>;
        let registro = modelo::VpsRegistro::novo(
            "teste".into(),
            "localhost".into(),
            22,
            "user".into(),
            SecretString::from("pass".to_string()),
            None,
            None,
            None,
            None,
        );

        let resultado =
            executar_exec_with_client(&registro, "echo test", cliente, FormatoSaida::Text, false)
                .await;
        assert!(resultado.is_err());
    }

    #[test]
    #[serial]
    fn caminho_config_padrao_com_ssh_cli_home_retorna_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home_dir = tmp.path().join("ssh-cli-home");
        std::fs::create_dir_all(&home_dir).unwrap();
        std::env::set_var("SSH_CLI_HOME", home_dir.to_str().unwrap());
        let resultado = caminho_config_padrao();
        std::env::remove_var("SSH_CLI_HOME");
        assert!(resultado.is_ok());
        assert!(resultado
            .unwrap()
            .to_str()
            .unwrap()
            .contains("ssh-cli-home"));
    }

    #[test]
    #[serial]
    fn caminho_config_padrao_com_path_traversal_retorna_erro() {
        std::env::set_var("SSH_CLI_HOME", "/tmp/../etc/config");
        let resultado = caminho_config_padrao();
        std::env::remove_var("SSH_CLI_HOME");
        assert!(resultado.is_err());
    }

    #[test]
    #[serial]
    fn caminho_config_padrao_sem_env_retorna_path_valido() {
        std::env::remove_var("SSH_CLI_HOME");
        let resultado = caminho_config_padrao();
        if let Ok(path) = resultado {
            assert!(path.to_str().unwrap().contains("ssh-cli"));
        }
    }

    #[test]
    fn escapar_senha_shell_simples() {
        assert_eq!(escapar_senha_shell("abc123"), "'abc123'");
    }

    #[test]
    fn escapar_senha_shell_com_single_quote() {
        assert_eq!(escapar_senha_shell("ab'cd"), "'ab'\\''cd'");
    }

    #[test]
    fn escapar_senha_shell_com_especiais() {
        // $, @, ~, !, ` são seguros dentro de single quotes
        assert_eq!(escapar_senha_shell("p@ss$w0rd!"), "'p@ss$w0rd!'");
    }

    #[test]
    fn escapar_senha_shell_vazia() {
        assert_eq!(escapar_senha_shell(""), "''");
    }

    #[test]
    fn escapar_senha_shell_unicode() {
        assert_eq!(escapar_senha_shell("café☕"), "'café☕'");
    }

    #[test]
    fn escapar_senha_shell_senha_usuario() {
        // Senha real do caso de uso
        assert_eq!(
            escapar_senha_shell("Ih8Tml@Ymnwku1:G@W~2"),
            "'Ih8Tml@Ymnwku1:G@W~2'"
        );
    }

    #[test]
    fn sudo_cmd_com_senha_formato_correto() {
        let senha = "test123";
        let comando = "apt update";
        let escaped = escapar_senha_shell(senha);
        let sudo_cmd = format!("printf '%s\\n' {} | sudo -S -p '' {}", escaped, comando);
        assert_eq!(
            sudo_cmd,
            "printf '%s\\n' 'test123' | sudo -S -p '' apt update"
        );
    }

    #[test]
    fn sudo_cmd_sem_senha_formato_correto() {
        let comando = "apt update";
        let sudo_cmd = format!("sudo {}", comando);
        assert_eq!(sudo_cmd, "sudo apt update");
    }

    #[test]
    fn aplicar_overrides_com_todos_os_campos() {
        use secrecy::ExposeSecret;
        let mut vps = modelo::VpsRegistro::novo(
            "srv".into(),
            "1.2.3.4".into(),
            22,
            "root".into(),
            SecretString::from("senha_original".to_string()),
            Some(30_000),
            Some(50_000),
            None,
            None,
        );
        aplicar_overrides(
            &mut vps,
            Some("nova_senha".to_string()),
            Some("nova_sudo".to_string()),
            Some(60_000),
        );
        assert_eq!(vps.senha.expose_secret(), "nova_senha");
        assert_eq!(
            vps.senha_sudo.as_ref().unwrap().expose_secret(),
            "nova_sudo"
        );
        assert_eq!(vps.timeout_ms, 60_000);
    }

    #[test]
    fn aplicar_overrides_preserva_campos_quando_none() {
        use secrecy::ExposeSecret;
        let mut vps = modelo::VpsRegistro::novo(
            "srv".into(),
            "1.2.3.4".into(),
            22,
            "root".into(),
            SecretString::from("senha_original".to_string()),
            Some(30_000),
            Some(50_000),
            Some(SecretString::from("sudo_original".to_string())),
            None,
        );
        aplicar_overrides(&mut vps, None, None, None);
        assert_eq!(vps.senha.expose_secret(), "senha_original");
        assert_eq!(
            vps.senha_sudo.as_ref().unwrap().expose_secret(),
            "sudo_original"
        );
        assert_eq!(vps.timeout_ms, 30_000);
    }

    #[test]
    fn construir_configuracao_com_timeout_diferente() {
        let registro = modelo::VpsRegistro::novo(
            "srv".into(),
            "host.example.com".into(),
            2222,
            "admin".into(),
            SecretString::from("pass".to_string()),
            Some(120_000),
            Some(50_000),
            None,
            None,
        );
        let cfg = construir_configuracao(&registro);
        assert_eq!(cfg.timeout_ms, 120_000);
    }
}
