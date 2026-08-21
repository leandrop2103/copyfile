use crate::core::copier::{run_copy_batch, CopyController};
use crate::core::queue::TransferQueue;
use crate::core::scanner::scan_sources;
use crate::core::types::{
    CopyConfig, CopyEvent, FileEntry, FileStatus, LogEntry, LogLevel, PostCopyAction,
};
use crate::gui::theme::configure_theme;
use crate::gui::views::{
    render_completion_modal, render_file_list, render_header, render_logs, render_options,
    render_queue_view, render_stats, ListFilter, LogFilter, SpeedHistory,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::{self, CentralPanel, TopBottomPanel};
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
    logs: Vec<LogEntry>,

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

    // Performance Speed Calculations
    start_time: Option<Instant>,
    last_speed_sample: Instant,
    bytes_since_last_sample: u64,
    current_speed_bps: f64,
    average_speed_bps: f64,
    elapsed_time: Duration,
    eta: Option<Duration>,

    // UI View State
    active_tab: AppTab,
    list_filter: ListFilter,
    log_filter: LogFilter,
    search_query: String,
    auto_scroll: bool,

    // Performance Speed History
    speed_history: SpeedHistory,

    // Transfer Queue
    transfer_queue: TransferQueue,
    active_job_id: Option<usize>,

    // Windows Taskbar Progress & Modals
    taskbar: crate::core::taskbar::TaskbarProgress,
    hwnd: isize,
    show_completion_modal: bool,
    show_sources_manager: bool,
    sources_search_query: String,
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
            logs: Vec::new(),
            total_bytes: 0,
            copied_bytes: 0,
            processed_bytes: 0,
            copied_files_count: 0,
            skipped_files_count: 0,
            error_files_count: 0,
            current_file_name: None,
            current_file_copied: 0,
            current_file_total: 0,
            start_time: None,
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
            list_filter: ListFilter::All,
            log_filter: LogFilter::All,
            search_query: String::new(),
            auto_scroll: true,
            taskbar: crate::core::taskbar::TaskbarProgress::new(),
            hwnd: 0,
            show_completion_modal: false,
            show_sources_manager: false,
            sources_search_query: String::new(),
        }
    }
}

