use crate::prelude::*;
use std::thread::current;
use tokio::runtime::Handle;
use tokio::sync::futures::Notified;

/// Unique identifier for a [`Worker`].
pub type WorkerId = usize;

/// An instruction sent to a [`Worker`].
pub enum Instruction<'a, T: ICommandInfo> {
    Wait(Notified<'a>),
    Stop,
    Execute(T::Request, T::Command),
}

/// A worker that executes commands
///
/// The worker is instructed by a [`CommandMediator`].
pub struct Worker {
    id: WorkerId,
    handle: JoinHandle<()>,
}

impl Worker {
    pub(super) fn new<T: ICommandInfo + 'static>(
        id: WorkerId,
        mediator: Arc<CommandMediator<T>>,
    ) -> Self {
        let thread = current().id();
        trace!(worker = id, ?thread, "Spawning");
        let handle = Handle::current().spawn(async move {
            internal_loop(mediator, id).await;
        });
        trace!(worker = id, "Spawned");
        Self { id, handle }
    }

    /// Wait for the worker to stop
    pub(super) async fn wait_for_stop(self) {
        let _ = self.handle.await;
    }
}

async fn internal_loop<T: ICommandInfo>(mediator: Arc<CommandMediator<T>>, worker: WorkerId) {
    let thread = current().id();
    trace!(worker, ?thread, "Starting");
    loop {
        match mediator.get_instruction().await {
            Instruction::Execute(request, command) => {
                let command_id = command.to_string();
                debug!(worker, ?thread, %command, "Executing");
                let result = command.execute().await;
                mediator.completed(request, result).await;
                trace!(worker, command = command_id, "Executed");
            }
            Instruction::Wait(notified) => {
                trace!(worker, "Waiting");
                notified.await;
            }
            Instruction::Stop => {
                trace!(worker, "Stopping");
                break;
            }
        }
    }
    trace!(worker, ?thread, "Stopped");
}

impl Display for Worker {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        write!(formatter, "Worker {:02}", self.id)
    }
}
