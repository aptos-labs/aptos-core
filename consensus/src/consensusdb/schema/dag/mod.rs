// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schemas for DAG.
//!

use crate::{
    consensusdb::schema::ensure_slice_len_eq,
    dag::{CertifiedNode, Node, NodeId, Vote},
    define_schema,
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_schemadb::{
    schema::{KeyCodec, ValueCodec},
    ColumnFamilyName,
};
use std::mem::size_of;

pub const DAG0_NODE_CF_NAME: ColumnFamilyName = "dag0:node";
pub const DAG1_NODE_CF_NAME: ColumnFamilyName = "dag1:node";
pub const DAG2_NODE_CF_NAME: ColumnFamilyName = "dag2:node";

define_schema!(Dag0NodeSchema, (), Node, DAG0_NODE_CF_NAME);
define_schema!(Dag1NodeSchema, (), Node, DAG1_NODE_CF_NAME);
define_schema!(Dag2NodeSchema, (), Node, DAG2_NODE_CF_NAME);

impl KeyCodec<Dag0NodeSchema> for () {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(())
    }
}

impl ValueCodec<Dag0NodeSchema> for Node {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl KeyCodec<Dag1NodeSchema> for () {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(())
    }
}

impl ValueCodec<Dag1NodeSchema> for Node {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl KeyCodec<Dag2NodeSchema> for () {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(())
    }
}

impl ValueCodec<Dag2NodeSchema> for Node {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub const DAG0_VOTE_CF_NAME: ColumnFamilyName = "dag0_dag_vote";
pub const DAG1_VOTE_CF_NAME: ColumnFamilyName = "dag1_dag_vote";
pub const DAG2_VOTE_CF_NAME: ColumnFamilyName = "dag2_dag_vote";

define_schema!(Dag0VoteSchema, NodeId, Vote, DAG0_VOTE_CF_NAME);
define_schema!(Dag1VoteSchema, NodeId, Vote, DAG1_VOTE_CF_NAME);
define_schema!(Dag2VoteSchema, NodeId, Vote, DAG2_VOTE_CF_NAME);

impl KeyCodec<Dag0VoteSchema> for NodeId {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<Dag0VoteSchema> for Vote {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl KeyCodec<Dag1VoteSchema> for NodeId {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<Dag1VoteSchema> for Vote {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl KeyCodec<Dag2VoteSchema> for NodeId {

    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<Dag2VoteSchema> for Vote {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub const DAG0_CERTIFIED_NODE_CF_NAME: ColumnFamilyName = "dag0_certified_node";
pub const DAG1_CERTIFIED_NODE_CF_NAME: ColumnFamilyName = "dag1_certified_node";
pub const DAG2_CERTIFIED_NODE_CF_NAME: ColumnFamilyName = "dag2_certified_node";

define_schema!(
    Dag0CertifiedNodeSchema,
    HashValue,
    CertifiedNode,
    DAG0_CERTIFIED_NODE_CF_NAME
);

define_schema!(
    Dag1CertifiedNodeSchema,
    HashValue,
    CertifiedNode,
    DAG1_CERTIFIED_NODE_CF_NAME
);
define_schema!(
    Dag2CertifiedNodeSchema,
    HashValue,
    CertifiedNode,
    DAG2_CERTIFIED_NODE_CF_NAME
);

impl KeyCodec<Dag0CertifiedNodeSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<Dag0CertifiedNodeSchema> for CertifiedNode {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl KeyCodec<Dag1CertifiedNodeSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<Dag1CertifiedNodeSchema> for CertifiedNode {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}




impl KeyCodec<Dag2CertifiedNodeSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<Dag2CertifiedNodeSchema> for CertifiedNode {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}
