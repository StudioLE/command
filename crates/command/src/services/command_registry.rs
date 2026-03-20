use crate::prelude::*;

/// Map request types to their handlers.
pub struct CommandRegistry<T: ICommandInfo> {
    handlers: HashMap<TypeId, T::Handler>,
}

impl<T: ICommandInfo> CommandRegistry<T> {
    /// Create an empty [`CommandRegistry`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::default(),
        }
    }

    /// Register a handler for a request type.
    #[allow(clippy::as_conversions)]
    pub fn register<
        R: Executable + Send + Sync + 'static,
        H: Execute<R, R::Response, R::ExecutionError>,
    >(
        &mut self,
        handler: Arc<H>,
    ) where
        Arc<H>: Into<T::Handler>,
    {
        let request_type = TypeId::of::<R>();
        self.handlers.insert(request_type, handler.into());
    }

    /// Resolve a request into a command by matching it to a registered handler.
    #[allow(clippy::as_conversions)]
    pub fn resolve<R: Executable + Send + Sync + 'static>(
        &self,
        request: R,
    ) -> Result<T::Command, Report<QueueError>> {
        let request_type = TypeId::of::<R>();
        let handler = self
            .handlers
            .get(&request_type)
            .ok_or_else(|| {
                Report::new(QueueError::NoMatch)
                    .attach_with("request_type", || String::from(type_name::<R>()))
                    .attach("request", request.to_string())
            })?
            .clone();
        let command = T::Command::new(request, handler);
        Ok(command)
    }
}

impl<T: ICommandInfo + 'static> Service for CommandRegistry<T> {
    type Error = ServiceError;

    async fn from_services(_services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        let registry = CommandRegistry::new();
        Ok(registry)
    }
}

/// Errors returned by [`CommandRegistry::resolve`].
#[derive(Debug, Error)]
pub enum QueueError {
    #[error("Unable to match request to command")]
    NoMatch,
    #[error("Unable to match request to command")]
    IncorrectCommandType,
}
