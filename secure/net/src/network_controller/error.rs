// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::network_controller;
use crossbeam_channel::{RecvError, SendError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
/// Different reasons for executor service fails to execute a block.
pub enum Error {
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<SendError<network_controller::Message>> for Error {
    fn from(error: SendError<network_controller::Message>) -> Self {
        Self::InternalError(error.to_string())
    }
}

impl From<RecvError> for Error {
    fn from(error: RecvError) -> Self {
        Self::InternalError(error.to_string())
    }
}

impl From<bcs::Error> for Error {
    fn from(error: bcs::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<crate::Error> for Error {
    fn from(error: crate::Error) -> Self {
        Self::InternalError(error.to_string())
    }
}
