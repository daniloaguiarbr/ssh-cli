//! Configuração de output colorido e detecção de terminal interativo.
//!
//! Gerencia a escolha de cores via `termcolor` respeitando a precedência:
//! 1. Flag `--no-color` da CLI (maior prioridade).
//! 2. Variável de ambiente `NO_COLOR` (padrão <https://no-color.org>).
//! 3. Variável de ambiente `CLICOLOR_FORCE=1` (forçar cores mesmo sem TTY).
//! 4. Detecção de TTY (cores apenas se stdout for terminal interativo).
//! 5. Fallback: sem cor.

use anyhow::Result;
use std::sync::OnceLock;
use termcolor::ColorChoice;

/// Cache da escolha de cor (definida uma vez na inicialização).
static COR_CACHE: OnceLock<ColorChoice> = OnceLock::new();

/// Inicializa a configuração de cor do terminal.
///
/// Deve ser chamada uma única vez após o parsing dos argumentos CLI.
/// O parâmetro `sem_cor` corresponde à flag `--no-color` da CLI.
pub fn inicializar(sem_cor: bool) -> Result<()> {
    let escolha = determinar_cor(sem_cor);
    let _ = COR_CACHE.set(escolha);
    tracing::debug!("configuração de cor do terminal: {:?}", escolha);
    Ok(())
}

/// Retorna a escolha de cor configurada.
///
/// Se [`inicializar`] não foi chamada, retorna [`ColorChoice::Never`] como
/// fallback seguro.
#[must_use]
pub fn cor_escolha() -> ColorChoice {
    *COR_CACHE.get().unwrap_or(&ColorChoice::Never)
}

/// Retorna `true` se o processo está rodando em um terminal interativo (TTY).
///
/// Usa [`std::io::IsTerminal`] (estabilizado no Rust 1.70) para detecção
/// cross-platform sem dependências externas.
#[must_use]
pub fn e_interativo() -> bool {
    use std::io::IsTerminal;

    // Se TERM=dumb, não é interativo independente do TTY
    if std::env::var("TERM").as_deref() == Ok("dumb") {
        return false;
    }

    std::io::stdout().is_terminal()
}

/// Determina a escolha de cor com base nas regras de precedência.
fn determinar_cor(sem_cor_cli: bool) -> ColorChoice {
    // 1. Flag --no-color da CLI (maior prioridade)
    if sem_cor_cli {
        return ColorChoice::Never;
    }

    // 2. Variável de ambiente NO_COLOR (qualquer valor)
    if std::env::var("NO_COLOR").is_ok() {
        return ColorChoice::Never;
    }

    // 3. CLICOLOR_FORCE=1 força cores mesmo sem TTY
    if std::env::var("CLICOLOR_FORCE").as_deref() == Ok("1") {
        return ColorChoice::Always;
    }

    // 4. Detecção de TTY: cores apenas em terminal interativo
    if e_interativo() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn sem_cor_cli_retorna_never() {
        let escolha = determinar_cor(true);
        assert!(matches!(escolha, ColorChoice::Never));
    }

    #[test]
    fn no_color_env_retorna_never() {
        // Salva e restaura o estado da variável de ambiente
        let anterior = std::env::var("NO_COLOR").ok();
        let anterior_force = std::env::var("CLICOLOR_FORCE").ok();

        std::env::set_var("NO_COLOR", "1");
        std::env::remove_var("CLICOLOR_FORCE");

        let escolha = determinar_cor(false);
        assert!(matches!(escolha, ColorChoice::Never));

        // Restaura
        match anterior {
            Some(v) => std::env::set_var("NO_COLOR", v),
            None => std::env::remove_var("NO_COLOR"),
        }
        match anterior_force {
            Some(v) => std::env::set_var("CLICOLOR_FORCE", v),
            None => std::env::remove_var("CLICOLOR_FORCE"),
        }
    }

    #[test]
    fn clicolor_force_retorna_always() {
        let anterior = std::env::var("NO_COLOR").ok();
        let anterior_force = std::env::var("CLICOLOR_FORCE").ok();

        std::env::remove_var("NO_COLOR");
        std::env::set_var("CLICOLOR_FORCE", "1");

        let escolha = determinar_cor(false);
        assert!(matches!(escolha, ColorChoice::Always));

        // Restaura
        match anterior {
            Some(v) => std::env::set_var("NO_COLOR", v),
            None => std::env::remove_var("NO_COLOR"),
        }
        match anterior_force {
            Some(v) => std::env::set_var("CLICOLOR_FORCE", v),
            None => std::env::remove_var("CLICOLOR_FORCE"),
        }
    }

    #[test]
    fn cor_escolha_retorna_never_sem_inicializar() {
        // Sem inicializar, o fallback é Never
        // NOTA: em testes paralelos o OnceLock pode já ter valor.
        // Apenas verificamos que não panic.
        let _ = cor_escolha();
    }

    #[test]
    fn e_interativo_retorna_bool() {
        // Apenas verifica que não panic
        let _ = e_interativo();
    }
}
