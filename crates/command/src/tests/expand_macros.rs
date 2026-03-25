#![allow(dead_code)]
use crate::prelude::*;

define_commands_web! {
    Delay(DelayRequest),
}

#[cfg(feature = "server")]
define_commands_server! {
    Delay(DelayRequest, DelayHandler),
}
