//! Particularidades da plataforma Linux.
//!
//! Detecta sandboxes (Flatpak, Snap) e resolve caminhos XDG.

use tracing::debug;

/// Detecta se o ssh-cli está executando dentro de um sandbox.
pub fn detectar_sandbox() {
    if std::env::var("FLATPAK_ID").is_ok() {
        debug!("executando dentro de sandbox Flatpak");
    } else if std::env::var("SNAP").is_ok() {
        debug!("executando dentro de sandbox Snap");
    }
}

#[cfg(test)]
mod testes {
    use super::detectar_sandbox;
    use serial_test::serial;

    #[test]
    #[serial]
    fn detectar_sandbox_flatpak_sem_panic() {
        std::env::set_var("FLATPAK_ID", "org.teste.App");
        std::env::remove_var("SNAP");
        detectar_sandbox();
        std::env::remove_var("FLATPAK_ID");
    }

    #[test]
    #[serial]
    fn detectar_sandbox_snap_sem_panic() {
        std::env::remove_var("FLATPAK_ID");
        std::env::set_var("SNAP", "/snap/app");
        detectar_sandbox();
        std::env::remove_var("SNAP");
    }
}
