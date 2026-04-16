//! Definição de argumentos CLI via `clap` derive e dispatcher.
//!
//! O ssh-cli MVP tem os seguintes modos de operação:
//!
//! 1. **CRUD de VPS** — `ssh-cli vps add|list|remove|edit|show|path`.
//! 2. **Seleção de ativa** — `ssh-cli connect <NOME>` (grava em `config.toml.active`).
//! 3. **Execução remota** — `ssh-cli exec|sudo-exec|scp|tunnel`.
//! 4. **Health check** — `ssh-cli health-check [VPS]`.
//! 5. **Completions** — `ssh-cli completions <SHELL>`.
//!
//! ZERO arquivo `.env`. Toda configuração é gerenciada via comandos explícitos.

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

/// Formato de saída suportado pela CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum FormatoSaida {
    /// Texto legível por humanos (padrão).
    #[default]
    Text,
    /// JSON estruturado.
    Json,
}

/// Argumentos globais do ssh-cli.
#[derive(Debug, Parser)]
#[command(
    name = "ssh-cli",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("SSH_CLI_COMMIT_HASH"), ")"),
    about = "CLI Rust para LLMs operarem servidores via SSH.",
    long_about = None,
)]
pub struct Argumentos {
    /// Força o idioma da CLI (ex.: `pt-BR`, `en-US`).
    #[arg(long, global = true, value_name = "LOCALE")]
    pub lang: Option<String>,

    /// Aumenta a verbosidade de logs em stderr.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suprime output não-JSON (modo silencioso).
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Override do diretório de configuração (útil para testes).
    #[arg(long, global = true, value_name = "DIR")]
    pub config_dir: Option<PathBuf>,

    /// Desativa cores no output.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Formato global de saída (text, json).
    #[arg(long, global = true, value_enum, default_value_t = FormatoSaida::Text)]
    pub output_format: FormatoSaida,

    /// Subcomando a executar.
    #[command(subcommand)]
    pub comando: Comando,
}

/// Subcomandos de primeiro nível.
#[derive(Debug, Subcommand)]
pub enum Comando {
    /// Gerencia VPSs cadastradas (add, list, remove, edit, show, path).
    Vps {
        /// Ação específica do CRUD de VPS.
        #[command(subcommand)]
        acao: AcaoVps,
    },

    /// Define a VPS ativa (grava `active = "<NOME>"` no `config.toml`).
    Connect {
        /// Nome da VPS previamente adicionada via `vps add`.
        nome: String,
    },

    /// Executa um comando na VPS via SSH (stdout/stderr capturados).
    Exec {
        /// Nome da VPS previamente adicionada via `vps add`.
        vps_nome: String,

        /// Comando shell a executar.
        comando: String,

        /// Saída em JSON.
        #[arg(long)]
        json: bool,
    },

    /// Executa um comando com `sudo` na VPS via SSH.
    SudoExec {
        /// Nome da VPS previamente adicionada via `vps add`.
        vps_nome: String,

        /// Comando shell a executar com privilégios sudo.
        comando: String,

        /// Saída em JSON.
        #[arg(long)]
        json: bool,
    },

    /// Transferência de arquivos via SCP (upload/download).
    Scp {
        /// Ação específica do SCP.
        #[command(subcommand)]
        acao: AcaoScp,
    },

    /// Cria um tunnel SSH (port-forward local).
    Tunnel {
        /// Nome da VPS previamente adicionada via `vps add`.
        vps_nome: String,

        /// Porta local para escuta (ex.: 8080).
        porta_local: u16,

        /// Host remoto accesible via SSH (ex.: 127.0.0.1).
        host_remoto: String,

        /// Porta remota (ex.: 5432).
        porta_remota: u16,
    },

    /// Verifica conectividade SSH com uma VPS.
    HealthCheck {
        /// Nome da VPS a verificar (usa VPS ativa se omitido).
        vps_nome: Option<String>,
    },

