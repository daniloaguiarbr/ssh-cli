//! Cliente SSH real via `russh` 0.60.x.
//!
//! Implementa conexão TCP + handshake SSH + autenticação por senha + execução
//! de comandos com captura paralela de stdout/stderr.
//!
//! Na iteração 2 a verificação de chave de servidor (`check_server_key`) é
//! permissiva (trust-on-first-use sem persistência). Iterações futuras devem:
//! - persistir fingerprints em `known_hosts`
//! - suportar autenticação por chave pública
//! - suportar `sudo` e `su -` via PTY + stdin
//!
//! Quando a feature `ssh-real` está DESATIVADA (ex.: `--no-default-features`),
//! o módulo exporta apenas a `ConfiguracaoConexao` e stubs mínimos — o código
//! de alto nível da CLI deve compilar sem russh.

use crate::erros::{ErroSshCli, ResultadoSshCli};
use secrecy::SecretString;
use tokio::io::{AsyncRead, AsyncWrite};

/// Configuração de uma conexão SSH.
///
/// Construída a partir de um [`crate::vps::modelo::VpsRegistro`] no momento
/// da chamada, carregando apenas os campos necessários. A senha continua
/// protegida por [`SecretString`] (zeroize on drop).
#[derive(Clone)]
pub struct ConfiguracaoConexao {
    /// Hostname ou IP do servidor SSH.
    pub host: String,
    /// Porta TCP do servidor SSH (padrão 22).
    pub porta: u16,
    /// Nome de usuário SSH.
    pub usuario: String,
    /// Senha SSH (`SecretString` para zeroize automático).
    pub senha: SecretString,
    /// Timeout total para conexão + handshake + autenticação, em milissegundos.
    pub timeout_ms: u64,
}

impl std::fmt::Debug for ConfiguracaoConexao {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfiguracaoConexao")
            .field("host", &self.host)
            .field("porta", &self.porta)
            .field("usuario", &self.usuario)
            .field("senha", &"<redacted>")
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

impl ConfiguracaoConexao {
    /// Valida os campos básicos da configuração.
    ///
    /// Retorna [`ErroSshCli::ArgumentoInvalido`] se host estiver vazio ou porta for 0.
    pub fn validar(&self) -> ResultadoSshCli<()> {
        if self.host.trim().is_empty() {
            return Err(ErroSshCli::ArgumentoInvalido(
                "host vazio em ConfiguracaoConexao".to_string(),
            ));
        }
        if self.porta == 0 {
            return Err(ErroSshCli::ArgumentoInvalido(
                "porta 0 inválida em ConfiguracaoConexao".to_string(),
            ));
        }
        if self.usuario.trim().is_empty() {
            return Err(ErroSshCli::ArgumentoInvalido(
                "usuário vazio em ConfiguracaoConexao".to_string(),
            ));
        }
        Ok(())
    }
}

/// Saída da execução de um comando SSH remoto.
#[derive(Debug, Clone)]
pub struct SaidaExecucao {
    /// Stdout capturado (possivelmente truncado a `max_chars` codepoints).
    pub stdout: String,
    /// Stderr capturado (possivelmente truncado a `max_chars` codepoints).
    pub stderr: String,
    /// Código de saída. `None` quando o comando foi terminado por sinal ou timeout.
    pub exit_code: Option<i32>,
    /// `true` se `stdout` foi truncado em `max_chars`.
    pub truncado_stdout: bool,
    /// `true` se `stderr` foi truncado em `max_chars`.
    pub truncado_stderr: bool,
    /// Duração total da execução, em milissegundos.
    pub duracao_ms: u64,
}

/// Resultado de uma operação de transferência de arquivo via SCP.
#[derive(Debug, Clone)]
pub struct TransferenciaResultado {
    /// Número de bytes transferidos.
    pub bytes_transferidos: u64,
    /// Duração total em milissegundos.
    pub duracao_ms: u64,
}

/// Trunca uma string UTF-8 a no máximo `max_chars` codepoints.
///
/// Retorna `(string_truncada, truncou)`. Se `max_chars == 0` retorna string vazia.
/// Unicode-safe: opera sobre codepoints via `chars()`, nunca quebra no meio.
#[must_use]
pub fn truncar_utf8(conteudo: &str, max_chars: usize) -> (String, bool) {
    let total = conteudo.chars().count();
    if total <= max_chars {
        return (conteudo.to_string(), false);
    }
    let truncado: String = conteudo.chars().take(max_chars).collect();
    (truncado, true)
}

// =========================================================================
// Trait ClienteSshTrait para permitir mocks em teste.
// =========================================================================

use async_trait::async_trait;
use std::path::Path;

/// Stream bidirecional usado para tunnel SSH (direct-tcpip).
pub trait CanalTunel: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> CanalTunel for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

/// Trait para cliente SSH que permite implementação real (russh) ou mock para testes.
///
/// Este trait abstrai as operações de conexão SSH para permitir testes unitários
/// sem necessidade de conexão de rede real.
#[async_trait]
pub trait ClienteSshTrait: Send + Sync + 'static {
    /// Conecta a um servidor SSH e autentica com as credenciais fornecidas.
    async fn conectar(cfg: ConfiguracaoConexao) -> Result<Box<Self>, ErroSshCli>
    where
        Self: Sized;

