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

impl<T: ICommandInfo + 'static> Service for CommandRunner<T> {
    type Error = ServiceError;

    async fn from_services(services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        Ok(Self::new(
            services.get_service().await?,
            services.get_service().await?,
            services.get_service().await?,
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
        let services = ServiceProvider::new()
            .with_commands()
            .await
            .expect("should be able to create services with commands");
        let runner = services
            .get_service::<CommandRunner<CommandInfo>>()
            .await
            .expect("should be able to get runner");
        let events = services
            .get_service::<CommandEvents<CommandInfo>>()
            .await
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
