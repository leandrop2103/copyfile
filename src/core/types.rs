//! Core data types and structures for CopyFile.

use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Actions to execute automatically after all files have been copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PostCopyAction {
    /// No automatic action.
    #[default]
    None,
    /// Play notification sound.
    PlaySound,
    /// Safely eject the destination USB drive.
    EjectDisk,
    /// Put the PC to sleep.
    Sleep,
    /// Shutdown the PC.
    Shutdown,
}

impl PostCopyAction {
    /// List all available actions.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::None,
            Self::PlaySound,
            Self::EjectDisk,
            Self::Sleep,
            Self::Shutdown,
        ]
    }

    /// User-facing label in Portuguese.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::None => "Nenhuma ação adicional",
            Self::PlaySound => "🔔 Tocar som de conclusão",
            Self::EjectDisk => "⏏ Ejetar HD de destino com segurança",
            Self::Sleep => "💤 Suspender o computador",
            Self::Shutdown => "⛔ Desligar o computador",
        }
    }
}

/// Represents the status of an individual file in the copy queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File is queued and waiting to be processed.
    Pending,
    /// Currently copying with bytes transferred.
    Copying {
        /// Number of bytes copied so far.
        bytes_copied: u64,
        /// Total file size in bytes.
        total_bytes: u64,
    },
    /// Currently computing or verifying xxHash checksum.
    Verifying,
    /// Successfully copied and verified.
    Success {
        /// xxHash checksum string (e.g. hex).
        hash: String,
        /// Time taken in milliseconds.
        duration_ms: u64,
    },
    /// Skipped based on overwrite policy or existing identical file.
    Skipped {
        /// Detailed reason for skipping.
        reason: String,
    },
    /// Error occurred while reading, copying, or verifying.
    Error {
        /// Detailed error message.
        error: String,
    },
}

impl FileStatus {
    /// Returns a short display label for the status.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "Pendente",
            Self::Copying { .. } => "Copiando...",
            Self::Verifying => "Verificando...",
            Self::Success { .. } => "Concluído",
            Self::Skipped { .. } => "Ignorado",
            Self::Error { .. } => "Erro",
        }
    }
}

/// Represents an item (file or directory) in the copy list.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Unique identifier in the batch.
    pub id: usize,
    /// Absolute path to the source file or directory.
    pub source_path: PathBuf,
    /// Destination path where this file will be copied.
    pub dest_path: PathBuf,
    /// Relative path preserved from the root source selection.
    pub relative_path: PathBuf,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes.
    pub size: u64,
    /// Current transfer/verification status.
    pub status: FileStatus,
    /// xxHash checksum hex string if calculated.
    pub hash: Option<String>,
    /// File attributes representation (e.g., "-a-", "RSHA").
    pub attributes: String,
    /// Creation timestamp if available.
    pub created_time: Option<SystemTime>,
    /// Last modification timestamp if available.
    pub modified_time: Option<SystemTime>,
}

/// Overwrite policy for existing destination files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverwritePolicy {
    /// Overwrite only if the source file has a newer modification time.
    #[default]
    IfNewer,
    /// Calculate xxHash: Skip if hash is identical, overwrite if hash differs.
    IfHashDiffers,
    /// Always overwrite destination files unconditionally.
    Always,
    /// Never overwrite destination files; skip them.
    Never,
    /// Overwrite if file size or modified timestamp differs.
    IfDiffers,
}

impl OverwritePolicy {
    /// List all selectable options.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::IfNewer,
            Self::IfHashDiffers,
            Self::Always,
            Self::Never,
            Self::IfDiffers,
        ]
    }

    /// User-facing description in Portuguese.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::IfNewer => "Substituir se mais recente",
            Self::IfHashDiffers => "Verificar xxHash: Pular se idêntico, substituir se diferente",
            Self::Always => "Sempre substituir",
            Self::Never => "Pular existentes",
            Self::IfDiffers => "Substituir se tamanho/data diferir",
        }
    }
}

/// Transfer mode: standard copy, safe move, or mirror synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferMode {
    /// Copies files and preserves the originals in source directory.
    #[default]
    Copy,
    /// Moves files: safely deletes the source file only after 100% verified destination copy.
    Move,
    /// Mirror synchronization: copies new/updated files and deletes orphan files from destination.
    Sync,
}

impl TransferMode {
    /// List all available transfer modes.
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Copy, Self::Move, Self::Sync]
    }

    /// User-facing label in Portuguese.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Copy => "Copiar (Manter Origens)",
            Self::Move => "Mover / Recortar Seguro",
            Self::Sync => "Sincronizar (Espelho / Mirror)",
        }
    }
}

