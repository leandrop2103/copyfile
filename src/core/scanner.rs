//! Fast multi-threaded directory and file scanner using Rayon.

use crate::core::metadata::extract_attributes_string;
use crate::core::types::{FileEntry, FileStatus};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Normalizes a path for long-path safety (`\\?\`) on Windows when exceeding 240 chars.
#[must_use]
pub fn normalize_long_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if s.len() >= 240 && !s.starts_with(r"\\?\") {
            if let Some(unc_rest) = s.strip_prefix(r"\\") {
                return PathBuf::from(format!(r"\\?\UNC\{unc_rest}"));
            }
            if path.is_absolute() {
                return PathBuf::from(format!(r"\\?\{s}"));
            }
        }
        path.to_path_buf()
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Scans a list of source paths in parallel and maps them to target paths inside `dest_root`.
/// Automatically skips `dest_root` if it is nested inside a source folder to prevent self-copy loops.
///
/// # Arguments
/// * `sources` - List of selected files and directories.
/// * `dest_root` - Target destination directory.
///
/// # Returns
/// A tuple with the list of `FileEntry` objects and the total combined size in bytes.
#[must_use]
pub fn scan_sources(sources: &[PathBuf], dest_root: &Path) -> (Vec<FileEntry>, u64) {
    let dest_canonical_lower = fs::canonicalize(dest_root)
        .unwrap_or_else(|_| dest_root.to_path_buf())
        .to_string_lossy()
        .to_lowercase();

    let mut entries: Vec<FileEntry> = sources
        .par_iter()
        .flat_map(|src| {
            let mut local = Vec::new();
            if !src.exists() {
                return local;
            }

            if src.is_file() {
                if let Ok(meta) = fs::metadata(src) {
                    let file_name = src.file_name().unwrap_or_default();
                    let dest_path = dest_root.join(file_name);
                    let size = meta.len();

                    let attributes = extract_attributes_string(src, &meta);
                    let created_time = meta.created().ok();
                    let modified_time = meta.modified().ok();

                    local.push(FileEntry {
                        id: 0,
                        source_path: src.clone(),
                        dest_path,
                        relative_path: PathBuf::from(file_name),
                        is_dir: false,
                        size,
                        status: FileStatus::Pending,
                        hash: None,
                        attributes,
                        created_time,
                        modified_time,
                    });
                }
            } else if src.is_dir() {
                let clean_src_str = src.to_string_lossy();
                let clean_src_str = clean_src_str.trim_end_matches(std::path::is_separator);
                let folder_name = Path::new(clean_src_str).file_name().unwrap_or_default();
                let mut dir_entries = scan_dir_parallel(
                    src,
                    src,
                    folder_name.as_ref(),
                    dest_root,
                    &dest_canonical_lower,
                );
                local.append(&mut dir_entries);
            }
            local
        })
        .collect();

    let mut total_bytes = 0u64;
    for (idx, entry) in entries.iter_mut().enumerate() {
        entry.id = idx;
        total_bytes += entry.size;
    }

    (entries, total_bytes)
}

fn scan_dir_parallel(
    dir_root: &Path,
    current_dir: &Path,
    folder_prefix: &Path,
    dest_root: &Path,
    dest_canonical_lower: &str,
) -> Vec<FileEntry> {
    let safe_dir = normalize_long_path(current_dir);
    let read_dir = match fs::read_dir(&safe_dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut files = Vec::new();
    let mut subdirs = Vec::new();

    for entry_res in read_dir {
        let Ok(entry) = entry_res else {
            continue;
        };

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue; // Skip symlinks and junctions to prevent infinite recursion
        }

        // Preserve clean (non-\\?\) path when original was clean so UI and reports remain readable
        let path = current_dir.join(entry.file_name());

        if file_type.is_dir() {
            // Skip destination folder if nested inside source to prevent infinite copy loops
            let cand_canon = fs::canonicalize(&path)
                .unwrap_or_else(|_| path.clone())
                .to_string_lossy()
                .to_lowercase();
            if cand_canon == dest_canonical_lower {
                continue;
            }
            subdirs.push(path);
        } else if file_type.is_file() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let sub_rel = path.strip_prefix(dir_root).unwrap_or(&path);
            let relative_path = folder_prefix.join(sub_rel);
            let dest_path = dest_root.join(&relative_path);
            let size = meta.len();

            let attributes = extract_attributes_string(&path, &meta);
            let created_time = meta.created().ok();
            let modified_time = meta.modified().ok();

            files.push(FileEntry {
                id: 0,
                source_path: path,
                dest_path,
                relative_path,
                is_dir: false,
                size,
                status: FileStatus::Pending,
                hash: None,
                attributes,
                created_time,
                modified_time,
            });
        }
    }

    if !subdirs.is_empty() {
        let mut child_files: Vec<FileEntry> = subdirs
            .par_iter()
            .flat_map(|sub| {
                scan_dir_parallel(
                    dir_root,
                    sub,
                    folder_prefix,
                    dest_root,
                    dest_canonical_lower,
                )
            })
            .collect();
        files.append(&mut child_files);
    }

    files
}

/// Identifies orphan files and empty directories in the destination that do not exist in the sources list.
/// Used specifically for `TransferMode::Sync` (Mirror mode).
#[must_use]
pub fn find_mirror_orphan_entries(
    sources: &[PathBuf],
    dest_root: &Path,
    scanned_entries: &[FileEntry],
) -> Vec<PathBuf> {
    use std::collections::HashSet;

    let valid_dest_paths: HashSet<String> = scanned_entries
        .iter()
        .map(|e| e.dest_path.to_string_lossy().to_lowercase())
        .collect();

    let mut orphans = Vec::new();

    for src in sources {
        if !src.exists() {
            continue;
        }
        let clean_src_str = src.to_string_lossy();
        let clean_src_str = clean_src_str.trim_end_matches(std::path::is_separator);
        let folder_name = Path::new(clean_src_str).file_name().unwrap_or_default();
        let target_dest = dest_root.join(folder_name);

        if target_dest.exists() && target_dest.is_dir() {
            scan_dest_orphans_recursive(&target_dest, &valid_dest_paths, &mut orphans);
        }
    }

    orphans
}

fn scan_dest_orphans_recursive(
    current_dir: &Path,
    valid_dest_paths: &std::collections::HashSet<String>,
    orphans: &mut Vec<PathBuf>,
) {
    let read_dir = match fs::read_dir(current_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for entry_res in read_dir {
        let Ok(entry) = entry_res else {
            continue;
        };

        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|ft| ft.is_dir());

        if is_dir {
            scan_dest_orphans_recursive(&path, valid_dest_paths, orphans);
            if fs::read_dir(&path).is_ok_and(|mut rd| rd.next().is_none()) {
                orphans.push(path);
            }
        } else if !valid_dest_paths.contains(&path.to_string_lossy().to_lowercase()) {
            orphans.push(path);
        }
    }
}
