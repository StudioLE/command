//! Macros for defining command types.
mod generic_macro;
#[cfg(feature = "server")]
mod server_macro;
mod web_macro;

#[allow(unused)]
pub use generic_macro::*;
#[cfg(feature = "server")]
pub use server_macro::*;
pub use web_macro::*;
