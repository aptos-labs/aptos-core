// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Node types of [`JellyfishMerkleTree`](crate::JellyfishMerkleTree)
//!
//! This module defines two types of Jellyfish Merkle tree nodes: [`InternalNode`]
//! and [`LeafNode`] as building blocks of a 256-bit
//! [`JellyfishMerkleTree`](crate::JellyfishMerkleTree). [`InternalNode`] represents a 4-level
//! binary tree to optimize for IOPS: it compresses a tree with 31 nodes into one node with 16
//! children at the lowest level. [`LeafNode`] stores the full key and the value associated.

#[cfg(test)]
mod node_type_test;

use crate::{
    get_hash,
    metrics::{VELOR_JELLYFISH_INTERNAL_ENCODED_BYTES, VELOR_JELLYFISH_LEAF_ENCODED_BYTES},
    Key, TreeReader,
};
use anyhow::{ensure, Context, Result};
use velor_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use velor_types::{
    nibble::{nibble_path::NibblePath, Nibble, ROOT_NIBBLE_HEIGHT},
    proof::{definition::NodeInProof, SparseMerkleInternalNode, SparseMerkleLeafNode},
    transaction::Version,
};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::cast::FromPrimitive;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{collection::btree_map, prelude::*};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{prelude::*, Cursor, Read, SeekFrom, Write},
    mem::size_of,
    sync::Arc,
};
use thiserror::Error;

/// The unique key of each node.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct NodeKey {
    // The version at which the node is created.
    version: Version,
    // The nibble path this node represents in the tree.
    nibble_path: NibblePath,
}

impl NodeKey {
    /// Creates a new `NodeKey`.
    pub fn new(version: Version, nibble_path: NibblePath) -> Self {
        Self {
            version,
            nibble_path,
        }
    }

    /// A shortcut to generate a node key consisting of a version and an empty nibble path.
    pub fn new_empty_path(version: Version) -> Self {
        Self::new(version, NibblePath::new_even(vec![]))
    }

    /// Gets the version.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Gets the nibble path.
    pub fn nibble_path(&self) -> &NibblePath {
        &self.nibble_path
    }

    /// Generates a child node key based on this node key.
    pub fn gen_child_node_key(&self, version: Version, n: Nibble) -> Self {
        let mut node_nibble_path = self.nibble_path().clone();
        node_nibble_path.push(n);
        Self::new(version, node_nibble_path)
    }

    /// Generates parent node key at the same version based on this node key.
    pub fn gen_parent_node_key(&self) -> Self {
        let mut node_nibble_path = self.nibble_path().clone();
        assert!(
            node_nibble_path.pop().is_some(),
            "Current node key is root.",
        );
        Self::new(self.version, node_nibble_path)
    }

    /// Sets the version to the given version.
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    /// Serializes to bytes for physical storage enforcing the same order as that in memory.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = vec![];
        out.write_u64::<BigEndian>(self.version())?;
        out.write_u8(self.nibble_path().num_nibbles() as u8)?;
        out.write_all(self.nibble_path().bytes())?;
        Ok(out)
    }

    /// Recovers from serialized bytes in physical storage.
    pub fn decode(val: &[u8]) -> Result<NodeKey> {
        let mut reader = Cursor::new(val);
        let version = reader.read_u64::<BigEndian>()?;
        let num_nibbles = reader.read_u8()? as usize;
        ensure!(
            num_nibbles <= ROOT_NIBBLE_HEIGHT,
            "Invalid number of nibbles: {}",
            num_nibbles,
        );
        let mut nibble_bytes = Vec::with_capacity((num_nibbles + 1) / 2);
        reader.read_to_end(&mut nibble_bytes)?;
        ensure!(
            (num_nibbles + 1) / 2 == nibble_bytes.len(),
            "encoded num_nibbles {} mismatches nibble path bytes {:?}",
            num_nibbles,
            nibble_bytes
        );
        let nibble_path = if num_nibbles % 2 == 0 {
            NibblePath::new_even(nibble_bytes)
        } else {
            let padding = nibble_bytes.last().unwrap() & 0x0F;
            ensure!(
                padding == 0,
                "Padding nibble expected to be 0, got: {}",
                padding,
            );
            NibblePath::new_odd(nibble_bytes)
        };
        Ok(NodeKey::new(version, nibble_path))
    }

    pub fn unpack(self) -> (Version, NibblePath) {
        (self.version, self.nibble_path)
    }

    // Returns the shard_id of the NodeKey, or None if it is root.
    pub fn get_shard_id(&self) -> Option<usize> {
        self.nibble_path().get_shard_id()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeType {
    Leaf,
    Null,
    /// A internal node that haven't been finished the leaf count migration, i.e. None or not all
    /// of the children leaf counts are known.
    Internal {
        leaf_count: usize,
    },
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for NodeType {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(NodeType::Leaf),
            (2..100usize).prop_map(|leaf_count| NodeType::Internal { leaf_count })
        ]
        .boxed()
    }
}

