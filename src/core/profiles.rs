//! User profile detection and quick backup preset discovery.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Preset backup item discovered on the system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPreset {
    /// Human-readable label (e.g. "Área de Trabalho", "Documentos").
    pub name: String,
    /// Absolute path to the folder.
    pub path: PathBuf,
    /// Whether this folder currently exists on disk.
    pub exists: bool,
}

/// Detects standard backup folders for the current user.
#[must_use]
pub fn get_current_user_presets() -> Vec<BackupPreset> {
    let mut presets = Vec::new();

    #[cfg(windows)]
    {
        if let Ok(user_profile) = env::var("USERPROFILE") {
            let base = PathBuf::from(user_profile);

            let folders = [
                ("Área de Trabalho", base.join("Desktop")),
                ("Documentos", base.join("Documents")),
                ("Downloads", base.join("Downloads")),
                ("Imagens", base.join("Pictures")),
                ("Vídeos", base.join("Videos")),
                ("Músicas", base.join("Music")),
            ];

            for (name, path) in folders {
                let exists = path.exists();
                presets.push(BackupPreset {
                    name: name.to_string(),
                    path,
                    exists,
                });
            }

            // Browser Bookmarks and Profiles
            if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
                let local = PathBuf::from(local_app_data);
                let chrome = local.join("Google").join("Chrome").join("User Data");
                if chrome.exists() {
                    presets.push(BackupPreset {
                        name: "Google Chrome (Dados de Usuário)".to_string(),
                        path: chrome,
                        exists: true,
                    });
                }

                let edge = local.join("Microsoft").join("Edge").join("User Data");
                if edge.exists() {
                    presets.push(BackupPreset {
                        name: "Microsoft Edge (Dados de Usuário)".to_string(),
                        path: edge,
                        exists: true,
                    });
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = env::var("HOME") {
            let base = PathBuf::from(home);
            let folders = [
                ("Desktop", base.join("Desktop")),
                ("Documents", base.join("Documents")),
                ("Downloads", base.join("Downloads")),
                ("Pictures", base.join("Pictures")),
                ("Videos", base.join("Videos")),
            ];
            for (name, path) in folders {
                let exists = path.exists();
                presets.push(BackupPreset {
                    name: name.to_string(),
                    path,
                    exists,
                });
            }
        }
    }

    presets
}

/// Discovers all user profiles available in `C:\Users`.
#[must_use]
pub fn list_all_windows_users() -> Vec<(String, PathBuf)> {
    let mut users = Vec::new();
    let users_dir = Path::new("C:\\Users");

    if let Ok(entries) = fs::read_dir(users_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let lower = name.to_lowercase();
                    // Exclude system users
                    if !lower.starts_with("public")
                        && !lower.starts_with("default")
                        && !lower.starts_with("all users")
                    {
                        users.push((name.to_string(), path));
                    }
                }
            }
        }
    }

    users
}

/// Returns standard backup folders for a specific user profile path.
#[must_use]
pub fn get_presets_for_user_path(user_dir: &Path) -> Vec<BackupPreset> {
    let folders = [
        ("Área de Trabalho", user_dir.join("Desktop")),
        ("Documentos", user_dir.join("Documents")),
        ("Downloads", user_dir.join("Downloads")),
        ("Imagens", user_dir.join("Pictures")),
        ("Vídeos", user_dir.join("Videos")),
        ("Músicas", user_dir.join("Music")),
    ];

    let mut presets = Vec::new();
    for (name, path) in folders {
        let exists = path.exists();
        if exists {
            presets.push(BackupPreset {
                name: name.to_string(),
                path,
                exists: true,
            });
        }
    }

    let chrome = user_dir
        .join("AppData")
        .join("Local")
        .join("Google")
        .join("Chrome")
        .join("User Data");
    if chrome.exists() {
        presets.push(BackupPreset {
            name: "Google Chrome".to_string(),
            path: chrome,
            exists: true,
        });
    }

    presets
}
