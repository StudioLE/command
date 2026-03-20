use crate::prelude::*;
use std::time::Instant;
use tracing::Level;
use tracing::dispatcher::DefaultGuard;
use tracing::level_filters::LevelFilter;
use tracing::subscriber::set_default;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::layer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

const TEST_LOG_LEVEL: Level = Level::DEBUG;

#[must_use]
pub fn init_test_logger() -> DefaultGuard {
    let targets = get_targets().with_default(LevelFilter::from_level(TEST_LOG_LEVEL));
    let layer = layer()
        .compact()
        .with_test_writer()
        // .with_writer(stderr)
        .with_target(false)
        .with_timer(ElapsedTime::default())
        .with_filter(targets);
    let registry = Registry::default().with(layer);
    set_default(registry)
}

#[must_use]
pub fn get_targets() -> Targets {
    Targets::new()
}

struct ElapsedTime {
    start: Instant,
}

impl Default for ElapsedTime {
    fn default() -> Self {
        ElapsedTime {
            start: Instant::now(),
        }
    }
}

impl FormatTime for ElapsedTime {
    fn format_time(&self, w: &mut Writer<'_>) -> FmtResult {
        let elapsed = self.start.elapsed();
        write!(w, "{:.3}", elapsed.as_secs_f64())
    }
}
