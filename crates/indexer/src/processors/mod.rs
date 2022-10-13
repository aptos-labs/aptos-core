// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod coin_processor;
pub mod default_processor;
pub mod marketplace_processor;
pub mod token_processor;

use self::coin_processor::NAME as COIN_PROCESSOR_NAME;
use self::default_processor::NAME as DEFAULT_PROCESSOR_NAME;
use self::marketplace_processor::NAME as MARKETPLACE_PROCESSOR_NAME;
use self::token_processor::NAME as TOKEN_PROCESSOR_NAME;

pub enum Processor {
    CoinProcessor,
    DefaultProcessor,
    TokenProcessor,
    MarketplaceProcessor,
}

impl Processor {
    pub fn from_string(input_str: &String) -> Self {
        match input_str.as_str() {
            DEFAULT_PROCESSOR_NAME => Self::DefaultProcessor,
            TOKEN_PROCESSOR_NAME => Self::TokenProcessor,
            MARKETPLACE_PROCESSOR_NAME => Self::MarketplaceProcessor,
            COIN_PROCESSOR_NAME => Self::CoinProcessor,
            _ => panic!("Processor unsupported {}", input_str),
        }
    }
}
