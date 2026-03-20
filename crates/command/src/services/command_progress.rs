use crate::prelude::*;

/// Snapshot of command queue progress.
#[derive(Clone, Debug, Default)]
pub struct CommandProgress {
    /// Total number of commands queued
    pub total: usize,
    /// Number of commands currently queued
    pub queued: usize,
    /// Number of commands currently executing
    pub executing: usize,
    /// Number of commands completed
    pub completed: usize,
}
