use crate::core::copier::{run_copy_batch, CopyController};
use crate::core::queue::TransferQueue;
use crate::core::scanner::scan_sources;
use crate::core::types::{
    CopyConfig, CopyEvent, FileEntry, FileStatus, LogEntry, LogLevel, PostCopyAction,
};
use crate::gui::theme::{apply_theme_mode, configure_theme, AppPalette, ThemeMode};
use crate::gui::views::{
    render_completion_modal, render_file_list, render_header, render_logs, render_options,
    render_queue_view, render_stats, ListFilter, LogFilter, SpeedHistory,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::{self, CentralPanel, Color32, RichText, TopBottomPanel};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Maximum number of log entries retained in memory to keep RAM flat on massive backups.
const MAX_LOG_ENTRIES: usize = 25_000;

/// Active navigation tab in the main interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTab {
    #[default]
    FileList,
    Logs,
    Queue,
    Options,
    Stats,
}

/// The main CopyFile GUI application struct.
pub struct CopyFileApp {
    // Paths
    sources: Vec<PathBuf>,
    destination: Option<PathBuf>,

    // Engine & Configuration
    config: CopyConfig,
    controller: CopyController,
    is_running: bool,
    is_paused: bool,

    // Channels
    event_tx: Sender<CopyEvent>,
    event_rx: Receiver<CopyEvent>,

    // Data State
    entries: Vec<FileEntry>,
    logs: VecDeque<LogEntry>,

    // Progress & Metrics
    total_bytes: u64,
    copied_bytes: u64,
    processed_bytes: u64,
    copied_files_count: usize,
    skipped_files_count: usize,
    error_files_count: usize,
    current_file_name: Option<String>,
    current_file_copied: u64,
    current_file_total: u64,
    last_active_entry_id: Option<usize>,

    // Performance Speed Calculations
    start_time: Option<Instant>,
    paused_duration: Duration,
    pause_started_at: Option<Instant>,
    last_speed_sample: Instant,
    bytes_since_last_sample: u64,
    current_speed_bps: f64,
    average_speed_bps: f64,
    elapsed_time: Duration,
    eta: Option<Duration>,

    // UI View State
    active_tab: AppTab,
    theme_mode: ThemeMode,
    list_filter: ListFilter,
    log_filter: LogFilter,
    search_query: String,
    log_search_query: String,
    auto_scroll: bool,

    // Performance Speed History
    speed_history: SpeedHistory,

    // Transfer Queue
    transfer_queue: TransferQueue,
    active_job_id: Option<usize>,

    // Windows Taskbar Progress & Modals
    taskbar: crate::core::taskbar::TaskbarProgress,
    hwnd: isize,
    last_taskbar_state: Option<(crate::core::taskbar::TaskbarState, u64)>,
    show_completion_modal: bool,
    show_sources_manager: bool,
    show_paste_prompt_modal: bool,
    pub show_about_modal: bool,
    needs_focus: bool,
    auto_start: bool,
    sources_search_query: String,

    // Auto-Updater State
    pub available_update: std::sync::Arc<std::sync::Mutex<Option<crate::core::RemoteVersionInfo>>>,
    pub update_status: std::sync::Arc<std::sync::Mutex<crate::core::UpdateStatus>>,
}

impl Default for CopyFileApp {
    fn default() -> Self {
        let (event_tx, event_rx) = unbounded();
        Self {
            sources: Vec::new(),
            destination: None,
            config: CopyConfig::default(),
            controller: CopyController::new(),
            is_running: false,
            is_paused: false,
            event_tx,
            event_rx,
            entries: Vec::new(),
            logs: VecDeque::with_capacity(256),
            total_bytes: 0,
            copied_bytes: 0,
            processed_bytes: 0,
            copied_files_count: 0,
            skipped_files_count: 0,
            error_files_count: 0,
            current_file_name: None,
            current_file_copied: 0,
            current_file_total: 0,
            last_active_entry_id: None,
            start_time: None,
            paused_duration: Duration::ZERO,
            pause_started_at: None,
            last_speed_sample: Instant::now(),
            bytes_since_last_sample: 0,
            current_speed_bps: 0.0,
            average_speed_bps: 0.0,
            elapsed_time: Duration::ZERO,
            eta: None,
            speed_history: SpeedHistory::default(),
            transfer_queue: TransferQueue::default(),
            active_job_id: None,
            active_tab: AppTab::FileList,
            theme_mode: ThemeMode::System,
            list_filter: ListFilter::All,
            log_filter: LogFilter::All,
            search_query: String::new(),
            log_search_query: String::new(),
            auto_scroll: true,
            taskbar: crate::core::taskbar::TaskbarProgress::new(),
            hwnd: 0,
            last_taskbar_state: None,
            show_completion_modal: false,
            show_sources_manager: false,
            show_paste_prompt_modal: false,
            show_about_modal: false,
            needs_focus: true,
            auto_start: false,
            sources_search_query: String::new(),
            available_update: std::sync::Arc::new(std::sync::Mutex::new(None)),
            update_status: std::sync::Arc::new(std::sync::Mutex::new(
                crate::core::UpdateStatus::Idle,
            )),
        }
    }
}

