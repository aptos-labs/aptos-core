// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::U64;
use aptos_openapi::impl_poem_type;
use poem_openapi::registry::{MetaSchema, MetaSchemaRef};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// A string-encoded U64 value or an explicit JSON null.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct NullableU64(pub Option<U64>);

impl NullableU64 {
    pub fn new(value: Option<u64>) -> Self {
        Self(value.map(Into::into))
    }

    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }
}

impl_poem_type!(
    NullableU64,
    "",
    (
        description = Some("A string-encoded U64 value or an explicit JSON null."),
        one_of = vec![
            U64::schema_ref(),
            MetaSchemaRef::Inline(Box::new(MetaSchema::new("null"))),
        ]
    )
);

/// The version range for an epoch. The current open epoch returns an explicit
/// `null` for `last_version`.
#[derive(Clone, Debug, Deserialize, Eq, Object, PartialEq, Serialize)]
pub struct Epoch {
    pub epoch: U64,
    pub first_version: U64,
    pub last_version: NullableU64,
}

impl Epoch {
    pub fn new(epoch: u64, first_version: u64, last_version: Option<u64>) -> Self {
        Self {
            epoch: epoch.into(),
            first_version: first_version.into(),
            last_version: NullableU64::new(last_version),
        }
    }
}
