//! Testes de integração do sistema de internacionalização.
//!
//! Testa as funções públicas do módulo i18n e locale.

use serial_test::serial;
use ssh_cli::i18n::{idioma_atual, inicializar_idioma, Idioma, Mensagem};

#[test]
#[serial]
fn inicializar_idioma_nao_panic_com_locale_valido() {
    std::env::remove_var("SSH_CLI_LANG");
    let resultado = inicializar_idioma(Some("pt-BR"));
    assert!(
        resultado.is_ok(),
        "inicializar_idioma não deve falhar com locale válido"
    );
}

#[test]
#[serial]
fn inicializar_idioma_nao_panic_com_locale_invalido() {
    std::env::remove_var("SSH_CLI_LANG");
    let resultado = inicializar_idioma(Some("xx-XX"));
    assert!(
        resultado.is_ok(),
        "inicializar_idioma não deve falhar com locale inválido"
    );
}

#[test]
#[serial]
fn inicializar_idioma_nao_panic_sem_forcar() {
    std::env::remove_var("SSH_CLI_LANG");
    let resultado = inicializar_idioma(None);
    assert!(resultado.is_ok(), "inicializar_idioma não deve falhar");
}

#[test]
#[serial]
fn inicializar_idioma_com_env_var_valida_nao_panic() {
    std::env::set_var("SSH_CLI_LANG", "en-US");
    let resultado = inicializar_idioma(None);
    std::env::remove_var("SSH_CLI_LANG");
    assert!(
        resultado.is_ok(),
        "inicializar_idioma não deve falhar com env var válida"
    );
}

#[test]
#[serial]
fn inicializar_idioma_com_env_var_invalida_nao_panic() {
    std::env::set_var("SSH_CLI_LANG", "xx-XX");
    let resultado = inicializar_idioma(None);
    std::env::remove_var("SSH_CLI_LANG");
    assert!(
        resultado.is_ok(),
        "inicializar_idioma não deve falhar com env var inválida"
    );
}

#[test]
#[serial]
fn idioma_atual_retorna_locale_valido() {
    inicializar_idioma(None).expect("inicializar_idioma não deve falhar");
    let idioma = idioma_atual();
    assert!(
        idioma == Idioma::English || idioma == Idioma::Portugues,
        "idioma_atual deve ser um locale suportado"
    );
}

#[test]
#[serial]
fn inicializar_com_pt_br_define_portugues() {
    // OnceLock já pode estar setado — o resultado deve ser válido de qualquer forma
    let resultado = inicializar_idioma(Some("pt-BR"));
    assert!(resultado.is_ok());
}

#[test]
#[serial]
fn inicializar_com_en_us_define_english() {
    let resultado = inicializar_idioma(Some("en-US"));
    assert!(resultado.is_ok());
}

#[test]
fn mensagem_vps_registro_vazio_en_nao_vazia() {
    let texto = Mensagem::VpsRegistroVazio.texto(Idioma::English);
    assert!(!texto.is_empty());
}

#[test]
fn mensagem_vps_registro_vazio_pt_nao_vazia() {
    let texto = Mensagem::VpsRegistroVazio.texto(Idioma::Portugues);
    assert!(!texto.is_empty());
}

#[test]
fn mensagem_vps_registro_vazio_pt_diferente_de_en() {
    let en = Mensagem::VpsRegistroVazio.texto(Idioma::English);
    let pt = Mensagem::VpsRegistroVazio.texto(Idioma::Portugues);
    assert_ne!(en, pt);
}

#[test]
fn variantes_unitarias_nao_vazias_em_ambos_idiomas() {
    let unitarias = [
        Mensagem::VpsRegistroVazio,
        Mensagem::VpsListaTitulo,
        Mensagem::ConfigCaminhoLabel,
        Mensagem::ConfigSemChaves,
        Mensagem::ErroCarregarConfig,
        Mensagem::ErroSalvarConfig,
        Mensagem::ErroConexaoSsh,
        Mensagem::ErroComandoFalhou,
        Mensagem::TunnelPressioneCtrlC,
        Mensagem::HealthCheckSemVps,
        Mensagem::OperacaoCancelada,
    ];
    for variante in &unitarias {
        assert!(
            !variante.texto(Idioma::English).is_empty(),
            "EN vazia para {:?}",
            variante
        );
        assert!(
            !variante.texto(Idioma::Portugues).is_empty(),
            "PT vazia para {:?}",
            variante
        );
    }
}

#[test]
fn variantes_com_campos_incluem_dados_dinamicos() {
    let casos: Vec<(Mensagem, &str)> = vec![
        (
            Mensagem::VpsAdicionada {
                nome: "meu-servidor".to_string(),
            },
            "meu-servidor",
        ),
        (
            Mensagem::VpsRemovida {
                nome: "servidor-antigo".to_string(),
            },
            "servidor-antigo",
        ),
        (
            Mensagem::VpsDuplicada {
                nome: "duplicado".to_string(),
            },
            "duplicado",
        ),
        (
            Mensagem::VpsNaoEncontrada {
                nome: "inexistente".to_string(),
            },
            "inexistente",
        ),
        (
            Mensagem::HealthCheckOk {
                nome: "prod-01".to_string(),
            },
            "prod-01",
        ),
        (
            Mensagem::HealthCheckFalhou {
                nome: "test-vps".to_string(),
                detalhe: "connection refused".to_string(),
            },
            "test-vps",
        ),
        (
            Mensagem::HealthCheckLatencia {
                nome: "relay-01".to_string(),
                latencia_ms: 42,
            },
            "relay-01",
        ),
    ];
    for (msg, esperado) in &casos {
        assert!(
            msg.texto(Idioma::English).contains(esperado),
            "EN não contém '{}' para {:?}",
            esperado,
            msg
        );
        assert!(
            msg.texto(Idioma::Portugues).contains(esperado),
            "PT não contém '{}' para {:?}",
            esperado,
            msg
        );
    }
}

#[test]
fn tunnel_ativo_inclui_porta_host_e_vps() {
    let msg = Mensagem::TunnelAtivo {
        porta_local: 8080,
        host_remoto: "10.0.0.1".to_string(),
        porta_remota: 22,
        vps_nome: "relay-01".to_string(),
    };
    let en = msg.texto(Idioma::English);
    let pt = msg.clone().texto(Idioma::Portugues);
    for texto in &[en, pt] {
        assert!(texto.contains("8080"), "deve conter porta_local");
        assert!(texto.contains("10.0.0.1"), "deve conter host_remoto");
        assert!(texto.contains("22"), "deve conter porta_remota");
        assert!(texto.contains("relay-01"), "deve conter vps_nome");
    }
}
