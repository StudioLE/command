//! Per-write handle that suspends progress bars during output.
use indicatif::MultiProgress;
use std::io::{self, Write};

/// Writer that suspends [`MultiProgress`] bars during each write to `stderr`.
///
/// - Produced by [`ProgressWriterFactory::make_writer`]
/// - Each [`Write::write`] call serializes through [`MultiProgress::suspend`]
pub struct ProgressWriter<'a> {
    multi: &'a MultiProgress,
}

impl<'a> ProgressWriter<'a> {
    /// Create a new [`ProgressWriter`] that borrows the given [`MultiProgress`].
    pub(crate) fn new(multi: &'a MultiProgress) -> Self {
        Self { multi }
    }
}

impl Write for ProgressWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.multi
            .suspend(|| io::stderr().lock().write_all(buf))
            .map(|()| buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.multi.suspend(|| io::stderr().lock().flush())
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use indicatif::ProgressDrawTarget;

    /// `write` returns the byte count without panicking when the bar is hidden.
    #[test]
    fn progress_writer_write() {
        // Arrange
        let multi = MultiProgress::with_draw_target(ProgressDrawTarget::hidden());
        let mut writer = ProgressWriter::new(&multi);
        // Act
        let count = writer.write(b"x").expect("write should succeed");
        // Assert
        assert_eq!(count, 1);
    }
}