/// Each child of [`InternalNode`] encapsulates a nibble forking at this node.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct Child {
    /// The hash value of this child node.
    pub hash: HashValue,
    /// `version`, the `nibble_path` of the [`NodeKey`] of this [`InternalNode`] the child belongs
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

    pub fn for_node<K: Key>(
        parent_key: &NodeKey,
        child_index: Nibble,
        child_node: &Node<K>,
        hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
        version: Version,
    ) -> Self {
        let key = parent_key.gen_child_node_key(version, child_index);
        Self {
            hash: get_hash(&key, child_node, hash_cache),
            version,
            node_type: child_node.node_type(),
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf)
    }

    pub fn leaf_count(&self) -> usize {
        match self.node_type {
            NodeType::Leaf => 1,
            NodeType::Internal { leaf_count } => leaf_count,
            NodeType::Null => unreachable!("Child cannot be Null"),
        }
    }
}

/// [`Children`] is just a collection of children belonging to a [`InternalNode`], indexed from 0 to
/// 15, inclusive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Children(Arc<[(Nibble, Child)]>);

impl Children {
    pub fn from_sorted(children: impl IntoIterator<Item = (Nibble, Child)>) -> Self {
        let children = children.into_iter().collect::<Vec<_>>();

        Self(children.into())
    }

    fn get(&self, nibble: &Nibble) -> Option<&Child> {
        self.0
            .binary_search_by_key(nibble, |(nibble, _child)| *nibble)
            .ok()
            .map(|idx| &self.0[idx].1)
    }

