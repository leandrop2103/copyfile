//! # Módulo de Atualização Automática (`updater.rs`)
//!
//! Este módulo gerencia o download e a substituição a quente do executável da aplicação
//! diretamente do repositório público de releases no GitHub (`leandrop2103/copyfile`).
//!
//! ## Técnica de Substituição a Quente no Windows:
//! No Windows, um executável em execução não pode ser sobrescrito diretamente.
//! 1. O download é gravado diretamente em arquivo temporário `copyfile.exe.new`.
//! 2. Validamos integralmente o tamanho (`Content-Length`) e o cabeçalho PE (`MZ` + `PE\0\0`).
//! 3. Renomeamos o executável atual para `copyfile.exe.old`.
//! 4. Movemos `copyfile.exe.new` para `copyfile.exe`.
//! 5. Disparamos a nova versão atualizada com a flag `--cleanup-old`.
//! 6. O novo processo aguarda a liberação e remove `copyfile.exe.old` do disco.

use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tracing::{info, warn};

/// URLs oficiais do repositório GitHub para verificação e download
pub const GITHUB_API_LATEST: &str =
    "https://api.github.com/repos/leandrop2103/copyfile/releases/latest";
pub const GITHUB_DIRECT_DOWNLOAD: &str =
    "https://github.com/leandrop2103/copyfile/releases/latest/download/copyfile.exe";

/// Informações sobre uma versão remota disponível
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteVersionInfo {
    pub version: String,
    pub download_url: String,
    pub release_notes: String,
}

/// Status do processo de atualização
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    /// Ocioso / Pronto para verificar
    Idle,
    /// Consultando a API do GitHub Releases
    Checking,
    /// Baixando o novo executável (Progresso de 0.0 a 1.0)
    Downloading(f32),
    /// Atualização concluída com sucesso (Aguardando reinicialização)
    Success(String),
    /// Falha no processo de atualização (Mensagem de erro)
    Error(String),
}

/// Compara duas versões em formato SemVer ("1.0.1" vs "1.0.0").
#[must_use]
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let clean = v
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()?;
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

#[must_use]
pub fn is_newer_version(remote: &str, current: &str) -> bool {
    if let (Some(r), Some(c)) = (parse_version(remote), parse_version(current)) {
        r > c
    } else {
        remote.trim() != current.trim()
    }
}

/// Gera caminhos auxiliares `.old` e `.new` preservando o nome completo do executável.
fn update_sidecar_paths(current_exe: &Path) -> (PathBuf, PathBuf) {
    let mut old_os = current_exe.as_os_str().to_os_string();
    old_os.push(".old");
    let mut new_os = current_exe.as_os_str().to_os_string();
    new_os.push(".new");
    (PathBuf::from(old_os), PathBuf::from(new_os))
}

/// Valida integridade estrutural de um executável PE Windows (`MZ` + assinatura `PE\0\0`).
pub fn validate_windows_pe(path: &Path) -> Result<()> {
    let mut f =
        File::open(path).context("Não foi possível abrir o arquivo baixado para validação")?;
    let file_len = f.metadata()?.len();
    if file_len < 1024 {
        bail!("Arquivo baixado é muito pequeno ({file_len} bytes) para ser um executável válido");
    }

    let mut dos_header = [0u8; 64];
    f.read_exact(&mut dos_header)
        .context("Falha ao ler cabeçalho DOS do executável")?;

    if dos_header[0] != b'M' || dos_header[1] != b'Z' {
        bail!("Assinatura DOS (MZ) ausente: arquivo não é um executável Windows válido");
    }

    let pe_offset = u32::from_le_bytes([
        dos_header[60],
        dos_header[61],
        dos_header[62],
        dos_header[63],
    ]) as u64;

    if pe_offset + 4 > file_len {
        bail!("Offset de cabeçalho PE inválido ({pe_offset})");
    }

    f.seek(SeekFrom::Start(pe_offset))
        .context("Falha ao posicionar leitura no cabeçalho PE")?;
    let mut pe_sig = [0u8; 4];
    f.read_exact(&mut pe_sig)
        .context("Falha ao ler assinatura PE")?;

    if pe_sig != [b'P', b'E', 0, 0] {
        bail!("Assinatura PE\\0\\0 inválida: executável corrompido");
    }

    Ok(())
}

