// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::code::ModuleCode;
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Debug)]
pub struct MockDeserializedCode(usize);

impl MockDeserializedCode {
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    pub fn value(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
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

pub fn mock_deserialized_code<E>(
    value: usize,
    extension: E,
) -> Arc<ModuleCode<MockDeserializedCode, MockVerifiedCode, E>> {
    Arc::new(ModuleCode::from_deserialized(
        MockDeserializedCode::new(value),
        Arc::new(extension),
    ))
}

pub fn mock_verified_code<E>(
    value: usize,
    extension: E,
) -> Arc<ModuleCode<MockDeserializedCode, MockVerifiedCode, E>> {
    Arc::new(ModuleCode::from_verified(
        MockVerifiedCode::new(value),
        Arc::new(extension),
    ))
}

#[derive(Clone, Debug)]
pub struct MockExtension {
    #[allow(dead_code)]
    size: usize,
}

impl MockExtension {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}

pub fn mock_extension(size: usize) -> Arc<MockExtension> {
    Arc::new(MockExtension::new(size))
}
