// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Node types of [`JellyfishMerkleTree`](crate::JellyfishMerkleTree)
//!
//! This module defines two types of Jellyfish Merkle tree nodes: [`InternalNode`]
//! and [`LeafNode`] as building blocks of a 256-bit
//! [`JellyfishMerkleTree`](crate::JellyfishMerkleTree). [`InternalNode`] represents a 4-level
//! binary tree to optimize for IOPS: it compresses a tree with 31 nodes into one node with 16
//! chidren at the lowest level. [`LeafNode`] stores the full key and the value associated.

#[cfg(test)]
mod node_type_test;

use crate::metrics::{APTOS_BSMT_INTERNAL_ENCODED_BYTES, APTOS_BSMT_LEAF_ENCODED_BYTES};
use anyhow::{ensure, Result};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_types::{
    proof::{SparseMerkleInternalNode, SparseMerkleLeafNode},
    state_store::node_path::{ChildIndex, NodePath},
    transaction::Version,
};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use itertools::Itertools;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::cast::FromPrimitive;

use bitvec::vec::BitVec;
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::HashMap,
    io::{prelude::*, Cursor, Read, SeekFrom, Write},
    mem::size_of,
};
use thiserror::Error;

/// The unique key of each node.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct NodeKey {
    // The version at which the node is created.
    version: Version,
    // The position this node represents in the tree.
    path: NodePath,
}

impl NodeKey {
    /// Creates a new `NodeKey`.
    pub fn new(version: Version, path: NodePath) -> Self {
        Self { version, path }
    }

    /// A shortcut to generate a node key consisting of a version and an empty path.
    pub fn new_empty_path(version: Version) -> Self {
        Self::new(version, NodePath::new(BitVec::new()))
    }

    /// Gets the version.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Gets the path.
    pub fn path(&self) -> &NodePath {
        &self.path
    }

    /// Generates a child node key based on this node key, false -> left, true -> right.
    pub fn gen_child_node_key(&self, version: Version, child_index: ChildIndex) -> Self {
        let mut path = self.path().clone();
        path.push(child_index);
        Self::new(version, path)
    }

    /// Generates parent node key at the same version based on this node key.
    pub fn gen_parent_node_key(&self) -> Self {
        let mut path = self.path().clone();
        assert!(path.pop().is_some(), "Current node key is root.",);
        Self::new(self.version, path)
    }

