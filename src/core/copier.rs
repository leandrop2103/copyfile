//! High-performance parallel file copy pipeline with xxHash3 verification.

use crate::core::hasher::{compute_file_xxhash, FastHasher};
use crate::core::metadata::copy_metadata;
use crate::core::types::{CopyConfig, CopyEvent, FileEntry, LogEntry, LogLevel, OverwritePolicy};
use crossbeam_channel::Sender;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Controller for pausing, resuming, and aborting copy operations.
#[derive(Debug, Clone)]
pub struct CopyController {
    paused: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

impl CopyController {
    /// Creates a new active controller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Toggles or sets pause state.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    /// Returns true if copying is currently paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Signals cancellation of the entire copy process.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns true if cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CopyController {
    fn default() -> Self {
        Self::new()
    }
}

/// Executes parallel file copying for all entries in the batch.
pub fn run_copy_batch(
    entries: Vec<FileEntry>,
    config: CopyConfig,
    controller: CopyController,
    event_tx: Sender<CopyEvent>,
) {
    let start_time = Instant::now();
    let num_threads = config.num_threads.max(1);

    let pool = match rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
    {
        Ok(p) => p,
        Err(e) => {
            let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                LogLevel::Error,
                None,
                format!("Falha ao inicializar ThreadPool Rayon: {e}"),
            )));
            return;
        }
    };

    let entries_arc = Arc::new(entries);
    let config_arc = Arc::new(config);
    let total_count = entries_arc.len();

    pool.scope(|s| {
        for idx in 0..total_count {
            let entries_clone = Arc::clone(&entries_arc);
            let config_clone = Arc::clone(&config_arc);
            let controller_clone = controller.clone();
            let tx_clone = event_tx.clone();

            s.spawn(move |_| {
                if controller_clone.is_cancelled() {
                    return;
                }

                let entry = &entries_clone[idx];
                process_single_file(entry, &config_clone, &controller_clone, &tx_clone);
            });
        }
    });

    let _ = event_tx.send(CopyEvent::AllCompleted {
        total_duration: start_time.elapsed(),
    });
}

