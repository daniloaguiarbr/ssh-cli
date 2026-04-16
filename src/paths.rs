//! Validação e normalização de caminhos de arquivo.
//!
//! Fornece funções para validar nomes de arquivo de forma segura e
//! cross-platform, prevenindo path traversal, nomes reservados do Windows
//! e caracteres proibidos.

use anyhow::{bail, Result};
use unicode_normalization::UnicodeNormalization;

/// Nomes reservados pelo sistema de arquivos do Windows (case-insensitive).
const NOMES_RESERVADOS_WINDOWS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Caracteres proibidos em nomes de arquivo (proibidos no Windows ou problemáticos
/// em sistemas Unix).
const CHARS_PROIBIDOS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];

/// Valida um nome de arquivo (sem separadores de caminho).
///
/// Rejeita:
/// - Strings vazias.
/// - Nomes com componentes `..` (path traversal).
/// - Caracteres proibidos.
/// - Nomes reservados do Windows (case-insensitive).
/// - Nomes que terminam com ponto ou espaço (problemáticos no Windows).
///
/// # Examples
///
/// ```
/// use ssh_cli::paths::validar_nome;
///
/// assert!(validar_nome("meu-servidor").is_ok());
/// assert!(validar_nome("../etc/passwd").is_err());
/// assert!(validar_nome("CON").is_err());
/// ```
pub fn validar_nome(nome: &str) -> Result<()> {
    if nome.is_empty() {
        bail!("nome de arquivo não pode ser vazio");
    }

    if nome.contains("..") {
        bail!("nome de arquivo contém componente de path traversal: '{nome}'");
    }

    for c in CHARS_PROIBIDOS {
        if nome.contains(*c) {
            bail!(
                "nome de arquivo contém caractere proibido '{}': '{nome}'",
                c.escape_default()
            );
        }
    }

    let nome_upper = nome.to_uppercase();
    // Verifica também sem extensão (ex.: "NUL.txt" é proibido no Windows)
    let raiz = nome_upper.split('.').next().unwrap_or(&nome_upper);
    if NOMES_RESERVADOS_WINDOWS.contains(&raiz) {
        bail!("nome de arquivo usa nome reservado do Windows: '{nome}'");
    }

    if nome.ends_with('.') || nome.ends_with(' ') {
        bail!("nome de arquivo não pode terminar com ponto ou espaço: '{nome}'");
    }

    Ok(())
}

/// Normaliza um nome de arquivo para a forma NFC do Unicode.
///
/// A normalização NFC é necessária para garantir comparações consistentes
/// entre diferentes sistemas operacionais (macOS usa NFD, Linux usa NFC).
#[must_use]
pub fn normalizar_nfc(nome: &str) -> String {
    nome.nfc().collect()
}

/// Valida e normaliza um nome de arquivo em uma única operação.
///
/// Retorna o nome normalizado para NFC se passar em todas as validações.
pub fn validar_e_normalizar(nome: &str) -> Result<String> {
    validar_nome(nome)?;
    Ok(normalizar_nfc(nome))
}

