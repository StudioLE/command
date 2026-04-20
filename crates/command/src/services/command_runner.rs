#![allow(dead_code)]

use crate::prelude::*;
use tokio::sync::MutexGuard;

/// Current state of the [`CommandRunner`].
#[derive(Clone, Copy, Debug, Default, Eq, Error, PartialEq)]
pub enum RunnerStatus {
    #[default]
    #[error("Runner is stopped")]
    Stopped,
    #[error("Stopping when the active commands are complete")]
    Stopping,
    #[error("Stopping when the queue is empty")]
    Draining,
    #[error("Running")]
    Running,
}

/// Queue and execute commands across a pool of workers.
pub struct CommandRunner<T: ICommandInfo> {
    mediator: Arc<CommandMediator<T>>,
    registry: Arc<CommandRegistry<T>>,
    workers: Arc<WorkerPool<T>>,
}

impl<T: ICommandInfo + 'static> FromServicesAsync for CommandRunner<T> {
    type Error = ResolveError;

    async fn from_services_async(services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        Ok(Self::new(
            services.get::<CommandMediator<T>>()?,
            services.get_async::<CommandRegistry<T>>().await?,
            services.get::<WorkerPool<T>>()?,
        ))
    }
}

impl<T: ICommandInfo + 'static> CommandRunner<T> {
    /// Create a new [`CommandRunner`].
    #[must_use]
    pub fn new(
        mediator: Arc<CommandMediator<T>>,
        registry: Arc<CommandRegistry<T>>,
        workers: Arc<WorkerPool<T>>,
    ) -> Self {
        Self {
            mediator,
            registry,
            workers,
        }
    }

    /// Start any number of workers.
    ///
    /// Each worker will have a unique ID.
    ///
    /// Status will be set to `Running`.
    pub async fn start(&self, worker_count: usize) {
        self.workers.start(worker_count).await;
    }

    /// Stop workers after draining the queue.
    pub async fn drain(&self) {
        self.mediator
            .set_runner_status(RunnerStatus::Draining)
            .await;
        self.workers.wait_for_stop().await;
    }

    /// Stop workers after their current work is complete
    pub async fn stop(&self) {
        self.mediator
            .set_runner_status(RunnerStatus::Stopping)
            .await;
        self.workers.wait_for_stop().await;
    }

    /// Queue a command as a request.
    pub async fn queue_request<R: Executable + Into<T::Request> + Send + Sync + 'static>(
        &self,
        request: R,
    ) -> Result<(), Report<QueueError>> {
        trace!(%request, type = type_name::<R>(), "Queueing");
        let command = self.registry.resolve(request.clone())?;
        trace!(%request, type = type_name::<R>(), "Resolved command");
        self.mediator.queue(request.into(), command).await;
        Ok(())
    }

    /// Lock and return the current command status map.
    ///
    /// The [`MutexGuard`] must be dropped promptly or [`Worker`] execution will block.
    pub async fn get_commands(&self) -> MutexGuard<'_, HashMap<T::Request, CommandStatus<T>>> {
        self.mediator.get_commands().await
    }

    /// Take completed results for a specific request type.
    ///
    /// - Removes matching completed entries from the command map
    /// - Entries still `Queued` or `Executing` are left in the map
    pub async fn take_completed<R>(&self) -> Vec<(R, Result<R::Response, R::ExecutionError>)>
    where
        R: Executable + TryFrom<T::Request, Error = T::Request>,
        R::Response: TryFrom<T::Success, Error = T::Success>,
        R::ExecutionError: TryFrom<T::Failure, Error = T::Failure>,
    {
        let mut commands = self.mediator.get_commands().await;
        let keys: Vec<T::Request> = commands
            .iter()
            .filter(|(k, status)| {
                R::try_from((*k).clone()).is_ok()
                    && matches!(
                        status,
                        CommandStatus::Succeeded(_) | CommandStatus::Failed(_)
                    )
            })
            .map(|(k, _)| k.clone())
            .collect();
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let Some(status) = commands.remove(&key) else {
                unreachable!("already filtered to existing key");
            };
            let request = R::try_from(key).expect("already filtered to matching variant");
            let result = match status {
                CommandStatus::Succeeded(success) => Ok(R::Response::try_from(success)
                    .expect("request variant should match success variant")),
                CommandStatus::Failed(failure) => Err(R::ExecutionError::try_from(failure)
                    .expect("request variant should match failure variant")),
                _ => unreachable!("filtered to completed only"),
            };
            results.push((request, result));
        }
        results
    }

    /// Take succeeded results for a specific request type.
    ///
    /// - Removes matching succeeded entries from the command map
    /// - `Failed`, `Queued`, and `Executing` entries are left in the map
    pub async fn take_succeeded<R>(&self) -> Vec<(R, R::Response)>
    where
        R: Executable + TryFrom<T::Request, Error = T::Request>,
        R::Response: TryFrom<T::Success, Error = T::Success>,
    {
        let mut commands = self.mediator.get_commands().await;
        let keys: Vec<T::Request> = commands
            .iter()
            .filter(|(k, status)| {
                R::try_from((*k).clone()).is_ok() && matches!(status, CommandStatus::Succeeded(_))
            })
            .map(|(k, _)| k.clone())
            .collect();
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let Some(CommandStatus::Succeeded(success)) = commands.remove(&key) else {
                unreachable!("already filtered to succeeded");
            };
            let request = R::try_from(key).expect("already filtered to matching variant");
            let response = R::Response::try_from(success)
                .expect("request variant should match success variant");
            results.push((request, response));
        }
        results
    }

    /// Take failed results for a specific request type.
    ///
    /// - Removes matching failed entries from the command map
    /// - `Succeeded`, `Queued`, and `Executing` entries are left in the map
    pub async fn take_failed<R>(&self) -> Vec<(R, R::ExecutionError)>
    where
        R: Executable + TryFrom<T::Request, Error = T::Request>,
        R::ExecutionError: TryFrom<T::Failure, Error = T::Failure>,
    {
        let mut commands = self.mediator.get_commands().await;
        let keys: Vec<T::Request> = commands
            .iter()
            .filter(|(k, status)| {
                R::try_from((*k).clone()).is_ok() && matches!(status, CommandStatus::Failed(_))
            })
            .map(|(k, _)| k.clone())
            .collect();
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            let Some(CommandStatus::Failed(failure)) = commands.remove(&key) else {
                unreachable!("already filtered to failed");
            };
            let request = R::try_from(key).expect("already filtered to matching variant");
            let error = R::ExecutionError::try_from(failure)
                .expect("request variant should match failure variant");
            results.push((request, error));
        }
        results
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    use std::time::Duration;
    use tokio::time::sleep;

    const WORKER_COUNT: usize = 3;
    const A_COUNT: usize = 10;
    const B_COUNT: usize = 10;
    const A_DURATON: u64 = 100;
    const B_DURATON: u64 = 100;
    #[allow(clippy::as_conversions, clippy::integer_division)]
    const A_TOTAL_DURATON: u64 = (A_COUNT / WORKER_COUNT) as u64 * A_DURATON;

    #[tokio::test]
    async fn command_runner() {
        // Arrange
        let services = ServiceBuilder::new().with_commands().build();
        let runner = services
            .get_async::<CommandRunner<CommandInfo>>()
            .await
            .expect("should be able to get runner");
        let events = services
            .get::<CommandEvents<CommandInfo>>()
            .expect("should be able to get events");
        events.start().await;
        let _logger = init_test_logger();

        // Act
        runner.start(WORKER_COUNT).await;

        info!("Adding {A_COUNT} commands to queue");
        for i in 1..=A_COUNT {
            let request = DelayRequest::new(format!("A{i}"), A_DURATON);
            runner
                .queue_request(request)
                .await
                .expect("should be able to queue command");
        }
        info!("Added {A_COUNT} commands to queue");

        // Assert
        let length = events
            .count()
            .await
            .get_currently_queued()
            .expect("should be able to subtract");
        debug!("Queue: {length}");
        // assert_eq!(length, A_COUNT, "Queue immediately after sending batch A");

        wait(50).await;
        let length = events
            .count()
            .await
            .get_currently_queued()
            .expect("should be able to subtract");
        debug!("Queue: {length}");
        assert_ne!(length, 0, "Queue soon after adding batch A");

        wait(A_TOTAL_DURATON + 100).await;
        let length = events
            .count()
            .await
            .get_currently_queued()
            .expect("should be able to subtract");
        debug!("Queue: {length}");
        assert_eq!(length, 0, "Queue after batch A should have completed");

        info!("Adding {B_COUNT} commands to queue");
        for i in 1..=B_COUNT {
            let request = DelayRequest::new(format!("B{i}"), B_DURATON);
            runner
                .queue_request(request)
                .await
                .expect("should be able to queue command");
        }
        info!("Added {B_COUNT} commands to queue");

        wait(50).await;
        info!("Requesting stop");
        runner.workers.stop().await;
        info!("Completed stop");

        let count = events.count().await;
        let length = count
            .get_currently_queued()
            .expect("should be able to subtract");
        debug!("Queue: {length}");
        assert_eq!(length, 7, "Queue after stop");
        let length = count.succeeded;
        debug!("Succeeded: {length}");
        assert_eq!(length, 13, "Succeeded after stop");
    }

    async fn wait(wait: u64) {
        sleep(Duration::from_millis(wait)).await;
        info!("Waiting {wait} ms");
    }
}