impl CopyFileApp {
    /// Creates a new application instance with optional initial sources.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>, initial_sources: Vec<PathBuf>) -> Self {
        configure_theme(&cc.egui_ctx);
        Self {
            sources: initial_sources,
            ..Default::default()
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

        self.is_running = true;
        self.is_paused = false;
        self.show_completion_modal = false;
        self.controller = CopyController::new();
        self.start_time = Some(Instant::now());
        self.last_speed_sample = Instant::now();
        self.bytes_since_last_sample = 0;
        self.copied_bytes = 0;
        self.processed_bytes = 0;
        self.copied_files_count = 0;
        self.skipped_files_count = 0;
        self.error_files_count = 0;

        let sources = self.sources.clone();
        let config = self.config.clone();
        let controller = self.controller.clone();
        let event_tx = self.event_tx.clone();

        self.logs.push(LogEntry::new(
            LogLevel::Info,
            None,
            format!(
                "Iniciando escaneamento de {} fontes para {}",
                sources.len(),
                dest.display()
            ),
        ));

        std::thread::spawn(move || {
            let (scanned_entries, total_bytes) = scan_sources(&sources, &dest);
            let _ = event_tx.send(CopyEvent::ScanCompleted {
                entries: scanned_entries.clone(),
                total_bytes,
            });

            // In Mirror Sync mode, remove orphan/obsolete files from destination
            if config.transfer_mode == crate::core::types::TransferMode::Sync {
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

            if !scanned_entries.is_empty() {
                run_copy_batch(scanned_entries, config, controller, event_tx);
            } else {
                let _ = event_tx.send(CopyEvent::AllCompleted {
                    total_duration: Duration::ZERO,
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

        self.error_files_count = 0;
        self.is_running = true;
        self.is_paused = false;
        self.show_completion_modal = false;
        self.controller = CopyController::new();
        self.start_time = Some(Instant::now());
        self.last_speed_sample = Instant::now();
        self.bytes_since_last_sample = 0;

        let config = self.config.clone();
        let controller = self.controller.clone();
        let event_tx = self.event_tx.clone();

        self.logs.push(LogEntry::new(
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

    /// Toggles pause/resume state.
    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        self.controller.set_paused(self.is_paused);
        let status_msg = if self.is_paused {
            "Operação pausada pelo usuário"
        } else {
            "Operação retomada"
        };
        self.logs
            .push(LogEntry::new(LogLevel::Info, None, status_msg));
    }

    /// Cancels active copy process.
    pub fn cancel_copy(&mut self) {
        self.controller.cancel();
        self.is_running = false;
        self.is_paused = false;
        self.logs.push(LogEntry::new(
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
        }
    }

    /// Enqueues the currently selected sources and destination into the transfer queue.
    pub fn enqueue_current_selection(&mut self) {
        if let Some(dest) = self.destination.clone() {
            if !self.sources.is_empty() {
                let id =
                    self.transfer_queue
                        .enqueue(self.sources.clone(), dest, self.config.clone());
                self.logs.push(LogEntry::new(
                    LogLevel::Info,
                    None,
                    format!("Lote de transferência enfileirado com sucesso (ID #{id})."),
                ));
            }
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
                    self.logs.push(LogEntry::new(
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
                CopyEvent::FileCompleted {
                    id,
                    hash,
                    duration_ms,
                } => {
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
                    let file_size = self.entries.get(id).map_or(0, |e| e.size);
                    self.processed_bytes += file_size;
                    if let Some(entry) = self.entries.get_mut(id) {
                        entry.status = FileStatus::Error { error };
                    }
                    self.error_files_count += 1;
                }
                CopyEvent::AllCompleted { total_duration } => {
                    self.is_running = false;
                    self.is_paused = false;
                    self.elapsed_time = total_duration;
                    self.current_speed_bps = 0.0;
                    self.eta = None;
                    self.current_file_name = None;
                    self.current_file_copied = 0;
                    self.current_file_total = 0;
                    self.processed_bytes = self.total_bytes;
                    self.show_completion_modal = true;

                    // Send Windows Native Notification & Sound Alert
                    if self.config.enable_notifications {
                        let vol_str = crate::gui::views::header::format_bytes(self.total_bytes);
                        let time_str = crate::gui::views::header::format_duration(total_duration);
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

                    self.logs.push(LogEntry::new(
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
                    if let Some(job_id) = self.active_job_id {
                        self.transfer_queue.mark_completed(job_id);
                        self.active_job_id = None;
                    }

                    if let Some(next_job) = self.transfer_queue.start_next_job().cloned() {
                        self.sources = next_job.sources;
                        self.destination = Some(next_job.destination);
                        self.config = next_job.config;
                        self.active_job_id = Some(next_job.id);
                        self.start_copy();
                    }
                }
                CopyEvent::Log(log_entry) => {
                    self.logs.push(log_entry);
                }
            }
        }

        // Update speed and ETA calculations every 250ms
        if self.is_running && !self.is_paused {
            if let Some(start) = self.start_time {
                self.elapsed_time = start.elapsed();
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

                    if self.current_speed_bps > 0.0 {
                        let remaining_bytes = self.total_bytes.saturating_sub(self.processed_bytes);
                        let remaining_secs = remaining_bytes as f64 / self.current_speed_bps;
                        self.eta = Some(Duration::from_secs_f64(remaining_secs));
                    }
                }
            }
        }
    }

    /// Handles file & folder drag-and-drop directly into the window.
    fn handle_drag_and_drop(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for dropped in &i.raw.dropped_files {
                    if let Some(path) = &dropped.path {
                        if self.destination.is_none() && self.sources.len() == 1 {
                            // If source already has an item, set second drop as destination
                            self.destination = Some(path.clone());
                        } else if !self.sources.contains(path) {
                            self.sources.push(path.clone());
                        }
                    }
                }
            }
        });
    }

    /// Executes the requested post-copy power/audio/eject action.
    fn execute_post_copy_action(action: PostCopyAction, dest: Option<&PathBuf>) {
        match action {
            PostCopyAction::None => {}
            PostCopyAction::PlaySound => {
                #[cfg(windows)]
                {
                    use std::process::Command;
                    let _ = Command::new("powershell")
                        .args([
                            "-NoProfile",
                            "-Command",
                            "[System.Media.SystemSounds]::Asterisk.Play()",
                        ])
                        .spawn();
                }
            }
            PostCopyAction::EjectDisk => {
                #[cfg(windows)]
                {
                    if let Some(d) = dest {
                        if let Some(prefix) = d.to_str().and_then(|s| s.get(0..2)) {
                            use std::process::Command;
                            let drive_letter = prefix.trim_end_matches(':');
                            let script = format!("(New-Object -comObject Shell.Application).Namespace(17).ParseName('{drive_letter}:').InvokeVerb('Eject')");
                            let _ = Command::new("powershell")
                                .args(["-NoProfile", "-Command", &script])
                                .spawn();
                        }
                    }
                }
            }
            PostCopyAction::Sleep => {
                #[cfg(windows)]
                {
                    use std::process::Command;
                    let _ = Command::new("rundll32.exe")
                        .args(["powrprof.dll,SetSuspendState", "0,1,0"])
                        .spawn();
                }
            }
            PostCopyAction::Shutdown => {
                #[cfg(windows)]
                {
                    use std::process::Command;
                    let _ = Command::new("shutdown.exe")
                        .args([
                            "/s",
                            "/t",
                            "60",
                            "/c",
                            "CopyFile: Transferência de backup concluída com sucesso.",
                        ])
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
        self.handle_events();

        if self.is_running {
            ctx.request_repaint_after(Duration::from_millis(16)); // 60 FPS update during copy
        }

        // Update Windows Taskbar Progress
        #[cfg(windows)]
        {
            if self.hwnd == 0 {
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

            let taskbar_state = if !self.is_running {
                crate::core::taskbar::TaskbarState::NoProgress
            } else if self.is_paused {
                crate::core::taskbar::TaskbarState::Paused
            } else if self.error_files_count > 0 {
                crate::core::taskbar::TaskbarState::Error
            } else {
                crate::core::taskbar::TaskbarState::Normal
            };

            self.taskbar.update(
                self.hwnd,
                self.processed_bytes,
                self.total_bytes,
                taskbar_state,
            );
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

        // --- Tab Navigation Bar ---
        TopBottomPanel::top("tabs_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let list_title = format!("📑 Lista de Arquivos ({})", self.entries.len());
                let logs_title = format!("📜 Registro / Log ({})", self.logs.len());
                let queue_title = format!("📑 Fila ({})", self.transfer_queue.waiting_count());

                ui.selectable_value(&mut self.active_tab, AppTab::FileList, list_title);
                ui.selectable_value(&mut self.active_tab, AppTab::Logs, logs_title);
                ui.selectable_value(&mut self.active_tab, AppTab::Queue, queue_title);
                ui.selectable_value(&mut self.active_tab, AppTab::Options, "⚙ Opções");
                ui.selectable_value(&mut self.active_tab, AppTab::Stats, "📊 Estatísticas");
            });
        });

        // --- Main Central Body ---
        CentralPanel::default().show(ctx, |ui| match self.active_tab {
            AppTab::FileList => {
                let mut retry_requested = false;
                render_file_list(
                    ui,
                    &self.entries,
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
                render_logs(ui, &self.logs, &mut self.log_filter, || {
                    logs_cleared = true;
                });
                if logs_cleared {
                    self.logs.clear();
                }
            }
            AppTab::Queue => {
                let mut enqueue_requested = false;
                render_queue_view(
                    ui,
                    &mut self.transfer_queue,
                    self.is_running,
                    &self.sources,
                    &self.destination,
                    || enqueue_requested = true,
                );
                if enqueue_requested {
                    self.enqueue_current_selection();
                }
            }
            AppTab::Options => {
                render_options(ui, &mut self.config, self.is_running);
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
    }
}