/// Consulta o GitHub Releases retornando o resultado detalhado (incluindo erros de rede).
///
/// # Errors
/// Retorna mensagem de erro legível em português se a consulta HTTP falhar.
pub fn check_for_updates_detailed() -> Result<Option<RemoteVersionInfo>, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    info!(
        "Verificando atualizações no GitHub (versão instalada: v{})...",
        current_version
    );

    let agent = ureq::builder()
        .timeout_connect(std::time::Duration::from_secs(6))
        .timeout_read(std::time::Duration::from_secs(8))
        .build();

    let gh_resp = agent
        .get(GITHUB_API_LATEST)
        .set("User-Agent", &format!("CopyFile/{current_version}"))
        .set("Accept", "application/vnd.github.v3+json")
        .call();

    match gh_resp {
        Ok(resp) if resp.status() == 200 => {
            let json_str = resp
                .into_string()
                .map_err(|e| format!("Erro ao ler resposta do GitHub: {e}"))?;
            let json = serde_json::from_str::<serde_json::Value>(&json_str)
                .map_err(|e| format!("Resposta JSON inválida do GitHub: {e}"))?;

            let tag_name = json.get("tag_name").and_then(|v| v.as_str()).unwrap_or("");
            let clean_tag = tag_name.trim().trim_start_matches(['v', 'V']);
            let body = json
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("Melhorias e correções de desempenho.");

            // Encontra a URL do binário copyfile.exe nos assets da release
            let mut dl_url = GITHUB_DIRECT_DOWNLOAD.to_string();
            if let Some(assets) = json.get("assets").and_then(|a| a.as_array()) {
                for asset in assets {
                    if asset
                        .get("name")
                        .and_then(|n| n.as_str())
                        .is_some_and(|n| n.eq_ignore_ascii_case("copyfile.exe"))
                    {
                        if let Some(url) =
                            asset.get("browser_download_url").and_then(|u| u.as_str())
                        {
                            dl_url = url.to_string();
                            break;
                        }
                    }
                }
            }

            if !clean_tag.is_empty() && is_newer_version(clean_tag, current_version) {
                info!(
                    "Nova versão encontrada no GitHub: v{} (atual: v{})",
                    clean_tag, current_version
                );
                return Ok(Some(RemoteVersionInfo {
                    version: clean_tag.to_string(),
                    download_url: dl_url,
                    release_notes: body.to_string(),
                }));
            }

            info!(
                "CopyFile já está na versão mais recente (v{}).",
                current_version
            );
            Ok(None)
        }
        Ok(resp) => {
            let msg = format!("GitHub retornou status HTTP {}", resp.status());
            warn!("{msg}");
            Err(msg)
        }
        Err(e) => {
            let msg = format!("Não foi possível conectar ao GitHub: {e}");
            warn!("{msg}");
            Err(msg)
        }
    }
}

/// Consulta o GitHub Releases para verificar se há uma versão mais recente disponível.
#[must_use]
pub fn check_for_updates() -> Option<RemoteVersionInfo> {
    check_for_updates_detailed().ok().flatten()
}

/// Executa o download da nova versão e realiza a substituição a quente do executável no Windows.
///
/// # Errors
/// Retorna erro se o download falhar, se o arquivo for inválido ou se a renomeação falhar.
pub fn perform_auto_update<F>(custom_url: Option<String>, progress_callback: F) -> Result<()>
where
    F: Fn(UpdateStatus) + Send + 'static,
{
    progress_callback(UpdateStatus::Downloading(0.0));

    let download_url = custom_url.unwrap_or_else(|| GITHUB_DIRECT_DOWNLOAD.to_string());

    info!(
        "Iniciando download da atualização a partir de '{}'...",
        download_url
    );
    let agent = ureq::builder()
        .redirects(5)
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(30))
        .build();

    let response = agent
        .get(&download_url)
        .set(
            "User-Agent",
            &format!("CopyFile/{}", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .with_context(|| format!("Falha ao conectar no endereço de atualização: {download_url}"))?;

    let content_length = response
        .header("Content-Length")
        .and_then(|l| l.parse::<u64>().ok())
        .unwrap_or(0);

    info!("Tamanho reportado da atualização: {} bytes", content_length);

    // Determina caminhos dos executáveis
    let current_exe =
        std::env::current_exe().context("Falha ao determinar caminho do executável atual")?;
    let (old_exe, new_exe) = update_sidecar_paths(&current_exe);

    // Gravação em streaming direto para arquivo temporário no disco
    {
        let file = File::create(&new_exe).with_context(|| {
            format!(
                "Falha ao criar arquivo temporário {:?} (verifique permissões de escrita/Administrador)",
                new_exe
            )
        })?;
        let mut writer = BufWriter::with_capacity(128 * 1024, file);
        let mut reader = response.into_reader();
        let mut buffer = [0u8; 65536];
        let mut total_read: u64 = 0;
        let mut last_report = Instant::now();
        let mut last_reported_pct: f32 = -1.0;

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    writer
                        .write_all(&buffer[..n])
                        .context("Erro ao gravar dados do download no disco")?;
                    total_read += n as u64;

                    if content_length > 0 {
                        let progress = (total_read as f32 / content_length as f32).clamp(0.0, 1.0);
                        if (progress - last_reported_pct) >= 0.01
                            || last_report.elapsed().as_millis() >= 60
                        {
                            progress_callback(UpdateStatus::Downloading(progress));
                            last_reported_pct = progress;
                            last_report = Instant::now();
                        }
                    }
                }
                Err(e) => {
                    drop(writer);
                    let _ = fs::remove_file(&new_exe);
                    bail!("Erro durante o download do fluxo de dados: {e}");
                }
            }
        }

        writer
            .flush()
            .context("Falha ao descarregar buffers do executável baixado")?;

        if content_length > 0 && total_read != content_length {
            drop(writer);
            let _ = fs::remove_file(&new_exe);
            bail!(
                "Download incompleto: recebidos {} de {} bytes",
                total_read,
                content_length
            );
        }

        info!("Download concluído com sucesso: {} bytes.", total_read);
    }

    // Validação rigorosa do binário PE Windows (MZ + assinatura PE\0\0)
    if let Err(e) = validate_windows_pe(&new_exe) {
        let _ = fs::remove_file(&new_exe);
        return Err(e);
    }

    progress_callback(UpdateStatus::Downloading(1.0));

    // Substituição a quente do executável no Windows
    if old_exe.exists() {
        let _ = fs::remove_file(&old_exe);
    }

    // Renomeia atual -> .old
    if let Err(e) = fs::rename(&current_exe, &old_exe) {
        let _ = fs::remove_file(&new_exe);
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            bail!(
                "Permissão negada ao substituir {:?}. Execute o CopyFile como Administrador para atualizar nesta pasta.",
                current_exe
            );
        }
        return Err(e).context("Falha ao mover executável atual para backup");
    }

    // Renomeia .new -> atual
    if let Err(e) = fs::rename(&new_exe, &current_exe) {
        // Restaura o backup em caso de erro
        let _ = fs::rename(&old_exe, &current_exe);
        return Err(e).context("Falha ao posicionar novo executável");
    }

    info!("Executável substituído com sucesso por {:?}", current_exe);

    // Reinicia o aplicativo atualizado em novo processo com instrução de limpeza
    Command::new(&current_exe)
        .arg("--cleanup-old")
        .spawn()
        .context("Falha ao iniciar processo atualizado")?;

    progress_callback(UpdateStatus::Success(
        "Atualização concluída com sucesso! Reiniciando...".to_string(),
    ));

    std::thread::sleep(std::time::Duration::from_millis(350));
    std::process::exit(0);
}

