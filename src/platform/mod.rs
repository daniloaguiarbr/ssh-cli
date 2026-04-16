//! Abstrações condicionais por sistema operacional.
//!
//! A inicialização de plataforma ([`inicializar_plataforma`]) é a PRIMEIRA operação
//! executada no `main()`. Ela configura:
//!
//! - **Windows**: codepage UTF-8 (65001) via `SetConsoleOutputCP` e `SetConsoleCP`
//! - **Linux**: detecção de sandbox (Flatpak/Snap) e caminhos XDG
//! - **macOS**: resolução de caminhos de config em `~/Library/Application Support`

use anyhow::Result;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// Inicializa a plataforma antes de qualquer I/O.
///
/// DEVE ser chamado como a primeira operação em `main()`.
pub fn inicializar_plataforma() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::configurar_codepage_utf8()?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::detectar_sandbox();
    }

    #[cfg(target_os = "macos")]
    {
        macos::inicializar();
    }

    Ok(())
}

/// Normaliza uma linha de stdin removendo `\r` final (CRLF → LF).
///
/// Necessário no Windows onde pipes podem emitir `\r\n`.
#[must_use]
pub fn normalizar_linha_stdin(linha: &str) -> &str {
    // Remove qualquer combinação de CR/LF do final.
    linha.trim_end_matches(['\r', '\n'])
}

/// Retorna `true` se stdout está conectado a um terminal (TTY).
#[must_use]
pub fn e_tty() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdout())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn normalizar_remove_cr_final() {
        assert_eq!(normalizar_linha_stdin("teste\r"), "teste");
        assert_eq!(normalizar_linha_stdin("teste\r\n"), "teste");
        assert_eq!(normalizar_linha_stdin("teste\n"), "teste");
        assert_eq!(normalizar_linha_stdin("teste"), "teste");
    }

    #[test]
    fn normalizar_string_vazia() {
        assert_eq!(normalizar_linha_stdin(""), "");
    }

    #[test]
    fn normalizar_apenas_newlines() {
        assert_eq!(normalizar_linha_stdin("\n\n\n"), "");
    }

    #[test]
    fn normalizar_mistos_crlf_lf() {
        assert_eq!(
            normalizar_linha_stdin("linha1\r\nlinha2\r\nlinha3"),
            "linha1\r\nlinha2\r\nlinha3"
        );
    }

    #[test]
    fn normalizar_com_espacos() {
        assert_eq!(
            normalizar_linha_stdin("texto com espacos  \r\n"),
            "texto com espacos  "
        );
    }

    #[test]
    fn e_tty_retorna_bool() {
        let _ = e_tty();
    }
}
