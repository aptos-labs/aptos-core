// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::vm::environment::AptosEnvironment;
use move_binary_format::deserializer::DeserializerConfig;

/// Anything implementing this trait can deserialize code (scripts or modules) or anything
/// else that needs a [DeserializerConfig].
pub trait WithDeserializerConfig {
    fn deserializer_config(&self) -> &DeserializerConfig;
}

impl WithDeserializerConfig for AptosEnvironment {
    fn deserializer_config(&self) -> &DeserializerConfig {
        todo!()
    }
}

// Dummy implementation for testing.
impl WithDeserializerConfig for () {
    fn deserializer_config(&self) -> &DeserializerConfig {
        todo!()
    }
}