/// Valida que um caminho não contém componentes de path traversal.
///
/// Verifica todos os segmentos do caminho separados por `/` ou `\`.
pub fn validar_sem_traversal(caminho: &str) -> Result<()> {
    if caminho.is_empty() {
        bail!("caminho não pode ser vazio");
    }

    let segmentos = caminho.split(['/', '\\']);
    for segmento in segmentos {
        if segmento == ".." {
            bail!("caminho contém componente de path traversal: '{caminho}'");
        }
    }

    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nome_valido_comum_passa() {
        assert!(validar_nome("meu-servidor").is_ok());
        assert!(validar_nome("vps_01").is_ok());
        assert!(validar_nome("servidor.produção").is_ok());
    }

    #[test]
    fn nome_vazio_rejeitado() {
        assert!(validar_nome("").is_err());
    }

    #[test]
    fn path_traversal_rejeitado() {
        assert!(validar_nome("..").is_err());
        assert!(validar_nome("../etc/passwd").is_err());
        assert!(validar_nome("foo/../bar").is_err());
    }

    #[test]
    fn chars_proibidos_rejeitados() {
        assert!(validar_nome("foo/bar").is_err());
        assert!(validar_nome("foo\\bar").is_err());
        assert!(validar_nome("foo:bar").is_err());
        assert!(validar_nome("foo*bar").is_err());
        assert!(validar_nome("foo?bar").is_err());
    }

    #[test]
    fn nomes_reservados_windows_rejeitados() {
        assert!(validar_nome("CON").is_err());
        assert!(validar_nome("con").is_err());
        assert!(validar_nome("NUL.txt").is_err());
        assert!(validar_nome("COM1").is_err());
        assert!(validar_nome("LPT9").is_err());
    }

    #[test]
    fn nome_terminando_com_ponto_rejeitado() {
        assert!(validar_nome("arquivo.").is_err());
    }

    #[test]
    fn nome_terminando_com_espaco_rejeitado() {
        assert!(validar_nome("arquivo ").is_err());
    }

    #[test]
    fn normalizar_nfc_retorna_string() {
        let resultado = normalizar_nfc("servidor");
        assert_eq!(resultado, "servidor");
    }

    #[test]
    fn validar_e_normalizar_retorna_string_valida() {
        let resultado = validar_e_normalizar("meu-servidor").unwrap();
        assert_eq!(resultado, "meu-servidor");
    }

    #[test]
    fn validar_sem_traversal_aceita_caminho_normal() {
        assert!(validar_sem_traversal("/home/usuario/arquivo.txt").is_ok());
        assert!(validar_sem_traversal("relative/path/file.txt").is_ok());
    }

    #[test]
    fn validar_sem_traversal_rejeita_traversal() {
        assert!(validar_sem_traversal("/home/../etc/passwd").is_err());
        assert!(validar_sem_traversal("../secreto").is_err());
    }

    #[test]
    fn validar_sem_traversal_rejeita_vazio() {
        assert!(validar_sem_traversal("").is_err());
    }

    #[test]
    fn nome_com_acentos_brasileiros_valido() {
        assert!(validar_nome("produção").is_ok());
        assert!(validar_nome("ação-configuração").is_ok());
    }

    #[test]
    fn nome_com_unicode_cjk_valido() {
        assert!(validar_nome("server-\u{4e16}\u{754c}").is_ok());
    }

    #[test]
    fn nome_com_emoji_valido() {
        assert!(validar_nome("server-\u{1f680}").is_ok());
    }

    #[test]
    fn nome_windows_reservado_case_misto_rejeitado() {
        assert!(validar_nome("cOn").is_err());
        assert!(validar_nome("Nul").is_err());
        assert!(validar_nome("lPt1").is_err());
    }

    #[test]
    fn normalizar_nfc_converte_nfd_para_nfc() {
        let nfd = "e\u{0301}"; // e + combining acute
        let nfc = "\u{00e9}"; // é precomposed
        assert_eq!(normalizar_nfc(nfd), nfc);
    }

    #[test]
    fn normalizar_nfc_preserva_nfc() {
        let nfc = "\u{00e9}";
        assert_eq!(normalizar_nfc(nfc), nfc);
    }

    #[test]
    fn normalizar_nfc_idempotente() {
        let input = "cafe\u{0301}";
        let once = normalizar_nfc(input);
        let twice = normalizar_nfc(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn validar_e_normalizar_nfd_converte() {
        let resultado = validar_e_normalizar("cafe\u{0301}").unwrap();
        assert_eq!(resultado, "caf\u{00e9}");
    }

    #[test]
    fn validar_sem_traversal_com_backslash_rejeitado() {
        assert!(validar_sem_traversal("foo\\..\\bar").is_err());
    }

    #[test]
    fn validar_sem_traversal_dot_solo_aceita() {
        assert!(validar_sem_traversal("./arquivo").is_ok());
    }
}
