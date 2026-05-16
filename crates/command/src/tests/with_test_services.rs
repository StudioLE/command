//! Extension trait for configuring a [`ServiceBuilder`] with test services.

use crate::prelude::*;
use studiole_logging::prelude::*;

/// Register test services on a [`ServiceBuilder`].
pub trait WithTestServices {
    /// Register logging and command services for tests.
    fn with_test_services(self) -> Self;
}

impl WithTestServices for ServiceBuilder {
    fn with_test_services(self) -> Self {
        self.with_logging(create_logger).with_commands()
    }
}

#[expect(clippy::unnecessary_wraps, reason = "trait signature")]
fn create_logger(_services: &ServiceProvider) -> Result<Logger, Report<ResolveError>> {
    Ok(LoggerBuilder::new().with_level(LogLevel::Debug).build())
}
