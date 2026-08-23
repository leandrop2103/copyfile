//! Headless CLI execution runner for automated backups and scripts.

use crate::core::copier::{run_copy_batch, CopyController};
use crate::core::format::{format_bytes, format_duration};
use crate::core::scanner::scan_sources;
use crate::core::types::{CopyConfig, CopyEvent, LogLevel, OverwritePolicy};
use clap::Parser;
use crossbeam_channel::unbounded;
use std::path::PathBuf;
use std::time::Instant;

/// Command-line arguments for CopyFile CLI mode.
#[derive(Parser, Debug)]
#[command(name = "copyfile")]
#[command(about = "Ultra-fast parallel file backup and transfer tool with xxHash3 verification")]
pub struct CliArgs {
    /// Source files or directories to copy
    #[arg(short, long, required = false, num_args = 1..)]
    pub sources: Vec<PathBuf>,

    /// Target destination directory
    #[arg(short, long, required = false)]
    pub dest: Option<PathBuf>,

    /// Number of parallel worker threads (default: CPU cores * 2)
    #[arg(short = 't', long)]
    pub threads: Option<usize>,

    /// Disable xxHash3 checksum calculation and verification
    #[arg(long, default_value_t = false)]
    pub no_verify: bool,

    /// Stop immediately on first error instead of ignoring
    #[arg(long, default_value_t = false)]
    pub fail_fast: bool,

    /// Overwrite rule: if-newer (default), always, never, if-differs
    #[arg(long, default_value = "if-newer")]
    pub overwrite: String,

    /// Run in headless CLI mode without opening GUI window
    #[arg(long, default_value_t = false)]
    pub cli: bool,

    /// Pastes files currently in Windows Clipboard to the specified destination directory
    #[arg(long = "paste-to")]
    pub paste_to: Option<PathBuf>,

    /// Prompts user to paste with CopyFile or Windows Explorer in the specified destination directory
    #[arg(long = "prompt-paste")]
    pub prompt_paste: Option<PathBuf>,

    /// Runs as a background daemon intercepting Ctrl+V in Windows Explorer
    #[arg(long = "watch-paste", default_value_t = false)]
    pub watch_paste: bool,
}

/// Runs the file transfer in CLI mode with console output.
///
/// # Errors
/// Returns an error if source or destination paths are invalid.
pub fn run_cli(args: CliArgs) -> anyhow::Result<()> {
    let Some(dest) = args.dest else {
        anyhow::bail!("Destino (--dest) é obrigatório no modo CLI.");
    };

    if args.sources.is_empty() {
        anyhow::bail!("Pelo menos uma origem (--sources) deve ser especificada.");
    }

    let overwrite_policy = match args.overwrite.to_lowercase().as_str() {
        "always" => OverwritePolicy::Always,
        "never" => OverwritePolicy::Never,
        "if-differs" => OverwritePolicy::IfDiffers,
        "if-hash-differs" | "hash" => OverwritePolicy::IfHashDiffers,
        _ => OverwritePolicy::IfNewer,
    };

    let mut config = CopyConfig {
        verify_checksum: !args.no_verify,
        ignore_errors: !args.fail_fast,
        preserve_metadata: true,
        overwrite_policy,
        ..Default::default()
    };

    if let Some(t) = args.threads {
        config.num_threads = t;
    }

    println!("============================================================");
    println!("  CopyFile - Modo Linha de Comando (CLI)");
    println!("  Origens: {} itens", args.sources.len());
    println!("  Destino: {}", dest.display());
    println!(
        "  Threads: {} | xxHash3: {} | Ignorar Erros: {}",
        config.num_threads, config.verify_checksum, config.ignore_errors
    );
    println!("============================================================\n");

    println!("Escaneando arquivos...");
    let scan_start = Instant::now();
    let (entries, total_bytes) = scan_sources(&args.sources, &dest);
    println!(
        "Encontrados {} arquivos ({}) em {:?}",
        entries.len(),
        format_bytes(total_bytes),
        scan_start.elapsed()
    );

    if entries.is_empty() {
        println!("Nenhum arquivo encontrado para copiar.");
        return Ok(());
    }

    let (event_tx, event_rx) = unbounded();
    let controller = CopyController::new();

    let config_for_thread = config.clone();
    let controller_for_thread = controller.clone();
    let copy_thread = std::thread::spawn(move || {
        run_copy_batch(entries, config_for_thread, controller_for_thread, event_tx);
    });

    let mut copied_bytes = 0u64;
    let mut copied_count = 0usize;
    let mut skipped_count = 0usize;
    let mut error_count = 0usize;
    let start_transfer = Instant::now();

    while let Ok(event) = event_rx.recv() {
        match event {
            CopyEvent::FileProgress { chunk_delta, .. } => {
                copied_bytes += chunk_delta;
                let pct = if total_bytes > 0 {
                    (copied_bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                let elapsed_secs = start_transfer.elapsed().as_secs_f64();
                let speed_bps = if elapsed_secs > 0.0 {
                    copied_bytes as f64 / elapsed_secs
                } else {
                    0.0
                };
                print!(
                    "\r[Progresso: {:5.1}%] {} / {} ({}/s)   ",
                    pct,
                    format_bytes(copied_bytes),
                    format_bytes(total_bytes),
                    format_bytes(speed_bps as u64)
                );
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            CopyEvent::FileCompleted { .. } => {
                copied_count += 1;
            }
            CopyEvent::FileSkipped { .. } => {
                skipped_count += 1;
            }
            CopyEvent::FileFailed { error, .. } => {
                error_count += 1;
                eprintln!("\n[ERRO]: {error}");
                if !config.ignore_errors {
                    controller.cancel();
                    break;
                }
            }
            CopyEvent::Log(log) => {
                if log.level == LogLevel::Error {
                    eprintln!("\n[LOG ERRO]: {}", log.message);
                }
            }
            CopyEvent::AllCompleted { total_duration } => {
                println!("\n\n============================================================");
                println!(
                    "  Transferência Concluída em {}",
                    format_duration(total_duration)
                );
                println!("  Copiados: {copied_count} | Ignorados: {skipped_count} | Erros: {error_count}");
                let avg_speed = if total_duration.as_secs_f64() > 0.0 {
                    copied_bytes as f64 / total_duration.as_secs_f64()
                } else {
                    0.0
                };
                println!("  Velocidade Média: {}/s", format_bytes(avg_speed as u64));
                println!("============================================================");
                break;
            }
            _ => {}
        }
    }

    let _ = copy_thread.join();

    if error_count > 0 {
        anyhow::bail!(
            "{} arquivo(s) falharam durante a transferência.",
            error_count
        );
    }

    Ok(())
}
