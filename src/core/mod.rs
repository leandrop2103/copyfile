//! Core engine module for CopyFile.

pub mod copier;
pub mod hasher;
pub mod metadata;
pub mod scanner;
pub mod types;

pub use copier::{run_copy_batch, CopyController};
pub use hasher::{compute_file_xxhash, FastHasher};
pub use metadata::copy_metadata;
pub use scanner::scan_sources;
pub use types::*;
