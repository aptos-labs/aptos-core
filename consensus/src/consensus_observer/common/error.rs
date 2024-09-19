// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_network::protocols::network::RpcError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid message error: {0}")]
    InvalidMessageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Aptos network rpc error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Subscription disconnected: {0}")]
    SubscriptionDisconnected(String),

    #[error("Subscription progress stopped: {0}")]
    SubscriptionProgressStopped(String),

    #[error("Subscriptions reset: {0}")]
    SubscriptionsReset(String),

    #[error("Subscription suboptimal: {0}")]
    SubscriptionSuboptimal(String),

    #[error("Subscription timeout: {0}")]
    SubscriptionTimeout(String),

    #[error("Unexpected error encountered: {0}")]
    UnexpectedError(String),
}

impl Error {
    /// Returns a summary label for the error
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::InvalidMessageError(_) => "invalid_message_error",
            Self::NetworkError(_) => "network_error",
            Self::RpcError(_) => "rpc_error",
            Self::SubscriptionDisconnected(_) => "subscription_disconnected",
            Self::SubscriptionProgressStopped(_) => "subscription_progress_stopped",
            Self::SubscriptionsReset(_) => "subscriptions_reset",
            Self::SubscriptionSuboptimal(_) => "subscription_suboptimal",
            Self::SubscriptionTimeout(_) => "subscription_timeout",
            Self::UnexpectedError(_) => "unexpected_error",
        }
    }
}

impl From<aptos_network::application::error::Error> for Error {
    fn from(error: aptos_network::application::error::Error) -> Self {
        Error::NetworkError(error.to_string())
    }
}