    fn iter(&self) -> impl Iterator<Item = (&Nibble, &Child)> {
        self.0.iter().map(|(nibble, child)| (nibble, child))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Represents a 4-level subtree with 16 children at the bottom level. Theoretically, this reduces
/// IOPS to query a tree by 4x since we compress 4 levels in a standard Merkle tree into 1 node.
/// Though we choose the same internal node structure as that of Patricia Merkle tree, the root hash
/// computation logic is similar to a 4-level sparse Merkle tree except for some customizations. See
/// the `CryptoHash` trait implementation below for details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalNode {
    /// Up to 16 children.
    children: Children,
    /// Total number of leaves under this internal node
    leaf_count: usize,
}

/// Computes the hash of internal node according to [`JellyfishMerkleTree`](crate::JellyfishMerkleTree)
/// data structure in the logical view. `start` and `nibble_height` determine a subtree whose
/// root hash we want to get. For an internal node with 16 children at the bottom level, we compute
/// the root hash of it as if a full binary Merkle tree with 16 leaves as below:
///
/// ```text
///   4 ->              +------ root hash ------+
///                     |                       |
///   3 ->        +---- # ----+           +---- # ----+
///               |           |           |           |
///   2 ->        #           #           #           #
///             /   \       /   \       /   \       /   \
///   1 ->     #     #     #     #     #     #     #     #
///           / \   / \   / \   / \   / \   / \   / \   / \
///   0 ->   0   1 2   3 4   5 6   7 8   9 A   B C   D E   F
///   ^
/// height
/// ```
///
/// As illustrated above, at nibble height 0, `0..F` in hex denote 16 children hashes.  Each `#`
/// means the hash of its two direct children, which will be used to generate the hash of its
/// parent with the hash of its sibling. Finally, we can get the hash of this internal node.
///
/// However, if an internal node doesn't have all 16 children exist at height 0 but just a few of
/// them, we have a modified hashing rule on top of what is stated above:
/// 1. From top to bottom, a node will be replaced by a leaf child if the subtree rooted at this
/// node has only one child at height 0 and it is a leaf child.
/// 2. From top to bottom, a node will be replaced by the placeholder node if the subtree rooted at
/// this node doesn't have any child at height 0. For example, if an internal node has 3 leaf
/// children at index 0, 3, 8, respectively, and 1 internal node at index C, then the computation
/// graph will be like:
///
/// ```text
///   4 ->              +------ root hash ------+
///                     |                       |
///   3 ->        +---- # ----+           +---- # ----+
///               |           |           |           |
///   2 ->        #           @           8           #
///             /   \                               /   \
///   1 ->     0     3                             #     @
///                                               / \
///   0 ->                                       C   @
///   ^
/// height
/// Note: @ denotes placeholder hash.
/// ```
#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for InternalNode {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        btree_map(any::<Nibble>(), any::<Child>(), 1..=16)
            .prop_map(|children| InternalNode::new(Children::from_sorted(children)))
            .boxed()
    }
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

        let leaf_count = children.iter().map(|(_, child)| child.leaf_count()).sum();
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
        self.merkle_hash(
            0,  /* start index */
            16, /* the number of leaves in the subtree of which we want the hash of root */
            self.generate_bitmaps(),
        )
    }

    pub fn children_sorted(&self) -> impl Iterator<Item = (&Nibble, &Child)> {
        self.children.iter()
    }

    pub fn serialize(&self, binary: &mut Vec<u8>) -> Result<()> {
        let (mut existence_bitmap, leaf_bitmap) = self.generate_bitmaps();
        binary.write_u16::<LittleEndian>(existence_bitmap)?;
        binary.write_u16::<LittleEndian>(leaf_bitmap)?;
        for (next_child, child) in self.children.iter() {
            serialize_u64_varint(child.version, binary);
            binary.extend(child.hash.to_vec());
            match child.node_type {
                NodeType::Leaf => (),
                NodeType::Internal { leaf_count } => {
                    serialize_u64_varint(leaf_count as u64, binary);
                },
                NodeType::Null => unreachable!("Child cannot be Null"),
            };
            existence_bitmap &= !(1 << next_child.as_usize());
        }
        Ok(())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = Cursor::new(data);
        let len = data.len();

        // Read and validate existence and leaf bitmaps
        let mut existence_bitmap = reader.read_u16::<LittleEndian>()?;
        let leaf_bitmap = reader.read_u16::<LittleEndian>()?;
        match existence_bitmap {
            0 => return Err(NodeDecodeError::NoChildren.into()),
            _ if (existence_bitmap & leaf_bitmap) != leaf_bitmap => {
                return Err(NodeDecodeError::ExtraLeaves {
                    existing: existence_bitmap,
                    leaves: leaf_bitmap,
                }
                .into())
            },
            _ => (),
        }

        // Reconstruct children
        let mut children = Vec::new();
        for _ in 0..existence_bitmap.count_ones() {
            let next_child = existence_bitmap.trailing_zeros() as u8;
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

            let child_bit = 1 << next_child;
            let node_type = if (leaf_bitmap & child_bit) != 0 {
                NodeType::Leaf
            } else {
                let leaf_count = deserialize_u64_varint(&mut reader)? as usize;
                NodeType::Internal { leaf_count }
            };

            children.push((
                Nibble::from(next_child),
                Child::new(hash, version, node_type),
            ));
            existence_bitmap &= !child_bit;
        }
        assert_eq!(existence_bitmap, 0);

        Self::new_impl(Children::from_sorted(children))
    }

    /// Gets the `n`-th child.
    pub fn child(&self, n: Nibble) -> Option<&Child> {
        self.children.get(&n)
    }

    /// Generates `existence_bitmap` and `leaf_bitmap` as a pair of `u16`s: child at index `i`
    /// exists if `existence_bitmap[i]` is set; child at index `i` is leaf node if
    /// `leaf_bitmap[i]` is set.
    pub fn generate_bitmaps(&self) -> (u16, u16) {
        let mut existence_bitmap = 0;
        let mut leaf_bitmap = 0;
        for (nibble, child) in self.children.iter() {
            let i = u8::from(*nibble);
            existence_bitmap |= 1u16 << i;
            if child.is_leaf() {
                leaf_bitmap |= 1u16 << i;
            }
        }
        // `leaf_bitmap` must be a subset of `existence_bitmap`.
        assert_eq!(existence_bitmap | leaf_bitmap, existence_bitmap);
        (existence_bitmap, leaf_bitmap)
    }

    /// Given a range [start, start + width), returns the sub-bitmap of that range.
    fn range_bitmaps(start: u8, width: u8, bitmaps: (u16, u16)) -> (u16, u16) {
        assert!(start < 16 && width.count_ones() == 1 && start % width == 0);
        assert!(width <= 16 && (start + width) <= 16);
        // A range with `start == 8` and `width == 4` will generate a mask 0b0000111100000000.
        // use as converting to smaller integer types when 'width == 16'
        let mask = (((1u32 << width) - 1) << start) as u16;
        (bitmaps.0 & mask, bitmaps.1 & mask)
    }

    fn merkle_hash(
        &self,
        start: u8,
        width: u8,
        (existence_bitmap, leaf_bitmap): (u16, u16),
    ) -> HashValue {
        // Given a bit [start, 1 << nibble_height], return the value of that range.
        let (range_existence_bitmap, range_leaf_bitmap) =
            Self::range_bitmaps(start, width, (existence_bitmap, leaf_bitmap));
        if range_existence_bitmap == 0 {
            // No child under this subtree
            *SPARSE_MERKLE_PLACEHOLDER_HASH
        } else if width == 1 || (range_existence_bitmap.count_ones() == 1 && range_leaf_bitmap != 0)
        {
            // Only 1 leaf child under this subtree or reach the lowest level
            let only_child_index = Nibble::from(range_existence_bitmap.trailing_zeros() as u8);
            self.child(only_child_index)
                .with_context(|| {
                    format!(
                        "Corrupted internal node: existence_bitmap indicates \
                         the existence of a non-exist child at index {:x}",
                        only_child_index
                    )
                })
                .unwrap()
                .hash
        } else {
            let left_child = self.merkle_hash(
                start,
                width / 2,
                (range_existence_bitmap, range_leaf_bitmap),
            );
            let right_child = self.merkle_hash(
                start + width / 2,
                width / 2,
                (range_existence_bitmap, range_leaf_bitmap),
            );
            SparseMerkleInternalNode::new(left_child, right_child).hash()
        }
    }

    fn gen_node_in_proof<K: crate::Key, R: TreeReader<K>>(
        &self,
        start: u8,
        width: u8,
        (existence_bitmap, leaf_bitmap): (u16, u16),
        (tree_reader, node_key): (&R, &NodeKey),
    ) -> Result<NodeInProof> {
        // Given a bit [start, 1 << nibble_height], return the value of that range.
        let (range_existence_bitmap, range_leaf_bitmap) =
            Self::range_bitmaps(start, width, (existence_bitmap, leaf_bitmap));
        Ok(if range_existence_bitmap == 0 {
            // No child under this subtree
            NodeInProof::Other(*SPARSE_MERKLE_PLACEHOLDER_HASH)
        } else if width == 1 || (range_existence_bitmap.count_ones() == 1 && range_leaf_bitmap != 0)
        {
            // Only 1 leaf child under this subtree or reach the lowest level
            let only_child_index = Nibble::from(range_existence_bitmap.trailing_zeros() as u8);
            let only_child = self
                .child(only_child_index)
                .with_context(|| {
                    format!(
                        "Corrupted internal node: existence_bitmap indicates \
                         the existence of a non-exist child at index {:x}",
                        only_child_index
                    )
                })
                .unwrap();
            if matches!(only_child.node_type, NodeType::Leaf) {
                let only_child_node_key =
                    node_key.gen_child_node_key(only_child.version, only_child_index);
                match tree_reader.get_node_with_tag(&only_child_node_key, "get_proof")? {
                    Node::Internal(_) => unreachable!(
                        "Corrupted internal node: in-memory leaf child is internal node on disk"
                    ),
                    Node::Leaf(leaf_node) => {
                        NodeInProof::Leaf(SparseMerkleLeafNode::from(leaf_node))
                    },
                    Node::Null => unreachable!("Child cannot be Null"),
                }
            } else {
                NodeInProof::Other(only_child.hash)
            }
        } else {
            let left_child = self.merkle_hash(
                start,
                width / 2,
                (range_existence_bitmap, range_leaf_bitmap),
            );
            let right_child = self.merkle_hash(
                start + width / 2,
                width / 2,
                (range_existence_bitmap, range_leaf_bitmap),
            );
            NodeInProof::Other(SparseMerkleInternalNode::new(left_child, right_child).hash())
        })
    }

    /// Gets the child and its corresponding siblings that are necessary to generate the proof for
    /// the `n`-th child. If it is an existence proof, the returned child must be the `n`-th
    /// child; otherwise, the returned child may be another child. See inline explanation for
    /// details. When calling this function with n = 11 (node `b` in the following graph), the
    /// range at each level is illustrated as a pair of square brackets:
    ///
    /// ```text
    ///     4      [f   e   d   c   b   a   9   8   7   6   5   4   3   2   1   0] -> root level
    ///            ---------------------------------------------------------------
    ///     3      [f   e   d   c   b   a   9   8] [7   6   5   4   3   2   1   0] width = 8
    ///                                  chs <--┘                        shs <--┘
    ///     2      [f   e   d   c] [b   a   9   8] [7   6   5   4] [3   2   1   0] width = 4
    ///                  shs <--┘               └--> chs
    ///     1      [f   e] [d   c] [b   a] [9   8] [7   6] [5   4] [3   2] [1   0] width = 2
    ///                          chs <--┘       └--> shs
    ///     0      [f] [e] [d] [c] [b] [a] [9] [8] [7] [6] [5] [4] [3] [2] [1] [0] width = 1
    ///     ^                chs <--┘   └--> shs
    ///     |   MSB|<---------------------- uint 16 ---------------------------->|LSB
    ///  height    chs: `child_half_start`         shs: `sibling_half_start`
    /// ```
    pub fn get_child_with_siblings<K: crate::Key, R: TreeReader<K>>(
        &self,
        node_key: &NodeKey,
        n: Nibble,
        reader: Option<&R>,
        out_siblings: &mut Vec<NodeInProof>,
        root_depth: usize,
        target_depth: usize,
    ) -> Result<Option<NodeKey>> {
        assert!(self.leaf_count > 1);

        let (existence_bitmap, leaf_bitmap) = self.generate_bitmaps();

        // Nibble height from 3 to 0.
        for h in (0..4).rev() {
            // Get the number of children of the internal node that each subtree at this height
            // covers.
            let width = 1 << h;
            let (child_half_start, sibling_half_start) = get_child_and_sibling_half_start(n, h);
            // Compute the root hash of the subtree rooted at the sibling of `r`.
            let depth = root_depth + 3 - h as usize;
            if depth >= target_depth {
                if let Some(reader) = reader {
                    out_siblings.push(self.gen_node_in_proof(
                        sibling_half_start,
                        width,
                        (existence_bitmap, leaf_bitmap),
                        (reader, node_key),
                    )?);
                } else {
                    out_siblings.push(
                        self.merkle_hash(
                            sibling_half_start,
                            width,
                            (existence_bitmap, leaf_bitmap),
                        )
                        .into(),
                    );
                }
            }
            let (range_existence_bitmap, range_leaf_bitmap) =
                Self::range_bitmaps(child_half_start, width, (existence_bitmap, leaf_bitmap));

            if range_existence_bitmap == 0 {
                // No child in this range.
                return Ok(None);
            } else if width == 1
                || (range_existence_bitmap.count_ones() == 1 && range_leaf_bitmap != 0)
            {
                // Return the only 1 leaf child under this subtree or reach the lowest level
                // Even this leaf child is not the n-th child, it should be returned instead of
                // `None` because it's existence indirectly proves the n-th child doesn't exist.
                // Please read proof format for details.
                let only_child_index = Nibble::from(range_existence_bitmap.trailing_zeros() as u8);
                let only_child_version = self
                    .child(only_child_index)
                    // Should be guaranteed by the self invariants, but these are not easy to express at the moment
                    .with_context(|| {
                        format!(
                            "Corrupted internal node: child_bitmap indicates \
                                 the existence of a non-exist child at index {:x}",
                            only_child_index
                        )
                    })
                    .unwrap()
                    .version;
                return Ok(Some(
                    node_key.gen_child_node_key(only_child_version, only_child_index),
                ));
            }
        }
        unreachable!("Impossible to get here without returning even at the lowest level.")
    }

    #[cfg(test)]
    pub(crate) fn get_child_with_siblings_for_test<K: crate::Key, R: TreeReader<K>>(
        &self,
        node_key: &NodeKey,
        n: Nibble,
        reader: Option<&R>,
    ) -> Result<(Option<NodeKey>, Vec<NodeInProof>)> {
        let mut sibilings = vec![];
        self.get_child_with_siblings(node_key, n, reader, &mut sibilings, 0, 0)
            .map(|n| (n, sibilings))
    }
}

