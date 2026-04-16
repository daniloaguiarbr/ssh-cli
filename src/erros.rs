//! Tipos de erro do ssh-cli.
//!
//! Define o enum [`ErroSshCli`] com todas as categorias de erro do domínio
//! usadas pela CLI.

use thiserror::Error;

/// Enum com todos os erros possíveis do ssh-cli.
#[derive(Debug, Error)]
pub enum ErroSshCli {
    /// Erro de I/O subjacente.
    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    /// Erro de serialização/deserialização JSON.
    #[error("erro de JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// Erro de deserialização TOML.
    #[error("erro de TOML (leitura): {0}")]
    TomlDe(#[from] toml::de::Error),

    /// Erro de serialização TOML.
    #[error("erro de TOML (escrita): {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// Erro de conexão SSH.
    #[error("erro de conexão SSH: {0}")]
    ConexaoSsh(String),

    /// Erro de autenticação SSH.
    #[error("erro de autenticação SSH: {0}")]
    AutenticacaoSsh(String),

    /// Falha ao estabelecer conexão TCP/SSH (passo anterior à autenticação).
    #[error("conexão SSH falhou: {0}")]
    ConexaoFalhou(String),

    /// Autenticação SSH rejeitada pelo servidor.
    #[error("autenticação SSH falhou")]
    AutenticacaoFalhou,

    /// Falha ao abrir ou operar um canal SSH.
    #[error("canal SSH falhou: {0}")]
    CanalFalhou(String),

    /// Timeout específico em operação SSH.
    #[error("timeout SSH após {0}ms")]
    TimeoutSsh(u64),

    /// Comando remoto terminou com código de saída diferente de zero.
    #[error("comando falhou com exit code {exit_code}: {stderr}")]
    ComandoFalhou {
        /// Código de saída retornado pelo comando remoto.
        exit_code: i32,
        /// Trecho (possivelmente truncado) de stderr.
        stderr: String,
    },

    /// VPS não encontrada no registro.
    #[error("VPS '{0}' não encontrada no registro")]
    VpsNaoEncontrada(String),

    /// VPS com nome duplicado no registro.
    #[error("VPS '{0}' já existe no registro")]
    VpsDuplicada(String),

    /// Arquivo local não encontrado.
    #[error("arquivo não encontrado: {0}")]
    ArquivoNaoEncontrado(String),

    /// Argumento inválido recebido via CLI.
    #[error("argumento inválido: {0}")]
    ArgumentoInvalido(String),

    /// Timeout excedido em operação.
    #[error("timeout excedido após {0}ms")]
    Timeout(u64),

    /// Erro de diretório XDG.
    #[error("diretório de configuração indisponível")]
    DiretorioXdg,

    /// Versão de schema incompatível.
    #[error("versão de schema incompatível: esperada {esperada}, encontrada {encontrada}")]
    SchemaIncompativel {
        /// Versão esperada.
        esperada: u32,
        /// Versão encontrada no arquivo.
        encontrada: u32,
    },

    /// Erro genérico não categorizado.
    #[error("erro: {0}")]
    Generico(String),
}

/// Exit codes padronizados conforme sysexits.h e convenções de sinais Unix.
pub mod exit_codes {
    /// Sucesso.
    pub const EX_OK: i32 = 0;
    /// Erro genérico de domínio.
    pub const EX_GENERAL: i32 = 1;
    /// Uso incorreto da CLI (argumento inválido).
    pub const EX_USAGE: i32 = 64;
    /// Dados de entrada inválidos.
    pub const EX_DATAERR: i32 = 65;
    /// Entrada não encontrada.
    pub const EX_NOINPUT: i32 = 66;
    /// Não foi possível criar saída.
    pub const EX_CANTCREAT: i32 = 73;
    /// Erro de I/O.
    pub const EX_IOERR: i32 = 74;
    /// Permissão negada.
    pub const EX_NOPERM: i32 = 77;
    /// Terminado por SIGINT (Ctrl+C).
    pub const EX_SIGINT: i32 = 130;
    /// Terminado por SIGTERM.
    pub const EX_SIGTERM: i32 = 143;
}

impl ErroSshCli {
    /// Retorna o exit code sysexits.h correspondente a este erro.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Io(_) => exit_codes::EX_IOERR,
            Self::Json(_) => exit_codes::EX_DATAERR,
            Self::TomlDe(_) => exit_codes::EX_DATAERR,
            Self::TomlSer(_) => exit_codes::EX_CANTCREAT,
            Self::ConexaoSsh(_) => exit_codes::EX_IOERR,
            Self::AutenticacaoSsh(_) => exit_codes::EX_IOERR,
            Self::ConexaoFalhou(_) => exit_codes::EX_IOERR,
            Self::AutenticacaoFalhou => exit_codes::EX_NOPERM,
            Self::CanalFalhou(_) => exit_codes::EX_IOERR,
            Self::TimeoutSsh(_) => exit_codes::EX_IOERR,
            Self::ComandoFalhou { exit_code, .. } => *exit_code,
            Self::VpsNaoEncontrada(_) => exit_codes::EX_NOINPUT,
            Self::VpsDuplicada(_) => exit_codes::EX_USAGE,
            Self::ArquivoNaoEncontrado(_) => exit_codes::EX_NOINPUT,
            Self::ArgumentoInvalido(_) => exit_codes::EX_USAGE,
            Self::Timeout(_) => exit_codes::EX_IOERR,
            Self::DiretorioXdg => exit_codes::EX_CANTCREAT,
            Self::SchemaIncompativel { .. } => exit_codes::EX_DATAERR,
            Self::Generico(_) => exit_codes::EX_GENERAL,
        }
    }
}

