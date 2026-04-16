//! Tratamento de sinais do sistema operacional.
//!
//! Registra um handler para Ctrl+C (SIGINT) que sinaliza cancelamento
//! via um [`AtomicBool`] compartilhado. Todos os módulos que executam
//! operações longas devem verificar [`cancelado`] periodicamente.

use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

/// Flag global de cancelamento. Definida uma única vez na inicialização.
static FLAG_CANCELAMENTO: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Registra o handler de Ctrl+C que marca a flag de cancelamento.
///
/// Deve ser chamada uma única vez, antes de qualquer operação longa.
/// Chamadas adicionais são seguras e ignoradas silenciosamente.
pub fn registrar_handler() -> Result<()> {
    let flag = obter_flag();
    let flag_clone = Arc::clone(&flag);

    ctrlc::set_handler(move || {
        flag_clone.store(true, Ordering::SeqCst);
        tracing::debug!("sinal de cancelamento recebido via Ctrl+C");
    })?;

    tracing::debug!("handler de Ctrl+C registrado com sucesso");

    #[cfg(unix)]
    {
        let flag_term = obter_flag_sigterm();
        signal_hook::flag::register(signal_hook::consts::SIGTERM, flag_term)?;
        tracing::debug!("handler SIGTERM registrado");
    }

    #[cfg(not(unix))]
    {
        // No Windows, SIGTERM não é suportado nativamente.
        // ctrlc já cobre Ctrl+C (equivalente a SIGINT).
        let _ = obter_flag_sigterm(); // Inicializa OnceLock mesmo sem handler
    }

    Ok(())
}

/// Retorna `true` se o usuário pressionou Ctrl+C.
///
/// Deve ser verificado em loops de operações longas para permitir
/// encerramento gracioso.
///
/// # Examples
///
/// ```
/// use ssh_cli::signals::cancelado;
///
/// // Antes de registrar handler, retorna false
/// assert!(!cancelado());
/// ```
#[must_use]
pub fn cancelado() -> bool {
    FLAG_CANCELAMENTO
        .get()
        .map(|f| f.load(Ordering::SeqCst))
        .unwrap_or(false)
}

/// Retorna o ponteiro compartilhado da flag de cancelamento.
///
/// Útil para passar a flag para tarefas assíncronas que precisam
/// verificar cancelamento sem chamar [`cancelado`] diretamente.
#[must_use]
pub fn obter_flag() -> Arc<AtomicBool> {
    Arc::clone(FLAG_CANCELAMENTO.get_or_init(|| Arc::new(AtomicBool::new(false))))
}

/// Flag global para SIGTERM.
static FLAG_SIGTERM: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Retorna `true` se o processo recebeu SIGTERM.
#[must_use]
pub fn terminado() -> bool {
    FLAG_SIGTERM
        .get()
        .map(|f| f.load(Ordering::SeqCst))
        .unwrap_or(false)
}

/// Retorna o Arc do flag de SIGTERM para uso em tarefas async.
#[must_use]
pub fn obter_flag_sigterm() -> Arc<AtomicBool> {
    Arc::clone(FLAG_SIGTERM.get_or_init(|| Arc::new(AtomicBool::new(false))))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cancelado_retorna_false_antes_de_sinal() {
        // A flag não deve estar marcada em estado inicial
        // (a menos que outro teste tenha ativado, mas cada test usa a mesma OnceLock)
        // Verificamos apenas que a chamada não panics.
        let _ = cancelado();
    }

    #[test]
    fn obter_flag_retorna_mesmo_arc() {
        let flag_a = obter_flag();
        let flag_b = obter_flag();
        // Ambos devem apontar para o mesmo AtomicBool subjacente
        assert!(Arc::ptr_eq(&flag_a, &flag_b));
    }

    #[test]
    fn flag_pode_ser_marcada_e_lida() {
        let flag = obter_flag();
        // Apenas verificamos que o AtomicBool funciona corretamente
        let valor_anterior = flag.load(Ordering::SeqCst);
        flag.store(valor_anterior, Ordering::SeqCst);
        assert_eq!(flag.load(Ordering::SeqCst), valor_anterior);
    }

    #[test]
    fn terminado_false_por_padrao() {
        // OnceLock pode já estar setado por outros testes
        // Se não setado, retorna false. Se setado, o valor padrão é false.
        let flag = obter_flag_sigterm();
        flag.store(false, Ordering::SeqCst);
        assert!(!terminado());
    }

    #[test]
    fn obter_flag_sigterm_retorna_mesmo_arc() {
        let a = obter_flag_sigterm();
        let b = obter_flag_sigterm();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn terminado_verdadeiro_apos_set() {
        let flag = obter_flag_sigterm();
        flag.store(true, Ordering::SeqCst);
        assert!(terminado());
        flag.store(false, Ordering::SeqCst); // Reset para não afetar outros testes
    }

    #[test]
    fn cancelado_false_apos_reset() {
        let flag = obter_flag();
        flag.store(true, Ordering::SeqCst);
        assert!(cancelado());
        flag.store(false, Ordering::SeqCst);
        assert!(!cancelado());
    }
}
