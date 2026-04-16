//! Mascaramento Unicode-safe de valores sensíveis (senhas, tokens).
//!
//! Regras:
//! - Valores com até 16 caracteres (inclusive): retorna `"***"`.
//! - Valores com mais de 16 caracteres: primeiros 12 + `...` + últimos 4.
//!
//! Usa `chars()` (e não indexação por bytes) para preservar grafemas Unicode.

/// Limite mínimo para mascarar (inclusive). Valores com comprimento menor ou
/// igual retornam `"***"`.
pub const LIMITE_MINIMO_MASCARAR: usize = 16;

/// Número de caracteres iniciais preservados.
pub const CHARS_INICIO: usize = 12;

/// Número de caracteres finais preservados.
pub const CHARS_FIM: usize = 4;

/// Mascara um valor sensível preservando início e fim.
///
/// # Exemplos
///
/// ```
/// use ssh_cli::mascaramento::mascarar;
///
/// assert_eq!(mascarar("curto"), "***");
/// assert_eq!(mascarar("1234567890abcdef"), "***"); // 16 chars
///
/// let longo = "0123456789abcdefghij";
/// assert_eq!(mascarar(longo), "0123456789ab...ghij");
/// ```
#[must_use]
pub fn mascarar(valor: &str) -> String {
    let total: usize = valor.chars().count();

    if total <= LIMITE_MINIMO_MASCARAR {
        return "***".to_string();
    }

    let inicio: String = valor.chars().take(CHARS_INICIO).collect();
    let fim: String = valor.chars().skip(total - CHARS_FIM).collect();

    format!("{inicio}...{fim}")
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn valor_vazio_retorna_triplo_asterisco() {
        assert_eq!(mascarar(""), "***");
    }

    #[test]
    fn valor_curto_retorna_triplo_asterisco() {
        assert_eq!(mascarar("abc"), "***");
    }

    #[test]
    fn valor_com_16_caracteres_retorna_triplo_asterisco() {
        assert_eq!(mascarar("1234567890abcdef"), "***");
    }

    #[test]
    fn valor_com_17_caracteres_mostra_inicio_e_fim() {
        let r = mascarar("1234567890abcdefg");
        assert_eq!(r, "1234567890ab...defg");
    }

    #[test]
    fn valor_longo_preserva_12_iniciais_e_4_finais() {
        let senha = "senha-secreta-muito-longa-aqui-123456";
        let r = mascarar(senha);
        assert!(r.starts_with("senha-secret"));
        assert!(r.ends_with("3456"));
        assert!(r.contains("..."));
    }

    #[test]
    fn valor_com_acentos_preserva_grafemas() {
        let acentuado = "ação-configuração-senha-segura-123";
        let r = mascarar(acentuado);
        assert!(r.starts_with("ação-configu"));
        assert!(r.contains("..."));
    }

    #[test]
    fn valor_com_unicode_nao_crasha() {
        let emojis = "🔒🔑🛡🔐✨🎉💎⚡🌟🔥🎨🚀🌈🍀🎯🎪🎭🎬🎮🎲";
        let _ = mascarar(emojis);
    }
}
