//! Transferência de arquivos via SCP sobre SSH.
//!
//! Wrapper que usa os métodos `upload` e `download` do [`ClienteSsh`].

use crate::cli::AcaoScp;
use crate::erros::ErroSshCli;
use crate::output;
use crate::ssh::cliente::{ClienteSsh, ClienteSshTrait, ConfiguracaoConexao};
use crate::vps;
use std::path::PathBuf;

/// Executa o subcomando SCP (upload/download).
pub async fn executar_scp(acao: AcaoScp, config_override: Option<PathBuf>) -> anyhow::Result<()> {
    if crate::signals::cancelado() {
        return Err(anyhow::anyhow!(crate::i18n::t(
            crate::i18n::Mensagem::OperacaoCancelada
        )));
    }

    match acao {
        AcaoScp::Upload {
            vps_nome,
            local,
            remote,
        } => {
            let registro = vps::buscar_por_nome(config_override.clone(), &vps_nome)?
                .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(vps_nome.clone()))?;

            let cfg = ConfiguracaoConexao {
                host: registro.host.clone(),
                porta: registro.porta,
                usuario: registro.usuario.clone(),
                senha: registro.senha.clone(),
                timeout_ms: registro.timeout_ms,
            };

            let cliente: Box<dyn ClienteSshTrait> =
                <ClienteSsh as ClienteSshTrait>::conectar(cfg).await?;
            executar_scp_upload_with_client(&registro, &local, &remote, cliente).await?;
        }
        AcaoScp::Download {
            vps_nome,
            remote,
            local,
        } => {
            let registro = vps::buscar_por_nome(config_override.clone(), &vps_nome)?
                .ok_or_else(|| ErroSshCli::VpsNaoEncontrada(vps_nome.clone()))?;

            let cfg = ConfiguracaoConexao {
                host: registro.host.clone(),
                porta: registro.porta,
                usuario: registro.usuario.clone(),
                senha: registro.senha.clone(),
                timeout_ms: registro.timeout_ms,
            };

            let cliente: Box<dyn ClienteSshTrait> =
                <ClienteSsh as ClienteSshTrait>::conectar(cfg).await?;
            executar_scp_download_with_client(&registro, &remote, &local, cliente).await?;
        }
    }
    Ok(())
}

/// Versão testável de upload SCP que aceita o cliente como parâmetro.
pub async fn executar_scp_upload_with_client(
    _registro: &crate::vps::modelo::VpsRegistro,
    local: &std::path::Path,
    remote: &std::path::Path,
    mut cliente: Box<dyn ClienteSshTrait>,
) -> anyhow::Result<()> {
    let resultado = cliente.upload(local, remote).await?;
    cliente.desconectar().await?;
    output::imprimir_sucesso(&format!(
        "Upload concluído: {} bytes em {}ms",
        resultado.bytes_transferidos, resultado.duracao_ms
    ));
    Ok(())
}

