// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::vm::environment::AptosEnvironment;
use move_binary_format::{
    deserializer::DeserializerConfig, errors::PartialVMError, CompiledModule,
};

/// Anything implementing this trait can be deserialized, e.g., code (scripts or modules).
pub trait Deserializable: Sized {
    type Error;

    fn deserialize(bytes: &[u8], config: &DeserializerConfig) -> Result<Self, Self::Error>;
}

impl Deserializable for CompiledModule {
    type Error = PartialVMError;

    fn deserialize(bytes: &[u8], config: &DeserializerConfig) -> Result<Self, Self::Error> {
        CompiledModule::deserialize_with_config(bytes, config)
    }
}

pub trait Deserializer {
    fn deserialize<T: Deserializable<Error = E>, E>(&self, bytes: &[u8]) -> Result<T, E>;
}

impl Deserializer for AptosEnvironment {
    fn deserialize<T: Deserializable<Error = E>, E>(&self, bytes: &[u8]) -> Result<T, E> {
        T::deserialize(bytes, &self.0.deserializer_config)
    }
}

// Dummy implementation for testing.
impl Deserializer for () {
    fn deserialize<T: Deserializable<Error = E>, E>(&self, _bytes: &[u8]) -> Result<T, E> {
        unreachable!("Irrelevant for tests")
    }
}
