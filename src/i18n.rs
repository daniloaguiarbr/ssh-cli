//! Sistema de internacionalização do ssh-cli.
//!
//! Fornece o enum `Idioma` bilíngue com enum `Mensagem` como única fonte de
//! strings de UI. A detecção de locale é delegada ao módulo `locale`.
//!
//! Precedência de seleção de idioma:
//! 1. Flag `--lang` da CLI
//! 2. Variável de ambiente `SSH_CLI_LANG`
//! 3. Locale do sistema via `sys_locale::get_locale()`
//! 4. Fallback: `Idioma::English`

use anyhow::Result;

/// Idioma suportado pelo sistema de internacionalização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Idioma {
    /// Inglês americano (en-US) — idioma padrão.
    English,
    /// Português brasileiro (pt-BR).
    Portugues,
}

/// Todas as mensagens de UI do sistema.
///
/// ÚNICA fonte de strings visíveis ao usuário. Cada variante possui tradução
/// exaustiva em `en()` e `pt()`. PROIBIDO usar string literal de UI fora deste enum.
///
/// Variantes com campos dinâmicos (ex.: `{ nome: String }`) permitem incluir
/// dados contextuais na mensagem. Mensagem não implementa `Copy` pois campos
/// `String` não são `Copy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mensagem {
    // VPS
    /// Nenhuma VPS cadastrada no arquivo de configuração.
    VpsRegistroVazio,
    /// Cabeçalho da listagem de VPS registradas.
    VpsListaTitulo,
    /// VPS adicionada com sucesso ao registro.
    VpsAdicionada {
        /// Nome da VPS adicionada.
        nome: String,
    },
    /// VPS removida com sucesso do registro.
    VpsRemovida {
        /// Nome da VPS removida.
        nome: String,
    },
    /// Tentativa de adicionar VPS já existente no registro.
    VpsDuplicada {
        /// Nome da VPS duplicada.
        nome: String,
    },
    /// VPS solicitada não foi encontrada no registro.
    VpsNaoEncontrada {
        /// Nome da VPS não encontrada.
        nome: String,
    },
    /// VPS ativa selecionada para operações subsequentes.
    VpsAtivaSelecionada {
        /// Nome da VPS selecionada.
        nome: String,
    },
    // Config
    /// Rótulo do caminho do arquivo de configuração.
    ConfigCaminhoLabel,
    /// Caminho atual do arquivo de configuração.
    ConfigCaminho {
        /// Caminho absoluto do arquivo de configuração.
        caminho: String,
    },
    /// Nenhuma chave de API configurada no sistema.
    ConfigSemChaves,
    // Erros
    /// Falha ao carregar o arquivo de configuração.
    ErroCarregarConfig,
    /// Falha ao salvar o arquivo de configuração.
    ErroSalvarConfig,
    /// Erro ao estabelecer conexão SSH com o servidor remoto.
    ErroConexaoSsh,
    /// Falha na execução de comando remoto via SSH.
    ErroComandoFalhou,
    /// Argumento inválido fornecido à operação.
    ErroArgumentoInvalido {
        /// Detalhe do argumento inválido.
        detalhe: String,
    },
    /// Erro genérico com descrição textual.
    ErroGenerico {
        /// Descrição do erro.
        detalhe: String,
    },
    // Tunnel
    /// Tunnel SSH ativo com informações de porta e host.
    TunnelAtivo {
        /// Porta local do tunnel.
        porta_local: u16,
        /// Host remoto destino.
        host_remoto: String,
        /// Porta remota destino.
        porta_remota: u16,
        /// Nome da VPS usada como relay.
        vps_nome: String,
    },
    /// Instrução para encerrar o tunnel via Ctrl+C.
    TunnelPressioneCtrlC,
    // Health Check
    /// Verificação de conectividade com VPS bem-sucedida.
    HealthCheckOk {
        /// Nome da VPS verificada.
        nome: String,
    },
    /// Nenhuma VPS ativa selecionada para health check.
    HealthCheckSemVps,
    /// Falha na verificação de conectividade com VPS.
    HealthCheckFalhou {
        /// Nome da VPS verificada.
        nome: String,
        /// Detalhe do erro.
        detalhe: String,
    },
    /// Resultado de health check com latência.
    HealthCheckLatencia {
        /// Nome da VPS verificada.
        nome: String,
        /// Latência em milissegundos.
        latencia_ms: u64,
    },
    /// Operação cancelada por sinal do usuário (Ctrl+C ou SIGTERM).
    OperacaoCancelada,
}

