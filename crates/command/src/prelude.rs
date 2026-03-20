//! Common imports re-exported for convenience.
#![expect(
    unused_imports,
    reason = "prelude re-exports used conditionally across modules"
)]

pub use crate::event_kind::*;
pub use crate::macros::*;
pub use crate::traits::*;
pub use crate::{define_commands, define_commands_web};

pub(crate) use async_trait::async_trait;
pub(crate) use serde::de::DeserializeOwned;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::any::{Any, TypeId, type_name};
pub(crate) use std::collections::{HashMap, VecDeque};
pub(crate) use std::convert::Infallible;
pub(crate) use std::error::Error;
pub(crate) use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
pub(crate) use std::hash::Hash;
pub(crate) use std::mem::take;
pub(crate) use std::sync::Arc;
pub(crate) use studiole_report::prelude::*;
pub(crate) use thiserror::Error;
pub(crate) use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "server")]
pub use crate::server_prelude::*;
#[cfg(test)]
pub(crate) use crate::tests::*;
