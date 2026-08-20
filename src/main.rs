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

    // If CLI mode is requested or command line arguments provided with --cli
    if args.cli || (args.dest.is_some() && !args.sources.is_empty()) {
        return run_cli(args);
    }

    // Launch GUI Mode with eframe
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("CopyFile - Cópia e Backup de Alta Performance")
            .with_inner_size(Vec2::new(1020.0, 680.0))
            .with_min_inner_size(Vec2::new(760.0, 500.0))
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "CopyFile",
        native_options,
        Box::new(|cc| Ok(Box::new(CopyFileApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("Falha ao inicializar interface gráfica: {e}"))?;

    Ok(())
}
