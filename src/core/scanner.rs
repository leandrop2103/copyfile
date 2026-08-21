//! Fast multi-threaded directory and file scanner.

use crate::core::metadata::extract_attributes_string;
use crate::core::types::{FileEntry, FileStatus};
use std::fs;
use std::path::{Path, PathBuf};

/// Scans a list of source paths and maps them to target paths inside `dest_root`.
///
/// # Arguments
/// * `sources` - List of selected files and directories.
/// * `dest_root` - Target destination directory.
///
/// # Returns
/// A tuple with the list of `FileEntry` objects and the total combined size in bytes.
#[must_use]
pub fn scan_sources(sources: &[PathBuf], dest_root: &Path) -> (Vec<FileEntry>, u64) {
    let mut entries = Vec::new();
    let mut total_bytes = 0u64;
    let mut current_id = 0;

    for src in sources {
        if !src.exists() {
            continue;
        }

        if src.is_file() {
            if let Ok(meta) = fs::metadata(src) {
                let file_name = src.file_name().unwrap_or_default();
                let dest_path = dest_root.join(file_name);
                let size = meta.len();
                total_bytes += size;

                let attributes = extract_attributes_string(src, &meta);
                let created_time = meta.created().ok();
                let modified_time = meta.modified().ok();

                entries.push(FileEntry {
                    id: current_id,
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
                current_id += 1;
            }
        } else if src.is_dir() {
            // Include folder name in relative destination
            let folder_name = src.file_name().unwrap_or_default();
            scan_dir_recursive(
                src,
                src,
                folder_name.as_ref(),
                dest_root,
                &mut entries,
                &mut total_bytes,
                &mut current_id,
            );
        }
    }

    (entries, total_bytes)
}

fn scan_dir_recursive(
    dir_root: &Path,
    current_dir: &Path,
    folder_prefix: &Path,
    dest_root: &Path,
    entries: &mut Vec<FileEntry>,
    total_bytes: &mut u64,
    current_id: &mut usize,
) {
    let read_dir = match fs::read_dir(current_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for entry_res in read_dir {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let sub_rel = path.strip_prefix(dir_root).unwrap_or(&path);
        let relative_path = folder_prefix.join(sub_rel);
        let dest_path = dest_root.join(&relative_path);

        if meta.is_dir() {
            scan_dir_recursive(
                dir_root,
                &path,
                folder_prefix,
                dest_root,
                entries,
                total_bytes,
                current_id,
            );
        } else if meta.is_file() {
            let size = meta.len();
            *total_bytes += size;

            let attributes = extract_attributes_string(&path, &meta);
            let created_time = meta.created().ok();
            let modified_time = meta.modified().ok();

            entries.push(FileEntry {
                id: *current_id,
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
            *current_id += 1;
        }
    }
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

    let valid_dest_paths: HashSet<&Path> = scanned_entries
        .iter()
        .map(|e| e.dest_path.as_path())
        .collect();

    let mut orphans = Vec::new();

    for src in sources {
        if !src.exists() {
            continue;
        }
        let folder_name = src.file_name().unwrap_or_default();
        let target_dest = dest_root.join(folder_name);

        if target_dest.exists() && target_dest.is_dir() {
            scan_dest_orphans_recursive(&target_dest, &valid_dest_paths, &mut orphans);
        }
    }

    orphans
}

fn scan_dest_orphans_recursive(
    current_dir: &Path,
    valid_dest_paths: &std::collections::HashSet<&Path>,
    orphans: &mut Vec<PathBuf>,
) {
    let read_dir = match fs::read_dir(current_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for entry_res in read_dir {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|ft| ft.is_dir());

        if is_dir {
            scan_dest_orphans_recursive(&path, valid_dest_paths, orphans);
            // Check if directory became empty
            if fs::read_dir(&path).is_ok_and(|mut rd| rd.next().is_none()) {
                orphans.push(path);
            }
        } else if !valid_dest_paths.contains(path.as_path()) {
            orphans.push(path);
        }
    }
}