/// Global configuration options for the copy engine.
#[derive(Debug, Clone)]
pub struct CopyConfig {
    /// Transfer mode (Copy or Safe Move).
    pub transfer_mode: TransferMode,
    /// Number of worker threads for parallel file operations.
    pub num_threads: usize,
    /// Streaming buffer size in KiB per thread (e.g. 1024 = 1MiB).
    pub buffer_size_kb: usize,
    /// Whether to calculate and verify xxHash3 checksums.
    pub verify_checksum: bool,
    /// Whether to continue copying remaining files when an error occurs.
    pub ignore_errors: bool,
    /// Whether to preserve creation/modification/access times and file attributes.
    pub preserve_metadata: bool,
    /// Whether to enable Direct I/O (bypass RAM cache) on large files (>64MB).
    pub direct_io: bool,
    /// Overwrite policy for duplicate files.
    pub overwrite_policy: OverwritePolicy,
    /// Post-copy action to execute on completion.
    pub post_action: PostCopyAction,
    /// Whether to send Windows Toast notifications upon completion.
    pub enable_notifications: bool,
    /// Whether to play a notification sound upon completion.
    pub enable_sound: bool,
}

impl Default for CopyConfig {
    fn default() -> Self {
        let logical_cpus =
            std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
        Self {
            transfer_mode: TransferMode::Copy,
            num_threads: (logical_cpus * 2).clamp(2, 16),
            buffer_size_kb: 1024, // 1 MB per buffer
            verify_checksum: true,
            ignore_errors: true,
            preserve_metadata: true,
            direct_io: true,
            overwrite_policy: OverwritePolicy::IfNewer,
            post_action: PostCopyAction::None,
            enable_notifications: true,
            enable_sound: true,
        }
    }
}

/// Severity level for structured log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Informational message.
    Info,
    /// Success notification.
    Success,
    /// Warning or skipped file message.
    Warn,
    /// Error message.
    Error,
}

impl LogLevel {
    /// Returns a string representation of the log level.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Success => "SUCCESS",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Structured log message for tracking file operations.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Time when the event occurred.
    pub timestamp: DateTime<Local>,
    /// Severity level.
    pub level: LogLevel,
    /// Associated file path if applicable.
    pub path: Option<PathBuf>,
    /// Descriptive message or error reason.
    pub message: String,
}

impl LogEntry {
    /// Creates a new log entry.
    #[must_use]
    pub fn new(level: LogLevel, path: Option<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Local::now(),
            level,
            path,
            message: message.into(),
        }
    }
}

/// Real-time progress snapshot for the UI and stats display.
#[derive(Debug, Clone, Default)]
pub struct ProgressSnapshot {
    /// Total number of files discovered.
    pub total_files: usize,
    /// Total number of files processed (copied + skipped + errors).
    pub processed_files: usize,
    /// Number of successfully copied files.
    pub copied_files: usize,
    /// Number of skipped files.
    pub skipped_files: usize,
    /// Number of files that resulted in error.
    pub error_files: usize,
    /// Total size of all files in bytes.
    pub total_bytes: u64,
    /// Total bytes copied so far.
    pub copied_bytes: u64,
    /// Current real-time transfer speed in bytes per second.
    pub current_speed_bps: f64,
    /// Average transfer speed in bytes per second.
    pub average_speed_bps: f64,
    /// Path of the file currently being processed (or representative worker).
    pub current_file_path: Option<String>,
    /// Bytes transferred for the current active file.
    pub current_file_copied: u64,
    /// Total size of the current active file.
    pub current_file_total: u64,
    /// Total elapsed time since starting.
    pub elapsed_time: Duration,
    /// Estimated time remaining to completion.
    pub estimated_remaining: Option<Duration>,
    /// Whether the copy engine is currently active.
    pub is_running: bool,
    /// Whether the copy engine is currently paused.
    pub is_paused: bool,
}

/// Events sent from the copy workers to the main UI loop.
#[derive(Debug)]
pub enum CopyEvent {
    /// Scanning discovered new files.
    ScanCompleted {
        entries: Vec<FileEntry>,
        total_bytes: u64,
    },
    /// File copy started.
    FileStarted { id: usize },
    /// Progress update for a specific file.
    FileProgress {
        id: usize,
        copied_bytes: u64,
        total_bytes: u64,
        chunk_delta: u64,
    },
    /// File completed successfully.
    FileCompleted {
        id: usize,
        hash: String,
        duration_ms: u64,
    },
    /// File was skipped.
    FileSkipped {
        id: usize,
        reason: String,
        hash: Option<String>,
    },
    /// File failed with an error.
    FileFailed { id: usize, error: String },
    /// Entire copy operation finished.
    AllCompleted { total_duration: Duration },
    /// New log message.
    Log(LogEntry),
}