impl Drop for CopyFileApp {
    fn drop(&mut self) {
        if self.is_running {
            self.controller.cancel();
            Self::set_system_sleep_prevented(false);
        }
    }
}

impl CopyFileApp {
    /// Creates a new application instance with optional initial sources, destination, and paste prompt mode.
    #[must_use]
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        initial_sources: Vec<PathBuf>,
        initial_destination: Option<PathBuf>,
        prompt_paste: bool,
        auto_start: bool,
    ) -> Self {
        let theme_mode = ThemeMode::load();
        configure_theme(&cc.egui_ctx, theme_mode);

        let available_update = std::sync::Arc::new(std::sync::Mutex::new(None));
        let update_status =
            std::sync::Arc::new(std::sync::Mutex::new(crate::core::UpdateStatus::Idle));

        // Checagem assíncrona de atualização no GitHub ao iniciar
        let available_update_bg = std::sync::Arc::clone(&available_update);
        let ctx_bg = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            if let Some(info) = crate::core::check_for_updates() {
                if let Ok(mut lock) = available_update_bg.lock() {
                    *lock = Some(info);
                }
                ctx_bg.request_repaint();
            }
        });

        let mut app = Self::default();
        app.sources = initial_sources;
        app.destination = initial_destination;
        app.show_paste_prompt_modal = prompt_paste;
        app.needs_focus = true;
        app.auto_start = auto_start;
        app.available_update = available_update;
        app.update_status = update_status;
        app.theme_mode = theme_mode;
        app
    }

    /// Appends a log entry while keeping memory bounded at `MAX_LOG_ENTRIES`.
    fn push_log(&mut self, entry: LogEntry) {
        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
    }

    /// Dispara o processo de auto-atualização em background thread com repaint contínuo da barra de progresso.
    pub fn trigger_auto_update(&self, ctx: &egui::Context) {
        // Prevent double-clicking while already downloading
        if let Ok(lock) = self.update_status.lock() {
            if matches!(
                *lock,
                crate::core::UpdateStatus::Downloading(_) | crate::core::UpdateStatus::Success(_)
            ) {
                return;
            }
        }

        let update_status = std::sync::Arc::clone(&self.update_status);
        let download_url = self
            .available_update
            .lock()
            .ok()
            .and_then(|l| l.as_ref().map(|info| info.download_url.clone()));
        let ctx_bg = ctx.clone();

        std::thread::spawn(move || {
            let update_status_clone = std::sync::Arc::clone(&update_status);
            let ctx_cb = ctx_bg.clone();
            let result = crate::core::perform_auto_update(download_url, move |status| {
                if let Ok(mut lock) = update_status_clone.lock() {
                    *lock = status;
                }
                ctx_cb.request_repaint();
            });

            if let Err(err) = result {
                if let Ok(mut lock) = update_status.lock() {
                    *lock = crate::core::UpdateStatus::Error(format!("{err:#}"));
                }
                ctx_bg.request_repaint();
            }
        });
    }

    /// Verifica manualmente por atualizações no GitHub com feedback visual imediato.
    pub fn check_updates_manual(&self, ctx: &egui::Context) {
        if let Ok(mut lock) = self.update_status.lock() {
            if matches!(
                *lock,
                crate::core::UpdateStatus::Checking | crate::core::UpdateStatus::Downloading(_)
            ) {
                return;
            }
            *lock = crate::core::UpdateStatus::Checking;
        }
        ctx.request_repaint();

        let available_update_bg = std::sync::Arc::clone(&self.available_update);
        let update_status_bg = std::sync::Arc::clone(&self.update_status);
        let ctx_bg = ctx.clone();
        std::thread::spawn(move || {
            match crate::core::check_for_updates_detailed() {
                Ok(maybe_info) => {
                    if let Ok(mut lock) = available_update_bg.lock() {
                        *lock = maybe_info;
                    }
                    if let Ok(mut lock) = update_status_bg.lock() {
                        *lock = crate::core::UpdateStatus::Idle;
                    }
                }
                Err(err_msg) => {
                    if let Ok(mut lock) = update_status_bg.lock() {
                        *lock = crate::core::UpdateStatus::Error(err_msg);
                    }
                }
            }
            ctx_bg.request_repaint();
        });
    }

    /// Prevents or restores Windows system sleep / hibernation during file transfers.
    fn set_system_sleep_prevented(prevent: bool) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Power::{
                SetThreadExecutionState, ES_AWAYMODE_REQUIRED, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
            };
            unsafe {
                if prevent {
                    SetThreadExecutionState(
                        ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_AWAYMODE_REQUIRED,
                    );
                } else {
                    SetThreadExecutionState(ES_CONTINUOUS);
                }
            }
        }
    }

    /// Starts scanning and file copying process.
    pub fn start_copy(&mut self) {
        let Some(dest) = self.destination.clone() else {
            return;
        };

        if self.sources.is_empty() {
            return;
        }

        Self::set_system_sleep_prevented(true);

        self.is_running = true;
        self.is_paused = false;
        self.show_completion_modal = false;
        self.controller = CopyController::new();
        self.start_time = Some(Instant::now());
        self.paused_duration = Duration::ZERO;
        self.pause_started_at = None;
        self.last_speed_sample = Instant::now();
        self.bytes_since_last_sample = 0;
        self.copied_bytes = 0;
        self.processed_bytes = 0;
        self.copied_files_count = 0;
        self.skipped_files_count = 0;
        self.error_files_count = 0;
        self.current_speed_bps = 0.0;
        self.average_speed_bps = 0.0;
        self.elapsed_time = Duration::ZERO;
        self.eta = None;
        self.last_active_entry_id = None;
        self.speed_history.clear();

        let sources = self.sources.clone();
        let config = self.config.clone();
        let controller = self.controller.clone();
        let event_tx = self.event_tx.clone();

        self.push_log(LogEntry::new(
            LogLevel::Info,
            None,
            format!(
                "Iniciando escaneamento paralelo de {} fonte(s) para {}",
                sources.len(),
                dest.display()
            ),
        ));

        std::thread::spawn(move || {
            let (scanned_entries, total_bytes) = scan_sources(&sources, &dest);
            if controller.is_cancelled() {
                let _ = event_tx.send(CopyEvent::AllCompleted {
                    total_duration: Duration::ZERO,
                    cancelled: true,
                });
                return;
            }

            let _ = event_tx.send(CopyEvent::ScanCompleted {
                entries: scanned_entries.clone(),
                total_bytes,
            });

            // In Mirror Sync mode, remove orphan/obsolete files from destination
            if config.transfer_mode == crate::core::types::TransferMode::Sync
                && !controller.is_cancelled()
            {
                let orphans = crate::core::scanner::find_mirror_orphan_entries(
                    &sources,
                    &dest,
                    &scanned_entries,
                );
                if !orphans.is_empty() {
                    let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                        LogLevel::Info,
                        None,
                        format!(
                            "Modo Espelho: removendo {} item(ns) obsoleto(s) do destino...",
                            orphans.len()
                        ),
                    )));
                    for orphan in orphans {
                        if controller.is_cancelled() {
                            break;
                        }
                        let is_dir = orphan.is_dir();
                        let res = if is_dir {
                            std::fs::remove_dir_all(&orphan)
                        } else {
                            std::fs::remove_file(&orphan)
                        };
                        if res.is_ok() {
                            let _ = event_tx.send(CopyEvent::Log(LogEntry::new(
                                LogLevel::Info,
                                None,
                                format!("Item obsoleto removido: {}", orphan.display()),
                            )));
                        }
                    }
                }
            }

            if !scanned_entries.is_empty() && !controller.is_cancelled() {
                run_copy_batch(scanned_entries, config, controller, event_tx);
            } else {
                let _ = event_tx.send(CopyEvent::AllCompleted {
                    total_duration: Duration::ZERO,
                    cancelled: controller.is_cancelled(),
                });
            }
        });
    }

    /// Retries copying only the files that previously failed with an error.
    pub fn retry_failed_files(&mut self) {
        if self.is_running {
            return;
        }

        let failed_indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(idx, e)| match e.status {
                FileStatus::Error { .. } => Some(idx),
                _ => None,
            })
            .collect();

        if failed_indices.is_empty() {
            return;
        }

        let mut retry_entries = Vec::new();

        for &idx in &failed_indices {
            if let Some(entry) = self.entries.get_mut(idx) {
                entry.status = FileStatus::Pending;
                retry_entries.push(entry.clone());
            }
        }

        self.total_bytes = retry_entries.iter().map(|e| e.size).sum();
        self.copied_bytes = 0;
        self.processed_bytes = 0;
        self.error_files_count = 0;
        self.is_running = true;
        self.is_paused = false;
        self.show_completion_modal = false;
        self.controller = CopyController::new();
        self.start_time = Some(Instant::now());
        self.paused_duration = Duration::ZERO;
        self.pause_started_at = None;
        self.last_speed_sample = Instant::now();
        self.bytes_since_last_sample = 0;
        self.last_active_entry_id = None;
        self.speed_history.clear();

        Self::set_system_sleep_prevented(true);

        let config = self.config.clone();
        let controller = self.controller.clone();
        let event_tx = self.event_tx.clone();

        self.push_log(LogEntry::new(
            LogLevel::Info,
            None,
            format!(
                "Iniciando reprocessamento de {} arquivo(s) que falharam anteriormente...",
                retry_entries.len()
            ),
        ));

        std::thread::spawn(move || {
            run_copy_batch(retry_entries, config, controller, event_tx);
        });
    }

    /// Toggles pause/resume state while preserving accurate active elapsed duration.
    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        self.controller.set_paused(self.is_paused);
        if self.is_paused {
            self.pause_started_at = Some(Instant::now());
            self.current_speed_bps = 0.0;
        } else if let Some(paused_at) = self.pause_started_at.take() {
            self.paused_duration += paused_at.elapsed();
            self.last_speed_sample = Instant::now();
            self.bytes_since_last_sample = 0;
        }
        let status_msg = if self.is_paused {
            "Operação pausada pelo usuário"
        } else {
            "Operação retomada"
        };
        self.push_log(LogEntry::new(LogLevel::Info, None, status_msg));
    }

    /// Cancels active copy process.
    pub fn cancel_copy(&mut self) {
        Self::set_system_sleep_prevented(false);
        self.controller.cancel();
        self.is_running = false;
        self.is_paused = false;
        self.pause_started_at = None;
        self.current_speed_bps = 0.0;
        self.eta = None;
        self.push_log(LogEntry::new(
            LogLevel::Warn,
            None,
            "Operação de cópia cancelada pelo usuário.",
        ));
    }

    /// Clears the queue and resets state.
    pub fn clear_all(&mut self) {
        if !self.is_running {
            self.sources.clear();
            self.entries.clear();
            self.total_bytes = 0;
            self.copied_bytes = 0;
            self.processed_bytes = 0;
            self.copied_files_count = 0;
            self.skipped_files_count = 0;
            self.error_files_count = 0;
            self.current_file_name = None;
            self.current_file_copied = 0;
            self.current_file_total = 0;
            self.last_active_entry_id = None;
            self.current_speed_bps = 0.0;
            self.average_speed_bps = 0.0;
            self.elapsed_time = Duration::ZERO;
            self.eta = None;
            self.speed_history.clear();
        }
    }

    /// Enqueues the currently selected sources and destination into the transfer queue.
    pub fn enqueue_current_selection(&mut self) {
        if let Some(dest) = self.destination.clone() {
            if !self.sources.is_empty() {
                let id =
                    self.transfer_queue
                        .enqueue(self.sources.clone(), dest, self.config.clone());
                self.push_log(LogEntry::new(
                    LogLevel::Info,
                    None,
                    format!("Lote de transferência enfileirado com sucesso (ID #{id})."),
                ));
            }
        }
    }

    /// Starts the next waiting job in the queue if no transfer is currently active.
    pub fn start_next_queued_job(&mut self) {
        if self.is_running {
            return;
        }
        if let Some(next_job) = self.transfer_queue.start_next_job().cloned() {
            self.sources = next_job.sources;
            self.destination = Some(next_job.destination);
            self.config = next_job.config;
            self.active_job_id = Some(next_job.id);
            self.start_copy();
        }
    }

    /// Processes incoming events from background copy worker threads.
    fn handle_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                CopyEvent::ScanCompleted {
                    entries,
                    total_bytes,
                } => {
                    self.total_bytes = total_bytes;
                    self.processed_bytes = 0;
                    self.entries = entries;
                    self.push_log(LogEntry::new(
                        LogLevel::Info,
                        None,
                        format!(
                            "Escaneamento concluído: {} arquivos encontrados ({:.2} MB)",
                            self.entries.len(),
                            total_bytes as f64 / (1024.0 * 1024.0)
                        ),
                    ));
                }
                CopyEvent::FileStarted { id } => {
                    self.last_active_entry_id = Some(id);
                    if let Some(entry) = self.entries.get_mut(id) {
                        entry.status = FileStatus::Copying {
                            bytes_copied: 0,
                            total_bytes: entry.size,
                        };
                        self.current_file_name = Some(
                            entry
                                .source_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        );
                        self.current_file_copied = 0;
                        self.current_file_total = entry.size;
                    }
                }
                CopyEvent::FileProgress {
                    id,
                    copied_bytes,
                    total_bytes,
                    chunk_delta,
                } => {
                    self.last_active_entry_id = Some(id);
                    if let Some(entry) = self.entries.get_mut(id) {
                        entry.status = FileStatus::Copying {
                            bytes_copied: copied_bytes,
                            total_bytes,
                        };
                    }
                    self.copied_bytes += chunk_delta;
                    self.processed_bytes += chunk_delta;
                    self.bytes_since_last_sample += chunk_delta;
                    self.current_file_copied = copied_bytes;
                    self.current_file_total = total_bytes;
                }
                CopyEvent::FileVerifying { id } => {
                    self.last_active_entry_id = Some(id);
                    if let Some(entry) = self.entries.get_mut(id) {
                        entry.status = FileStatus::Verifying;
                    }
                }
                CopyEvent::FileCompleted {
                    id,
                    hash,
                    duration_ms,
                } => {
                    self.last_active_entry_id = Some(id);
                    if let Some(entry) = self.entries.get_mut(id) {
                        entry.status = FileStatus::Success {
                            hash: hash.clone(),
                            duration_ms,
                        };
                        entry.hash = Some(hash);
                    }
                    self.copied_files_count += 1;
                }
                CopyEvent::FileSkipped { id, reason, hash } => {
                    self.last_active_entry_id = Some(id);
                    let file_size = self.entries.get(id).map_or(0, |e| e.size);
                    self.processed_bytes += file_size;
                    if let Some(entry) = self.entries.get_mut(id) {
                        entry.status = FileStatus::Skipped { reason };
                        if let Some(h) = hash {
                            entry.hash = Some(h);
                        }
                    }
                    self.skipped_files_count += 1;
                }
                CopyEvent::FileFailed { id, error } => {
                    self.last_active_entry_id = Some(id);
                    // Avoid double-counting bytes that were already counted via FileProgress before failing
                    let remaining_for_file = self.entries.get(id).map_or(0, |e| {
                        let already_counted = match e.status {
                            FileStatus::Copying { bytes_copied, .. } => bytes_copied,
                            _ => 0,
                        };
                        e.size.saturating_sub(already_counted)
                    });
                    self.processed_bytes += remaining_for_file;
                    if let Some(entry) = self.entries.get_mut(id) {
                        entry.status = FileStatus::Error { error };
                    }
                    self.error_files_count += 1;
                }
                CopyEvent::AllCompleted {
                    total_duration,
                    cancelled,
                } => {
                    Self::set_system_sleep_prevented(false);
                    self.is_running = false;
                    self.is_paused = false;
                    self.pause_started_at = None;
                    let active_duration = total_duration.saturating_sub(self.paused_duration);
                    self.elapsed_time = active_duration;
                    if active_duration.as_secs_f64() > 0.0 {
                        self.average_speed_bps =
                            self.copied_bytes as f64 / active_duration.as_secs_f64();
                    }
                    self.current_speed_bps = 0.0;
                    self.eta = None;
                    self.current_file_name = None;
                    self.current_file_copied = 0;
                    self.current_file_total = 0;

                    // CRITICAL FIX: Do NOT show completion modal, sound, toast, or shutdown if user cancelled!
                    if cancelled {
                        if let Some(job_id) = self.active_job_id.take() {
                            if let Some(job) = self
                                .transfer_queue
                                .jobs_mut()
                                .iter_mut()
                                .find(|j| j.id == job_id)
                            {
                                job.status = crate::core::queue::QueueStatus::Cancelled;
                            }
                        }
                        continue;
                    }

                    self.processed_bytes = self.total_bytes;
                    self.show_completion_modal = true;

                    // Send Windows Native Notification & Sound Alert
                    if self.config.enable_notifications {
                        let vol_str = crate::gui::views::header::format_bytes(self.total_bytes);
                        let time_str = crate::gui::views::header::format_duration(active_duration);
                        let mode_label = self.config.transfer_mode.label();
                        let msg = format!(
                            "{mode_label} concluído!\n{} arquivos transferidos ({vol_str}) em {time_str}.",
                            self.copied_files_count
                        );
                        crate::core::notify::send_completion_notification(
                            "⚡ CopyFile",
                            &msg,
                            self.config.enable_sound,
                        );
                    }

                    self.push_log(LogEntry::new(
                        LogLevel::Success,
                        None,
                        format!(
                            "Cópia concluída! Total: {} itens processados ({} copiados, {} ignorados, {} erros).",
                            self.entries.len(),
                            self.copied_files_count,
                            self.skipped_files_count,
                            self.error_files_count
                        ),
                    ));

                    Self::execute_post_copy_action(
                        self.config.post_action,
                        self.destination.as_ref(),
                    );

                    // Mark active queue job completed and start next if available
                    if let Some(job_id) = self.active_job_id.take() {
                        self.transfer_queue.mark_completed(job_id);
                    }

                    self.start_next_queued_job();
                }
                CopyEvent::Log(log_entry) => {
                    self.push_log(log_entry);
                }
            }
        }

        // Update speed and ETA calculations every 250ms (excluding paused time)
        if self.is_running && !self.is_paused {
            if let Some(start) = self.start_time {
                self.elapsed_time = start.elapsed().saturating_sub(self.paused_duration);
                let sample_elapsed = self.last_speed_sample.elapsed();

                if sample_elapsed >= Duration::from_millis(250) {
                    let secs = sample_elapsed.as_secs_f64();
                    self.current_speed_bps = self.bytes_since_last_sample as f64 / secs;
                    self.speed_history.push(self.current_speed_bps);
                    self.bytes_since_last_sample = 0;
                    self.last_speed_sample = Instant::now();

                    let total_secs = self.elapsed_time.as_secs_f64();
                    if total_secs > 0.0 {
                        self.average_speed_bps = self.copied_bytes as f64 / total_secs;
                    }

                    let eta_speed = if self.current_speed_bps > 0.0 {
                        self.current_speed_bps
                    } else {
                        self.average_speed_bps
                    };
                    if eta_speed > 0.0 {
                        let remaining_bytes = self.total_bytes.saturating_sub(self.processed_bytes);
                        let remaining_secs = remaining_bytes as f64 / eta_speed;
                        self.eta = Some(Duration::from_secs_f64(remaining_secs));
                    }
                }
            }
        }
    }

    /// Handles file & folder drag-and-drop directly into the window.
    fn handle_drag_and_drop(&mut self, ctx: &egui::Context) {
        if self.is_running {
            return;
        }
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for dropped in &i.raw.dropped_files {
                    if let Some(path) = &dropped.path {
                        if !self.sources.contains(path) {
                            self.sources.push(path.clone());
                        }
                    }
                }
            }
        });
    }

    /// Handles global keyboard productivity shortcuts (safely ignoring text input focus!).
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let typing = ctx.wants_keyboard_input();

        ctx.input(|i| {
            // Escape: Close any open modal or cancel active operation
            if i.key_pressed(egui::Key::Escape) {
                if self.show_about_modal {
                    self.show_about_modal = false;
                } else if self.show_paste_prompt_modal {
                    self.show_paste_prompt_modal = false;
                } else if self.show_completion_modal {
                    self.show_completion_modal = false;
                } else if self.show_sources_manager {
                    self.show_sources_manager = false;
                } else if self.is_running {
                    self.cancel_copy();
                }
            }

            // F5 or Ctrl+Enter: Start Copy / Sync
            if ((!typing && i.key_pressed(egui::Key::F5))
                || (i.modifiers.ctrl && i.key_pressed(egui::Key::Enter)))
                && !self.is_running
                && !self.sources.is_empty()
                && self.destination.is_some()
            {
                self.start_copy();
            }

            // Space: Pause / Resume ONLY when not typing in a search/text field!
            if !typing && i.key_pressed(egui::Key::Space) && self.is_running && !i.modifiers.ctrl {
                self.toggle_pause();
            }

            // Ctrl+O: Add Source Folders
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) && !self.is_running {
                if let Some(folders) = rfd::FileDialog::new().pick_folders() {
                    for f in folders {
                        if !self.sources.contains(&f) {
                            self.sources.push(f);
                        }
                    }
                }
            }

            // Ctrl+D: Choose Destination Folder
            if i.modifiers.ctrl && i.key_pressed(egui::Key::D) && !self.is_running {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.destination = Some(folder);
                }
            }

            // Ctrl+F: Switch to FileList tab
            if i.modifiers.ctrl && i.key_pressed(egui::Key::F) {
                self.active_tab = AppTab::FileList;
            }
        });
    }

    /// Executes the requested post-copy power/audio/eject action silently (zero console flash!).
    fn execute_post_copy_action(action: PostCopyAction, dest: Option<&PathBuf>) {
        match action {
            PostCopyAction::None => {}
            PostCopyAction::PlaySound => {
                #[cfg(windows)]
                {
                    extern "system" {
                        fn MessageBeep(uType: u32) -> i32;
                    }
                    unsafe {
                        let _ = MessageBeep(0x0000_0040);
                    }
                }
            }
            PostCopyAction::EjectDisk => {
                #[cfg(windows)]
                {
                    if let Some(d) = dest {
                        if let Some(prefix) = d.to_str().and_then(|s| s.get(0..2)) {
                            use std::os::windows::process::CommandExt;
                            use std::process::Command;
                            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                            let drive_letter = prefix.trim_end_matches(':');
                            let script = format!("(New-Object -comObject Shell.Application).Namespace(17).ParseName('{drive_letter}:').InvokeVerb('Eject')");
                            let _ = Command::new("powershell")
                                .args([
                                    "-NoProfile",
                                    "-NonInteractive",
                                    "-WindowStyle",
                                    "Hidden",
                                    "-Command",
                                    &script,
                                ])
                                .creation_flags(CREATE_NO_WINDOW)
                                .spawn();
                        }
                    }
                }
            }
            PostCopyAction::Sleep => {
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    use std::process::Command;
                    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                    let _ = Command::new("rundll32.exe")
                        .args(["powrprof.dll,SetSuspendState", "0,1,0"])
                        .creation_flags(CREATE_NO_WINDOW)
                        .spawn();
                }
            }
            PostCopyAction::Shutdown => {
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    use std::process::Command;
                    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                    let _ = Command::new("shutdown.exe")
                        .args([
                            "/s",
                            "/t",
                            "60",
                            "/c",
                            "CopyFile: Transferência de backup concluída com sucesso.",
                        ])
                        .creation_flags(CREATE_NO_WINDOW)
                        .spawn();
                }
            }
        }
    }

    /// Calculates the number of unique folders involved in the transfer.
    fn calculate_total_folders(&self) -> usize {
        let mut set = std::collections::HashSet::new();
        for entry in &self.entries {
            if let Some(parent) = entry.source_path.parent() {
                set.insert(parent.to_path_buf());
            }
        }
        set.len().max(self.sources.len())
    }
}

