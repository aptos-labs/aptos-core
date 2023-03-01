// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod default_processor;

use self::default_processor::NAME as DEFAULT_PROCESSOR_NAME;

pub enum Processor {
    DefaultProcessor,
}

impl Processor {
    pub fn from_string(input_str: &String) -> Self {
        match input_str.as_str() {
            DEFAULT_PROCESSOR_NAME => Self::DefaultProcessor,
            _ => panic!("Processor unsupported {}", input_str),
        }
    }
}
