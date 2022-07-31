// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// Defines and holds the publishing policies for the VM. There are three possible configurations:
/// 1. No module publishing, only allowlisted scripts are allowed.
/// 2. No module publishing, custom scripts are allowed.
/// 3. Both module publishing and custom scripts are allowed.
/// We represent these as an enum instead of a struct since allowlisting and module/script
/// publishing are mutually exclusive options.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VMPublishingOption {
    pub is_open_module: bool,
}

impl VMPublishingOption {
    pub fn locked() -> Self {
        Self {
            is_open_module: false,
        }
    }

    pub fn open() -> Self {
        Self {
            is_open_module: true,
        }
    }

    pub fn is_open_module(&self) -> bool {
        self.is_open_module
    }
}

impl OnChainConfig for VMPublishingOption {
    const MODULE_IDENTIFIER: &'static str = "transaction_publishing_option";
    const TYPE_IDENTIFIER: &'static str = "TransactionPublishingOption";
}
