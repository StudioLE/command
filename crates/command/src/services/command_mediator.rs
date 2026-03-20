use crate::prelude::*;
use tokio::sync::broadcast::{Receiver, Sender, channel};

const CHANNEL_CAPACITY: usize = 16;

/// A mediator between the [`CommandRunner`], [`Worker`] and [`CliProgress`] services.
pub struct CommandMediator<T: ICommandInfo> {
    /// Events
    events: Sender<T::Event>,
    /// Queue of commands to execute
    queue: Mutex<VecDeque<T::Request>>,
    /// Map of requests to their current status.
    commands: Mutex<HashMap<T::Request, CommandStatus<T>>>,
    /// Notify workers when new work is available
    notify_workers: Notify,
    /// Current status of the runner
    runner_status: Mutex<RunnerStatus>,
}

impl<T: ICommandInfo + 'static> Service for CommandMediator<T> {
    type Error = Infallible;

    async fn from_services(_services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        Ok(Self::new())
    }
}

impl<T: ICommandInfo> CommandMediator<T> {
    pub(super) fn new() -> Self {
        let (events, _) = channel::<T::Event>(CHANNEL_CAPACITY);
        Self {
            events,
            queue: Mutex::default(),
            notify_workers: Notify::default(),
            runner_status: Mutex::default(),
            commands: Mutex::default(),
        }
    }

    async fn get_runner_status(&self) -> RunnerStatus {
        *self.runner_status.lock().await
    }
}

// Implementation for `CommandRunner`
impl<T: ICommandInfo> CommandMediator<T> {
    pub(super) async fn set_runner_status(&self, status: RunnerStatus) {
        trace!(?status, "Set runner status");
        let mut status_guard = self.runner_status.lock().await;
        *status_guard = status;
        drop(status_guard);
        self.notify_workers.notify_waiters();
    }

    /// Add a command to the queue.
    ///
    /// If the request is already queued or executing then it's ignored and `false` is returned.
    ///
    /// If added to the queue then progress is updated and subscribers are notified.
    pub(super) async fn queue(&self, request: T::Request, command: T::Command) -> bool {
        trace!(?request, "Queueing");
        let mut commands = self.commands.lock().await;
        if let Some(CommandStatus::Queued(_) | CommandStatus::Executing) = commands.get(&request) {
            trace!(?request, "Skipping as already queued or executing");
            return false;
        }
        commands.insert(request.clone(), CommandStatus::Queued(command));
        drop(commands);
        let _ = self
            .events
            .send(T::Event::new(EventKind::Queued, request.clone(), None));
        let mut queue = self.queue.lock().await;
        queue.push_back(request.clone());
        drop(queue);
        trace!(?request, "Queued");
        trace!(?request, "Notifying worker");
        self.notify_workers.notify_one();
        true
    }

    /// Get the commands.
    ///
    /// Note: The [`MutexGuard`] must be dropped or the [`Worker`] will be unable to finish
    /// execution.
    pub(super) async fn get_commands(
        &self,
    ) -> MutexGuard<'_, HashMap<T::Request, CommandStatus<T>>> {
        self.commands.lock().await
    }
}

// Implementation for `Worker`
impl<T: ICommandInfo> CommandMediator<T> {
    /// Get the next instruction.
    #[allow(clippy::panic)]
    pub(super) async fn get_instruction(&self) -> Instruction<'_, T> {
        let notify = self.notify_workers.notified();
        let mut queue_guard = self.queue.lock().await;
        if self.get_runner_status().await == RunnerStatus::Stopping {
            return Instruction::Stop;
        }
        if let Some(request) = queue_guard.pop_front() {
            drop(queue_guard);
            let _ = self
                .events
                .send(T::Event::new(EventKind::Executing, request.clone(), None));
            let mut commands = self.commands.lock().await;
            let option = commands.insert(request.clone(), CommandStatus::Executing);
            drop(commands);
            let Some(CommandStatus::Queued(command)) = option else {
                panic!("command should be queued but was {option:?}");
            };
            return Instruction::Execute(request, command);
        }
        drop(queue_guard);
        if self.get_runner_status().await == RunnerStatus::Draining {
            return Instruction::Stop;
        }
        Instruction::Wait(notify)
    }

    /// Add the result of a completed execution.
    pub(super) async fn completed(
        &self,
        request: T::Request,
        result: Result<T::Success, T::Failure>,
    ) {
        let mut commands = self.commands.lock().await;
        match result {
            Ok(success) => {
                trace!(?request, "Command succeeded");
                commands.insert(request.clone(), CommandStatus::Succeeded(success.clone()));
                let _ =
                    self.events
                        .send(T::Event::new(EventKind::Succeeded, request, Some(success)));
            }
            Err(failure) => {
                warn!(?request, error = ?failure, "Command failed");
                commands.insert(request.clone(), CommandStatus::Failed(failure));
                let _ = self
                    .events
                    .send(T::Event::new(EventKind::Failed, request, None));
            }
        }
        drop(commands);
    }
}

// Implementation for event subscribers
impl<T: ICommandInfo> CommandMediator<T> {
    /// Subscribe to events.
    pub fn subscribe(&self) -> Receiver<T::Event> {
        self.events.subscribe()
    }
}
