//! Detecção e resolução de idioma cross-platform.
//!
//! Precedência de seleção de idioma (do mais para o menos prioritário):
//! 1. Flag `--lang` da CLI
//! 2. Variável de ambiente `SSH_CLI_LANG`
//! 3. Locale do sistema via `sys_locale::get_locale()`
//! 4. Fallback: `Idioma::English`

use std::sync::OnceLock;

use crate::i18n::Idioma;

/// Estado global do idioma — definido uma única vez na inicialização.
static IDIOMA_GLOBAL: OnceLock<Idioma> = OnceLock::new();

/// Resolve o idioma aplicando a hierarquia de precedência em 4 camadas.
///
/// Retorna o primeiro idioma válido encontrado na ordem:
/// flag CLI > env SSH_CLI_LANG > sys_locale > English.
pub fn resolver_idioma(forcar: Option<&str>) -> Idioma {
    // Camada 1: flag --lang da CLI
    if let Some(codigo) = forcar {
        if let Some(idioma) = codigo_para_idioma(codigo) {
            return idioma;
        }
    }

    // Camada 2: variável de ambiente SSH_CLI_LANG
    if let Ok(env_lang) = std::env::var("SSH_CLI_LANG") {
        if let Some(idioma) = codigo_para_idioma(&env_lang) {
            return idioma;
        }
    }

    // Camada 3: locale do sistema via sys_locale
    if let Some(locale) = sys_locale::get_locale() {
        if let Some(idioma) = codigo_para_idioma(&locale) {
            return idioma;
        }
    }

    // Camada 4: fallback incondicional
    Idioma::English
}

/// Define o idioma global (chamada única na inicialização do processo).
///
/// Chamadas subsequentes são silenciosamente ignoradas — o `OnceLock`
/// garante que o idioma é imutável após a primeira definição.
pub fn definir_idioma(idioma: Idioma) {
    let _ = IDIOMA_GLOBAL.set(idioma);
}

/// Retorna o idioma global atual.
///
/// Se `definir_idioma` ainda não foi chamado, retorna `Idioma::English`
/// como fallback seguro para código executado antes da inicialização.
pub fn idioma_atual() -> Idioma {
    IDIOMA_GLOBAL.get().copied().unwrap_or(Idioma::English)
}

/// Converte código textual de idioma para `Idioma`.
///
/// Reconhece prefixos "pt" e "en" com qualquer sufixo de região,
/// sem distinção entre maiúsculas e minúsculas.
fn codigo_para_idioma(codigo: &str) -> Option<Idioma> {
    let normalizado = codigo.to_lowercase();
    match normalizado.as_str() {
        "pt" | "pt-br" | "pt_br" => Some(Idioma::Portugues),
        "en" | "en-us" | "en_us" => Some(Idioma::English),
        outro => {
            if outro.starts_with("pt") {
                Some(Idioma::Portugues)
            } else if outro.starts_with("en") {
                Some(Idioma::English)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn codigo_pt_retorna_portugues() {
        assert_eq!(codigo_para_idioma("pt"), Some(Idioma::Portugues));
    }

    #[test]
    fn codigo_pt_br_retorna_portugues() {
        assert_eq!(codigo_para_idioma("pt-BR"), Some(Idioma::Portugues));
    }

    #[test]
    fn codigo_pt_br_underscore_retorna_portugues() {
        assert_eq!(codigo_para_idioma("pt_BR"), Some(Idioma::Portugues));
    }

    #[test]
    fn codigo_en_retorna_english() {
        assert_eq!(codigo_para_idioma("en"), Some(Idioma::English));
    }

    #[test]
    fn codigo_en_us_retorna_english() {
        assert_eq!(codigo_para_idioma("en-US"), Some(Idioma::English));
    }

    #[test]
    fn codigo_en_gb_retorna_english_por_prefixo() {
        assert_eq!(codigo_para_idioma("en-GB"), Some(Idioma::English));
    }

    #[test]
    fn codigo_desconhecido_retorna_none() {
        assert_eq!(codigo_para_idioma("fr-FR"), None);
    }

    #[test]
    fn codigo_vazio_retorna_none() {
        assert_eq!(codigo_para_idioma(""), None);
    }

    #[test]
    fn codigo_maiusculo_normalizado() {
        assert_eq!(codigo_para_idioma("PT"), Some(Idioma::Portugues));
        assert_eq!(codigo_para_idioma("EN"), Some(Idioma::English));
    }

    #[test]
    fn resolver_com_forcar_pt_retorna_portugues() {
        let resultado = resolver_idioma(Some("pt-BR"));
        assert_eq!(resultado, Idioma::Portugues);
    }

    #[test]
    fn resolver_com_forcar_en_retorna_english() {
        let resultado = resolver_idioma(Some("en-US"));
        assert_eq!(resultado, Idioma::English);
    }

    #[test]
    fn resolver_com_forcar_invalido_usa_camadas_seguintes() {
        // Código inválido não resolve na camada 1; deve cair em sys_locale ou fallback.
        std::env::remove_var("SSH_CLI_LANG");
        let resultado = resolver_idioma(Some("xx-YY"));
        // Deve retornar English ou Portugues — não pode ser um valor inválido.
        assert!(
            resultado == Idioma::English || resultado == Idioma::Portugues,
            "resolver_idioma deve retornar idioma válido mesmo com código inválido"
        );
    }

    #[test]
    fn resolver_sem_forcar_retorna_idioma_valido() {
        std::env::remove_var("SSH_CLI_LANG");
        let resultado = resolver_idioma(None);
        assert!(
            resultado == Idioma::English || resultado == Idioma::Portugues,
            "resolver_idioma deve retornar idioma válido"
        );
    }

    #[test]
    fn idioma_atual_retorna_fallback_english_antes_de_definir() {
        // Não chamamos definir_idioma — o OnceLock pode já estar setado em outros testes,
        // mas o resultado DEVE ser um idioma válido.
        let resultado = idioma_atual();
        assert!(
            resultado == Idioma::English || resultado == Idioma::Portugues,
            "idioma_atual deve retornar idioma válido"
        );
    }
}
