pub mod clipboard;
pub mod copier;
pub mod disk;
pub mod exporter;
pub mod hasher;
pub mod metadata;
pub mod notify;
pub mod profiles;
pub mod queue;
pub mod scanner;
pub mod shell;
pub mod taskbar;
pub mod types;

pub use clipboard::{get_active_explorer_path, get_clipboard_files, has_clipboard_files};
pub use copier::{run_copy_batch, CopyController};
pub use disk::{get_disk_space, DiskSpaceInfo};
pub use hasher::{compute_file_xxhash, FastHasher};
pub use metadata::copy_metadata;
pub use profiles::{
    get_current_user_presets, get_presets_for_user_path, list_all_windows_users, BackupPreset,
};
pub use scanner::scan_sources;
pub use shell::{is_context_menu_registered, register_context_menu, unregister_context_menu};
pub use taskbar::{TaskbarProgress, TaskbarState};
pub use types::*;
