//! UI sub-views for CopyFile.

pub mod file_list;
pub mod header;
pub mod logs;
pub mod modal;
pub mod options;
pub mod queue;
pub mod speed_graph;
pub mod stats;

pub use file_list::{render_file_list, ListFilter};
pub use header::render_header;
pub use logs::{render_logs, LogFilter};
pub use modal::{
    render_about_modal, render_completion_modal, render_paste_prompt_modal,
    render_sources_manager_modal,
};
pub use options::render_options;
pub use queue::render_queue_view;
pub use speed_graph::{render_speed_graph, SpeedHistory};
pub use stats::render_stats;
