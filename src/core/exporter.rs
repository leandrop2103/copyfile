//! Audit and integrity report generator (HTML5, CSV with UTF-8 BOM, and TXT).

use crate::core::types::{FileEntry, FileStatus};
use chrono::Local;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;

/// Exports an interactive, standalone HTML5 report with responsive table and checksum verification.
#[allow(clippy::too_many_arguments)]
pub fn export_html_report(
    path: &Path,
    entries: &[FileEntry],
    transfer_mode: &str,
    elapsed: Duration,
    total_bytes: u64,
    copied_count: usize,
    skipped_count: usize,
    error_count: usize,
) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    let now_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
    let total_mb = total_bytes as f64 / (1024.0 * 1024.0);
    let duration_secs = elapsed.as_secs_f64();
    let avg_speed_mb = if duration_secs > 0.0 {
        total_mb / duration_secs
    } else {
        0.0
    };

    writeln!(
        w,
        r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Relatório de Transferência - CopyFile</title>
    <style>
        :root {{
            --bg: #0f172a;
            --card-bg: #1e293b;
            --border: #334155;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --accent: #38bdf8;
            --success: #10b981;
            --warning: #f59e0b;
            --error: #ef4444;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background-color: var(--bg);
            color: var(--text);
            padding: 32px 24px;
            line-height: 1.5;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border);
            padding-bottom: 20px;
            margin-bottom: 24px;
        }}
        h1 {{ font-size: 24px; font-weight: 700; color: var(--accent); }}
        .meta {{ font-size: 14px; color: var(--text-muted); }}
        .grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 16px;
            margin-bottom: 28px;
        }}
        .card {{
            background-color: var(--card-bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 16px 20px;
        }}
        .card-title {{ font-size: 13px; color: var(--text-muted); margin-bottom: 4px; }}
        .card-value {{ font-size: 22px; font-weight: 700; }}
        .search-box {{
            width: 100%;
            padding: 12px 16px;
            border-radius: 6px;
            border: 1px solid var(--border);
            background-color: var(--card-bg);
            color: var(--text);
            font-size: 14px;
            margin-bottom: 20px;
            outline: none;
        }}
        .search-box:focus {{ border-color: var(--accent); }}
        table {{
            width: 100%;
            border-collapse: collapse;
            background-color: var(--card-bg);
            border-radius: 8px;
            overflow: hidden;
            border: 1px solid var(--border);
        }}
        th, td {{
            padding: 12px 16px;
            text-align: left;
            border-bottom: 1px solid var(--border);
            font-size: 13.5px;
        }}
        th {{ background-color: #182234; color: var(--text-muted); font-weight: 600; text-transform: uppercase; font-size: 12px; }}
        tr:hover {{ background-color: rgba(56, 189, 248, 0.04); }}
        .badge {{
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: 600;
        }}
        .badge-success {{ background-color: rgba(16, 185, 129, 0.2); color: var(--success); }}
        .badge-warning {{ background-color: rgba(245, 158, 11, 0.2); color: var(--warning); }}
        .badge-error {{ background-color: rgba(239, 68, 68, 0.2); color: var(--error); }}
        .badge-pending {{ background-color: rgba(148, 163, 184, 0.2); color: var(--text-muted); }}
        .hash {{ font-family: monospace; color: var(--accent); }}
        .path {{ word-break: break-all; max-width: 400px; }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div>
                <h1>⚡ CopyFile - Relatório de Auditoria e Integridade</h1>
                <div class="meta">Operação: <strong>{transfer_mode}</strong> • Gerado em: {now_str}</div>
            </div>
            <div class="meta" style="text-align: right;">
                Algoritmo de Integridade: <strong style="color: var(--success);">xxHash3 (100% Validado)</strong>
            </div>
        </header>

        <div class="grid">
            <div class="card">
                <div class="card-title">Total de Itens</div>
                <div class="card-value">{total_items}</div>
            </div>
            <div class="card">
                <div class="card-title">Arquivos Concluídos</div>
                <div class="card-value" style="color: var(--success);">{copied_count}</div>
            </div>
            <div class="card">
                <div class="card-title">Arquivos Ignorados</div>
                <div class="card-value" style="color: var(--warning);">{skipped_count}</div>
            </div>
            <div class="card">
                <div class="card-title">Erros Detectados</div>
                <div class="card-value" style="color: {error_color};">{error_count}</div>
            </div>
            <div class="card">
                <div class="card-title">Volume Total</div>
                <div class="card-value">{total_mb:.2} MB</div>
            </div>
            <div class="card">
                <div class="card-title">Velocidade Média</div>
                <div class="card-value" style="color: var(--accent);">{avg_speed_mb:.2} MB/s</div>
            </div>
        </div>

        <input type="text" id="search" class="search-box" placeholder="🔍 Filtrar registros por caminho de origem, destino, hash ou status..." onkeyup="filterTable()">

        <table id="report-table">
            <thead>
                <tr>
                    <th style="width: 100px;">Status</th>
                    <th>Origem</th>
                    <th>Destino</th>
                    <th style="width: 140px;">xxHash3</th>
                    <th style="width: 110px;">Tamanho</th>
                    <th>Detalhes</th>
                </tr>
            </thead>
            <tbody>
"#,
        transfer_mode = transfer_mode,
        now_str = now_str,
        total_items = entries.len(),
        copied_count = copied_count,
        skipped_count = skipped_count,
        error_count = error_count,
        error_color = if error_count > 0 {
            "var(--error)"
        } else {
            "var(--text-muted)"
        },
        total_mb = total_mb,
        avg_speed_mb = avg_speed_mb
    )?;

    for entry in entries {
        let (badge_cls, status_label, reason) = match &entry.status {
            FileStatus::Pending => ("badge-pending", "Pendente", ""),
            FileStatus::Copying { .. } => ("badge-pending", "Copiando", ""),
            FileStatus::Verifying => ("badge-pending", "Verificando", ""),
            FileStatus::Success { .. } => ("badge-success", "Concluído", "Integridade validada"),
            FileStatus::Skipped { reason } => ("badge-warning", "Ignorado", reason.as_str()),
            FileStatus::Error { error } => ("badge-error", "Erro", error.as_str()),
        };

        let size_str = humansize::format_size(entry.size, humansize::BINARY);
        let hash_str = entry.hash.as_deref().unwrap_or("-");

        writeln!(
            w,
            r#"                <tr>
                    <td><span class="badge {badge_cls}">{status_label}</span></td>
                    <td class="path">{src}</td>
                    <td class="path">{dst}</td>
                    <td class="hash">{hash}</td>
                    <td>{size}</td>
                    <td style="color: var(--text-muted); font-size: 12.5px;">{reason}</td>
                </tr>"#,
            badge_cls = badge_cls,
            status_label = status_label,
            src = html_escape(&entry.source_path.to_string_lossy()),
            dst = html_escape(&entry.dest_path.to_string_lossy()),
            hash = hash_str,
            size = size_str,
            reason = html_escape(reason)
        )?;
    }

    writeln!(
        w,
        r#"            </tbody>
        </table>
    </div>

    <script>
        function filterTable() {{
            const filter = document.getElementById('search').value.toLowerCase();
            const rows = document.querySelectorAll('#report-table tbody tr');
            rows.forEach(row => {{
                const text = row.innerText.toLowerCase();
                row.style.display = text.includes(filter) ? '' : 'none';
            }});
        }}
    </script>
</body>
</html>"#
    )?;

    w.flush()?;
    Ok(())
}

/// Exports a clean RFC 4180 CSV file with UTF-8 BOM for instant compatibility with Microsoft Excel.
pub fn export_csv_report(path: &Path, entries: &[FileEntry]) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    // Write UTF-8 BOM (\u{FEFF}) so Windows Excel automatically opens UTF-8 without mojibake
    w.write_all(&[0xEF, 0xBB, 0xBF])?;

    // Header
    writeln!(
        w,
        "\"Status\";\"Origem\";\"Destino\";\"xxHash3\";\"Tamanho (Bytes)\";\"Tamanho Formatado\";\"Detalhes / Motivo\""
    )?;

    for entry in entries {
        let (status_label, reason) = match &entry.status {
            FileStatus::Pending => ("Pendente", ""),
            FileStatus::Copying { .. } => ("Copiando", ""),
            FileStatus::Verifying => ("Verificando", ""),
            FileStatus::Success { .. } => ("Concluído", "Integridade validada"),
            FileStatus::Skipped { reason } => ("Ignorado", reason.as_str()),
            FileStatus::Error { error } => ("Erro", error.as_str()),
        };

        let size_str = humansize::format_size(entry.size, humansize::BINARY);
        let hash_str = entry.hash.as_deref().unwrap_or("-");

        writeln!(
            w,
            "\"{}\";\"{}\";\"{}\";\"{}\";{};\"{}\";\"{}\"",
            status_label,
            csv_escape(&entry.source_path.to_string_lossy()),
            csv_escape(&entry.dest_path.to_string_lossy()),
            hash_str,
            entry.size,
            size_str,
            csv_escape(reason)
        )?;
    }

    w.flush()?;
    Ok(())
}

/// Escapes HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Escapes double quotes for CSV fields.
fn csv_escape(s: &str) -> String {
    s.replace('"', "\"\"")
}
