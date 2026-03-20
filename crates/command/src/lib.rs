//! Command execution framework with event subscription and progress tracking.
mod event_kind;
mod macros;
pub mod prelude;
#[cfg(feature = "server")]
mod server_prelude;
#[cfg(feature = "server")]
mod services;
#[cfg(test)]
mod tests;
mod traits;
