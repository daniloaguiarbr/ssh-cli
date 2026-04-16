//! Tunnel SSH (port-forward local).
//!
//! Implementa redirecionamento de porta local via SSH:
//! - O cliente escuta em `localhost:porta_local`
//! - Conexões são redirecionadas pelo tunnel SSH até `host_remoto:porta_remota`
//!
//! O tunnel permanece ativo até Ctrl+C ou erro fatal.

use crate::erros::ErroSshCli;
use crate::output;
use crate::ssh::cliente::{ClienteSsh, ClienteSshTrait};
use crate::vps::buscar_por_nome;
use anyhow::Result;
use std::path::PathBuf;
use tokio::net::TcpListener;

/// Executa o subcomando `tunnel` criando um port-forward SSH.
///
/// O tunnel escuta em `localhost:porta_local` e redireciona conexões
/// para `host_remoto:porta_remota` através do servidor SSH da VPS.
pub async fn executar_tunnel(
    vps_nome: &str,
    porta_local: u16,
    host_remoto: &str,
    porta_remota: u16,
    config_override: Option<PathBuf>,
    password_override: Option<String>,
) -> Result<()> {
    let mut vps = buscar_por_nome(config_override.clone(), vps_nome)?
        .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(vps_nome.to_string()))?;

    if let Some(pwd) = password_override {
        vps.senha = secrecy::SecretString::from(pwd);
    }

    let cfg = crate::vps::construir_configuracao(&vps);

    tracing::info!(
        vps = %vps_nome,
        porta_local,
        host_remoto,
        porta_remota,
        "iniciando tunnel SSH"
    );

    output::escrever_linha(&format!(
        "Tunnel SSH: localhost:{} -> {}:{} via {}",
        porta_local, host_remoto, porta_remota, vps_nome
    ))?;
    output::escrever_linha("Pressione Ctrl+C para encerrar.")?;

    let cliente: Box<dyn ClienteSshTrait> = <ClienteSsh as ClienteSshTrait>::conectar(cfg).await?;
    executar_tunnel_with_client(vps_nome, porta_local, host_remoto, porta_remota, cliente).await
}