impl Mensagem {
    /// Retorna a string da mensagem no idioma especificado.
    ///
    /// Método determinístico para uso em testes — não depende de estado global.
    pub fn texto(&self, idioma: Idioma) -> String {
        match idioma {
            Idioma::English => en(self),
            Idioma::Portugues => pt(self),
        }
    }
}

/// Inicializa o sistema de i18n detectando o locale do SO.
///
/// Se `forcar` for `Some(...)`, esse idioma sobrescreve a detecção automática.
pub fn inicializar_idioma(forcar: Option<&str>) -> Result<()> {
    let idioma = crate::locale::resolver_idioma(forcar);
    crate::locale::definir_idioma(idioma);
    Ok(())
}

/// Retorna o idioma atualmente configurado.
#[must_use]
pub fn idioma_atual() -> Idioma {
    crate::locale::idioma_atual()
}

/// Retorna a string da mensagem no idioma global atual.
///
/// Usa o estado global inicializado por `inicializar_idioma`.
/// Em testes, prefira `Mensagem::texto(idioma)` para determinismo.
///
/// # Examples
///
/// ```
/// use ssh_cli::i18n::{t, inicializar_idioma, Mensagem};
///
/// inicializar_idioma(Some("en-US")).unwrap();
/// let texto = t(Mensagem::VpsRegistroVazio);
/// assert!(!texto.is_empty());
/// ```
#[must_use]
pub fn t(msg: Mensagem) -> String {
    msg.texto(idioma_atual())
}

/// Traduções para inglês americano.
fn en(msg: &Mensagem) -> String {
    match msg {
        Mensagem::VpsRegistroVazio => "No VPS registered.".to_string(),
        Mensagem::VpsListaTitulo => "Registered VPS:".to_string(),
        Mensagem::VpsAdicionada { nome } => format!("VPS '{nome}' added successfully."),
        Mensagem::VpsRemovida { nome } => format!("VPS '{nome}' removed successfully."),
        Mensagem::VpsDuplicada { nome } => format!("VPS '{nome}' is already registered."),
        Mensagem::VpsNaoEncontrada { nome } => format!("VPS '{nome}' not found."),
        Mensagem::VpsAtivaSelecionada { nome } => format!("Active VPS: '{nome}'."),
        Mensagem::ConfigCaminhoLabel => "Configuration file:".to_string(),
        Mensagem::ConfigCaminho { caminho } => caminho.clone(),
        Mensagem::ConfigSemChaves => "No API keys configured.".to_string(),
        Mensagem::ErroCarregarConfig => "Failed to load configuration.".to_string(),
        Mensagem::ErroSalvarConfig => "Failed to save configuration.".to_string(),
        Mensagem::ErroConexaoSsh => "SSH connection error.".to_string(),
        Mensagem::ErroComandoFalhou => "Command execution failed.".to_string(),
        Mensagem::ErroArgumentoInvalido { detalhe } => format!("Invalid argument: {detalhe}"),
        Mensagem::ErroGenerico { detalhe } => detalhe.clone(),
        Mensagem::TunnelAtivo {
            porta_local,
            host_remoto,
            porta_remota,
            vps_nome,
        } => format!(
            "SSH tunnel active: localhost:{porta_local} -> {host_remoto}:{porta_remota} via {vps_nome}"
        ),
        Mensagem::TunnelPressioneCtrlC => "Press Ctrl+C to terminate.".to_string(),
        Mensagem::HealthCheckOk { nome } => format!("Health check passed for '{nome}'."),
        Mensagem::HealthCheckSemVps => {
            "No active VPS. Use 'ssh-cli connect <NAME>' first.".to_string()
        }
        Mensagem::HealthCheckFalhou { nome, detalhe } => {
            format!("Health check FAILED for '{nome}': {detalhe}")
        }
        Mensagem::HealthCheckLatencia { nome, latencia_ms } => {
            format!("Health check OK for '{nome}' ({latencia_ms}ms)")
        }
        Mensagem::OperacaoCancelada => "Operation cancelled by user.".to_string(),
    }
}