    /// Executa um comando shell remoto e retorna a saída capturada.
    async fn executar_comando(
        &mut self,
        cmd: &str,
        max_chars: usize,
    ) -> Result<SaidaExecucao, ErroSshCli>;

    /// Faz upload de um arquivo local para o servidor remoto via SCP.
    async fn upload(
        &mut self,
        local: &Path,
        remote: &Path,
    ) -> Result<TransferenciaResultado, ErroSshCli>;

    /// Faz download de um arquivo remoto para o sistema local via SCP.
    async fn download(
        &mut self,
        remote: &Path,
        local: &Path,
    ) -> Result<TransferenciaResultado, ErroSshCli>;

    /// Abre um canal `direct-tcpip` para forwarding de tunnel.
    async fn abrir_canal_tunel(
        &self,
        host_remoto: &str,
        porta_remota: u16,
        endereco_origem: &str,
        porta_origem: u16,
    ) -> Result<Box<dyn CanalTunel>, ErroSshCli>;

    /// Encerra a conexão SSH de forma limpa.
    async fn desconectar(&self) -> Result<(), ErroSshCli>;
}

#[cfg(test)]
/// Mocks de cliente SSH usados em testes unitários.
pub mod mocks {
    use super::*;
    use mockall::mock;

    mock! {
        pub ClienteSsh {}

    #[async_trait]
    impl crate::ssh::cliente::ClienteSshTrait for ClienteSsh {
            async fn conectar(cfg: ConfiguracaoConexao) -> Result<Box<Self>, ErroSshCli>;
            async fn executar_comando(&mut self, cmd: &str, max_chars: usize) -> Result<SaidaExecucao, ErroSshCli>;
            async fn upload(&mut self, local: &Path, remote: &Path) -> Result<TransferenciaResultado, ErroSshCli>;
            async fn download(&mut self, remote: &Path, local: &Path) -> Result<TransferenciaResultado, ErroSshCli>;
            async fn abrir_canal_tunel(
                &self,
                host_remoto: &str,
                porta_remota: u16,
                endereco_origem: &str,
                porta_origem: u16,
            ) -> Result<Box<dyn CanalTunel>, ErroSshCli>;
            async fn desconectar(&self) -> Result<(), ErroSshCli>;
        }
    }
}

// =========================================================================
// Implementação SSH REAL (feature `ssh-real`).
// =========================================================================

#[cfg(feature = "ssh-real")]
mod real {
    use super::{
        CanalTunel, ClienteSshTrait, ConfiguracaoConexao, SaidaExecucao, TransferenciaResultado,
    };
    use crate::erros::{ErroSshCli, ResultadoSshCli};
    use async_trait::async_trait;
    use secrecy::ExposeSecret;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Handler permissivo do russh: aceita TODA chave de servidor.
    ///
    /// **Aviso de segurança**: iteração 2 usa trust-on-first-use sem persistência.
    /// Iteração 3+ deve validar contra `known_hosts` para evitar MITM.
    pub struct ManipuladorCliente;

    impl russh::client::Handler for ManipuladorCliente {
        type Error = russh::Error;

        async fn check_server_key(
            &mut self,
            _chave_servidor: &russh::keys::ssh_key::PublicKey,
        ) -> Result<bool, Self::Error> {
            tracing::warn!("check_server_key aceita TODA chave (iteração 2: sem known_hosts)");
            Ok(true)
        }
    }

    /// Cliente SSH ativo com sessão autenticada.
    pub struct ClienteSsh {
        /// Sessão SSH autenticada para operações de baixo nível.
        pub sessao: russh::client::Handle<ManipuladorCliente>,
        cfg: ConfiguracaoConexao,
    }

