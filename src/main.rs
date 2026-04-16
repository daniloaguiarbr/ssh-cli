//! Ponto de entrada do binário ssh-cli.
//!
//! Mantém a lógica mínima: configura runtime tokio e chama `ssh_cli::run()`.

#[cfg(feature = "musl-allocator")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("erro ao criar runtime: {e}");
            std::process::exit(ssh_cli::erros::exit_codes::EX_IOERR);
        }
    };

    let resultado = runtime.block_on(ssh_cli::run());

    match resultado {
        Ok(()) => {
            if ssh_cli::signals::terminado() {
                std::process::exit(ssh_cli::erros::exit_codes::EX_SIGTERM);
            }
            if ssh_cli::signals::cancelado() {
                std::process::exit(ssh_cli::erros::exit_codes::EX_SIGINT);
            }
            std::process::exit(ssh_cli::erros::exit_codes::EX_OK);
        }
        Err(e) => {
            if ssh_cli::signals::terminado() {
                std::process::exit(ssh_cli::erros::exit_codes::EX_SIGTERM);
            }
            if ssh_cli::signals::cancelado() {
                std::process::exit(ssh_cli::erros::exit_codes::EX_SIGINT);
            }
            if let Some(erro_ssh) = e.downcast_ref::<ssh_cli::erros::ErroSshCli>() {
                eprintln!("{erro_ssh}");
                std::process::exit(erro_ssh.exit_code());
            }
            eprintln!("{e}");
            std::process::exit(ssh_cli::erros::exit_codes::EX_GENERAL);
        }
    }
}
