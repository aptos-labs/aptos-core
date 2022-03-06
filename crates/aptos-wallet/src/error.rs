// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Diem Wallet Error is a convenience enum for generating arbitrary WalletErrors. Currently, only
/// the AptosWalletGeneric error is being used, but there are plans to add more specific errors as
/// the Diem Wallet matures
#[derive(Debug, Error)]
pub enum WalletError {
    /// generic error message
    #[error("{0}")]
    AptosWalletGeneric(String),
}