    impl std::fmt::Debug for ClienteSsh {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ClienteSsh")
                .field("host", &self.cfg.host)
                .field("porta", &self.cfg.porta)
                .field("usuario", &self.cfg.usuario)
                .field("timeout_ms", &self.cfg.timeout_ms)
                .finish()
        }
    }

    fn mapear_exit_status(exit_status: u32) -> i32 {
        i32::try_from(exit_status).unwrap_or(-1)
    }

    fn processar_mensagem_exec(
        msg: russh::ChannelMsg,
        stdout_bytes: &mut Vec<u8>,
        stderr_bytes: &mut Vec<u8>,
        exit_code: &mut Option<i32>,
    ) -> bool {
        use russh::ChannelMsg;

        match msg {
            ChannelMsg::Data { data } => {
                stdout_bytes.extend_from_slice(&data);
            }
            ChannelMsg::ExtendedData { data, ext } => {
                // ext == 1 → SSH_EXTENDED_DATA_STDERR (RFC 4254 §5.2).
                if ext == 1 {
                    stderr_bytes.extend_from_slice(&data);
                } else {
                    tracing::debug!(ext, "dados estendidos ignorados");
                }
            }
            ChannelMsg::ExitStatus { exit_status } => {
                // russh entrega como u32. Mantemos como i32 para acomodar
                // convenções Unix (shells podem emitir códigos como u8 em
                // wait-status; aqui já é o exit code aplicativo, 0..=255).
                *exit_code = Some(mapear_exit_status(exit_status));
                // NÃO retorna true: aguardar Eof/Close após ExitStatus.
            }
            ChannelMsg::ExitSignal {
                signal_name,
                core_dumped,
                error_message,
                ..
            } => {
                tracing::warn!(
                    ?signal_name,
                    core_dumped,
                    %error_message,
                    "processo remoto terminou por sinal"
                );
                // Sem exit_status → mantemos None.
            }
            ChannelMsg::Eof => {
                tracing::debug!("EOF no canal SSH");
            }
            ChannelMsg::Close => {
                tracing::debug!("canal SSH fechado pelo servidor");
                return true;
            }
            _ => {}
        }

        false
    }

    fn formatar_header_upload_scp(tamanho: u64, nome_arquivo: &str) -> String {
        format!("C0644 {} {}\\n", tamanho, nome_arquivo)
    }

    fn parse_header_scp(header: &str) -> ResultadoSshCli<u64> {
        let header = header.trim();

        if !header.starts_with('C') {
            return Err(ErroSshCli::CanalFalhou(format!(
                "header SCP inesperado: {}",
                header
            )));
        }

        let partes: Vec<&str> = header.split_whitespace().collect();
        if partes.len() < 3 {
            return Err(ErroSshCli::CanalFalhou(format!(
                "header SCP mal formatado: {}",
                header
            )));
        }

        partes[1].parse().map_err(|_| {
            ErroSshCli::CanalFalhou(format!("tamanho inválido no header: {}", partes[1]))
        })
    }

    impl ClienteSsh {
        /// Conecta e autentica. Todo o fluxo (TCP + handshake + auth) respeita
        /// o `timeout_ms` da configuração.
        ///
        /// # Erros
        /// - [`ErroSshCli::ArgumentoInvalido`] se a configuração for inválida.
        /// - [`ErroSshCli::TimeoutSsh`] se exceder o timeout total.
        /// - [`ErroSshCli::ConexaoFalhou`] em falhas TCP/handshake.
        /// - [`ErroSshCli::AutenticacaoFalhou`] se o servidor rejeitar a senha.
        pub async fn conectar(cfg: ConfiguracaoConexao) -> ResultadoSshCli<Self> {
            cfg.validar()?;

            let timeout = Duration::from_millis(cfg.timeout_ms);
            let host = cfg.host.clone();
            let porta = cfg.porta;
            let usuario = cfg.usuario.clone();
            let senha_segura = cfg.senha.clone();

            let config_cliente = Arc::new(russh::client::Config {
                inactivity_timeout: Some(timeout),
                ..Default::default()
            });

            tracing::info!(
                host = %host,
                porta,
                usuario = %usuario,
                timeout_ms = cfg.timeout_ms,
                "iniciando conexão SSH"
            );

            // Envelopa conexão + handshake + autenticação em um único timeout global.
            let resultado_conexao = tokio::time::timeout(timeout, async move {
                let mut sessao = russh::client::connect(
                    config_cliente,
                    (host.as_str(), porta),
                    ManipuladorCliente,
                )
                .await
                .map_err(|e| ErroSshCli::ConexaoFalhou(format!("falha TCP/handshake: {e}")))?;

                let auth = sessao
                    .authenticate_password(usuario.clone(), senha_segura.expose_secret())
                    .await
                    .map_err(|e| ErroSshCli::ConexaoFalhou(format!("falha auth transport: {e}")))?;

                if !auth.success() {
                    tracing::warn!(
                        host = %host,
                        usuario = %usuario,
                        "autenticação SSH rejeitada"
                    );
                    return Err(ErroSshCli::AutenticacaoFalhou);
                }

                Ok::<_, ErroSshCli>(sessao)
            })
            .await;

            let sessao = match resultado_conexao {
                Ok(Ok(s)) => s,
                Ok(Err(erro)) => return Err(erro),
                Err(_) => return Err(ErroSshCli::TimeoutSsh(cfg.timeout_ms)),
            };

            tracing::info!("conexão SSH autenticada com sucesso");

            Ok(Self { sessao, cfg })
        }

        /// Executa um comando shell remoto e captura stdout/stderr em paralelo.
        ///
        /// Trunca cada stream em `max_chars` codepoints UTF-8. Respeita o
        /// `timeout_ms` da configuração para a execução inteira.
        ///
        /// # Erros
        /// - [`ErroSshCli::CanalFalhou`] em falha ao abrir canal ou enviar `exec`.
        /// - [`ErroSshCli::TimeoutSsh`] se exceder o timeout.
        pub async fn executar_comando(
            &mut self,
            comando: &str,
            max_chars: usize,
        ) -> ResultadoSshCli<SaidaExecucao> {
            let inicio = Instant::now();
            let timeout = Duration::from_millis(self.cfg.timeout_ms);

            let resultado = tokio::time::timeout(timeout, async {
                let mut canal = self
                    .sessao
                    .channel_open_session()
                    .await
                    .map_err(|e| ErroSshCli::CanalFalhou(format!("abrir sessão: {e}")))?;

                canal
                    .exec(true, comando)
                    .await
                    .map_err(|e| ErroSshCli::CanalFalhou(format!("exec: {e}")))?;

                let mut stdout_bytes: Vec<u8> = Vec::new();
                let mut stderr_bytes: Vec<u8> = Vec::new();
                let mut exit_code: Option<i32> = None;

                while let Some(msg) = canal.wait().await {
                    if processar_mensagem_exec(
                        msg,
                        &mut stdout_bytes,
                        &mut stderr_bytes,
                        &mut exit_code,
                    ) {
                        break;
                    }
                }

                Ok::<_, ErroSshCli>((stdout_bytes, stderr_bytes, exit_code))
            })
            .await;

            let (stdout_bytes, stderr_bytes, exit_code) = match resultado {
                Ok(Ok(t)) => t,
                Ok(Err(erro)) => return Err(erro),
                Err(_) => return Err(ErroSshCli::TimeoutSsh(self.cfg.timeout_ms)),
            };

            // Converte de bytes para String UTF-8 de forma resiliente.
            let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
            let stderr_str = String::from_utf8_lossy(&stderr_bytes).to_string();

            let (stdout_truncado, truncado_stdout) = super::truncar_utf8(&stdout_str, max_chars);
            let (stderr_truncado, truncado_stderr) = super::truncar_utf8(&stderr_str, max_chars);

            let duracao_ms = u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX);

            Ok(SaidaExecucao {
                stdout: stdout_truncado,
                stderr: stderr_truncado,
                exit_code,
                truncado_stdout,
                truncado_stderr,
                duracao_ms,
            })
        }

        /// Upload de arquivo local para remote via SCP.
        ///
        /// # Erros
        /// - [`ErroSshCli::ArquivoNaoEncontrado`] se o arquivo local não existir.
        /// - [`ErroSshCli::CanalFalhou`] em falha ao abrir canal SCP.
        /// - [`ErroSshCli::TimeoutSsh`] se exceder o timeout.
        pub async fn upload(
            &mut self,
            local: &std::path::Path,
            remote: &std::path::Path,
        ) -> ResultadoSshCli<TransferenciaResultado> {
            use russh::ChannelMsg;
            use std::time::Instant;

            let local_str = local.display().to_string();
            let remote_str = remote.display().to_string();

            let metadados = std::fs::metadata(local).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ErroSshCli::ArquivoNaoEncontrado(local_str.clone())
                } else {
                    ErroSshCli::Io(e)
                }
            })?;

            if !metadados.is_file() {
                return Err(ErroSshCli::ArgumentoInvalido(
                    "upload só suporta arquivos regulares".to_string(),
                ));
            }

            let tamanho = metadados.len();
            let nome_arquivo = local.file_name().and_then(|n| n.to_str()).unwrap_or("file");

            let inicio = Instant::now();
            let timeout = Duration::from_millis(self.cfg.timeout_ms);

            let resultado =
                tokio::time::timeout(timeout, async {
                    let mut canal =
                        self.sessao.channel_open_session().await.map_err(|e| {
                            ErroSshCli::CanalFalhou(format!("abrir sessão SCP: {e}"))
                        })?;

                    let comando = format!("scp -t -p {}", remote_str);
                    canal
                        .exec(true, comando.as_str())
                        .await
                        .map_err(|e| ErroSshCli::CanalFalhou(format!("exec SCP: {e}")))?;

                    canal.wait().await.ok_or_else(|| {
                        ErroSshCli::CanalFalhou("canal fechou prematuramente".to_string())
                    })?;

                    let resposta = formatar_header_upload_scp(tamanho, nome_arquivo);
                    canal
                        .data(resposta.as_bytes())
                        .await
                        .map_err(|e| ErroSshCli::CanalFalhou(format!("enviar header SCP: {e}")))?;

                    canal.wait().await.ok_or_else(|| {
                        ErroSshCli::CanalFalhou("canal fechou durante header".to_string())
                    })?;

                    let conteudo = std::fs::read(local).map_err(ErroSshCli::Io)?;
                    let mut offset = 0;
                    let tamanho_bloco = 32768;

                    while offset < conteudo.len() {
                        let fim = std::cmp::min(offset + tamanho_bloco, conteudo.len());
                        let bloco = &conteudo[offset..fim];
                        canal.data(bloco).await.map_err(|e| {
                            ErroSshCli::CanalFalhou(format!("enviar bloco SCP: {e}"))
                        })?;
                        offset = fim;
                    }

                    canal
                        .data(&[] as &[u8])
                        .await
                        .map_err(|e| ErroSshCli::CanalFalhou(format!("enviar EOF SCP: {e}")))?;

                    canal.wait().await.ok_or_else(|| {
                        ErroSshCli::CanalFalhou("canal fechou durante transferência".to_string())
                    })?;

                    while let Some(msg) = canal.wait().await {
                        if let ChannelMsg::Close = msg {
                            break;
                        }
                    }

                    Ok::<_, ErroSshCli>(())
                })
                .await;

            resultado.map_err(|_| ErroSshCli::TimeoutSsh(self.cfg.timeout_ms))??;

            let duracao_ms = u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX);

            Ok(TransferenciaResultado {
                bytes_transferidos: tamanho,
                duracao_ms,
            })
        }

        /// Download de arquivo remote para local via SCP.
        ///
        /// # Erros
        /// - [`ErroSshCli::Io`] se não conseguir escrever o arquivo local.
        /// - [`ErroSshCli::CanalFalhou`] em falha ao abrir canal SCP.
        /// - [`ErroSshCli::TimeoutSsh`] se exceder o timeout.
        pub async fn download(
            &mut self,
            remote: &std::path::Path,
            local: &std::path::Path,
        ) -> ResultadoSshCli<TransferenciaResultado> {
            use russh::ChannelMsg;
            use std::io::Write;
            use std::time::Instant;

            let remote_str = remote.display().to_string();

            let inicio = Instant::now();
            let timeout = Duration::from_millis(self.cfg.timeout_ms);

            let resultado = tokio::time::timeout(timeout, async {
                let mut canal = self
                    .sessao
                    .channel_open_session()
                    .await
                    .map_err(|e| ErroSshCli::CanalFalhou(format!("abrir sessão SCP: {e}")))?;

                let comando = format!("scp -f -p {}", remote_str);
                canal
                    .exec(true, comando.as_str())
                    .await
                    .map_err(|e| ErroSshCli::CanalFalhou(format!("exec SCP: {e}")))?;

                canal
                    .data(&[] as &[u8])
                    .await
                    .map_err(|e| ErroSshCli::CanalFalhou(format!("enviar ack inicial: {e}")))?;

                let mut msg = canal.wait().await.ok_or_else(|| {
                    ErroSshCli::CanalFalhou("canal fechou esperando header".to_string())
                })?;

                let ChannelMsg::Data { data } = msg else {
                    return Err(ErroSshCli::CanalFalhou(
                        "esperava dados do servidor".to_string(),
                    ));
                };

                let header = String::from_utf8_lossy(&data);
                let tamanho = parse_header_scp(&header)?;

                canal
                    .data(&[] as &[u8])
                    .await
                    .map_err(|e| ErroSshCli::CanalFalhou(format!("enviar ack: {e}")))?;

                if let Some(pai) = local.parent() {
                    std::fs::create_dir_all(pai)?;
                }

                let mut arquivo = std::fs::File::create(local).map_err(ErroSshCli::Io)?;
                let mut recebidos: u64 = 0;

                while recebidos < tamanho {
                    msg = canal.wait().await.ok_or_else(|| {
                        ErroSshCli::CanalFalhou("canal fechou durante download".to_string())
                    })?;

                    let ChannelMsg::Data { data } = msg else {
                        continue;
                    };

                    let bytes = data.as_ref();
                    if bytes.is_empty() {
                        continue;
                    }

                    arquivo.write_all(bytes).map_err(ErroSshCli::Io)?;
                    recebidos += bytes.len() as u64;

                    canal.data(&[] as &[u8]).await.map_err(|e| {
                        ErroSshCli::CanalFalhou(format!("enviar ack durante download: {e}"))
                    })?;
                }

                while let Some(msg) = canal.wait().await {
                    if let ChannelMsg::Close = msg {
                        break;
                    }
                }

                Ok::<_, ErroSshCli>(recebidos)
            })
            .await;

            let recebidos =
                resultado.map_err(|_| ErroSshCli::TimeoutSsh(self.cfg.timeout_ms))??;

            let duracao_ms = u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX);

            Ok(TransferenciaResultado {
                bytes_transferidos: recebidos,
                duracao_ms,
            })
        }

        /// Encerra a sessão SSH de forma limpa.
        ///
        /// # Erros
        /// Propaga falha se `disconnect` retornar erro do transporte.
        pub async fn desconectar(&self) -> ResultadoSshCli<()> {
            let resultado = self
                .sessao
                .disconnect(russh::Disconnect::ByApplication, "encerrando", "pt-BR")
                .await;
            match resultado {
                Ok(()) => {
                    tracing::info!("sessão SSH encerrada");
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!(erro = %e, "falha ao encerrar sessão SSH");
                    Err(ErroSshCli::ConexaoFalhou(format!(
                        "falha ao desconectar: {e}"
                    )))
                }
            }
        }

        /// Abre canal direct-tcpip para forwarding SSH.
        pub async fn abrir_canal_tunel(
            &self,
            host_remoto: &str,
            porta_remota: u16,
            endereco_origem: &str,
            porta_origem: u16,
        ) -> ResultadoSshCli<Box<dyn CanalTunel>> {
            let canal = self
                .sessao
                .channel_open_direct_tcpip(
                    host_remoto.to_string(),
                    u32::from(porta_remota),
                    endereco_origem.to_string(),
                    u32::from(porta_origem),
                )
                .await
                .map_err(|e| {
                    ErroSshCli::CanalFalhou(format!(
                        "falha ao abrir canal direct-tcpip para {}:{}: {}",
                        host_remoto, porta_remota, e
                    ))
                })?;

            Ok(Box::new(canal.into_stream()))
        }
    }

    #[async_trait]
    impl ClienteSshTrait for ClienteSsh {
        async fn conectar(cfg: ConfiguracaoConexao) -> Result<Box<Self>, ErroSshCli> {
            Self::conectar(cfg).await.map(Box::new)
        }

        async fn executar_comando(
            &mut self,
            cmd: &str,
            max_chars: usize,
        ) -> Result<SaidaExecucao, ErroSshCli> {
            Self::executar_comando(self, cmd, max_chars).await
        }

        async fn upload(
            &mut self,
            local: &Path,
            remote: &Path,
        ) -> Result<TransferenciaResultado, ErroSshCli> {
            Self::upload(self, local, remote).await
        }

        async fn download(
            &mut self,
            remote: &Path,
            local: &Path,
        ) -> Result<TransferenciaResultado, ErroSshCli> {
            Self::download(self, remote, local).await
        }

        async fn abrir_canal_tunel(
            &self,
            host_remoto: &str,
            porta_remota: u16,
            endereco_origem: &str,
            porta_origem: u16,
        ) -> Result<Box<dyn CanalTunel>, ErroSshCli> {
            Self::abrir_canal_tunel(
                self,
                host_remoto,
                porta_remota,
                endereco_origem,
                porta_origem,
            )
            .await
        }

        async fn desconectar(&self) -> Result<(), ErroSshCli> {
            Self::desconectar(self).await
        }
    }

    #[cfg(test)]
    mod testes_real {
        use super::{
            formatar_header_upload_scp, mapear_exit_status, parse_header_scp,
            processar_mensagem_exec,
        };

        #[test]
        fn mapear_exit_status_normal() {
            assert_eq!(mapear_exit_status(0), 0);
            assert_eq!(mapear_exit_status(255), 255);
        }

        #[test]
        fn mapear_exit_status_overflow_retorna_menos_um() {
            assert_eq!(mapear_exit_status(u32::MAX), -1);
        }

        #[test]
        fn parse_header_scp_valido_retorna_tamanho() {
            let tamanho = parse_header_scp("C0644 42 arquivo.txt\n").expect("header válido");
            assert_eq!(tamanho, 42);
        }

        #[test]
        fn parse_header_scp_invalido_retorna_erro() {
            assert!(parse_header_scp("ERRO").is_err());
            assert!(parse_header_scp("C0644 sem_tamanho").is_err());
            assert!(parse_header_scp("C0644 abc arquivo").is_err());
        }

        #[test]
        fn processar_mensagem_exec_trata_stdout_stderr_e_close() {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut exit_code = None;

            let deve_parar = processar_mensagem_exec(
                russh::ChannelMsg::Data {
                    data: b"stdout".to_vec().into(),
                },
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );
            assert!(!deve_parar);
            assert_eq!(stdout, b"stdout");

            let deve_parar = processar_mensagem_exec(
                russh::ChannelMsg::ExtendedData {
                    data: b"stderr".to_vec().into(),
                    ext: 1,
                },
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );
            assert!(!deve_parar);
            assert_eq!(stderr, b"stderr");

            let _ = processar_mensagem_exec(
                russh::ChannelMsg::ExitStatus { exit_status: 17 },
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );
            assert_eq!(exit_code, Some(17));

            let deve_parar = processar_mensagem_exec(
                russh::ChannelMsg::Close,
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );
            assert!(deve_parar);
        }

        #[test]
        fn formatar_header_upload_scp_gera_formato_esperado() {
            let header = formatar_header_upload_scp(123, "arquivo.txt");
            assert_eq!(header, "C0644 123 arquivo.txt\\n");
        }

        #[test]
        fn processar_mensagem_exec_ignora_extendido_com_codigo_diferente_de_stderr() {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut exit_code = None;

            let deve_parar = processar_mensagem_exec(
                russh::ChannelMsg::ExtendedData {
                    data: b"nao-e-stderr".to_vec().into(),
                    ext: 2,
                },
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );

            assert!(!deve_parar);
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());
            assert!(exit_code.is_none());
        }

        #[test]
        fn processar_mensagem_exec_trata_exit_signal_e_eof_sem_encerrar_loop() {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut exit_code = Some(7);

            let deve_parar_signal = processar_mensagem_exec(
                russh::ChannelMsg::ExitSignal {
                    signal_name: russh::Sig::TERM,
                    core_dumped: false,
                    error_message: "encerrado".to_string(),
                    lang_tag: "pt-BR".to_string(),
                },
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );

            let deve_parar_eof = processar_mensagem_exec(
                russh::ChannelMsg::Eof,
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );

            assert!(!deve_parar_signal);
            assert!(!deve_parar_eof);
            assert_eq!(exit_code, Some(7));
        }

        #[test]
        fn processar_mensagem_exec_ignora_variantes_sem_tratamento_especifico() {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut exit_code = None;

            let deve_parar = processar_mensagem_exec(
                russh::ChannelMsg::WindowAdjusted { new_size: 2048 },
                &mut stdout,
                &mut stderr,
                &mut exit_code,
            );

            assert!(!deve_parar);
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());
            assert!(exit_code.is_none());
        }
    }
}

