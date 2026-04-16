//! Particularidades da plataforma Windows.
//!
//! Configura codepage UTF-8 (65001) ANTES de qualquer I/O para garantir que
//! caracteres acentuados (ç, ã, é, ü, ñ) não corrompam no stdin/stdout quando
//! rodando em cmd.exe, PowerShell 5.1 ou PowerShell 7.

use anyhow::Result;

/// Configura o codepage da console para UTF-8 (65001).
#[cfg(target_os = "windows")]
pub fn configurar_codepage_utf8() -> Result<()> {
    use windows_sys::Win32::System::Console::{SetConsoleCP, SetConsoleOutputCP};
    const CP_UTF8: u32 = 65001;
    unsafe {
        let ok_output = SetConsoleOutputCP(CP_UTF8);
        let ok_input = SetConsoleCP(CP_UTF8);
        if ok_output == 0 {
            tracing::warn!("falha ao configurar SetConsoleOutputCP(65001)");
        }
        if ok_input == 0 {
            tracing::warn!("falha ao configurar SetConsoleCP(65001)");
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn configurar_codepage_utf8() -> Result<()> {
    Ok(())
}
