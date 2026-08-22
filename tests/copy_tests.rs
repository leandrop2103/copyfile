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

#[test]
fn test_export_csv_and_html_reports() {
    use copyfile::core::exporter::{export_csv_report, export_html_report};
    use copyfile::core::types::{FileEntry, FileStatus};
    use std::time::Duration;

    let (src, dst) = setup_temp_dir("reports");
    let entries = vec![
        FileEntry {
            id: 0,
            source_path: src.join("doc1.pdf"),
            dest_path: dst.join("doc1.pdf"),
            relative_path: PathBuf::from("doc1.pdf"),
            is_dir: false,
            size: 1024 * 1024 * 5, // 5 MB
            status: FileStatus::Success {
                hash: "xxh3:abcdef1234567890".to_string(),
                duration_ms: 45,
            },
            hash: Some("xxh3:abcdef1234567890".to_string()),
            attributes: "-a-".to_string(),
            created_time: None,
            modified_time: None,
        },
        FileEntry {
            id: 1,
            source_path: src.join("doc2.txt"),
            dest_path: dst.join("doc2.txt"),
            relative_path: PathBuf::from("doc2.txt"),
            is_dir: false,
            size: 512,
            status: FileStatus::Skipped {
                reason: "Arquivo já existe e é idêntico".to_string(),
            },
            hash: None,
            attributes: "-a-".to_string(),
            created_time: None,
            modified_time: None,
        },
    ];

    let html_path = dst.join("relatorio.html");
    let csv_path = dst.join("relatorio.csv");

    export_html_report(
        &html_path,
        &entries,
        "Copiar (Manter Origens)",
        Duration::from_secs(2),
        5 * 1024 * 1024 + 512,
        1,
        1,
        0,
    )
    .unwrap();

    export_csv_report(&csv_path, &entries).unwrap();

    assert!(html_path.exists());
    assert!(csv_path.exists());

    let html_content = fs::read_to_string(&html_path).unwrap();
    assert!(html_content.contains("xxHash3 (100% Validado)"));
    assert!(html_content.contains("doc1.pdf"));

    let csv_bytes = fs::read(&csv_path).unwrap();
    // Validate UTF-8 BOM
    assert_eq!(&csv_bytes[0..3], &[0xEF, 0xBB, 0xBF]);
    let csv_content = String::from_utf8_lossy(&csv_bytes[3..]);
    assert!(csv_content.contains("\"Concluído\""));
    assert!(csv_content.contains("\"Ignorado\""));
}

#[test]
fn test_transfer_queue_management() {
    use copyfile::core::queue::{QueueStatus, TransferQueue};

    let mut queue = TransferQueue::new();
    assert_eq!(queue.waiting_count(), 0);

    let id1 = queue.enqueue(
        vec![PathBuf::from("C:\\pasta1")],
        PathBuf::from("D:\\backup"),
        CopyConfig::default(),
    );
    let id2 = queue.enqueue(
        vec![PathBuf::from("C:\\pasta2")],
        PathBuf::from("D:\\backup"),
        CopyConfig::default(),
    );

    assert_eq!(queue.waiting_count(), 2);
    assert_eq!(queue.jobs().len(), 2);

    let next = queue.start_next_job().unwrap();
    assert_eq!(next.id, id1);
    assert_eq!(next.status, QueueStatus::Running);
    assert_eq!(queue.waiting_count(), 1);

    queue.mark_completed(id1);
    assert_eq!(queue.jobs()[0].status, QueueStatus::Completed);

    queue.remove_job(id2);
    assert_eq!(queue.waiting_count(), 0);
}

#[test]
fn test_mirror_sync_mode() {
    use copyfile::core::scanner::find_mirror_orphan_entries;
    use copyfile::core::TransferMode;

    let (src, dst) = setup_temp_dir("mirror_sync");

    // Source files: file_a.txt and file_b.txt
    let file_a = src.join("file_a.txt");
    let file_b = src.join("file_b.txt");
    fs::write(&file_a, "File A Content").unwrap();
    fs::write(&file_b, "File B Content").unwrap();

    // Destination target subtree already has file_a.txt (same) and orphan.txt (obsolete)
    let dst_src_folder = dst.join("src");
    fs::create_dir_all(&dst_src_folder).unwrap();

    let dst_file_a = dst_src_folder.join("file_a.txt");
    let dst_orphan = dst_src_folder.join("orphan.txt");
    fs::write(&dst_file_a, "File A Content").unwrap();
    fs::write(&dst_orphan, "Obsolete Orphan Content").unwrap();

    let (entries, _total_bytes) = scan_sources(std::slice::from_ref(&src), &dst);
    assert_eq!(entries.len(), 2);

    let orphans = find_mirror_orphan_entries(std::slice::from_ref(&src), &dst, &entries);
    assert_eq!(orphans.len(), 1);
    assert_eq!(orphans[0], dst_orphan);

    // Delete orphans as done in Mirror Sync
    for orphan in &orphans {
        let _ = fs::remove_file(orphan);
    }

    assert!(!dst_orphan.exists());

    let (tx, rx) = unbounded();
    let controller = CopyController::new();
    let config = CopyConfig {
        transfer_mode: TransferMode::Sync,
        verify_checksum: true,
        ignore_errors: true,
        preserve_metadata: true,
        overwrite_policy: OverwritePolicy::Always,
        num_threads: 2,
        buffer_size_kb: 256,
        ..Default::default()
    };

    run_copy_batch(entries, config, controller, tx);

    let mut completed = 0;
    while let Ok(event) = rx.recv() {
        if let CopyEvent::FileCompleted { .. } = event {
            completed += 1;
        }
    }

    assert_eq!(completed, 2);

    let dst_file_b = dst_src_folder.join("file_b.txt");
    assert!(dst_file_a.exists());
    assert!(dst_file_b.exists());
    assert!(!dst_orphan.exists());
}

#[test]
fn test_clipboard_functions() {
    use copyfile::core::{get_clipboard_files, has_clipboard_files};

    // Calling clipboard functions in test environment should not panic
    let _ = has_clipboard_files();
    let _ = get_clipboard_files();
}