fn process_single_file(
    entry: &FileEntry,
    config: &CopyConfig,
    controller: &CopyController,
    event_tx: &Sender<CopyEvent>,
) {
    let src = &entry.source_path;
    let dst = &entry.dest_path;

    // Check if cancelled
    if controller.is_cancelled() {
        return;
    }

    // Check Overwrite Policy
    if dst.exists() {
        match check_overwrite_policy(src, dst, config.overwrite_policy) {
            Ok(should_copy) => {
                if !should_copy {
                    let reason = format!("Ignorado pela regra {:?}", config.overwrite_policy);
                    let _ = event_tx.send(CopyEvent::FileSkipped {
                        id: entry.id,
                        reason: reason.clone(),
                    });
                    let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                        LogLevel::Warn,
                        Some(src.clone()),
                        format!("Arquivo ignorado: {reason}"),
                    )));
                    return;
                }
            }
            Err(e) => {
                let err_msg = format!("Erro ao verificar destino: {e}");
                let _ = event_tx.send(CopyEvent::FileFailed {
                    id: entry.id,
                    error: err_msg.clone(),
                });
                let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                    LogLevel::Error,
                    Some(src.clone()),
                    err_msg,
                )));
                return;
            }
        }
    }

    // Notify file started
    let _ = event_tx.send(CopyEvent::FileStarted { id: entry.id });
    let start_file_time = Instant::now();

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                let err_msg = format!("Falha ao criar diretório {}: {e}", parent.display());
                let _ = event_tx.send(CopyEvent::FileFailed {
                    id: entry.id,
                    error: err_msg.clone(),
                });
                let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                    LogLevel::Error,
                    Some(src.clone()),
                    err_msg,
                )));
                return;
            }
        }
    }

    // Open Source File
    let mut src_file = match File::open(src) {
        Ok(f) => f,
        Err(e) => {
            let err_msg = format!("Falha ao abrir origem: {e}");
            let _ = event_tx.send(CopyEvent::FileFailed {
                id: entry.id,
                error: err_msg.clone(),
            });
            let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                LogLevel::Error,
                Some(src.clone()),
                err_msg,
            )));
            return;
        }
    };

    // Create Destination File
    let mut dst_file = match File::create(dst) {
        Ok(f) => f,
        Err(e) => {
            let err_msg = format!("Falha ao criar destino: {e}");
            let _ = event_tx.send(CopyEvent::FileFailed {
                id: entry.id,
                error: err_msg.clone(),
            });
            let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                LogLevel::Error,
                Some(src.clone()),
                err_msg,
            )));
            return;
        }
    };

    let buffer_size = (config.buffer_size_kb * 1024).max(64 * 1024);
    let mut buffer = vec![0u8; buffer_size];
    let mut hasher = FastHasher::new();
    let mut bytes_copied = 0u64;
    let total_bytes = entry.size;

    loop {
        if controller.is_cancelled() {
            // Delete incomplete destination file on cancel
            drop(dst_file);
            let _ = fs::remove_file(dst);
            return;
        }

        // Handle pause
        while controller.is_paused() {
            if controller.is_cancelled() {
                drop(dst_file);
                let _ = fs::remove_file(dst);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        let bytes_read = match src_file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let err_msg = format!("Erro na leitura: {e}");
                let _ = event_tx.send(CopyEvent::FileFailed {
                    id: entry.id,
                    error: err_msg.clone(),
                });
                let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                    LogLevel::Error,
                    Some(src.clone()),
                    err_msg,
                )));
                return;
            }
        };

        if config.verify_checksum {
            hasher.update(&buffer[..bytes_read]);
        }

        if let Err(e) = dst_file.write_all(&buffer[..bytes_read]) {
            let err_msg = format!("Erro na gravação: {e}");
            let _ = event_tx.send(CopyEvent::FileFailed {
                id: entry.id,
                error: err_msg.clone(),
            });
            let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                LogLevel::Error,
                Some(src.clone()),
                err_msg,
            )));
            return;
        }

        bytes_copied += bytes_read as u64;
        let _ = event_tx.send(CopyEvent::FileProgress {
            id: entry.id,
            copied_bytes: bytes_copied,
            total_bytes,
            chunk_delta: bytes_read as u64,
        });
    }

    if let Err(e) = dst_file.flush() {
        let err_msg = format!("Erro ao sincronizar arquivo com disco: {e}");
        let _ = event_tx.send(CopyEvent::FileFailed {
            id: entry.id,
            error: err_msg.clone(),
        });
        return;
    }
    drop(dst_file);
    drop(src_file);

    let src_hash = if config.verify_checksum {
        hasher.finalize_short_hex()
    } else {
        String::new()
    };

    // Verify Destination Integrity if requested
    if config.verify_checksum {
        let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
            LogLevel::Info,
            Some(dst.clone()),
            format!("Verificando xxHash ({src_hash})..."),
        )));

        match compute_file_xxhash(dst, buffer_size) {
            Ok(dst_hash) => {
                if dst_hash != src_hash {
                    let err_msg = format!(
                        "Falha de integridade! Hash origem: {src_hash} != destino: {dst_hash}"
                    );
                    let _ = event_tx.send(CopyEvent::FileFailed {
                        id: entry.id,
                        error: err_msg.clone(),
                    });
                    let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                        LogLevel::Error,
                        Some(src.clone()),
                        err_msg,
                    )));
                    return;
                }
            }
            Err(e) => {
                let err_msg = format!("Erro ao verificar checksum de destino: {e}");
                let _ = event_tx.send(CopyEvent::FileFailed {
                    id: entry.id,
                    error: err_msg.clone(),
                });
                return;
            }
        }
    }

    // Preserve Metadata (Timestamps, Windows Attributes, Permissions)
    if config.preserve_metadata {
        if let Err(e) = copy_metadata(src, dst) {
            let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                LogLevel::Warn,
                Some(src.clone()),
                format!("Aviso ao preservar metadados: {e}"),
            )));
        }
    }

    let duration_ms = start_file_time.elapsed().as_millis() as u64;
    let _ = event_tx.send(CopyEvent::FileCompleted {
        id: entry.id,
        hash: src_hash.clone(),
        duration_ms,
    });
    let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
        LogLevel::Success,
        Some(src.clone()),
        format!("Copiado com sucesso! [{src_hash}] ({duration_ms}ms)"),
    )));
}

fn check_overwrite_policy(
    src: &Path,
    dst: &Path,
    policy: OverwritePolicy,
) -> std::io::Result<bool> {
    match policy {
        OverwritePolicy::Always => Ok(true),
        OverwritePolicy::Never => Ok(false),
        OverwritePolicy::IfNewer => {
            let src_meta = fs::metadata(src)?;
            let dst_meta = fs::metadata(dst)?;
            let src_mtime = src_meta.modified()?;
            let dst_mtime = dst_meta.modified()?;
            Ok(src_mtime > dst_mtime)
        }
        OverwritePolicy::IfDiffers => {
            let src_meta = fs::metadata(src)?;
            let dst_meta = fs::metadata(dst)?;
            if src_meta.len() != dst_meta.len() {
                return Ok(true);
            }
            let src_mtime = src_meta.modified()?;
            let dst_mtime = dst_meta.modified()?;
            Ok(src_mtime != dst_mtime)
        }
    }
}
