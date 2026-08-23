//! Smart Transfer Queue manager for sequential multi-batch execution.

use crate::core::types::CopyConfig;
use chrono::{DateTime, Local};
use std::path::PathBuf;

/// Status of a transfer job in the execution queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    Waiting,
    Running,
    Completed,
    Cancelled,
}

impl QueueStatus {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Waiting => "⏳ Na Fila",
            Self::Running => "⚡ Em Execução",
            Self::Completed => "✔ Concluído",
            Self::Cancelled => "✖ Cancelado",
        }
    }
}

/// A discrete transfer job added to the queue.
#[derive(Debug, Clone)]
pub struct TransferJob {
    pub id: usize,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub config: CopyConfig,
    pub added_at: DateTime<Local>,
    pub status: QueueStatus,
}

/// Transfer Queue manager.
#[derive(Debug, Clone, Default)]
pub struct TransferQueue {
    jobs: Vec<TransferJob>,
    next_id: usize,
}

impl TransferQueue {
    #[must_use]
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            next_id: 1,
        }
    }

    /// Enqueues a new transfer job.
    pub fn enqueue(
        &mut self,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        config: CopyConfig,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        self.jobs.push(TransferJob {
            id,
            sources,
            destination,
            config,
            added_at: Local::now(),
            status: QueueStatus::Waiting,
        });

        id
    }

    /// Returns a reference to all jobs in the queue.
    #[must_use]
    pub fn jobs(&self) -> &[TransferJob] {
        &self.jobs
    }

    /// Returns a mutable reference to all jobs.
    pub fn jobs_mut(&mut self) -> &mut Vec<TransferJob> {
        &mut self.jobs
    }

    /// Returns the number of waiting jobs.
    #[must_use]
    pub fn waiting_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.status == QueueStatus::Waiting)
            .count()
    }

    /// Pops the next waiting job and marks it as Running.
    pub fn start_next_job(&mut self) -> Option<&TransferJob> {
        if let Some(pos) = self
            .jobs
            .iter()
            .position(|j| j.status == QueueStatus::Waiting)
        {
            self.jobs[pos].status = QueueStatus::Running;
            Some(&self.jobs[pos])
        } else {
            None
        }
    }

    /// Marks a job as completed.
    pub fn mark_completed(&mut self, id: usize) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = QueueStatus::Completed;
        }
    }

    /// Removes or cancels a job by ID.
    pub fn remove_job(&mut self, id: usize) {
        self.jobs.retain(|j| j.id != id);
    }

    /// Clears completed or cancelled jobs.
    pub fn clear_finished(&mut self) {
        self.jobs
            .retain(|j| j.status == QueueStatus::Waiting || j.status == QueueStatus::Running);
    }

    /// Moves a job up in the queue.
    pub fn move_up(&mut self, id: usize) {
        if let Some(pos) = self.jobs.iter().position(|j| j.id == id) {
            if pos > 0
                && self.jobs[pos].status == QueueStatus::Waiting
                && self.jobs[pos - 1].status == QueueStatus::Waiting
            {
                self.jobs.swap(pos, pos - 1);
            }
        }
    }

    /// Moves a job down in the queue.
    pub fn move_down(&mut self, id: usize) {
        if let Some(pos) = self.jobs.iter().position(|j| j.id == id) {
            if pos + 1 < self.jobs.len()
                && self.jobs[pos].status == QueueStatus::Waiting
                && self.jobs[pos + 1].status == QueueStatus::Waiting
            {
                self.jobs.swap(pos, pos + 1);
            }
        }
    }
}