/// Alias de `Result` usando o tipo de erro do ssh-cli.
pub type ResultadoSshCli<T> = std::result::Result<T, ErroSshCli>;

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn vps_nao_encontrada_mensagem_contem_nome() {
        let erro = ErroSshCli::VpsNaoEncontrada("producao".into());
        assert!(erro.to_string().contains("producao"));
    }

    #[test]
    fn vps_duplicada_mensagem_contem_nome() {
        let erro = ErroSshCli::VpsDuplicada("vps-1".into());
        let msg = erro.to_string();
        assert!(msg.contains("vps-1"));
        assert!(msg.contains("já existe"));
    }

    #[test]
    fn erro_io_exibe_mensagem() {
        let erro = ErroSshCli::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "arquivo nao encontrado",
        ));
        let msg = erro.to_string();
        assert!(msg.contains("I/O") || msg.contains("arquivo nao encontrado"));
    }

    #[test]
    fn erro_toml_de_exibe_mensagem() {
        let toml_err = "invalid TOML".parse::<toml::Value>().unwrap_err();
        let erro = ErroSshCli::TomlDe(toml_err);
        let msg = erro.to_string();
        assert!(msg.contains("TOML") || msg.contains("leitura"));
    }

    #[test]
    fn erro_tipo_servidor_vps_nao_encontrada() {
        let erro = ErroSshCli::VpsNaoEncontrada("servidor-x".into());
        let msg = erro.to_string();
        assert!(msg.contains("servidor-x"));
        assert!(msg.contains("não encontrada") || msg.contains("not found"));
    }

    #[test]
    fn exit_code_io_retorna_ioerr() {
        let e = ErroSshCli::Io(std::io::Error::other("teste"));
        assert_eq!(e.exit_code(), exit_codes::EX_IOERR);
    }

    #[test]
    fn exit_code_autenticacao_falhou_retorna_noperm() {
        assert_eq!(
            ErroSshCli::AutenticacaoFalhou.exit_code(),
            exit_codes::EX_NOPERM
        );
    }

    #[test]
    fn exit_code_vps_nao_encontrada_retorna_noinput() {
        let e = ErroSshCli::VpsNaoEncontrada("teste".to_string());
        assert_eq!(e.exit_code(), exit_codes::EX_NOINPUT);
    }

    #[test]
    fn exit_code_comando_falhou_propaga_exit_code_remoto() {
        let e = ErroSshCli::ComandoFalhou {
            exit_code: 127,
            stderr: "not found".to_string(),
        };
        assert_eq!(e.exit_code(), 127);
    }

    #[test]
    fn exit_code_argumento_invalido_retorna_usage() {
        let e = ErroSshCli::ArgumentoInvalido("bad".to_string());
        assert_eq!(e.exit_code(), exit_codes::EX_USAGE);
    }

    #[test]
    fn exit_code_diretorio_xdg_retorna_cantcreat() {
        assert_eq!(
            ErroSshCli::DiretorioXdg.exit_code(),
            exit_codes::EX_CANTCREAT
        );
    }
}
