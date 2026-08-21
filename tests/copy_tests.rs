//! Integration tests for CopyFile engine, xxHash3 verification, and metadata preservation.

use copyfile::core::copier::{run_copy_batch, CopyController};
use copyfile::core::hasher::compute_file_xxhash;
use copyfile::core::metadata::copy_metadata;
use copyfile::core::scanner::scan_sources;
use copyfile::core::types::{CopyConfig, CopyEvent, OverwritePolicy};
use crossbeam_channel::unbounded;
use filetime::{set_file_mtime, FileTime};
use std::fs;
use std::path::PathBuf;

fn setup_temp_dir(name: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("copyfile_test_{name}_{}", std::process::id()));
    let src = base.join("src");
    let dst = base.join("dst");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    (src, dst)
}

#[test]
fn test_xxhash3_integrity() {
    let (src, _) = setup_temp_dir("xxhash");
    let test_file = src.join("sample.dat");

    let content = b"High performance backup test with xxHash3 algorithm in Rust.";
    fs::write(&test_file, content).unwrap();

    let hash1 = compute_file_xxhash(&test_file, 64 * 1024).unwrap();
    let hash2 = compute_file_xxhash(&test_file, 64 * 1024).unwrap();

    assert_eq!(hash1, hash2);
    assert!(hash1.contains(':'));
}

#[test]
fn test_batch_copy_and_verification() {
    let (src, dst) = setup_temp_dir("batch_copy");

    // Create nested source files
    let sub = src.join("nested_folder");
    fs::create_dir_all(&sub).unwrap();

    let file1 = src.join("file1.txt");
    let file2 = sub.join("file2.bin");

    fs::write(&file1, "Content of file 1").unwrap();
    fs::write(&file2, vec![0xAB; 1024 * 500]).unwrap(); // 500 KB

    let (entries, total_bytes) = scan_sources(std::slice::from_ref(&src), &dst);
    assert_eq!(entries.len(), 2);
    assert_eq!(total_bytes, 17 + 1024 * 500);

    let (tx, rx) = unbounded();
    let controller = CopyController::new();
    let config = CopyConfig {
        verify_checksum: true,
        ignore_errors: true,
        preserve_metadata: true,
        overwrite_policy: OverwritePolicy::Always,
        num_threads: 2,
        buffer_size_kb: 256,
        ..Default::default()
    };

    run_copy_batch(entries, config, controller, tx);

    let mut completed_files = 0;
    while let Ok(event) = rx.recv() {
        if let CopyEvent::FileCompleted { .. } = event {
            completed_files += 1;
        }
    }

    assert_eq!(completed_files, 2);

    // Verify destination files exist
    let dst_file1 = dst.join("src").join("file1.txt");
    let dst_file2 = dst.join("src").join("nested_folder").join("file2.bin");

    assert!(dst_file1.exists());
    assert!(dst_file2.exists());

    let content_dst1 = fs::read_to_string(&dst_file1).unwrap();
    assert_eq!(content_dst1, "Content of file 1");
}

#[test]
fn test_overwrite_policy_if_hash_differs() {
    let (src, dst) = setup_temp_dir("hash_policy");
    let file1_src = src.join("identico.txt");
    let file2_src = src.join("alterado.txt");

    fs::write(&file1_src, "Conteúdo 100% idêntico").unwrap();
    fs::write(&file2_src, "Conteúdo Novo Atualizado").unwrap();

    // In destination, create one identical file and one differing file
    let dst_src_folder = dst.join("src");
    fs::create_dir_all(&dst_src_folder).unwrap();

    let file1_dst = dst_src_folder.join("identico.txt");
    let file2_dst = dst_src_folder.join("alterado.txt");

    fs::write(&file1_dst, "Conteúdo 100% idêntico").unwrap();
    fs::write(&file2_dst, "Conteúdo Velho Antigo").unwrap();

    let (entries, _) = scan_sources(std::slice::from_ref(&src), &dst);
    assert_eq!(entries.len(), 2);

    let (tx, rx) = unbounded();
    let controller = CopyController::new();
    let config = CopyConfig {
        verify_checksum: true,
        ignore_errors: true,
        preserve_metadata: true,
        overwrite_policy: OverwritePolicy::IfHashDiffers,
        num_threads: 2,
        buffer_size_kb: 256,
        ..Default::default()
    };

    run_copy_batch(entries, config, controller, tx);

    let mut skipped = 0;
    let mut completed = 0;
    while let Ok(event) = rx.recv() {
        match event {
            CopyEvent::FileSkipped { .. } => skipped += 1,
            CopyEvent::FileCompleted { .. } => completed += 1,
            _ => {}
        }
    }

    assert_eq!(skipped, 1);
    assert_eq!(completed, 1);

    // Verify alterado.txt was overwritten with new content
    let content_dst2 = fs::read_to_string(&file2_dst).unwrap();
    assert_eq!(content_dst2, "Conteúdo Novo Atualizado");
}

#[test]
fn test_metadata_preservation() {
    let (src, dst) = setup_temp_dir("metadata");
    let file_src = src.join("meta_src.txt");
    let file_dst = dst.join("meta_dst.txt");

    fs::write(&file_src, "Metadata preservation test").unwrap();
    fs::write(&file_dst, "Metadata preservation test").unwrap();

    // Set a known past modification time on source
    let past_time = FileTime::from_unix_time(1_600_000_000, 0);
    set_file_mtime(&file_src, past_time).unwrap();

    copy_metadata(&file_src, &file_dst).unwrap();

    let meta_src = fs::metadata(&file_src).unwrap();
    let meta_dst = fs::metadata(&file_dst).unwrap();

    let mtime_src = FileTime::from_last_modification_time(&meta_src);
    let mtime_dst = FileTime::from_last_modification_time(&meta_dst);

    assert_eq!(mtime_src.unix_seconds(), mtime_dst.unix_seconds());
}

#[test]
fn test_disk_space_detection() {
    let temp_dir = std::env::temp_dir();
    let space = copyfile::core::get_disk_space(&temp_dir);
    assert!(space.is_some());
    let info = space.unwrap();
    assert!(info.total_bytes > 0);
}

#[test]
fn test_user_presets_detection() {
    let presets = copyfile::core::get_current_user_presets();
    assert!(!presets.is_empty());
}

#[test]
fn test_safe_move_mode() {
    let (src, dst) = setup_temp_dir("safe_move");
    let file_src = src.join("arquivo_para_mover.txt");
    fs::write(&file_src, "Conteúdo que será movido com segurança").unwrap();

    let (entries, _) = scan_sources(std::slice::from_ref(&src), &dst);
    assert_eq!(entries.len(), 1);

    let (tx, rx) = unbounded();
    let controller = CopyController::new();
    let config = CopyConfig {
        transfer_mode: copyfile::core::TransferMode::Move,
        verify_checksum: true,
        ignore_errors: false,
        preserve_metadata: true,
        overwrite_policy: OverwritePolicy::Always,
        num_threads: 1,
        buffer_size_kb: 128,
        ..Default::default()
    };

    run_copy_batch(entries, config, controller, tx);

    let mut completed = 0;
    while let Ok(event) = rx.recv() {
        if let CopyEvent::FileCompleted { .. } = event {
            completed += 1;
        }
    }

    assert_eq!(completed, 1);

    // Destination must exist with correct content
    let file_dst = dst.join("src").join("arquivo_para_mover.txt");
    assert!(file_dst.exists());
    let content = fs::read_to_string(&file_dst).unwrap();
    assert_eq!(content, "Conteúdo que será movido com segurança");

    // Source must have been safely deleted
    assert!(!file_src.exists());
}