/// Traduções para português brasileiro.
fn pt(msg: &Mensagem) -> String {
    match msg {
        Mensagem::VpsRegistroVazio => "Nenhum VPS cadastrado.".to_string(),
        Mensagem::VpsListaTitulo => "VPS cadastrados:".to_string(),
        Mensagem::VpsAdicionada { nome } => format!("VPS '{nome}' adicionada com sucesso."),
        Mensagem::VpsRemovida { nome } => format!("VPS '{nome}' removida com sucesso."),
        Mensagem::VpsDuplicada { nome } => format!("VPS '{nome}' já está cadastrada."),
        Mensagem::VpsNaoEncontrada { nome } => format!("VPS '{nome}' não encontrada."),
        Mensagem::VpsAtivaSelecionada { nome } => format!("VPS ativa: '{nome}'."),
        Mensagem::ConfigCaminhoLabel => "Arquivo de configuração:".to_string(),
        Mensagem::ConfigCaminho { caminho } => caminho.clone(),
        Mensagem::ConfigSemChaves => "Nenhuma chave de API configurada.".to_string(),
        Mensagem::ErroCarregarConfig => "Falha ao carregar configuração.".to_string(),
        Mensagem::ErroSalvarConfig => "Falha ao salvar configuração.".to_string(),
        Mensagem::ErroConexaoSsh => "Erro de conexão SSH.".to_string(),
        Mensagem::ErroComandoFalhou => "Falha na execução do comando.".to_string(),
        Mensagem::ErroArgumentoInvalido { detalhe } => format!("Argumento inválido: {detalhe}"),
        Mensagem::ErroGenerico { detalhe } => detalhe.clone(),
        Mensagem::TunnelAtivo {
            porta_local,
            host_remoto,
            porta_remota,
            vps_nome,
        } => format!(
            "Tunnel SSH: localhost:{porta_local} -> {host_remoto}:{porta_remota} via {vps_nome}"
        ),
        Mensagem::TunnelPressioneCtrlC => "Pressione Ctrl+C para encerrar.".to_string(),
        Mensagem::HealthCheckOk { nome } => format!("Health check bem-sucedido para '{nome}'."),
        Mensagem::HealthCheckSemVps => {
            "Nenhuma VPS ativa. Use 'ssh-cli connect <NOME>' primeiro.".to_string()
        }
        Mensagem::HealthCheckFalhou { nome, detalhe } => {
            format!("Health check FALHOU para '{nome}': {detalhe}")
        }
        Mensagem::HealthCheckLatencia { nome, latencia_ms } => {
            format!("Health check OK para '{nome}' ({latencia_ms}ms)")
        }
        Mensagem::OperacaoCancelada => "Operação cancelada pelo usuário.".to_string(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn idioma_enum_e_copy() {
        let a = Idioma::English;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn mensagem_nao_e_copy_mas_e_clone() {
        let m = Mensagem::VpsAdicionada {
            nome: "vps-01".to_string(),
        };
        let m2 = m.clone();
        assert_eq!(m, m2);
    }

    #[test]
    fn vps_registro_vazio_en() {
        assert_eq!(
            Mensagem::VpsRegistroVazio.texto(Idioma::English),
            "No VPS registered."
        );
    }

    #[test]
    fn vps_registro_vazio_pt() {
        assert_eq!(
            Mensagem::VpsRegistroVazio.texto(Idioma::Portugues),
            "Nenhum VPS cadastrado."
        );
    }

    #[test]
    fn vps_adicionada_inclui_nome_en() {
        let msg = Mensagem::VpsAdicionada {
            nome: "prod-01".to_string(),
        };
        assert_eq!(
            msg.texto(Idioma::English),
            "VPS 'prod-01' added successfully."
        );
    }

    #[test]
    fn vps_adicionada_inclui_nome_pt() {
        let msg = Mensagem::VpsAdicionada {
            nome: "prod-01".to_string(),
        };
        assert_eq!(
            msg.texto(Idioma::Portugues),
            "VPS 'prod-01' adicionada com sucesso."
        );
    }

    #[test]
    fn vps_removida_inclui_nome() {
        let msg = Mensagem::VpsRemovida {
            nome: "dev-01".to_string(),
        };
        assert!(msg.texto(Idioma::English).contains("dev-01"));
        assert!(msg.texto(Idioma::Portugues).contains("dev-01"));
    }

    #[test]
    fn vps_duplicada_inclui_nome() {
        let msg = Mensagem::VpsDuplicada {
            nome: "staging".to_string(),
        };
        assert!(msg.texto(Idioma::English).contains("staging"));
        assert!(msg.texto(Idioma::Portugues).contains("staging"));
    }

    #[test]
    fn vps_nao_encontrada_inclui_nome() {
        let msg = Mensagem::VpsNaoEncontrada {
            nome: "inexistente".to_string(),
        };
        assert!(msg.texto(Idioma::English).contains("inexistente"));
        assert!(msg.texto(Idioma::Portugues).contains("inexistente"));
    }

    #[test]
    fn tunnel_ativo_inclui_todos_os_campos() {
        let msg = Mensagem::TunnelAtivo {
            porta_local: 8080,
            host_remoto: "1.2.3.4".to_string(),
            porta_remota: 22,
            vps_nome: "meu-servidor".to_string(),
        };
        let en = msg.texto(Idioma::English);
        assert!(en.contains("8080"));
        assert!(en.contains("1.2.3.4"));
        assert!(en.contains("22"));
        assert!(en.contains("meu-servidor"));
    }

    #[test]
    fn erro_argumento_invalido_inclui_detalhe() {
        let msg = Mensagem::ErroArgumentoInvalido {
            detalhe: "porta fora do intervalo".to_string(),
        };
        assert!(msg
            .texto(Idioma::English)
            .contains("porta fora do intervalo"));
        assert!(msg
            .texto(Idioma::Portugues)
            .contains("porta fora do intervalo"));
    }

    #[test]
    fn health_check_ok_inclui_nome() {
        let msg = Mensagem::HealthCheckOk {
            nome: "prod-01".to_string(),
        };
        assert!(msg.texto(Idioma::English).contains("prod-01"));
        assert!(msg.texto(Idioma::Portugues).contains("prod-01"));
    }

    #[test]
    fn todas_variantes_unitarias_en_nao_vazias() {
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
        for v in &unitarias {
            let texto = v.texto(Idioma::English);
            assert!(!texto.is_empty(), "EN vazia para {:?}", v);
        }
    }

    #[test]
    fn todas_variantes_unitarias_pt_nao_vazias() {
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
        for v in &unitarias {
            let texto = v.texto(Idioma::Portugues);
            assert!(!texto.is_empty(), "PT vazia para {:?}", v);
        }
    }

    #[test]
    fn traducoes_pt_diferentes_de_en_para_unitarias() {
        let pares = [
            (Mensagem::VpsRegistroVazio, Mensagem::VpsRegistroVazio),
            (Mensagem::ErroConexaoSsh, Mensagem::ErroConexaoSsh),
            (Mensagem::HealthCheckSemVps, Mensagem::HealthCheckSemVps),
            (Mensagem::OperacaoCancelada, Mensagem::OperacaoCancelada),
        ];
        for (a, b) in &pares {
            let en = a.texto(Idioma::English);
            let pt = b.texto(Idioma::Portugues);
            assert_ne!(en, pt, "EN == PT para {:?}", a);
        }
    }

    #[test]
    fn health_check_falhou_inclui_nome_e_detalhe() {
        let msg = Mensagem::HealthCheckFalhou {
            nome: "prod-01".to_string(),
            detalhe: "timeout".to_string(),
        };
        assert!(msg.texto(Idioma::English).contains("prod-01"));
        assert!(msg.texto(Idioma::English).contains("timeout"));
        assert!(msg.texto(Idioma::Portugues).contains("prod-01"));
        assert!(msg.texto(Idioma::Portugues).contains("timeout"));
    }

    #[test]
    fn health_check_latencia_inclui_nome_e_ms() {
        let msg = Mensagem::HealthCheckLatencia {
            nome: "relay-01".to_string(),
            latencia_ms: 42,
        };
        assert!(msg.texto(Idioma::English).contains("relay-01"));
        assert!(msg.texto(Idioma::English).contains("42"));
        assert!(msg.texto(Idioma::Portugues).contains("relay-01"));
        assert!(msg.texto(Idioma::Portugues).contains("42"));
    }

    #[test]
    fn inicializar_idioma_sem_forcar_nao_panic() {
        let resultado = inicializar_idioma(None);
        assert!(resultado.is_ok());
    }

    #[test]
    fn inicializar_idioma_com_pt_br_funciona() {
        let resultado = inicializar_idioma(Some("pt-BR"));
        assert!(resultado.is_ok());
    }

    #[test]
    fn idioma_atual_retorna_valor_valido() {
        let idioma = idioma_atual();
        assert!(idioma == Idioma::English || idioma == Idioma::Portugues);
    }
}