impl eframe::App for CopyFileApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_drag_and_drop(ctx);
        self.handle_keyboard_shortcuts(ctx);
        self.handle_events();

        if self.needs_focus {
            self.needs_focus = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            #[cfg(windows)]
            {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    BringWindowToTop, FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
                };
                if self.hwnd == 0 {
                    use std::ffi::OsStr;
                    use std::os::windows::ffi::OsStrExt;
                    let title: Vec<u16> =
                        OsStr::new("CopyFile - Cópia e Backup de Alta Performance")
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect();
                    let h = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
                    if !h.is_null() {
                        self.hwnd = h as isize;
                    }
                }
                if self.hwnd != 0 {
                    unsafe {
                        ShowWindow(self.hwnd as _, SW_RESTORE);
                        BringWindowToTop(self.hwnd as _);
                        SetForegroundWindow(self.hwnd as _);
                    }
                }
            }
        }

        if self.auto_start && !self.sources.is_empty() && self.destination.is_some() {
            self.auto_start = false;
            self.start_copy();
        }

        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(16)); // 60 FPS update during copy
        }

        // Update Windows Taskbar Progress ONLY when state or integer percentage actually changes
        #[cfg(windows)]
        {
            if self.hwnd == 0 && self.is_running {
                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
                let title: Vec<u16> = OsStr::new("CopyFile - Cópia e Backup de Alta Performance")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let h = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
                if !h.is_null() {
                    self.hwnd = h as isize;
                }
            }

            if self.hwnd != 0 {
                let taskbar_state = if !self.is_running {
                    crate::core::taskbar::TaskbarState::NoProgress
                } else if self.is_paused {
                    crate::core::taskbar::TaskbarState::Paused
                } else if self.error_files_count > 0 {
                    crate::core::taskbar::TaskbarState::Error
                } else {
                    crate::core::taskbar::TaskbarState::Normal
                };

                let pct_bucket = (self.processed_bytes.min(self.total_bytes) * 100)
                    .checked_div(self.total_bytes)
                    .unwrap_or(0);

                let next_sig = (taskbar_state, pct_bucket);
                if self.last_taskbar_state != Some(next_sig) {
                    self.last_taskbar_state = Some(next_sig);
                    self.taskbar.update(
                        self.hwnd,
                        self.processed_bytes.min(self.total_bytes),
                        self.total_bytes,
                        taskbar_state,
                    );
                }
            }
        }

        // --- Top Header Panel ---
        TopBottomPanel::top("header_panel").show(ctx, |ui| {
            let mut start_requested = false;
            let mut pause_requested = false;
            let mut cancel_requested = false;
            let mut clear_requested = false;
            let mut open_sources_requested = false;

            render_header(
                ui,
                &mut self.sources,
                &mut self.destination,
                &mut self.config,
                self.is_running,
                self.is_paused,
                &self.current_file_name,
                self.current_file_copied,
                self.current_file_total,
                self.processed_bytes,
                self.total_bytes,
                self.copied_files_count,
                self.skipped_files_count,
                self.error_files_count,
                self.entries.len(),
                self.current_speed_bps,
                self.elapsed_time,
                self.eta,
                || start_requested = true,
                || pause_requested = true,
                || cancel_requested = true,
                || clear_requested = true,
                || open_sources_requested = true,
            );

            if start_requested {
                self.start_copy();
            }
            if pause_requested {
                self.toggle_pause();
            }
            if cancel_requested {
                self.cancel_copy();
            }
            if clear_requested {
                self.clear_all();
            }
            if open_sources_requested {
                self.show_sources_manager = true;
            }
        });

        // --- Tab Navigation Bar (Compact labels -> Zero overlap with update badge at 800px!) ---
        TopBottomPanel::top("tabs_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let list_title = format!("📑 Arquivos ({})", self.entries.len());
                let logs_title = format!("📜 Logs ({})", self.logs.len());
                let queue_title = format!("📋 Fila ({})", self.transfer_queue.waiting_count());

                ui.selectable_value(&mut self.active_tab, AppTab::FileList, list_title);
                ui.selectable_value(&mut self.active_tab, AppTab::Logs, logs_title);
                ui.selectable_value(&mut self.active_tab, AppTab::Queue, queue_title);
                ui.selectable_value(&mut self.active_tab, AppTab::Options, "⚙ Opções");
                ui.selectable_value(&mut self.active_tab, AppTab::Stats, "📊 Estatísticas");

                // Lado Direito: Botão "Sobre", Alternador de Tema e Pill compacta de Atualização
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pal = AppPalette::from_ui(ui);

                    let about_btn = egui::Button::new(
                        RichText::new("ℹ Sobre")
                            .size(13.0)
                            .strong()
                            .color(pal.text_primary),
                    );
                    if ui
                        .add(about_btn)
                        .on_hover_text("Informações do programa, versão e atualização")
                        .clicked()
                    {
                        self.show_about_modal = true;
                    }

                    let active_mode_name = if pal.is_dark { "Escuro" } else { "Claro" };
                    let theme_hover = match self.theme_mode {
                        ThemeMode::System => format!(
                            "Tema: Automático (Sistema atual: {active_mode_name})\nClique para alternar manualmente"
                        ),
                        ThemeMode::Light => {
                            "Tema: Modo Claro\nClique para alternar para Modo Escuro".to_string()
                        }
                        ThemeMode::Dark => {
                            "Tema: Modo Escuro\nClique para voltar ao Automático (Sistema)"
                                .to_string()
                        }
                    };
                    let theme_btn = egui::Button::new(
                        RichText::new(self.theme_mode.short_label())
                            .size(13.0)
                            .strong()
                            .color(pal.text_primary),
                    );
                    if ui.add(theme_btn).on_hover_text(theme_hover).clicked() {
                        self.theme_mode = self.theme_mode.next_cycle(pal.is_dark);
                        apply_theme_mode(ctx, self.theme_mode);
                        self.theme_mode.save();
                    }

                    let has_update = self
                        .available_update
                        .lock()
                        .ok()
                        .and_then(|l| l.as_ref().map(|i| i.version.clone()));

                    if let Some(ver) = has_update {
                        let update_pill = egui::Button::new(
                            RichText::new(format!("⚡ v{ver}"))
                                .strong()
                                .size(12.5)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(0, 150, 60));

                        if ui
                            .add(update_pill)
                            .on_hover_text(format!(
                                "Nova versão v{ver} disponível! Clique para baixar e atualizar"
                            ))
                            .clicked()
                        {
                            self.show_about_modal = true;
                        }
                    }
                });
            });
        });

        // --- Main Central Body ---
        CentralPanel::default().show(ctx, |ui| match self.active_tab {
            AppTab::FileList => {
                let mut retry_requested = false;
                render_file_list(
                    ui,
                    &self.entries,
                    self.copied_files_count,
                    self.skipped_files_count,
                    self.error_files_count,
                    self.last_active_entry_id,
                    &mut self.list_filter,
                    &mut self.search_query,
                    &mut self.auto_scroll,
                    self.is_running,
                    || retry_requested = true,
                );
                if retry_requested {
                    self.retry_failed_files();
                }
            }
            AppTab::Logs => {
                let mut logs_cleared = false;
                render_logs(
                    ui,
                    &self.logs,
                    &mut self.log_filter,
                    &mut self.log_search_query,
                    || {
                        logs_cleared = true;
                    },
                );
                if logs_cleared {
                    self.logs.clear();
                }
            }
            AppTab::Queue => {
                let mut enqueue_requested = false;
                let mut start_queue_requested = false;
                render_queue_view(
                    ui,
                    &mut self.transfer_queue,
                    self.is_running,
                    &self.sources,
                    &self.destination,
                    || enqueue_requested = true,
                    || start_queue_requested = true,
                );
                if enqueue_requested {
                    self.enqueue_current_selection();
                }
                if start_queue_requested {
                    self.start_next_queued_job();
                }
            }
            AppTab::Options => {
                let available_update = self.available_update.lock().ok().and_then(|l| l.clone());
                let update_status = self
                    .update_status
                    .lock()
                    .ok()
                    .map(|l| l.clone())
                    .unwrap_or(crate::core::UpdateStatus::Idle);
                let mut trigger_update = false;
                let mut check_update = false;
                let mut open_about = false;

                render_options(
                    ui,
                    &mut self.config,
                    &mut self.theme_mode,
                    self.is_running,
                    &available_update,
                    &update_status,
                    || trigger_update = true,
                    || check_update = true,
                    || open_about = true,
                );

                if trigger_update {
                    self.trigger_auto_update(ctx);
                }
                if check_update {
                    self.check_updates_manual(ctx);
                }
                if open_about {
                    self.show_about_modal = true;
                }
            }
            AppTab::Stats => {
                render_stats(
                    ui,
                    self.entries.len(),
                    self.copied_files_count,
                    self.skipped_files_count,
                    self.error_files_count,
                    self.total_bytes,
                    self.copied_bytes,
                    self.processed_bytes,
                    self.current_speed_bps,
                    self.average_speed_bps,
                    self.elapsed_time,
                    self.eta,
                    &self.speed_history,
                );
            }
        });

        // --- Sources Manager Modal Dialog ---
        if self.show_sources_manager {
            crate::gui::views::render_sources_manager_modal(
                ctx,
                &mut self.show_sources_manager,
                &mut self.sources,
                &mut self.sources_search_query,
                self.is_running,
            );
        }

        // --- Completion Summary Modal Dialog ---
        if self.show_completion_modal {
            let total_folders = self.calculate_total_folders();
            let mode_name = self.config.transfer_mode.label();
            let mut switch_to_logs = false;
            let mut retry_requested = false;

            render_completion_modal(
                ctx,
                &mut self.show_completion_modal,
                mode_name,
                total_folders,
                self.copied_files_count,
                self.skipped_files_count,
                self.error_files_count,
                self.entries.len(),
                self.total_bytes,
                self.elapsed_time,
                self.average_speed_bps,
                self.destination.as_ref(),
                &self.entries,
                || switch_to_logs = true,
                || retry_requested = true,
            );

            if switch_to_logs {
                self.active_tab = AppTab::Logs;
            }
            if retry_requested {
                self.retry_failed_files();
            }
        }

        // --- Paste Prompt Modal Dialog (TeraCopy-style) ---
        if self.show_paste_prompt_modal {
            let mut paste_copyfile = false;
            let mut paste_explorer = false;

            crate::gui::views::render_paste_prompt_modal(
                ctx,
                &mut self.show_paste_prompt_modal,
                self.sources.len(),
                self.destination.as_ref(),
                || paste_copyfile = true,
                || paste_explorer = true,
            );

            if paste_copyfile {
                self.start_copy();
            }
            if paste_explorer {
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    use std::process::Command;
                    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                    let _ = Command::new("powershell")
                        .args([
                            "-NoProfile",
                            "-NonInteractive",
                            "-WindowStyle",
                            "Hidden",
                            "-Command",
                            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^v')",
                        ])
                        .creation_flags(CREATE_NO_WINDOW)
                        .spawn();
                }
                std::process::exit(0);
            }
        }

        // --- About & Auto-Updater Modal Dialog ---
        if self.show_about_modal {
            let available_update = self.available_update.lock().ok().and_then(|l| l.clone());
            let update_status = self
                .update_status
                .lock()
                .ok()
                .map(|l| l.clone())
                .unwrap_or(crate::core::UpdateStatus::Idle);
            let mut trigger_update = false;
            let mut check_update = false;

            crate::gui::views::render_about_modal(
                ctx,
                &mut self.show_about_modal,
                &available_update,
                &update_status,
                || trigger_update = true,
                || check_update = true,
            );

            if trigger_update {
                self.trigger_auto_update(ctx);
            }
            if check_update {
                self.check_updates_manual(ctx);
            }
        }
    }
}
