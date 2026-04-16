//! Testes property-based para invariantes do ssh-cli.

use proptest::prelude::*;
use ssh_cli::mascaramento::mascarar;
use ssh_cli::paths::{normalizar_nfc, validar_nome, validar_sem_traversal};

proptest! {
    // ---------- mascarar ----------

    #[test]
    fn prop_mascarar_never_panics(s in "\\PC*") {
        let _ = mascarar(&s);
    }

    #[test]
    fn prop_mascarar_short_returns_stars(s in "\\PC{0,16}") {
        // Qualquer string com até 16 chars (contados em Unicode scalar values)
        // deve retornar "***".
        prop_assert_eq!(mascarar(&s), "***");
    }

    #[test]
    fn prop_mascarar_long_contains_ellipsis(s in "\\PC{17,100}") {
        let resultado = mascarar(&s);
        prop_assert!(
            resultado.contains("..."),
            "mascarar deve conter '...' para strings longas, obtido: {resultado}"
        );
    }

    #[test]
    fn prop_mascarar_never_returns_input_for_long(s in "\\PC{17,100}") {
        let resultado = mascarar(&s);
        prop_assert_ne!(
            resultado,
            s,
            "mascarar NUNCA deve retornar o valor original para strings longas"
        );
    }

    #[test]
    fn prop_mascarar_long_starts_with_first_12_chars(s in "\\PC{17,200}") {
        let resultado = mascarar(&s);
        let primeiros_12: String = s.chars().take(12).collect();
        prop_assert!(
            resultado.starts_with(&primeiros_12),
            "mascarar deve preservar os primeiros 12 chars, obtido: {resultado}"
        );
    }

    #[test]
    fn prop_mascarar_long_ends_with_last_4_chars(s in "\\PC{17,200}") {
        let resultado = mascarar(&s);
        let total = s.chars().count();
        let ultimos_4: String = s.chars().skip(total - 4).collect();
        prop_assert!(
            resultado.ends_with(&ultimos_4),
            "mascarar deve preservar os últimos 4 chars, obtido: {resultado}"
        );
    }

    // ---------- validar_nome ----------

    #[test]
    fn prop_validar_nome_never_panics(s in "\\PC*") {
        let _ = validar_nome(&s);
    }

    #[test]
    fn prop_validar_nome_rejects_slash(prefixo in "[a-z]{1,5}", sufixo in "[a-z]{1,5}") {
        // Nomes com '/' devem ser rejeitados.
        let nome = format!("{prefixo}/{sufixo}");
        prop_assert!(
            validar_nome(&nome).is_err(),
            "validar_nome deve rejeitar nome com '/': {nome}"
        );
    }

    #[test]
    fn prop_validar_nome_rejects_backslash(prefixo in "[a-z]{1,5}", sufixo in "[a-z]{1,5}") {
        let nome = format!("{prefixo}\\{sufixo}");
        prop_assert!(
            validar_nome(&nome).is_err(),
            "validar_nome deve rejeitar nome com '\\': {nome}"
        );
    }

    #[test]
    fn prop_validar_nome_rejects_null_byte(prefixo in "[a-z]{1,5}", sufixo in "[a-z]{1,5}") {
        let nome = format!("{prefixo}\0{sufixo}");
        prop_assert!(
            validar_nome(&nome).is_err(),
            "validar_nome deve rejeitar nome com byte nulo: {nome}"
        );
    }

    #[test]
    fn prop_validar_nome_rejects_traversal_substring(prefixo in "[a-z]{1,5}", sufixo in "[a-z]{1,5}") {
        // validar_nome rejeita qualquer string que contenha ".."
        let nome = format!("{prefixo}..{sufixo}");
        prop_assert!(
            validar_nome(&nome).is_err(),
            "validar_nome deve rejeitar nome contendo '..': {nome}"
        );
    }

    #[test]
    fn prop_validar_nome_accepts_alphanumeric_dash(s in "[a-zA-Z0-9\\-]{1,50}") {
        // Nomes alfanuméricos com hífen devem sempre ser aceitos.
        prop_assert!(
            validar_nome(&s).is_ok(),
            "validar_nome deve aceitar nome alfanumérico com hífen: {s}"
        );
    }

    // ---------- validar_sem_traversal ----------

    #[test]
    fn prop_validar_sem_traversal_never_panics(s in "\\PC*") {
        let _ = validar_sem_traversal(&s);
    }

    #[test]
    fn prop_validar_sem_traversal_rejects_dotdot_segment(
        prefixo in "[a-z]{0,5}",
        sufixo in "[a-z]{1,5}"
    ) {
        // Um segmento exatamente ".." separado por '/' deve ser rejeitado.
        let caminho = format!("{prefixo}/../{sufixo}");
        prop_assert!(
            validar_sem_traversal(&caminho).is_err(),
            "validar_sem_traversal deve rejeitar caminho com '../': {caminho}"
        );
    }

    #[test]
    fn prop_validar_sem_traversal_accepts_simple_path(s in "[a-z][a-z0-9\\-]{0,20}") {
        // Um segmento simples sem separadores deve ser aceito.
        prop_assert!(
            validar_sem_traversal(&s).is_ok(),
            "validar_sem_traversal deve aceitar caminho simples: {s}"
        );
    }

    // ---------- normalizar_nfc ----------

    #[test]
    fn prop_normalizar_nfc_never_panics(s in "\\PC*") {
        let _ = normalizar_nfc(&s);
    }

    #[test]
    fn prop_normalizar_nfc_idempotent(s in "\\PC*") {
        let once = normalizar_nfc(&s);
        let twice = normalizar_nfc(&once);
        prop_assert_eq!(
            once,
            twice,
            "normalizar_nfc deve ser idempotente"
        );
    }
}
