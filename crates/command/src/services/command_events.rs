use crate::prelude::*;
use tokio::spawn;
use tokio::sync::broadcast::error::RecvError;
use tracing::{error, warn};

/// Collect and query command lifecycle events.
pub struct CommandEvents<T: ICommandInfo> {
    mediator: Arc<CommandMediator<T>>,
    events: Arc<Mutex<Vec<T::Event>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

/// Aggregate counts of command lifecycle events.
#[derive(Debug, Default)]
pub struct CommandEventCounts {
    /// Number of queued events received.
    pub queued: usize,
    /// Number of executing events received.
    pub executing: usize,
    /// Number of succeeded events received.
    pub succeeded: usize,
    /// Number of failed events received.
    pub failed: usize,
}

impl<T: ICommandInfo + 'static> CommandEvents<T> {
    /// Create a new [`CommandEvents`] backed by a [`CommandMediator`].
    #[must_use]
    pub fn new(mediator: Arc<CommandMediator<T>>) -> Self {
        Self {
            mediator,
            events: Arc::default(),
            handle: Mutex::default(),
        }
    }

    /// Start listening for events from the [`CommandMediator`].
    pub async fn start(&self) {
        let mut handle_guard = self.handle.lock().await;
        if handle_guard.is_some() {
            return;
        }
        let mediator = self.mediator.clone();
        let mut receiver = mediator.subscribe();
        let events = self.events.clone();
        let handle = spawn(async move {
            loop {
                match receiver.recv().await {
                    Err(RecvError::Lagged(count)) => {
                        warn!("CommandEvents missed {count} events due to lagging");
                    }
                    Err(RecvError::Closed) => {
                        error!("Event pipe was closed. CommandEvents can't proceed.");
                        break;
                    }
                    Ok(event) => {
                        let mut events_guard = events.lock().await;
                        events_guard.push(event);
                        drop(events_guard);
                    }
                }
            }
        });
        *handle_guard = Some(handle);
    }

    /// Lock and return the collected events.
    pub async fn get(&self) -> MutexGuard<'_, Vec<T::Event>> {
        self.events.lock().await
    }

    /// Count events by [`EventKind`].
    pub async fn count(&self) -> CommandEventCounts {
        let mut counts = CommandEventCounts::default();
        let events = self.events.lock().await;
        for event in events.iter() {
            match event.get_kind() {
                EventKind::Queued => counts.queued += 1,
                EventKind::Executing => counts.executing += 1,
                EventKind::Succeeded => counts.succeeded += 1,
                EventKind::Failed => counts.failed += 1,
            }
        }
        counts
    }
}

impl CommandEventCounts {
    /// Estimated number of commands currently waiting in the queue.
    ///
    /// - Returns `None` if the subtraction overflows
    /// - For accuracy, [`CommandEvents::start`] must be called before any events occur
    #[must_use]
    pub fn get_currently_queued(&self) -> Option<usize> {
        self.queued.checked_sub(self.executing)
    }

    /// Estimated number of commands currently being executed.
    ///
    /// - Returns `None` if the subtraction overflows
    /// - For accuracy, [`CommandEvents::start`] must be called before any events occur
    #[must_use]
    pub fn get_currently_executing(&self) -> Option<usize> {
        self.executing
            .checked_sub(self.succeeded)?
            .checked_sub(self.failed)
    }
}

impl<T: ICommandInfo + 'static> FromServices for CommandEvents<T> {
    type Error = ResolveError;

    fn from_services(services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        Ok(Self::new(services.get::<CommandMediator<T>>()?))
    }
}
