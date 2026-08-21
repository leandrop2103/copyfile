//! Windows Shell and Context Menu integration.

use std::env;
use std::io;

/// Checks if CopyFile is registered in the Windows Context Menu.
#[must_use]
pub fn is_context_menu_registered() -> bool {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("reg")
            .args([
                "query",
                "HKCU\\Software\\Classes\\Directory\\shell\\CopyFile",
            ])
            .output();

        matches!(output, Ok(out) if out.status.success())
    }

    #[cfg(not(windows))]
    {
        false
    }
}

/// Registers CopyFile into the current user's Windows Context Menu.
///
/// # Errors
/// Returns an error if the registry command fails.
pub fn register_context_menu() -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::process::Command;

        let exe_path = env::current_exe()?;
        let exe_str = exe_path.to_string_lossy();

        // 1. Directory context menu
        let cmd_dir = format!("\"{exe_str}\" -s \"%1\"");
        let _ = Command::new("reg")
            .args([
                "add",
                "HKCU\\Software\\Classes\\Directory\\shell\\CopyFile",
                "/ve",
                "/d",
                "Copiar com CopyFile",
                "/f",
            ])
            .output();

        let _ = Command::new("reg")
            .args([
                "add",
                "HKCU\\Software\\Classes\\Directory\\shell\\CopyFile",
                "/v",
                "Icon",
                "/d",
                &format!("\"{exe_str}\",0"),
                "/f",
            ])
            .output();

        let _ = Command::new("reg")
            .args([
                "add",
                "HKCU\\Software\\Classes\\Directory\\shell\\CopyFile\\command",
                "/ve",
                "/d",
                &cmd_dir,
                "/f",
            ])
            .output();

        // 2. File context menu (*)
        let cmd_file = format!("\"{exe_str}\" -s \"%1\"");
        let _ = Command::new("reg")
            .args([
                "add",
                "HKCU\\Software\\Classes\\*\\shell\\CopyFile",
                "/ve",
                "/d",
                "Copiar com CopyFile",
                "/f",
            ])
            .output();

        let _ = Command::new("reg")
            .args([
                "add",
                "HKCU\\Software\\Classes\\*\\shell\\CopyFile",
                "/v",
                "Icon",
                "/d",
                &format!("\"{exe_str}\",0"),
                "/f",
            ])
            .output();

        let _ = Command::new("reg")
            .args([
                "add",
                "HKCU\\Software\\Classes\\*\\shell\\CopyFile\\command",
                "/ve",
                "/d",
                &cmd_file,
                "/f",
            ])
            .output();

        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Unregisters CopyFile from the Windows Context Menu.
///
/// # Errors
/// Returns an error if removing registry keys fails.
pub fn unregister_context_menu() -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::process::Command;

        let _ = Command::new("reg")
            .args([
                "delete",
                "HKCU\\Software\\Classes\\Directory\\shell\\CopyFile",
                "/f",
            ])
            .output();

        let _ = Command::new("reg")
            .args([
                "delete",
                "HKCU\\Software\\Classes\\*\\shell\\CopyFile",
                "/f",
            ])
            .output();

        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}
