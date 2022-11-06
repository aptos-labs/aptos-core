// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::XERKLE_NODE_CF_NAME;
use anyhow::Result;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::Version;
use aptos_xerkle::node_type::NodeKey;
use byteorder::{BigEndian, WriteBytesExt};
use schemadb::define_schema;
use schemadb::schema::{KeyCodec, SeekKeyCodec, ValueCodec};
use std::mem::size_of;

type Node = aptos_xerkle::node_type::Node<StateKey>;

define_schema!(XerkleNodeSchema, NodeKey, Node, XERKLE_NODE_CF_NAME);

impl KeyCodec<XerkleNodeSchema> for NodeKey {
    fn encode_key(&self) -> Result<Vec<u8>> {
        todo!()
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        todo!()
    }
}

impl ValueCodec<XerkleNodeSchema> for Node {
    fn encode_value(&self) -> Result<Vec<u8>> {
        todo!()
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        todo!()
    }
}

impl SeekKeyCodec<XerkleNodeSchema> for (Version, u8) {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(size_of::<Version>() + size_of::<u8>());
        out.write_u64::<BigEndian>(self.0)?;
        out.write_u8(self.1)?;
        Ok(out)
    }
}