    /// Sets the version to the given version.
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    /// Serializes to bytes for physical storage enforcing the same order as that in memory.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = vec![];
        out.write_u64::<BigEndian>(self.version())?;
        out.write_u8(self.path().num_bits() as u8)?;
        out.write_all(self.path().bytes())?;
        Ok(out)
    }

    /// Recovers from serialized bytes in physical storage.
    pub fn decode(val: &[u8]) -> Result<NodeKey> {
        let mut reader = Cursor::new(val);
        let version = reader.read_u64::<BigEndian>()?;
        let num_bits = reader.read_u8()? as usize;
        ensure!(num_bits <= 256, "Invalid length of position: {}", num_bits,);
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(NodeKey::new(
            version,
            NodePath::new_from_vec(num_bits, bytes),
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeType {
    Leaf,
    /// A internal node that haven't been finished the leaf count migration, i.e. None or not all
    /// of the children leaf counts are known.
    Internal {
        leaf_count: usize,
    },
}

/// Each child of [`InternalNode`] encapsulates a nibble forking at this node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Child {
    /// The hash value of this child node.
    pub hash: HashValue,
    /// `version`, the `nibble_path` of the ['NodeKey`] of this [`InternalNode`] the child belongs
    /// to and the child's index constitute the [`NodeKey`] to uniquely identify this child node
    /// from the storage. Used by `[`NodeKey::gen_child_node_key`].
    pub version: Version,
    /// Indicates if the child is a leaf, or if it's an internal node, the total number of leaves
    /// under it (though it can be unknown during migration).
    pub node_type: NodeType,
}

impl Child {
    pub fn new(hash: HashValue, version: Version, node_type: NodeType) -> Self {
        Self {
            hash,
            version,
            node_type,
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf)
    }

    pub fn leaf_count(&self) -> usize {
        match self.node_type {
            NodeType::Leaf => 1,
            NodeType::Internal { leaf_count } => leaf_count,
        }
    }
}

/// [`Children`] is just a collection of children belonging to a [`InternalNode`], indexed from 0 to
/// 15, inclusive.
pub(crate) type Children = HashMap<ChildIndex, Child>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalNode {
    /// Up to 2 children.
    children: Children,
    /// Total number of leaves under this internal node
    leaf_count: usize,
}

impl InternalNode {
    /// Creates a new Internal node.
    pub fn new(children: Children) -> Self {
        Self::new_impl(children).expect("Input children are logical.")
    }

    pub fn new_impl(children: Children) -> Result<Self> {
        // Assert the internal node must have >= 1 children. If it only has one child, it cannot be
        // a leaf node. Otherwise, the leaf node should be a child of this internal node's parent.
        ensure!(!children.is_empty(), "Children must not be empty");
        if children.len() == 1 {
            ensure!(
                !children
                    .values()
                    .next()
                    .expect("Must have 1 element")
                    .is_leaf(),
                "If there's only one child, it must not be a leaf."
            );
        }

        let leaf_count = children.values().map(Child::leaf_count).sum();
        Ok(Self {
            children,
            leaf_count,
        })
    }

    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    pub fn node_type(&self) -> NodeType {
        NodeType::Internal {
            leaf_count: self.leaf_count,
        }
    }

    pub fn hash(&self) -> HashValue {
        SparseMerkleInternalNode::new(self.child_hash(false), self.child_hash(true)).hash()
    }

    pub fn children_sorted(&self) -> impl Iterator<Item = (&ChildIndex, &Child)> {
        self.children
            .iter()
            .sorted_by_key(|(child_index, _)| **child_index)
    }

    pub fn serialize(&self, binary: &mut Vec<u8>) -> Result<()> {
        let (mut existence_bitmap, leaf_bitmap) = self.generate_bitmaps();
        binary.write_u8(existence_bitmap)?;
        binary.write_u8(leaf_bitmap)?;
        for _ in 0..existence_bitmap.count_ones() {
            let next_child = existence_bitmap.trailing_zeros() == 1;
            let child = &self.children[&next_child];
            serialize_u64_varint(child.version, binary);
            binary.extend(child.hash.to_vec());
            match child.node_type {
                NodeType::Leaf => (),
                NodeType::Internal { leaf_count } => {
                    serialize_u64_varint(leaf_count as u64, binary);
                }
            };
            existence_bitmap &= !(1 << next_child as u8);
        }
        Ok(())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = Cursor::new(data);
        let len = data.len();

        // Read and validate existence and leaf bitmaps
        let mut existence_bitmap = reader.read_u8()?;
        let leaf_bitmap = reader.read_u8()?;
        match existence_bitmap {
            0 => return Err(NodeDecodeError::NoChildren.into()),
            _ if (existence_bitmap & leaf_bitmap) != leaf_bitmap => {
                return Err(NodeDecodeError::ExtraLeaves {
                    existing: existence_bitmap,
                    leaves: leaf_bitmap,
                }
                .into())
            }
            _ => (),
        }

        // Reconstruct children
        let mut children = HashMap::new();
        for _ in 0..existence_bitmap.count_ones() {
            let next_child = existence_bitmap.trailing_zeros() == 1;
            let version = deserialize_u64_varint(&mut reader)?;
            let pos = reader.position() as usize;
            let remaining = len - pos;

            ensure!(
                remaining >= size_of::<HashValue>(),
                "not enough bytes left, children: {}, bytes: {}",
                existence_bitmap.count_ones(),
                remaining
            );
            let hash = HashValue::from_slice(&reader.get_ref()[pos..pos + size_of::<HashValue>()])?;
            reader.seek(SeekFrom::Current(size_of::<HashValue>() as i64))?;

            let child_bit = 1 << next_child as u8;
            let node_type = if (leaf_bitmap & child_bit) != 0 {
                NodeType::Leaf
            } else {
                let leaf_count = deserialize_u64_varint(&mut reader)? as usize;
                NodeType::Internal { leaf_count }
            };

            children.insert(next_child, Child::new(hash, version, node_type));
            existence_bitmap &= !child_bit;
        }
        assert_eq!(existence_bitmap, 0);

        Self::new_impl(children)
    }

    /// Gets the `n`-th child.
    pub fn child(&self, n: ChildIndex) -> Option<&Child> {
        self.children.get(&n)
    }

    fn child_hash(&self, n: ChildIndex) -> HashValue {
        match self.child(n) {
            Some(child) => child.hash,
            None => *SPARSE_MERKLE_PLACEHOLDER_HASH,
        }
    }

    pub fn generate_bitmaps(&self) -> (u8, u8) {
        let mut existence_bitmap = 0;
        let mut leaf_bitmap = 0;
        for (child_index, child) in self.children.iter() {
            existence_bitmap |= 1u8 << (child_index.clone() as u8);
            if child.is_leaf() {
                leaf_bitmap |= 1u8 << (child_index.clone() as u8);
            }
        }
        // `leaf_bitmap` must be a subset of `existence_bitmap`.
        assert_eq!(existence_bitmap | leaf_bitmap, existence_bitmap);
        (existence_bitmap, leaf_bitmap)
    }

    pub fn get_child_with_sibling(
        &self,
        node_key: &NodeKey,
        child_index: ChildIndex,
    ) -> (Option<NodeKey>, HashValue) {
        let sibling_hash = self.child_hash(!child_index);

        match self.child(child_index) {
            Some(child) => (
                Some(node_key.gen_child_node_key(child.version, child_index)),
                sibling_hash,
            ),
            None => (None, sibling_hash),
        }
    }
}

/// Represents an account.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LeafNode<K> {
    // The hashed key associated with this leaf node.
    account_key: HashValue,
    // The hash of the value.
    value_hash: HashValue,
    // The key and version thats points to the value
    value_index: (K, Version),
}

impl<K> LeafNode<K>
where
    K: crate::Key,
{
    /// Creates a new leaf node.
    pub fn new(account_key: HashValue, value_hash: HashValue, value_index: (K, Version)) -> Self {
        Self {
            account_key,
            value_hash,
            value_index,
        }
    }

    /// Gets the account key, the hashed account address.
    pub fn account_key(&self) -> HashValue {
        self.account_key
    }

    /// Gets the associated value hash.
    pub fn value_hash(&self) -> HashValue {
        self.value_hash
    }

    /// Get the index key to locate the value.
    pub fn value_index(&self) -> &(K, Version) {
        &self.value_index
    }

    pub fn hash(&self) -> HashValue {
        SparseMerkleLeafNode::new(self.account_key, self.value_hash).hash()
    }
}

impl<K> From<LeafNode<K>> for SparseMerkleLeafNode {
    fn from(leaf_node: LeafNode<K>) -> Self {
        Self::new(leaf_node.account_key, leaf_node.value_hash)
    }
}

#[repr(u8)]
#[derive(FromPrimitive, ToPrimitive)]
enum NodeTag {
    Leaf = 1,
    Internal = 2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node<K> {
    /// A wrapper of [`InternalNode`].
    Internal(InternalNode),
    /// A wrapper of [`LeafNode`].
    Leaf(LeafNode<K>),
}

impl<K> From<InternalNode> for Node<K> {
    fn from(node: InternalNode) -> Self {
        Node::Internal(node)
    }
}

impl From<InternalNode> for Children {
    fn from(node: InternalNode) -> Self {
        node.children
    }
}

impl<K> From<LeafNode<K>> for Node<K> {
    fn from(node: LeafNode<K>) -> Self {
        Node::Leaf(node)
    }
}

impl<K> Node<K>
where
    K: crate::Key,
{
    /// Creates the [`Leaf`](Node::Leaf) variant.
    pub fn new_leaf(
        account_key: HashValue,
        value_hash: HashValue,
        value_index: (K, Version),
    ) -> Self {
        Node::Leaf(LeafNode::new(account_key, value_hash, value_index))
    }

    /// Returns `true` if the node is a leaf node.
    pub fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf(_))
    }

    /// Returns `NodeType`
    pub fn node_type(&self) -> NodeType {
        match self {
            // The returning value will be used to construct a `Child` of a internal node, while an
            // internal node will never have a child of Node::Null.
            Self::Leaf(_) => NodeType::Leaf,
            Self::Internal(n) => n.node_type(),
        }
    }

    /// Returns leaf count if known
    pub fn leaf_count(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Internal(internal_node) => internal_node.leaf_count,
        }
    }

    /// Serializes to bytes for physical storage.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = vec![];

        match self {
            Node::Internal(internal_node) => {
                out.push(NodeTag::Internal as u8);
                internal_node.serialize(&mut out)?;
                APTOS_BSMT_INTERNAL_ENCODED_BYTES.inc_by(out.len() as u64);
            }
            Node::Leaf(leaf_node) => {
                out.push(NodeTag::Leaf as u8);
                out.extend(bcs::to_bytes(&leaf_node)?);
                APTOS_BSMT_LEAF_ENCODED_BYTES.inc_by(out.len() as u64);
            }
        }
        Ok(out)
    }

    /// Computes the hash of nodes.
    pub fn hash(&self) -> HashValue {
        match self {
            Node::Internal(internal_node) => internal_node.hash(),
            Node::Leaf(leaf_node) => leaf_node.hash(),
        }
    }

    /// Recovers from serialized bytes in physical storage.
    pub fn decode(val: &[u8]) -> Result<Node<K>> {
        if val.is_empty() {
            return Err(NodeDecodeError::EmptyInput.into());
        }
        let tag = val[0];
        let node_tag = NodeTag::from_u8(tag);
        match node_tag {
            Some(NodeTag::Internal) => Ok(Node::Internal(InternalNode::deserialize(&val[1..])?)),
            Some(NodeTag::Leaf) => Ok(Node::Leaf(bcs::from_bytes(&val[1..])?)),
            None => Err(NodeDecodeError::UnknownTag { unknown_tag: tag }.into()),
        }
    }
}

