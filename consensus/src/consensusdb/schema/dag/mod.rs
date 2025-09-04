// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schemas for DAG.
//!

use crate::{
    consensusdb::schema::ensure_slice_len_eq,
    dag::{CertifiedNode, Node, NodeId, Vote},
    define_schema,
};
use anyhow::Result;
use velor_crypto::HashValue;
use velor_schemadb::{
    schema::{KeyCodec, ValueCodec},
    ColumnFamilyName,
};
use std::mem::size_of;

pub const NODE_CF_NAME: ColumnFamilyName = "node";

define_schema!(NodeSchema, (), Node, NODE_CF_NAME);

impl KeyCodec<NodeSchema> for () {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(())
    }
}

impl ValueCodec<NodeSchema> for Node {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub const DAG_VOTE_CF_NAME: ColumnFamilyName = "dag_vote";

define_schema!(DagVoteSchema, NodeId, Vote, DAG_VOTE_CF_NAME);

impl KeyCodec<DagVoteSchema> for NodeId {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<DagVoteSchema> for Vote {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub const CERTIFIED_NODE_CF_NAME: ColumnFamilyName = "certified_node";

define_schema!(
    CertifiedNodeSchema,
    HashValue,
    CertifiedNode,
    CERTIFIED_NODE_CF_NAME
);

impl KeyCodec<CertifiedNodeSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<CertifiedNodeSchema> for CertifiedNode {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}
