//! Motor SSH via `russh` 0.60.x (iteração 2).
//!
//! Nesta iteração implementamos:
//! - `cliente`: conexão SSH assíncrona com autenticação por senha e execução
//!   de comandos com captura paralela de stdout/stderr via `channel.wait()`.
//! - `tunel`: abertura de canal `direct-tcpip` para port forwarding local.
//!
//! Iterações futuras adicionarão:
//! - `pool`: pool de conexões com `Arc<RwLock<>>`
//! - `sftp`: operações SFTP com streaming
//! - `keepalive`: keepalive periódico e reconexão automática com backoff
//! - `known_hosts`: persistência e validação de fingerprints

pub mod cliente;

pub use cliente::{truncar_utf8, ClienteSsh, ConfiguracaoConexao, SaidaExecucao};
