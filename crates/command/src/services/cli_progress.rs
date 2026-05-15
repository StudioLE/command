use crate::prelude::*;
use indicatif::ProgressBar;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::spawn;
use tokio::sync::broadcast::error::RecvError;
use tracing::{error, warn};

/// Display command progress as a terminal progress bar.
pub struct CliProgress<T: ICommandInfo> {
    mediator: Arc<CommandMediator<T>>,
    bar: Arc<ProgressBar>,
    handle: Mutex<Option<JoinHandle<()>>>,
    finished: Arc<AtomicBool>,
}

impl<T: ICommandInfo + 'static> CliProgress<T> {
    /// Create a new [`CliProgress`] whose progress bar is attached to a shared
    /// [`MultiProgress`].
    ///
    /// - Bars added via `multi.add(...)` redraw cooperatively with each other
    /// - Pair with [`ProgressWriterFactory`] passed to `LoggerBuilder::with_writer`
    ///   to prevent collisions with `tracing` log output
    #[must_use]
    pub fn new(mediator: Arc<CommandMediator<T>>, multi: MultiProgress) -> Self {
        Self {
            mediator,
            bar: Arc::new(multi.add(ProgressBar::new(0))),
            handle: Mutex::default(),
            finished: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start listening for events and updating the progress bar.
    pub async fn start(&self) {
        let mut handle_guard = self.handle.lock().await;
        if handle_guard.is_some() {
            return;
        }
        let mediator = self.mediator.clone();
        let mut receiver = mediator.subscribe();
        let bar = self.bar.clone();
        let finished = self.finished.clone();
        let mut total: u64 = 0;
        let handle = spawn(async move {
            while !finished.load(Ordering::Acquire) {
                match receiver.recv().await {
                    Ok(event) => Self::handle_event(&bar, &mut total, event),
                    Err(RecvError::Lagged(count)) => {
                        warn!("CLI Progress missed {count} events due to lagging");
                    }
                    Err(RecvError::Closed) => {
                        error!("Event pipe was closed. CLI Progress can't proceed.");
                        break;
                    }
                }
            }
        });
        *handle_guard = Some(handle);
    }

    fn handle_event(bar: &ProgressBar, total: &mut u64, event: T::Event) {
        match event.get_kind() {
            EventKind::Queued => {
                *total += 1;
                bar.set_length(*total);
            }
            EventKind::Executing => {}
            EventKind::Succeeded | EventKind::Failed => {
                bar.inc(1);
            }
        }
    }

    /// Signal completion and abort the listener task.
    pub async fn finish(&self) {
        self.finished.store(true, Ordering::Release);
        let mut handle_guard = self.handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        drop(handle_guard);
        self.bar.finish();
    }

    /// Hide the progress bar output.
    #[cfg(test)]
    pub fn hide(&self) {
        self.bar
            .set_draw_target(indicatif::ProgressDrawTarget::hidden());
    }

    /// Progress bar position (completed items).
    #[cfg(test)]
    pub fn position(&self) -> u64 {
        self.bar.position()
    }

    /// Progress bar total length (queued items).
    #[cfg(test)]
    pub fn length(&self) -> Option<u64> {
        self.bar.length()
    }
}

impl<T: ICommandInfo + 'static> FromServices for CliProgress<T> {
    type Error = ResolveError;

    fn from_services(services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        let mediator = services.get::<CommandMediator<T>>()?;
        let factory = services.get::<ProgressWriterFactory>()?;
        Ok(Self::new(mediator, factory.multi()))
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    #![expect(
        clippy::as_conversions,
        reason = "usize to u64 cast in test assertions"
    )]
    use super::*;

    const COMMAND_COUNT: usize = CHANNEL_CAPACITY * 2;
    const WORKER_COUNT: usize = 4;
    const DELAY_MS: u64 = 1;

    #[tokio::test]
    async fn cli_progress_receives_all_events() {
        // Arrange
        let services = ServiceBuilder::new().with_commands().build();
        let runner = services
            .get_async::<CommandRunner<CommandInfo>>()
            .await
            .expect("should be able to get runner");
        let progress = services
            .get::<CliProgress<CommandInfo>>()
            .expect("should be able to get progress");
        let _logger = init_test_logger();
        progress.hide();

        // Act
        progress.start().await;
        runner.start(WORKER_COUNT).await;
        for i in 1..=COMMAND_COUNT {
            let request = DelayRequest::new(format!("P{i}"), DELAY_MS);
            runner
                .queue_request(request)
                .await
                .expect("should be able to queue request");
        }
        runner.drain().await;
        progress.finish().await;

        // Assert
        assert_eq!(
            progress.length(),
            Some(COMMAND_COUNT as u64),
            "progress bar total should match queued commands"
        );
        assert_eq!(
            progress.position(),
            COMMAND_COUNT as u64,
            "progress bar position should match completed commands"
        );
    }

    /// Direct construction attaches the bar to a shared [`MultiProgress`] and still receives events.
    #[tokio::test]
    async fn cli_progress_new_receives_all_events() {
        // Arrange
        use indicatif::ProgressDrawTarget;
        let multi = MultiProgress::with_draw_target(ProgressDrawTarget::hidden());
        let services = ServiceBuilder::new().with_commands().build();
        let mediator = services
            .get::<CommandMediator<CommandInfo>>()
            .expect("should be able to get mediator");
        let runner = services
            .get_async::<CommandRunner<CommandInfo>>()
            .await
            .expect("should be able to get runner");
        let progress = CliProgress::new(mediator, multi);
        let _logger = init_test_logger();

        // Act
        progress.start().await;
        runner.start(WORKER_COUNT).await;
        for i in 1..=COMMAND_COUNT {
            let request = DelayRequest::new(format!("M{i}"), DELAY_MS);
            runner
                .queue_request(request)
                .await
                .expect("should be able to queue request");
        }
        runner.drain().await;
        progress.finish().await;

        // Assert
        assert_eq!(
            progress.length(),
            Some(COMMAND_COUNT as u64),
            "progress bar total should match queued commands"
        );
        assert_eq!(
            progress.position(),
            COMMAND_COUNT as u64,
            "progress bar position should match completed commands"
        );
    }
}