/// Versão testável de download SCP que aceita o cliente como parâmetro.
pub async fn executar_scp_download_with_client(
    _registro: &crate::vps::modelo::VpsRegistro,
    remote: &std::path::Path,
    local: &std::path::Path,
    mut cliente: Box<dyn ClienteSshTrait>,
) -> anyhow::Result<()> {
    let resultado = cliente.download(remote, local).await?;
    cliente.desconectar().await?;
    output::imprimir_sucesso(&format!(
        "Download concluído: {} bytes em {}ms",
        resultado.bytes_transferidos, resultado.duracao_ms
    ));
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::erros::ErroSshCli;
    use crate::ssh::cliente::{CanalTunel, SaidaExecucao, TransferenciaResultado};
    use crate::vps::modelo::{VpsRegistro, SCHEMA_VERSION_ATUAL};
    use crate::vps::{self, ArquivoConfig};
    use async_trait::async_trait;
    use secrecy::SecretString;
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::path::Path;
    use tempfile::TempDir;

    struct ClienteFakeScp {
        upload_ok: bool,
        download_ok: bool,
        bytes_upload: u64,
        bytes_download: u64,
    }

    #[async_trait]
    impl ClienteSshTrait for ClienteFakeScp {
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
            if self.upload_ok {
                Ok(TransferenciaResultado {
                    bytes_transferidos: self.bytes_upload,
                    duracao_ms: 10,
                })
            } else {
                Err(ErroSshCli::CanalFalhou("upload falhou".to_string()))
            }
        }

        async fn download(
            &mut self,
            _remote: &Path,
            _local: &Path,
        ) -> Result<TransferenciaResultado, ErroSshCli> {
            if self.download_ok {
                Ok(TransferenciaResultado {
                    bytes_transferidos: self.bytes_download,
                    duracao_ms: 20,
                })
            } else {
                Err(ErroSshCli::CanalFalhou("download falhou".to_string()))
            }
        }

        async fn abrir_canal_tunel(
            &self,
            _host_remoto: &str,
            _porta_remota: u16,
            _endereco_origem: &str,
            _porta_origem: u16,
        ) -> Result<Box<dyn CanalTunel>, ErroSshCli> {
            Err(ErroSshCli::CanalFalhou(
                "não implementado em teste".to_string(),
            ))
        }

        async fn desconectar(&self) -> Result<(), ErroSshCli> {
            Ok(())
        }
    }

    fn registro_teste(nome: &str) -> VpsRegistro {
        VpsRegistro::novo(
            nome.to_string(),
            "127.0.0.1".to_string(),
            1,
            "root".to_string(),
            SecretString::from("senha-teste".to_string()),
            Some(100),
            Some(1000),
            None,
            None,
        )
    }

    fn salvar_config_com_vps(tmp: &TempDir, nome: &str) {
        let mut hosts = BTreeMap::new();
        hosts.insert(nome.to_string(), registro_teste(nome));
        let arquivo = ArquivoConfig {
            schema_version: SCHEMA_VERSION_ATUAL,
            hosts,
        };
        let caminho = tmp.path().join("config.toml");
        vps::salvar(&caminho, &arquivo).expect("salvar config teste");
    }

    #[tokio::test]
    async fn executar_scp_upload_with_client_retorna_ok() {
        let cliente = ClienteFakeScp {
            upload_ok: true,
            download_ok: true,
            bytes_upload: 128,
            bytes_download: 0,
        };
        let registro = registro_teste("vps-a");

        let resultado = executar_scp_upload_with_client(
            &registro,
            Path::new("/tmp/local.txt"),
            Path::new("/tmp/remote.txt"),
            Box::new(cliente),
        )
        .await;

        assert!(resultado.is_ok());
    }

    #[tokio::test]
    async fn executar_scp_download_with_client_retorna_ok() {
        let cliente = ClienteFakeScp {
            upload_ok: true,
            download_ok: true,
            bytes_upload: 0,
            bytes_download: 256,
        };
        let registro = registro_teste("vps-b");

        let resultado = executar_scp_download_with_client(
            &registro,
            Path::new("/tmp/remote.txt"),
            Path::new("/tmp/local.txt"),
            Box::new(cliente),
        )
        .await;

        assert!(resultado.is_ok());
    }

    #[tokio::test]
    async fn executar_scp_upload_with_client_retorna_erro() {
        let cliente = ClienteFakeScp {
            upload_ok: false,
            download_ok: true,
            bytes_upload: 0,
            bytes_download: 0,
        };
        let registro = registro_teste("vps-c");

        let resultado = executar_scp_upload_with_client(
            &registro,
            Path::new("/tmp/local.txt"),
            Path::new("/tmp/remote.txt"),
            Box::new(cliente),
        )
        .await;

        assert!(resultado.is_err());
    }

    #[tokio::test]
    async fn executar_scp_download_with_client_retorna_erro() {
        let cliente = ClienteFakeScp {
            upload_ok: true,
            download_ok: false,
            bytes_upload: 0,
            bytes_download: 0,
        };
        let registro = registro_teste("vps-d");

        let resultado = executar_scp_download_with_client(
            &registro,
            Path::new("/tmp/remote.txt"),
            Path::new("/tmp/local.txt"),
            Box::new(cliente),
        )
        .await;

        assert!(resultado.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn executar_scp_upload_tenta_conectar_quando_vps_existe() {
        let tmp = TempDir::new().expect("tempdir");
        salvar_config_com_vps(&tmp, "vps-upload");

        let resultado = executar_scp(
            AcaoScp::Upload {
                vps_nome: "vps-upload".to_string(),
                local: tmp.path().join("arquivo-local.txt"),
                remote: PathBuf::from("/tmp/arquivo-remoto.txt"),
            },
            Some(tmp.path().to_path_buf()),
        )
        .await;

        assert!(resultado.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn executar_scp_download_tenta_conectar_quando_vps_existe() {
        let tmp = TempDir::new().expect("tempdir");
        salvar_config_com_vps(&tmp, "vps-download");

        let resultado = executar_scp(
            AcaoScp::Download {
                vps_nome: "vps-download".to_string(),
                remote: PathBuf::from("/tmp/arquivo-remoto.txt"),
                local: tmp.path().join("arquivo-local.txt"),
            },
            Some(tmp.path().to_path_buf()),
        )
        .await;

        assert!(resultado.is_err());
    }
}
