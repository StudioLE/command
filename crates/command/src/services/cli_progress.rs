use crate::prelude::*;
use indicatif::ProgressBar;
use tokio::spawn;
use tokio::sync::broadcast::error::RecvError;
use tracing::{error, warn};

/// Display command progress as a terminal progress bar.
pub struct CliProgress<T: ICommandInfo> {
    mediator: Arc<CommandMediator<T>>,
    bar: Arc<ProgressBar>,
    handle: Mutex<Option<JoinHandle<()>>>,
    finished: Arc<Mutex<bool>>,
}

impl<T: ICommandInfo + 'static> CliProgress<T> {
    /// Create a new [`CliProgress`] backed by a [`CommandMediator`].
    #[must_use]
    pub fn new(mediator: Arc<CommandMediator<T>>) -> Self {
        Self {
            mediator,
            bar: Arc::new(ProgressBar::new(0)),
            handle: Mutex::default(),
            finished: Arc::new(Mutex::new(false)),
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
            while !*finished.lock().await {
                let event = match receiver.recv().await {
                    Err(RecvError::Lagged(count)) => {
                        warn!("CLI Progress missed {count} events due to lagging");
                        continue;
                    }
                    Err(RecvError::Closed) => {
                        error!("Event pipe was closed. CLI Progress can't proceed.");
                        break;
                    }
                    Ok(event) => event,
                };
                match event.get_kind() {
                    EventKind::Queued => {
                        total += 1;
                        bar.set_length(total);
                    }
                    EventKind::Executing => {}
                    EventKind::Succeeded | EventKind::Failed => {
                        bar.inc(1);
                    }
                }
            }
        });
        *handle_guard = Some(handle);
    }

    /// Stop listening and mark the progress bar as finished.
    pub async fn finish(&self) {
        let mut finished_guard = self.finished.lock().await;
        *finished_guard = true;
        drop(finished_guard);
        let mut handle_guard = self.handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        drop(handle_guard);
        self.bar.finish();
    }
}

impl<T: ICommandInfo + 'static> Service for CliProgress<T> {
    type Error = ServiceError;

    async fn from_services(services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        Ok(Self::new(services.get_service().await?))
    }
}
