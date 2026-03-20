use crate::prelude::*;

/// Lifecycle stage of a command.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum EventKind {
    Queued,
    Executing,
    Succeeded,
    Failed,
}