    /// Gera completions de shell (bash, zsh, fish, powershell, elvish).
    Completions {
        /// Shell para gerar completions.
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Ações do subcomando `vps`.
#[derive(Debug, Subcommand)]
pub enum AcaoVps {
    /// Adiciona uma nova VPS ao registro.
    Add {
        /// Nome único da VPS.
        #[arg(long)]
        name: String,

        /// Hostname ou IP.
        #[arg(long)]
        host: String,

        /// Porta SSH.
        #[arg(long, default_value_t = 22)]
        port: u16,

        /// Usuário SSH.
        #[arg(long)]
        user: String,

        /// Senha SSH.
        #[arg(long)]
        password: Option<String>,

        /// Timeout em milissegundos para comandos.
        #[arg(long, default_value_t = 30_000)]
        timeout: u64,

        /// Limite de caracteres por output (`"none"` ou `"0"` = ilimitado).
        #[arg(long, default_value = "100000")]
        max_chars: String,

        /// Senha para `sudo`.
        #[arg(long)]
        sudo_password: Option<String>,

        /// Senha para `su -`.
        #[arg(long)]
        su_password: Option<String>,
    },

    /// Lista todas as VPSs (senhas mascaradas).
    List {
        /// Saída em JSON (útil para pipes).
        #[arg(long)]
        json: bool,
    },

    /// Remove uma VPS do registro.
    Remove {
        /// Nome da VPS a remover.
        nome: String,
    },

    /// Edita campos de uma VPS existente.
    Edit {
        /// Nome da VPS a editar.
        nome: String,

        /// Novo hostname/IP.
        #[arg(long)]
        host: Option<String>,

        /// Nova porta SSH.
        #[arg(long)]
        port: Option<u16>,

        /// Novo usuário.
        #[arg(long)]
        user: Option<String>,

        /// Nova senha.
        #[arg(long)]
        password: Option<String>,

        /// Novo timeout.
        #[arg(long)]
        timeout: Option<u64>,

        /// Novo limite de caracteres.
        #[arg(long)]
        max_chars: Option<String>,

        /// Nova senha sudo.
        #[arg(long)]
        sudo_password: Option<String>,

        /// Nova senha su.
        #[arg(long)]
        su_password: Option<String>,
    },

    /// Exibe detalhes de uma VPS (senhas mascaradas).
    Show {
        /// Nome da VPS a exibir.
        nome: String,

        /// Saída em JSON.
        #[arg(long)]
        json: bool,
    },

    /// Exibe o caminho do arquivo de configuração.
    Path,
}

/// Ações do subcomando `scp`.
#[derive(Debug, Subcommand)]
pub enum AcaoScp {
    /// Upload de arquivo local para remote.
    Upload {
        /// Nome da VPS previamente adicionada via `vps add`.
        vps_nome: String,

        /// Caminho do arquivo local a enviar.
        local: PathBuf,

        /// Caminho destino no servidor remote.
        remote: PathBuf,
    },

    /// Download de arquivo remote para local.
    Download {
        /// Nome da VPS previamente adicionada via `vps add`.
        vps_nome: String,

        /// Caminho do arquivo no servidor remote.
        remote: PathBuf,

        /// Caminho local de destino.
        local: PathBuf,
    },
}

/// Faz parsing dos argumentos da CLI.
#[must_use]
pub fn parse_args() -> Argumentos {
    Argumentos::parse()
}

/// Inicializa `tracing-subscriber`. Precedência: `RUST_LOG` > `--verbose` > `--quiet` > `info`.
pub fn inicializar_logs(args: &Argumentos) {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = if std::env::var("RUST_LOG").is_ok() {
        EnvFilter::from_default_env()
    } else if args.verbose {
        EnvFilter::new("debug")
    } else if args.quiet {
        EnvFilter::new("error")
    } else {
        EnvFilter::new("info")
    };

    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(false)
        .try_init();
}

/// Gera completions de shell para stdout.
pub fn gerar_completions(shell: Shell) {
    use clap::CommandFactory;
    let mut cmd = Argumentos::command();
    clap_complete::generate(shell, &mut cmd, "ssh-cli", &mut std::io::stdout());
}

/// Executa o subcomando solicitado.
pub async fn executar(args: Argumentos) -> Result<()> {
    let config_override = args.config_dir.clone();
    let formato = args.output_format;

    match args.comando {
        Comando::Vps { acao } => {
            crate::vps::executar_comando_vps(acao, config_override, formato).await
        }
        Comando::Connect { nome } => crate::vps::executar_connect(&nome, config_override).await,
        Comando::Exec {
            vps_nome,
            comando,
            json,
        } => crate::vps::executar_exec(&vps_nome, &comando, config_override, formato, json).await,
        Comando::SudoExec {
            vps_nome,
            comando,
            json,
        } => {
            crate::vps::executar_sudo_exec(&vps_nome, &comando, config_override, formato, json)
                .await
        }
        Comando::Scp { acao } => crate::scp::executar_scp(acao, config_override).await,
        Comando::Tunnel {
            vps_nome,
            porta_local,
            host_remoto,
            porta_remota,
        } => {
            crate::tunnel::executar_tunnel(
                &vps_nome,
                porta_local,
                &host_remoto,
                porta_remota,
                config_override,
            )
            .await
        }
        Comando::HealthCheck { vps_nome } => {
            crate::vps::executar_health_check(vps_nome.as_deref(), config_override, formato).await
        }
        Comando::Completions { shell } => {
            gerar_completions(shell);
            Ok(())
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use clap::Parser;
    use serial_test::serial;
    use tempfile::TempDir;

    fn argumentos_teste(comando: Comando, config_dir: Option<PathBuf>) -> Argumentos {
        Argumentos {
            lang: None,
            verbose: false,
            quiet: false,
            config_dir,
            no_color: false,
            output_format: FormatoSaida::Text,
            comando,
        }
    }

    #[test]
    fn parser_entende_tunnel() {
        let args =
            Argumentos::try_parse_from(["ssh-cli", "tunnel", "vps-a", "8080", "127.0.0.1", "5432"])
                .expect("parser deve aceitar subcomando tunnel");

        match args.comando {
            Comando::Tunnel {
                vps_nome,
                porta_local,
                host_remoto,
                porta_remota,
            } => {
                assert_eq!(vps_nome, "vps-a");
                assert_eq!(porta_local, 8080);
                assert_eq!(host_remoto, "127.0.0.1");
                assert_eq!(porta_remota, 5432);
            }
            outro => panic!("comando inesperado: {outro:?}"),
        }
    }

    #[test]
    fn parser_entende_scp_upload() {
        let args = Argumentos::try_parse_from([
            "ssh-cli",
            "scp",
            "upload",
            "vps-a",
            "./arquivo-local.txt",
            "/tmp/arquivo-remoto.txt",
        ])
        .expect("parser deve aceitar scp upload");

        match args.comando {
            Comando::Scp {
                acao:
                    AcaoScp::Upload {
                        vps_nome,
                        local,
                        remote,
                    },
            } => {
                assert_eq!(vps_nome, "vps-a");
                assert_eq!(local, PathBuf::from("./arquivo-local.txt"));
                assert_eq!(remote, PathBuf::from("/tmp/arquivo-remoto.txt"));
            }
            outro => panic!("comando inesperado: {outro:?}"),
        }
    }

    #[test]
    #[serial]
    fn inicializar_logs_sem_panic_com_rust_log_definido() {
        std::env::set_var("RUST_LOG", "trace");
        let args = argumentos_teste(
            Comando::Connect {
                nome: "vps-a".to_string(),
            },
            None,
        );
        inicializar_logs(&args);
        std::env::remove_var("RUST_LOG");
    }

    #[test]
    #[serial]
    fn inicializar_logs_sem_panic_com_verbose() {
        std::env::remove_var("RUST_LOG");
        let mut args = argumentos_teste(
            Comando::Connect {
                nome: "vps-a".to_string(),
            },
            None,
        );
        args.verbose = true;
        inicializar_logs(&args);
    }

    #[test]
    #[serial]
    fn inicializar_logs_sem_panic_com_quiet() {
        std::env::remove_var("RUST_LOG");
        let mut args = argumentos_teste(
            Comando::Connect {
                nome: "vps-a".to_string(),
            },
            None,
        );
        args.quiet = true;
        inicializar_logs(&args);
    }

    #[test]
    #[serial]
    fn inicializar_logs_sem_panic_no_padrao_info() {
        std::env::remove_var("RUST_LOG");
        let args = argumentos_teste(
            Comando::Connect {
                nome: "vps-a".to_string(),
            },
            None,
        );
        inicializar_logs(&args);
    }

    #[tokio::test]
    async fn executar_branch_exec_retorna_erro_para_vps_inexistente() {
        let tmp = TempDir::new().expect("tempdir");
        let args = argumentos_teste(
            Comando::Exec {
                vps_nome: "inexistente".to_string(),
                comando: "echo ok".to_string(),
                json: false,
            },
            Some(tmp.path().to_path_buf()),
        );

        let resultado = executar(args).await;
        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_branch_sudo_exec_retorna_erro_para_vps_inexistente() {
        let tmp = TempDir::new().expect("tempdir");
        let args = argumentos_teste(
            Comando::SudoExec {
                vps_nome: "inexistente".to_string(),
                comando: "id".to_string(),
                json: false,
            },
            Some(tmp.path().to_path_buf()),
        );

        let resultado = executar(args).await;
        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_branch_scp_retorna_erro_para_vps_inexistente() {
        let tmp = TempDir::new().expect("tempdir");
        let args = argumentos_teste(
            Comando::Scp {
                acao: AcaoScp::Upload {
                    vps_nome: "inexistente".to_string(),
                    local: PathBuf::from("./arquivo-local.txt"),
                    remote: PathBuf::from("/tmp/arquivo-remoto.txt"),
                },
            },
            Some(tmp.path().to_path_buf()),
        );

        let resultado = executar(args).await;
        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_branch_tunnel_retorna_erro_para_vps_inexistente() {
        let tmp = TempDir::new().expect("tempdir");
        let args = argumentos_teste(
            Comando::Tunnel {
                vps_nome: "inexistente".to_string(),
                porta_local: 38080,
                host_remoto: "127.0.0.1".to_string(),
                porta_remota: 5432,
            },
            Some(tmp.path().to_path_buf()),
        );

        let resultado = executar(args).await;
        assert!(resultado.is_err());
    }

    #[test]
    fn test_parse_no_color() {
        let args = Argumentos::try_parse_from(["ssh-cli", "--no-color", "vps", "list"])
            .expect("parser deve aceitar --no-color");
        assert!(args.no_color);
    }

    #[test]
    fn test_parse_output_format_json() {
        let args =
            Argumentos::try_parse_from(["ssh-cli", "--output-format", "json", "vps", "list"])
                .expect("parser deve aceitar --output-format json");
        assert_eq!(args.output_format, FormatoSaida::Json);
    }

    #[test]
    fn test_parse_output_format_default() {
        let args = Argumentos::try_parse_from(["ssh-cli", "vps", "list"])
            .expect("parser deve aceitar subcomando sem output-format");
        assert_eq!(args.output_format, FormatoSaida::Text);
    }

    #[test]
    fn test_parse_completions_bash() {
        let args = Argumentos::try_parse_from(["ssh-cli", "completions", "bash"])
            .expect("parser deve aceitar completions bash");
        assert!(matches!(
            args.comando,
            Comando::Completions { shell: Shell::Bash }
        ));
    }

    #[test]
    fn test_parse_health_check_com_nome() {
        let args = Argumentos::try_parse_from(["ssh-cli", "health-check", "meu-vps"])
            .expect("parser deve aceitar health-check com nome");
        match args.comando {
            Comando::HealthCheck { vps_nome } => {
                assert_eq!(vps_nome, Some("meu-vps".to_string()));
            }
            outro => panic!("comando inesperado: {outro:?}"),
        }
    }

    #[test]
    fn test_parse_health_check_sem_nome() {
        let args = Argumentos::try_parse_from(["ssh-cli", "health-check"])
            .expect("parser deve aceitar health-check sem nome");
        match args.comando {
            Comando::HealthCheck { vps_nome } => {
                assert!(vps_nome.is_none());
            }
            outro => panic!("comando inesperado: {outro:?}"),
        }
    }

    #[test]
    fn test_parse_exec_json() {
        let args = Argumentos::try_parse_from(["ssh-cli", "exec", "vps1", "ls", "--json"])
            .expect("parser deve aceitar exec com --json");
        match args.comando {
            Comando::Exec {
                vps_nome,
                comando,
                json,
            } => {
                assert_eq!(vps_nome, "vps1");
                assert_eq!(comando, "ls");
                assert!(json);
            }
            outro => panic!("comando inesperado: {outro:?}"),
        }
    }
}
