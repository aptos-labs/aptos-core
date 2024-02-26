// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_types::PeerId;
use futures::channel::{mpsc, oneshot};
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Error: {0:?}")]
    Error(#[from] anyhow::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Bcs error: {0:?}")]
    BcsError(#[from] bcs::Error),

    #[error("Not connected with peer: {0}")]
    NotConnected(PeerId),

    #[error("Received invalid rpc response message")]
    InvalidRpcResponse,

    #[error("Application layer unexpectedly dropped response channel")]
    UnexpectedResponseChannelCancel,

    #[error("Error in application layer handling rpc request: {0:?}")]
    ApplicationError(anyhow::Error),

    #[error("Error sending on mpsc channel, connection likely shutting down: {0:?}")]
    MpscSendError(#[from] mpsc::SendError),

    #[error("Error sending on mpsc channel, connection likely shutting down")]
    TokioMpscSendError,

    #[error("Too many pending RPCs: {0}")]
    TooManyPending(u32),

    #[error("Rpc timed out")]
    TimedOut,
}

impl From<oneshot::Canceled> for RpcError {
    fn from(_: oneshot::Canceled) -> Self {
        RpcError::UnexpectedResponseChannelCancel
    }
}

impl From<tokio::time::error::Elapsed> for RpcError {
    fn from(_err: tokio::time::error::Elapsed) -> RpcError {
        RpcError::TimedOut
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for RpcError {
    fn from(_err: tokio::sync::mpsc::error::SendError<T>) -> RpcError {
        RpcError::TokioMpscSendError
    }
}