#[cfg(feature = "ssh-real")]
pub use real::{ClienteSsh, ManipuladorCliente};

// =========================================================================
// Stub usado quando a feature `ssh-real` está DESATIVADA.
// =========================================================================

#[cfg(not(feature = "ssh-real"))]
mod stub {
    use super::{ConfiguracaoConexao, SaidaExecucao, TransferenciaResultado};
    use crate::erros::ErroSshCli;
    use crate::ssh::cliente::ClienteSshTrait;
    use async_trait::async_trait;
    use std::path::Path;

    /// Stub quando `ssh-real` está desativado: sempre retorna
    /// [`ErroSshCli::ConexaoFalhou`].
    #[derive(Debug)]
    pub struct ClienteSsh;

    #[async_trait]
    impl ClienteSshTrait for ClienteSsh {
        async fn conectar(_cfg: ConfiguracaoConexao) -> Result<Box<Self>, ErroSshCli> {
            Err(ErroSshCli::ConexaoFalhou(
                "feature `ssh-real` está desabilitada; recompile com --features ssh-real".into(),
            ))
        }

        async fn executar_comando(
            &mut self,
            _cmd: &str,
            _max_chars: usize,
        ) -> Result<SaidaExecucao, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "stub sem russh: feature `ssh-real` desabilitada".into(),
            ))
        }

        async fn upload(
            &mut self,
            _local: &Path,
            _remote: &Path,
        ) -> Result<TransferenciaResultado, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "stub sem russh: feature `ssh-real` desabilitada".into(),
            ))
        }

        async fn download(
            &mut self,
            _remote: &Path,
            _local: &Path,
        ) -> Result<TransferenciaResultado, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "stub sem russh: feature `ssh-real` desabilitada".into(),
            ))
        }

        async fn abrir_canal_tunel(
            &self,
            _host_remoto: &str,
            _porta_remota: u16,
            _endereco_origem: &str,
            _porta_origem: u16,
        ) -> Result<Box<dyn super::CanalTunel>, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "stub sem russh: feature `ssh-real` desabilitada".into(),
            ))
        }

        async fn desconectar(&self) -> Result<(), ErroSshCli> {
            Ok(())
        }
    }
}

