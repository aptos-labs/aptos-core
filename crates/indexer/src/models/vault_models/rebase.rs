// Copyright © Aptos Foundation

/**
 * This file defines deserialized rebase module types.
 */

use aptos_api_types::deserialize_from_string;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rebase {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub elastic: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub base: BigDecimal,
}
