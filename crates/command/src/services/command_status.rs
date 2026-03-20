use crate::prelude::*;

/// Current state of a queued command.
pub enum CommandStatus<T: ICommandInfo> {
    Queued(T::Command),
    Executing,
    Succeeded(T::Success),
    Failed(T::Failure),
}

impl<T: ICommandInfo> Debug for CommandStatus<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Queued(_) => f.write_str("Queued"),
            Self::Executing => f.write_str("Executing"),
            Self::Succeeded(_) => f.write_str("Succeeded"),
            Self::Failed(_) => f.write_str("Failed"),
        }
    }
}
