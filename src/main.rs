#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! CopyFile - High Performance Parallel Backup & File Transfer Tool.

use clap::Parser;
use copyfile::cli::{run_cli, CliArgs};
use copyfile::gui::CopyFileApp;
use eframe::egui::Vec2;

fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber for structured logs
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "copyfile=info".into()),
        )
        .init();

    let args = CliArgs::parse();

    // If CLI mode is requested
    if args.cli || (args.dest.is_some() && !args.sources.is_empty()) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::AttachConsole;
            const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
            unsafe {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
        return run_cli(args);
    }

    // If background watcher daemon mode is requested
    if args.watch_paste {
        let is_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        copyfile::core::start_paste_interceptor(is_running);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }

    // Start paste interceptor in background while CopyFile is running
    if copyfile::core::is_autostart_registered() {
        let paste_interceptor_enabled =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        copyfile::core::start_paste_interceptor(paste_interceptor_enabled);
    }

    let mut initial_sources = args.sources;
    let mut initial_destination = args.dest;
    let mut prompt_paste = false;
    let mut auto_start = false;

    if let Some(dest_dir) = args.paste_to {
        let clip_files = copyfile::core::get_clipboard_files();
        if !clip_files.is_empty() {
            initial_sources = clip_files;
            initial_destination = Some(dest_dir);
            auto_start = true;
        }
    } else if let Some(dest_dir) = args.prompt_paste {
        let clip_files = copyfile::core::get_clipboard_files();
        if !clip_files.is_empty() {
            initial_sources = clip_files;
            initial_destination = Some(dest_dir);
            prompt_paste = true;
        }
    }

    // Launch GUI Mode with eframe
    let icon = copyfile::gui::create_app_icon();
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("CopyFile - Cópia e Backup de Alta Performance")
            .with_icon(icon)
            .with_inner_size(Vec2::new(1080.0, 720.0))
            .with_min_inner_size(Vec2::new(800.0, 540.0))
            .with_active(true)
            .with_visible(true)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "CopyFile",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(CopyFileApp::new(
                cc,
                initial_sources,
                initial_destination,
                prompt_paste,
                auto_start,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Falha ao inicializar interface gráfica: {e}"))?;

    Ok(())
}
