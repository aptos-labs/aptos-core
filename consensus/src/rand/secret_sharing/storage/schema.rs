// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::SecretShareKey;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
    ColumnFamilyName,
};

pub const SECRET_SHARE_CF_NAME: ColumnFamilyName = "secret_share";

define_schema!(
    SecretShareSchema,
    SecretShareKey,
    Vec<u8>,
    SECRET_SHARE_CF_NAME
);

impl KeyCodec<SecretShareSchema> for SecretShareKey {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<SecretShareSchema> for Vec<u8> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.clone())
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(data.to_vec())
    }
}
