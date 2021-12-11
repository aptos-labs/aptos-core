// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_api_types::U64;
use move_core_types::language_storage::StructTag;
use serde::{Deserialize, Serialize};

pub use diem_types::account_config::{BalanceResource, CORE_CODE_ADDRESS};

#[derive(Debug, Serialize, Deserialize)]
pub struct Diem {
    pub value: U64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    pub coin: Diem,
}

#[derive(Debug)]
pub struct AccountBalance {
    pub currency: StructTag,
    pub amount: u64,
}

impl AccountBalance {
    pub fn currency_code(&self) -> String {
        self.currency.name.to_string()
    }
}