/// Error thrown when a [`Node`] fails to be deserialized out of a byte sequence stored in physical
/// storage, via [`Node::decode`].
#[derive(Debug, Error, Eq, PartialEq)]
pub enum NodeDecodeError {
    /// Input is empty.
    #[error("Missing tag due to empty input")]
    EmptyInput,

    /// The first byte of the input is not a known tag representing one of the variants.
    #[error("lead tag byte is unknown: {}", unknown_tag)]
    UnknownTag { unknown_tag: u8 },

    /// No children found in internal node
    #[error("No children found in internal node")]
    NoChildren,

    /// Extra leaf bits set
    #[error(
        "Non-existent leaf bits set, existing: {}, leaves: {}",
        existing,
        leaves
    )]
    ExtraLeaves { existing: u8, leaves: u8 },
}

/// Helper function to serialize version in a more efficient encoding.
/// We use a super simple encoding - the high bit is set if more bytes follow.
fn serialize_u64_varint(mut num: u64, binary: &mut Vec<u8>) {
    for _ in 0..8 {
        let low_bits = num as u8 & 0x7f;
        num >>= 7;
        let more = match num {
            0 => 0u8,
            _ => 0x80,
        };
        binary.push(low_bits | more);
        if more == 0 {
            return;
        }
    }
    // Last byte is encoded raw; this means there are no bad encodings.
    assert_ne!(num, 0);
    assert!(num <= 0xff);
    binary.push(num as u8);
}

/// Helper function to deserialize versions from above encoding.
fn deserialize_u64_varint<T>(reader: &mut T) -> Result<u64>
where
    T: Read,
{
    let mut num = 0u64;
    for i in 0..8 {
        let byte = reader.read_u8()?;
        num |= u64::from(byte & 0x7f) << (i * 7);
        if (byte & 0x80) == 0 {
            return Ok(num);
        }
    }
    // Last byte is encoded as is.
    let byte = reader.read_u8()?;
    num |= u64::from(byte) << 56;
    Ok(num)
}