/// Given a nibble, computes the start position of its `child_half_start` and `sibling_half_start`
/// at `height` level.
pub(crate) fn get_child_and_sibling_half_start(n: Nibble, height: u8) -> (u8, u8) {
    // Get the index of the first child belonging to the same subtree whose root, let's say `r` is
    // at `height` that the n-th child belongs to.
    // Note: `child_half_start` will be always equal to `n` at height 0.
    let child_half_start = (0xFF << height) & u8::from(n);

    // Get the index of the first child belonging to the subtree whose root is the sibling of `r`
    // at `height`.
    let sibling_half_start = child_half_start ^ (1 << height);

    (child_half_start, sibling_half_start)
}

/// Represents an account.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LeafNode<K> {
    // The hashed key associated with this leaf node.
    account_key: HashValue,
    // The hash of the value.
    value_hash: HashValue,
    // The key and version that points to the value
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
    pub fn account_key(&self) -> &HashValue {
        &self.account_key
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
    Null = 3,
}

/// The concrete node type of [`JellyfishMerkleTree`](crate::JellyfishMerkleTree).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node<K> {
    /// A wrapper of [`InternalNode`].
    Internal(InternalNode),
    /// A wrapper of [`LeafNode`].
    Leaf(LeafNode<K>),
    /// Represents empty tree only
    Null,
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
    /// Creates the [`Internal`](Node::Internal) variant.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_internal(children: Children) -> Self {
        Node::Internal(InternalNode::new(children))
    }

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
            Self::Null => NodeType::Null,
        }
    }

    /// Returns leaf count if known
    pub fn leaf_count(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Internal(internal_node) => internal_node.leaf_count,
            Node::Null => 0,
        }
    }

    /// Serializes to bytes for physical storage.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = vec![];

        match self {
            Node::Internal(internal_node) => {
                out.push(NodeTag::Internal as u8);
                internal_node.serialize(&mut out)?;
                VELOR_JELLYFISH_INTERNAL_ENCODED_BYTES.inc_by(out.len() as u64);
            },
            Node::Leaf(leaf_node) => {
                out.push(NodeTag::Leaf as u8);
                out.extend(bcs::to_bytes(&leaf_node)?);
                VELOR_JELLYFISH_LEAF_ENCODED_BYTES.inc_by(out.len() as u64);
            },
            Node::Null => {
                out.push(NodeTag::Null as u8);
            },
        }
        Ok(out)
    }

    /// Computes the hash of nodes.
    pub fn hash(&self) -> HashValue {
        match self {
            Node::Internal(internal_node) => internal_node.hash(),
            Node::Leaf(leaf_node) => leaf_node.hash(),
            Node::Null => *SPARSE_MERKLE_PLACEHOLDER_HASH,
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
            Some(NodeTag::Null) => Ok(Node::Null),
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
    ExtraLeaves { existing: u16, leaves: u16 },
}

/// Helper function to serialize version in a more efficient encoding.
/// We use a super simple encoding - the high bit is set if more bytes follow.
fn serialize_u64_varint(mut num: u64, binary: &mut Vec<u8>) {
    for _ in 0..8 {
        let low_bits = num as u8 & 0x7F;
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
    assert!(num <= 0xFF);
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
        num |= u64::from(byte & 0x7F) << (i * 7);
        if (byte & 0x80) == 0 {
            return Ok(num);
        }
    }
    // Last byte is encoded as is.
    let byte = reader.read_u8()?;
    num |= u64::from(byte) << 56;
    Ok(num)
}
