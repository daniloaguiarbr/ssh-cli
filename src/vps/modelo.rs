//! Modelo de dados `VpsRegistro`.
//!
//! Senhas usam `SecretString` para zeroize automático via `Drop`. O TOML
//! gravado em disco contém a senha em texto claro (protegido por `chmod 0o600`).
//! `Debug` é customizado para NUNCA expor valores sensíveis.

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

/// Versão atual do schema do arquivo `config.toml`.
pub const SCHEMA_VERSION_ATUAL: u32 = 1;

/// Timeout padrão em milissegundos para operações SSH.
pub const TIMEOUT_PADRAO_MS: u64 = 30_000;

/// Limite padrão de caracteres em output capturado.
pub const MAX_CHARS_PADRAO: usize = 100_000;

/// Registro de uma VPS no arquivo de configuração.
#[derive(Clone, Serialize, Deserialize)]
pub struct VpsRegistro {
    /// Nome lógico único da VPS.
    pub nome: String,
    /// Hostname ou IP do servidor.
    pub host: String,
    /// Porta SSH.
    pub porta: u16,
    /// Usuário SSH.
    pub usuario: String,
    /// Senha SSH (em memória como `SecretString`).
    #[serde(with = "secret_string_serde")]
    pub senha: SecretString,
    /// Timeout em milissegundos.
    pub timeout_ms: u64,
    /// Limite de caracteres em output.
    pub max_chars: usize,
    /// Senha para `sudo` (opcional).
    #[serde(default, with = "opcao_secret_string_serde")]
    pub senha_sudo: Option<SecretString>,
    /// Senha para `su -` (opcional).
    #[serde(default, with = "opcao_secret_string_serde")]
    pub senha_su: Option<SecretString>,
    /// Versão do schema deste registro.
    pub schema_version: u32,
    /// Timestamp RFC 3339 de inclusão.
    pub adicionado_em: String,
}

impl std::fmt::Debug for VpsRegistro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VpsRegistro")
            .field("nome", &self.nome)
            .field("host", &self.host)
            .field("porta", &self.porta)
            .field("usuario", &self.usuario)
            .field("senha", &"<redacted>")
            .field("timeout_ms", &self.timeout_ms)
            .field("max_chars", &self.max_chars)
            .field(
                "senha_sudo",
                &self.senha_sudo.as_ref().map(|_| "<redacted>"),
            )
            .field("senha_su", &self.senha_su.as_ref().map(|_| "<redacted>"))
            .field("schema_version", &self.schema_version)
            .field("adicionado_em", &self.adicionado_em)
            .finish()
    }
}

impl VpsRegistro {
    /// Cria um novo registro aplicando defaults para timeout e `max_chars`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn novo(
        nome: String,
        host: String,
        porta: u16,
        usuario: String,
        senha: SecretString,
        timeout_ms: Option<u64>,
        max_chars: Option<usize>,
        senha_sudo: Option<SecretString>,
        senha_su: Option<SecretString>,
    ) -> Self {
        Self {
            nome,
            host,
            porta,
            usuario,
            senha,
            timeout_ms: timeout_ms.unwrap_or(TIMEOUT_PADRAO_MS),
            max_chars: max_chars.unwrap_or(MAX_CHARS_PADRAO),
            senha_sudo,
            senha_su,
            schema_version: SCHEMA_VERSION_ATUAL,
            adicionado_em: chrono::Utc::now().to_rfc3339(),
        }
    }
}

mod secret_string_serde {
    use super::{ExposeSecret, SecretString};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(valor: &SecretString, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(valor.expose_secret())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SecretString, D::Error> {
        let s = String::deserialize(d)?;
        Ok(SecretString::from(s))
    }
}

mod opcao_secret_string_serde {
    use super::{ExposeSecret, SecretString};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(valor: &Option<SecretString>, s: S) -> Result<S::Ok, S::Error> {
        match valor {
            Some(v) => s.serialize_some(v.expose_secret()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<SecretString>, D::Error> {
        let opt = Option::<String>::deserialize(d)?;
        Ok(opt.map(SecretString::from))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn novo_registro_aplica_defaults() {
        let r = VpsRegistro::novo(
            "teste".into(),
            "1.2.3.4".into(),
            22,
            "root".into(),
            SecretString::from("senha".to_string()),
            None,
            None,
            None,
            None,
        );
        assert_eq!(r.timeout_ms, TIMEOUT_PADRAO_MS);
        assert_eq!(r.max_chars, MAX_CHARS_PADRAO);
        assert_eq!(r.schema_version, SCHEMA_VERSION_ATUAL);
        assert!(!r.adicionado_em.is_empty());
    }

    #[test]
    fn debug_nao_exibe_senha() {
        let r = VpsRegistro::novo(
            "t".into(),
            "h".into(),
            22,
            "u".into(),
            SecretString::from("senha-super-secreta".to_string()),
            None,
            None,
            None,
            None,
        );
        let dbg = format!("{r:?}");
        assert!(!dbg.contains("senha-super-secreta"));
        assert!(dbg.contains("redacted"));
    }

    #[test]
    fn round_trip_toml_preserva_dados() {
        let r = VpsRegistro::novo(
            "producao".into(),
            "srv.exemplo.com".into(),
            2222,
            "admin".into(),
            SecretString::from("senha-do-admin-longa".to_string()),
            Some(5000),
            Some(50_000),
            Some(SecretString::from("sudopass".to_string())),
            None,
        );
        let toml_str = toml::to_string(&r).expect("serializar");
        let r2: VpsRegistro = toml::from_str(&toml_str).expect("deserializar");
        assert_eq!(r2.nome, "producao");
        assert_eq!(r2.porta, 2222);
        assert_eq!(r2.senha.expose_secret(), "senha-do-admin-longa");
        assert_eq!(
            r2.senha_sudo
                .as_ref()
                .map(|s| s.expose_secret().to_string()),
            Some("sudopass".to_string())
        );
        assert!(r2.senha_su.is_none());
    }
}
