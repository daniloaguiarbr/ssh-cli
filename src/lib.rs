//! # ssh-cli
//!
//! CLI Rust full-stack que dá a uma LLM (Claude Code, Cursor, Windsurf) a capacidade
//! de operar servidores remotos via SSH em um fluxo de subprocesso via stdin/stdout.
//!
//! ## Módulos
//!
//! | Módulo          | Responsabilidade                                              |
//! |-----------------|---------------------------------------------------------------|
//! | `cli`           | Definição de argumentos via `clap` derive e dispatcher        |
//! | `vps`           | CRUD e persistência de registros de VPS (XDG + TOML + 0o600)  |
//! | `ssh`           | Cliente SSH (stub nesta iteração; real via `russh` em v2+)    |
//! | `i18n`          | Internacionalização com enum `Mensagem` bilíngue              |
//! | `locale`        | Detecção e resolução de locale do sistema operacional         |
//! | `platform`      | Ajustes de plataforma (UTF-8 Windows, detecção TTY)           |
//! | `mascaramento`  | Mascaramento Unicode-safe de valores sensíveis                |
//! | `erros`         | Tipos de erro estruturados via `thiserror`                    |
//! | `output`        | Único módulo autorizado a `println!` (formatação CRUD)        |
//! | `paths`         | Validação e normalização de caminhos (anti-traversal, NFC)    |
//! | `signals`       | Handler de Ctrl+C com flag de cancelamento via `AtomicBool`   |
//! | `terminal`      | Detecção de TTY e escolha de cor via `termcolor`              |
//!
//! ## Entry point
//!
//! A função pública [`run`] é o ponto de entrada chamado por `main.rs`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod cli;
pub mod erros;
pub mod i18n;
pub mod locale;
pub mod mascaramento;
pub mod output;
pub mod paths;
pub mod platform;
pub mod scp;
pub mod signals;
pub mod ssh;
pub mod terminal;
pub mod tunnel;
pub mod vps;

use anyhow::Result;

/// Executa o ssh-cli a partir dos argumentos da linha de comando.
///
/// Esta é a função pública chamada por `main.rs`. Ela:
/// 1. Registra o handler de Ctrl+C para cancelamento gracioso.
/// 2. Inicializa a plataforma (codepage Windows UTF-8, detecção de TTY).
/// 3. Faz parsing dos argumentos via clap.
/// 4. Inicializa logs via `tracing-subscriber`.
/// 5. Inicializa configuração de cor do terminal.
/// 6. Inicializa i18n com o idioma detectado.
/// 7. Despacha para o subcomando apropriado (`vps`, `connect`, `exec`, `sudo-exec`, `scp`, `tunnel`).
pub async fn run() -> Result<()> {
    signals::registrar_handler()?;

    platform::inicializar_plataforma()?;

    let argumentos = cli::parse_args();

    cli::inicializar_logs(&argumentos);

    terminal::inicializar(argumentos.no_color)?;

    i18n::inicializar_idioma(argumentos.lang.as_deref())?;

    cli::executar(argumentos).await
}