/// Remove arquivos temporários de versões anteriores (`.exe.old` e `.exe.new`).
pub fn clean_old_update_files() {
    if let Ok(exe_path) = std::env::current_exe() {
        let (old_exe, new_exe) = update_sidecar_paths(&exe_path);

        if new_exe.exists() {
            let _ = std::fs::remove_file(&new_exe);
        }

        if old_exe.exists() {
            let is_cleanup_arg = std::env::args().any(|a| a == "--cleanup-old");
            std::thread::spawn(move || {
                if is_cleanup_arg {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                }
                for _ in 0..20 {
                    if std::fs::remove_file(&old_exe).is_ok() {
                        tracing::info!(
                            "Executável de versão anterior (.old) removido com sucesso."
                        );
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing_and_comparison() {
        assert_eq!(parse_version("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("V2.10.5"), Some((2, 10, 5)));
        assert_eq!(parse_version("1.5"), Some((1, 5, 0)));
        assert_eq!(parse_version("v1.1.0-rc1"), Some((1, 1, 0)));

        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.9"));
        assert!(is_newer_version("2.0.0", "1.99.99"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.1"));
        assert!(!is_newer_version("0.9.9", "1.0.0"));
    }

    #[test]
    fn test_validate_windows_pe_detects_corrupt_and_valid() {
        let tmp_bad = std::env::temp_dir().join("copyfile_test_bad_pe.bin");
        fs::write(&tmp_bad, vec![0u8; 2048]).unwrap();
        assert!(validate_windows_pe(&tmp_bad).is_err());
        let _ = fs::remove_file(&tmp_bad);

        let tmp_good = std::env::temp_dir().join("copyfile_test_good_pe.bin");
        let mut fake_pe = vec![0u8; 2048];
        fake_pe[0] = b'M';
        fake_pe[1] = b'Z';
        // e_lfanew = 128
        fake_pe[60..64].copy_from_slice(&128u32.to_le_bytes());
        fake_pe[128..132].copy_from_slice(&[b'P', b'E', 0, 0]);
        fs::write(&tmp_good, fake_pe).unwrap();
        assert!(validate_windows_pe(&tmp_good).is_ok());
        let _ = fs::remove_file(&tmp_good);
    }

    #[test]
    fn test_sidecar_paths_preserve_full_filename() {
        let exe = PathBuf::from(r"C:\Tools\copyfile.v1.exe");
        let (old, new) = update_sidecar_paths(&exe);
        assert_eq!(old, PathBuf::from(r"C:\Tools\copyfile.v1.exe.old"));
        assert_eq!(new, PathBuf::from(r"C:\Tools\copyfile.v1.exe.new"));
    }
}
