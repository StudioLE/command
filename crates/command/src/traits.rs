use crate::prelude::*;

/// Handle execution of a request.
#[async_trait]
pub trait Execute<In, Out, E> {
    /// Execute a request and return the result.
    async fn execute(&self, request: &In) -> Result<Out, E>;
}

/// A request that can be executed by a handler.
pub trait Executable: Clone + Display + Sized {
    /// Successful result type.
    type Response: Debug + Send + Sync;
    /// Error type returned on failure.
    type ExecutionError: Debug + Send + Sync;
}