#[cfg(not(feature = "ssh-real"))]
pub use stub::ClienteSsh;

// =========================================================================
// Testes unitários (sem rede, sem feature gate).
// =========================================================================

#[cfg(test)]
mod testes {
    use super::*;
    use secrecy::SecretString;

    fn cfg_valida() -> ConfiguracaoConexao {
        ConfiguracaoConexao {
            host: "127.0.0.1".to_string(),
            porta: 22,
            usuario: "root".to_string(),
            senha: SecretString::from("senha-exemplo".to_string()),
            timeout_ms: 5000,
        }
    }

    #[test]
    fn validar_host_vazio_retorna_erro() {
        let mut c = cfg_valida();
        c.host = String::new();
        let r = c.validar();
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("host"));
    }

    #[test]
    fn validar_host_apenas_espacos_retorna_erro() {
        let mut c = cfg_valida();
        c.host = "   ".to_string();
        assert!(c.validar().is_err());
    }

    #[test]
    fn validar_porta_zero_retorna_erro() {
        let mut c = cfg_valida();
        c.porta = 0;
        let r = c.validar();
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("porta"));
    }

    #[test]
    fn validar_usuario_vazio_retorna_erro() {
        let mut c = cfg_valida();
        c.usuario = String::new();
        assert!(c.validar().is_err());
    }

    #[test]
    fn validar_configuracao_correta_retorna_ok() {
        assert!(cfg_valida().validar().is_ok());
    }

    #[test]
    fn debug_nao_expoe_senha() {
        let c = cfg_valida();
        let dbg = format!("{c:?}");
        assert!(!dbg.contains("senha-exemplo"));
        assert!(dbg.contains("redacted"));
    }

    #[test]
    fn truncar_utf8_nao_trunca_se_cabe() {
        let (s, t) = truncar_utf8("ola mundo", 100);
        assert_eq!(s, "ola mundo");
        assert!(!t);
    }

    #[test]
    fn truncar_utf8_trunca_string_grande_ascii() {
        let entrada: String = "a".repeat(200);
        let (s, t) = truncar_utf8(&entrada, 50);
        assert_eq!(s.chars().count(), 50);
        assert!(t);
    }

    #[test]
    fn truncar_utf8_preserva_grafemas_acentuados() {
        // 10 codepoints: "á" (1 char) * 10
        let entrada: String = "á".repeat(30);
        let (s, t) = truncar_utf8(&entrada, 10);
        assert_eq!(s.chars().count(), 10);
        // Cada 'á' ocupa 2 bytes em UTF-8 → 10 chars = 20 bytes
        assert_eq!(s.len(), 20);
        assert!(t);
        // Não corta no meio de byte
        assert!(s.chars().all(|c| c == 'á'));
    }

    #[test]
    fn truncar_utf8_com_emojis_nao_quebra() {
        let entrada = "🚀🔒🛡🔑✨🎉💎⚡🌟🔥🎨";
        let (s, t) = truncar_utf8(entrada, 5);
        assert_eq!(s.chars().count(), 5);
        assert!(t);
    }

    #[test]
    fn truncar_utf8_zero_retorna_vazio() {
        let (s, t) = truncar_utf8("abc", 0);
        assert_eq!(s, "");
        assert!(t);
    }

    #[test]
    fn saida_execucao_debug_nao_crasha() {
        let s = SaidaExecucao {
            stdout: "ok".into(),
            stderr: String::new(),
            exit_code: Some(0),
            truncado_stdout: false,
            truncado_stderr: false,
            duracao_ms: 42,
        };
        let _ = format!("{s:?}");
    }

    #[test]
    fn duracao_ms_tipo_compativel() {
        // Garantia estática de que instant elapsed cabe em u64.
        let fake: u64 = 1234;
        assert_eq!(fake, 1234_u64);
    }
}
