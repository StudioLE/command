//! DI-managed factory that owns a shared [`MultiProgress`] and produces [`ProgressWriter`] handles.
use crate::prelude::*;
use indicatif::MultiProgress;
use tracing_subscriber::fmt::MakeWriter;

/// Factory owning a shared [`MultiProgress`] for coordinating progress bars and log output.
///
/// - Implements [`FromServices`] for automatic DI construction
/// - Implements [`MakeWriter`] so it can be passed to [`LoggerBuilder::with_writer`]
/// - Registered by `with_commands()`; resolved by both [`CliProgress`] and the logger
#[derive(Clone)]
pub struct ProgressWriterFactory {
    multi: MultiProgress,
}

impl ProgressWriterFactory {
    /// Clone of the inner [`MultiProgress`] handle.
    ///
    /// - Used by [`CliProgress::new`] to attach bars to the shared group
    #[must_use]
    pub fn multi(&self) -> MultiProgress {
        self.multi.clone()
    }
}

impl FromServices for ProgressWriterFactory {
    type Error = ResolveError;

    fn from_services(_services: &ServiceProvider) -> Result<Self, Report<Self::Error>> {
        Ok(Self {
            multi: MultiProgress::new(),
        })
    }
}

impl<'a> MakeWriter<'a> for ProgressWriterFactory {
    type Writer = ProgressWriter<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        ProgressWriter::new(&self.multi)
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use std::io::Write;

    /// `make_writer().write` returns the byte count.
    #[test]
    fn progress_writer_factory_make_writer_write() {
        // Arrange
        let factory = ProgressWriterFactory {
            multi: MultiProgress::with_draw_target(indicatif::ProgressDrawTarget::hidden()),
        };
        let mut writer = factory.make_writer();
        // Act
        let count = writer.write(b"x").expect("write should succeed");
        // Assert
        assert_eq!(count, 1);
    }

    /// `FromServices` constructs without error.
    #[test]
    fn progress_writer_factory_from_services() {
        // Arrange
        let services = ServiceBuilder::new().build();
        // Act
        let factory = ProgressWriterFactory::from_services(&services);
        // Assert
        assert!(factory.is_ok());
    }
}
