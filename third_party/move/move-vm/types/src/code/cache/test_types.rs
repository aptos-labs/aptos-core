// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::code::ModuleCode;
use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct MockDeserializedCode(usize);

impl MockDeserializedCode {
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    pub fn value(&self) -> usize {
        self.0
    }
}

pub struct MockVerifiedCode(Arc<MockDeserializedCode>);

impl MockVerifiedCode {
    pub fn new(value: usize) -> Self {
        Self(Arc::new(MockDeserializedCode(value)))
    }
}

impl Deref for MockVerifiedCode {
    type Target = Arc<MockDeserializedCode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn mock_deserialized_code<V: Clone + Ord>(
    value: usize,
    version: V,
) -> Arc<ModuleCode<MockDeserializedCode, MockVerifiedCode, (), V>> {
    Arc::new(ModuleCode::from_deserialized(
        MockDeserializedCode::new(value),
        Arc::new(()),
        version,
    ))
}

pub fn mock_verified_code<V: Clone + Ord>(
    value: usize,
    version: V,
) -> Arc<ModuleCode<MockDeserializedCode, MockVerifiedCode, (), V>> {
    Arc::new(ModuleCode::from_verified(
        MockVerifiedCode::new(value),
        Arc::new(()),
        version,
    ))
}