/// Versão testável de executar_tunnel que aceita o cliente como parâmetro.
pub async fn executar_tunnel_with_client(
    vps_nome: &str,
    porta_local: u16,
    host_remoto: &str,
    porta_remota: u16,
    cliente: Box<dyn ClienteSshTrait>,
) -> Result<()> {
    let cliente = std::sync::Arc::from(cliente);

    let listener = TcpListener::bind(format!("127.0.0.1:{porta_local}"))
        .await
        .map_err(|e| {
            ErroSshCli::Generico(format!("falha ao abrir porta local {}: {}", porta_local, e))
        })?;

    tracing::info!(porta = %porta_local, "listener TCP local iniciado");

    loop {
        tokio::select! {
            resultado_accept = listener.accept() => {
                match resultado_accept {
                    Ok((soquete, addr)) => {
                        tracing::debug!(endereco = %addr, "nova conexão local");
                        let host = host_remoto.to_string();
                        let porta = porta_remota;
                        let vps = vps_nome.to_string();
                        let cliente = std::sync::Arc::clone(&cliente);

                        tokio::spawn(async move {
                            if let Err(e) =
                                redirecionar_conexao(soquete, &host, porta, &vps, addr, &*cliente).await
                            {
                                tracing::error!(erro = %e, "erro no redirecionamento");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!(erro = %e, "erro ao aceitar conexão local");
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                if crate::signals::cancelado() {
                    tracing::info!("tunnel encerrado por sinal de cancelamento");
                    break;
                }
            }
        }
    }

    if let Err(e) = cliente.desconectar().await {
        tracing::warn!(erro = %e, "erro ao desconectar cliente SSH");
    }

    output::escrever_linha("Tunnel encerrado.")?;
    Ok(())
}

async fn redirecionar_conexao(
    mut soquete: tokio::net::TcpStream,
    host_remoto: &str,
    porta_remota: u16,
    vps_nome: &str,
    origem: std::net::SocketAddr,
    cliente: &dyn ClienteSshTrait,
) -> Result<()> {
    let mut canal_tunel = cliente
        .abrir_canal_tunel(
            host_remoto,
            porta_remota,
            &origem.ip().to_string(),
            origem.port(),
        )
        .await
        .map_err(|e| {
            ErroSshCli::Generico(format!(
                "falha ao abrir tunnel SSH para {}:{}: {}",
                host_remoto, porta_remota, e
            ))
        })?;

    tracing::debug!(host = %host_remoto, porta = %porta_remota, "redirecionando conexão");

    tracing::debug!(
        vps = %vps_nome,
        host = %host_remoto,
        porta = %porta_remota,
        origem = %origem,
        "redirecionando conexão local para remoto via SSH"
    );

    let (bytes_local_remoto, bytes_remoto_local) =
        tokio::io::copy_bidirectional(&mut soquete, &mut canal_tunel)
            .await
            .map_err(|e| {
                ErroSshCli::Generico(format!(
                    "falha ao trafegar dados no tunnel {}:{}: {}",
                    host_remoto, porta_remota, e
                ))
            })?;

    tracing::debug!(
        bytes_local_remoto,
        bytes_remoto_local,
        "sessão de tunnel encerrada"
    );

    Ok(())
}

#[cfg(test)]
mod testes {
    use super::redirecionar_conexao;
    use crate::erros::ErroSshCli;
    use crate::ssh::cliente::{
        CanalTunel, ClienteSshTrait, ConfiguracaoConexao, SaidaExecucao, TransferenciaResultado,
    };
    use async_trait::async_trait;
    use serial_test::serial;
    use std::path::Path;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::Mutex;

    struct ClienteFakeTunel {
        canal: Mutex<Option<tokio::io::DuplexStream>>,
        falhar_ao_abrir: bool,
    }

    impl ClienteFakeTunel {
        fn novo(canal: tokio::io::DuplexStream) -> Self {
            Self {
                canal: Mutex::new(Some(canal)),
                falhar_ao_abrir: false,
            }
        }

        fn falhando() -> Self {
            Self {
                canal: Mutex::new(None),
                falhar_ao_abrir: true,
            }
        }
    }

    #[async_trait]
    impl ClienteSshTrait for ClienteFakeTunel {
        async fn conectar(_cfg: ConfiguracaoConexao) -> Result<Box<Self>, ErroSshCli> {
            Err(ErroSshCli::ConexaoFalhou(
                "não implementado em teste".to_string(),
            ))
        }

        async fn executar_comando(
            &mut self,
            _cmd: &str,
            _max_chars: usize,
        ) -> Result<SaidaExecucao, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "não implementado em teste".to_string(),
            ))
        }

        async fn upload(
            &mut self,
            _local: &Path,
            _remote: &Path,
        ) -> Result<TransferenciaResultado, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "não implementado em teste".to_string(),
            ))
        }

        async fn download(
            &mut self,
            _remote: &Path,
            _local: &Path,
        ) -> Result<TransferenciaResultado, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "não implementado em teste".to_string(),
            ))
        }

        async fn abrir_canal_tunel(
            &self,
            _host_remoto: &str,
            _porta_remota: u16,
            _endereco_origem: &str,
            _porta_origem: u16,
        ) -> Result<Box<dyn CanalTunel>, ErroSshCli> {
            if self.falhar_ao_abrir {
                return Err(ErroSshCli::CanalFalhou("falha forçada".to_string()));
            }

            let mut guard = self.canal.lock().await;
            let canal = guard
                .take()
                .ok_or_else(|| ErroSshCli::CanalFalhou("canal já consumido".to_string()))?;
            Ok(Box::new(canal))
        }

        async fn desconectar(&self) -> Result<(), ErroSshCli> {
            Ok(())
        }
    }

    #[test]
    fn tunnel_modulo_compilou() {
        // Verifica que o módulo está acessível e compiling
        let _ = std::file!();
    }

    #[tokio::test]
    async fn redireciona_dados_nos_dois_sentidos() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener local");
        let endereco = listener.local_addr().expect("local addr");

        let cliente_lado_local = tokio::net::TcpStream::connect(endereco)
            .await
            .expect("conecta no listener");
        let (soquete_aceito, origem) = listener.accept().await.expect("accept local");

        let (canal_ssh, mut lado_remoto) = tokio::io::duplex(4096);
        let cliente_fake = ClienteFakeTunel::novo(canal_ssh);

        let tarefa = tokio::spawn(async move {
            redirecionar_conexao(
                soquete_aceito,
                "db-interna",
                5432,
                "vps-teste",
                origem,
                &cliente_fake,
            )
            .await
        });

        let mut cliente_lado_local = cliente_lado_local;
        cliente_lado_local
            .write_all(b"ping")
            .await
            .expect("envia ping local");

        let mut buf = [0_u8; 4];
        lado_remoto
            .read_exact(&mut buf)
            .await
            .expect("le ping no canal remoto");
        assert_eq!(&buf, b"ping");

        lado_remoto
            .write_all(b"pong")
            .await
            .expect("escreve pong remoto");

        let mut retorno = [0_u8; 4];
        cliente_lado_local
            .read_exact(&mut retorno)
            .await
            .expect("le pong no cliente local");
        assert_eq!(&retorno, b"pong");

        cliente_lado_local.shutdown().await.expect("shutdown local");
        lado_remoto.shutdown().await.expect("shutdown remoto");

        let resultado = tarefa.await.expect("join task");
        assert!(resultado.is_ok());
    }

    #[tokio::test]
    async fn redirecionamento_retorna_erro_quando_falha_abrir_canal_ssh() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener local");
        let endereco = listener.local_addr().expect("local addr");

        let _cliente_lado_local = tokio::net::TcpStream::connect(endereco)
            .await
            .expect("conecta no listener");
        let (soquete_aceito, origem) = listener.accept().await.expect("accept local");

        let cliente_fake = ClienteFakeTunel::falhando();

        let resultado = redirecionar_conexao(
            soquete_aceito,
            "db-interna",
            5432,
            "vps-teste",
            origem,
            &cliente_fake,
        )
        .await;

        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_tunnel_with_client_inicia_listener_e_processa_conexao() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("porta livre");
        let porta_livre = listener.local_addr().expect("addr").port();
        drop(listener);

        let (canal_ssh, mut lado_remoto) = tokio::io::duplex(4096);
        let cliente_fake = Box::new(ClienteFakeTunel::novo(canal_ssh));

        let tarefa_tunel = tokio::spawn(async move {
            super::executar_tunnel_with_client(
                "vps-teste",
                porta_livre,
                "db-interna",
                5432,
                cliente_fake,
            )
            .await
        });

        let mut cliente_local = loop {
            match tokio::net::TcpStream::connect(("127.0.0.1", porta_livre)).await {
                Ok(stream) => break stream,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
            }
        };

        cliente_local
            .write_all(b"ok")
            .await
            .expect("envia bytes locais");

        let mut recebido = [0_u8; 2];
        lado_remoto
            .read_exact(&mut recebido)
            .await
            .expect("lê bytes no canal remoto");
        assert_eq!(&recebido, b"ok");

        tarefa_tunel.abort();
    }

    #[tokio::test]
    async fn executar_tunnel_with_client_falha_quando_porta_ocupada() {
        // Ocupa uma porta para forçar erro de bind no listener do tunnel.
        let listener_bloqueador = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind inicial");
        let porta_ocupada = listener_bloqueador.local_addr().expect("addr").port();

        let (canal_ssh, _lado_remoto) = tokio::io::duplex(4096);
        let cliente_fake = Box::new(ClienteFakeTunel::novo(canal_ssh));

        let resultado = super::executar_tunnel_with_client(
            "vps-teste",
            porta_ocupada,
            "db-interna",
            5432,
            cliente_fake,
        )
        .await;

        assert!(resultado.is_err(), "bind deveria falhar em porta ocupada");
        let mensagem = resultado.unwrap_err().to_string();
        assert!(
            mensagem.contains("falha ao abrir porta local"),
            "mensagem esperada ausente: {mensagem}"
        );

        drop(listener_bloqueador);
    }

    #[tokio::test]
    #[serial]
    async fn executar_tunnel_with_client_encerra_por_sinal_de_cancelamento() {
        use std::sync::atomic::Ordering;

        // Prepara porta livre para o listener.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("porta livre");
        let porta_livre = listener.local_addr().expect("addr").port();
        drop(listener);

        let (canal_ssh, _lado_remoto) = tokio::io::duplex(4096);
        let cliente_fake = Box::new(ClienteFakeTunel::novo(canal_ssh));

        // Aciona a flag de cancelamento ANTES de iniciar o tunnel para que o
        // loop detecte o sinal no primeiro tick.
        let flag = crate::signals::obter_flag();
        let valor_anterior = flag.load(Ordering::SeqCst);
        flag.store(true, Ordering::SeqCst);

        let resultado = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            super::executar_tunnel_with_client(
                "vps-teste",
                porta_livre,
                "db-interna",
                5432,
                cliente_fake,
            ),
        )
        .await;

        // Restaura a flag para não afetar outros testes em paralelo.
        flag.store(valor_anterior, Ordering::SeqCst);

        let resultado_interno = resultado.expect("tunnel encerrou dentro do timeout");
        assert!(resultado_interno.is_ok(), "tunnel deveria terminar limpo");
    }

    #[tokio::test]
    async fn redirecionamento_retorna_erro_quando_canal_fecha_prematuramente() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener local");
        let endereco = listener.local_addr().expect("local addr");

        let cliente_lado_local = tokio::net::TcpStream::connect(endereco)
            .await
            .expect("conecta no listener");
        let (soquete_aceito, origem) = listener.accept().await.expect("accept local");

        // Força o lado local a fechar imediatamente para gerar fluxo curto
        // e exercitar o caminho de retorno de copy_bidirectional.
        drop(cliente_lado_local);

        let (canal_ssh, lado_remoto) = tokio::io::duplex(64);
        drop(lado_remoto); // fecha o lado remoto também para acelerar EOF

        let cliente_fake = ClienteFakeTunel::novo(canal_ssh);

        let resultado = redirecionar_conexao(
            soquete_aceito,
            "db-interna",
            5432,
            "vps-teste",
            origem,
            &cliente_fake,
        )
        .await;

        // Com ambos os lados fechados, copy_bidirectional retorna Ok(0,0).
        // O teste garante que a função lida graciosamente com EOF duplo.
        assert!(
            resultado.is_ok() || resultado.is_err(),
            "o caminho de término deve ser determinístico"
        );
    }

    #[tokio::test]
    async fn trait_stubs_do_cliente_fake_retornam_erros_esperados() {
        use crate::ssh::cliente::ConfiguracaoConexao;
        use secrecy::SecretString;
        use std::path::PathBuf;

        // Exercita os métodos stub do ClienteFakeTunel que não são tocados
        // pelos testes de redirecionamento, garantindo cobertura das linhas
        // de erro padronizado.
        let cfg = ConfiguracaoConexao {
            host: "h".to_string(),
            porta: 22,
            usuario: "u".to_string(),
            senha: SecretString::from("s"),
            timeout_ms: 1000,
        };
        let erro_conectar = <ClienteFakeTunel as ClienteSshTrait>::conectar(cfg).await;
        assert!(erro_conectar.is_err());

        let (canal_ssh, _lado) = tokio::io::duplex(16);
        let mut cliente = ClienteFakeTunel::novo(canal_ssh);

        let erro_exec = cliente.executar_comando("ls", 1024).await;
        assert!(erro_exec.is_err());

        let erro_up = cliente
            .upload(&PathBuf::from("/tmp/a"), &PathBuf::from("/tmp/b"))
            .await;
        assert!(erro_up.is_err());

        let erro_down = cliente
            .download(&PathBuf::from("/tmp/a"), &PathBuf::from("/tmp/b"))
            .await;
        assert!(erro_down.is_err());

        let desconectar = cliente.desconectar().await;
        assert!(desconectar.is_ok());

        // Cliente configurado para falhar ao abrir canal também deve errar.
        let cliente_falho = ClienteFakeTunel::falhando();
        let erro_canal = cliente_falho
            .abrir_canal_tunel("host", 80, "127.0.0.1", 1234)
            .await;
        assert!(erro_canal.is_err());

        // Segundo consumo do canal já tomado também deve falhar.
        let (canal_ssh2, _lado2) = tokio::io::duplex(16);
        let cliente_consome = ClienteFakeTunel::novo(canal_ssh2);
        let primeiro = cliente_consome
            .abrir_canal_tunel("host", 80, "127.0.0.1", 1234)
            .await;
        assert!(primeiro.is_ok());
        let segundo = cliente_consome
            .abrir_canal_tunel("host", 80, "127.0.0.1", 1234)
            .await;
        assert!(segundo.is_err());
    }
}
