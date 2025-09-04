// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ProofRead,
    sparse_merkle::{
        HashValueRef, UpdateError,
        node::{InternalNode, Node, NodeHandle, NodeInner},
        utils::{partition, swap_if},
    },
};
use aptos_crypto::{HashValue, hash::SPARSE_MERKLE_PLACEHOLDER_HASH};
use aptos_types::proof::{SparseMerkleLeafNode, SparseMerkleProofExt, definition::NodeInProof};
use aptos_vm::AptosVM;
use once_cell::sync::Lazy;
use std::cmp::Ordering;

static POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(AptosVM::get_num_proof_reading_threads())
        .thread_name(|index| format!("smt_update_{}", index))
        .build()
        .unwrap()
});

type Result<T> = std::result::Result<T, UpdateError>;

type InMemSubTree = super::node::SubTree;
type InMemInternal = InternalNode;

#[derive(Clone)]
enum InMemSubTreeInfo {
    Internal {
        subtree: InMemSubTree,
        node: InMemInternal,
    },
    Leaf {
        subtree: InMemSubTree,
        key: HashValue,
    },
    Unknown {
        subtree: InMemSubTree,
    },
    Empty,
}

impl InMemSubTreeInfo {
    fn create_leaf_with_update(update: (HashValue, HashValue), generation: u64) -> Self {
        let subtree = InMemSubTree::new_leaf(update.0, update.1, generation);
        Self::Leaf {
            key: update.0,
            subtree,
        }
    }

    fn create_leaf_with_proof(leaf: &SparseMerkleLeafNode, generation: u64) -> Self {
        let subtree = InMemSubTree::new_leaf(*leaf.key(), *leaf.value_hash(), generation);
        Self::Leaf {
            key: *leaf.key(),
            subtree,
        }
    }

    fn create_internal(left: Self, right: Self, generation: u64) -> Self {
        let node = InternalNode {
            left: left.into_subtree(),
            right: right.into_subtree(),
        };
        let subtree = InMemSubTree::NonEmpty {
            hash: node.calc_hash(),
            root: NodeHandle::new_shared(Node::new_internal_from_node(node.clone(), generation)),
        };

        Self::Internal { subtree, node }
    }

    fn create_unknown(hash: HashValue) -> Self {
        Self::Unknown {
            subtree: InMemSubTree::new_unknown(hash),
        }
    }

    fn into_subtree(self) -> InMemSubTree {
        match self {
            Self::Leaf { subtree, .. } => subtree,
            Self::Internal { subtree, .. } => subtree,
            Self::Unknown { subtree } => subtree,
            Self::Empty => InMemSubTree::Empty,
        }
    }

    fn combine(left: Self, right: Self, generation: u64) -> Self {
        // If there's a only leaf in the subtree,
        // rollup the leaf, otherwise create an internal node.
        match (&left, &right) {
            (Self::Empty, Self::Empty) => Self::Empty,
            (Self::Leaf { .. }, Self::Empty) => left,
            (Self::Empty, Self::Leaf { .. }) => right,
            _ => InMemSubTreeInfo::create_internal(left, right, generation),
        }
    }
}

#[derive(Clone)]
enum PersistedSubTreeInfo {
    ProofPathInternal { proof: SparseMerkleProofExt },
    ProofSibling { hash: HashValue },
    Leaf { leaf: SparseMerkleLeafNode },
}

#[derive(Clone)]
enum SubTreeInfo {
    InMem(InMemSubTreeInfo),
    Persisted(PersistedSubTreeInfo),
}

impl SubTreeInfo {
    fn new_empty() -> Self {
        Self::InMem(InMemSubTreeInfo::Empty)
    }

    fn new_proof_leaf(leaf: SparseMerkleLeafNode) -> Self {
        Self::Persisted(PersistedSubTreeInfo::Leaf { leaf })
    }

    fn new_proof_sibling(node_in_proof: &NodeInProof) -> Self {
        match node_in_proof {
            NodeInProof::Leaf(leaf) => Self::new_proof_leaf(*leaf),
            NodeInProof::Other(hash) => {
                if *hash == *SPARSE_MERKLE_PLACEHOLDER_HASH {
                    Self::InMem(InMemSubTreeInfo::Empty)
                } else {
                    Self::Persisted(PersistedSubTreeInfo::ProofSibling { hash: *hash })
                }
            },
        }
    }

    fn new_on_proof_path(proof: SparseMerkleProofExt, depth: usize) -> Self {
        match proof.bottom_depth().cmp(&depth) {
            Ordering::Greater => Self::Persisted(PersistedSubTreeInfo::ProofPathInternal { proof }),
            Ordering::Equal => match proof.leaf() {
                Some(leaf) => Self::new_proof_leaf(leaf),
                None => Self::new_empty(),
            },
            _ => unreachable!(),
        }
    }

    fn from_persisted(
        a_descendant_key: &HashValue,
        depth: usize,
        proof_reader: &impl ProofRead,
    ) -> Result<Self> {
        let proof = proof_reader
            .get_proof(a_descendant_key, depth)
            .ok_or(UpdateError::MissingProof)?;
        if depth > proof.bottom_depth() {
            return Err(UpdateError::ShortProof {
                key: *a_descendant_key,
                num_siblings: proof.bottom_depth(),
                depth,
            });
        }
        Ok(Self::new_on_proof_path(proof, depth))
    }

    fn from_in_mem(subtree: &InMemSubTree, generation: u64) -> Self {
        match &subtree {
            InMemSubTree::Empty => SubTreeInfo::new_empty(),
            InMemSubTree::NonEmpty { root, .. } => match root.get_if_in_mem() {
                Some(arc_node) => match arc_node.inner() {
                    NodeInner::Internal(internal_node) => {
                        SubTreeInfo::InMem(InMemSubTreeInfo::Internal {
                            node: internal_node.clone(),
                            subtree: subtree.weak(),
                        })
                    },
                    NodeInner::Leaf(leaf_node) => {
                        // Create a new leaf node at the new generation.
                        // This is only necessary when this leaf node moves its position on the tree,
                        // either because the leaf is "split" and moved down or moved up due to
                        // deletion.
                        // In contrast, if the node didn't move, a subtree.weak()
                        // should suffice, since it becomes "unknown" if persisted and pruned,
                        // and a proof from the DB in that case will reveal its information
                        // (since the position didn't change.) For the sake of simplicity we always
                        // create a new leaf here.
                        let node =
                            Node::new_leaf(*leaf_node.key(), *leaf_node.value_hash(), generation);
                        let subtree = InMemSubTree::NonEmpty {
                            hash: subtree.hash(),
                            root: NodeHandle::new_shared(node),
                        };

                        SubTreeInfo::InMem(InMemSubTreeInfo::Leaf {
                            key: *leaf_node.key(),
                            subtree,
                        })
                    },
                },
                None => SubTreeInfo::InMem(InMemSubTreeInfo::Unknown {
                    subtree: subtree.weak(),
                }),
            },
        }
    }

    fn is_unknown(&self) -> bool {
        matches!(self, Self::InMem(InMemSubTreeInfo::Unknown { .. }))
            || matches!(
                self,
                Self::Persisted(PersistedSubTreeInfo::ProofSibling { .. })
            )
    }

    fn into_children(
        self,
        a_descendent_key: &HashValue,
        depth: usize,
        proof_reader: &impl ProofRead,
        generation: u64,
    ) -> Result<(Self, Self)> {
        let myself = if self.is_unknown() {
            SubTreeInfo::from_persisted(a_descendent_key, depth, proof_reader)?
        } else {
            self
        };

        Ok(match &myself {
            SubTreeInfo::InMem(info) => match info {
                InMemSubTreeInfo::Empty => (Self::new_empty(), Self::new_empty()),
                InMemSubTreeInfo::Leaf { key, .. } => {
                    let key = *key;
                    swap_if(myself, SubTreeInfo::new_empty(), key.bit(depth))
                },
                InMemSubTreeInfo::Internal { node, .. } => (
                    // n.b. When we recurse into either side, the updates can be empty, where the
                    // specific type of the in-mem node is irrelevant, so the parsing of it can be
                    // lazy. But the saving seem not worth the complexity.
                    SubTreeInfo::from_in_mem(&node.left, generation),
                    SubTreeInfo::from_in_mem(&node.right, generation),
                ),
                InMemSubTreeInfo::Unknown { .. } => unreachable!(),
            },
            SubTreeInfo::Persisted(info) => match info {
                PersistedSubTreeInfo::Leaf { leaf } => {
                    let is_right = leaf.key().bit(depth);
                    swap_if(myself, SubTreeInfo::new_empty(), is_right)
                },
                PersistedSubTreeInfo::ProofPathInternal { proof } => {
                    let sibling_child =
                        SubTreeInfo::new_proof_sibling(proof.sibling_at_depth(depth + 1).unwrap());
                    let on_path_child =
                        SubTreeInfo::new_on_proof_path(myself.expect_into_proof(), depth + 1);
                    swap_if(on_path_child, sibling_child, a_descendent_key.bit(depth))
                },
                PersistedSubTreeInfo::ProofSibling { .. } => unreachable!(),
            },
        })
    }

    fn materialize(self, generation: u64) -> InMemSubTreeInfo {
        match self {
            Self::InMem(info) => info,
            Self::Persisted(info) => match info {
                PersistedSubTreeInfo::Leaf { leaf } => {
                    InMemSubTreeInfo::create_leaf_with_proof(&leaf, generation)
                },
                PersistedSubTreeInfo::ProofSibling { hash } => {
                    InMemSubTreeInfo::create_unknown(hash)
                },
                PersistedSubTreeInfo::ProofPathInternal { .. } => {
                    unreachable!()
                },
            },
        }
    }

    fn expect_into_proof(self) -> SparseMerkleProofExt {
        match self {
            SubTreeInfo::Persisted(PersistedSubTreeInfo::ProofPathInternal { proof }) => proof,
            _ => unreachable!("Known variant."),
        }
    }
}

pub struct SubTreeUpdater<'a, K, V> {
    depth: usize,
    info: SubTreeInfo,
    updates: &'a [(K, Option<V>)],
    generation: u64,
}

impl<'a, K, V> SubTreeUpdater<'a, K, V>
where
    K: 'a + HashValueRef + Sync,
    V: 'a + HashValueRef + Sync,
{
    pub(crate) fn update(
        root: InMemSubTree,
        updates: &'a [(K, Option<V>)],
        proof_reader: &'a impl ProofRead,
        generation: u64,
    ) -> Result<InMemSubTree> {
        let updater = Self {
            depth: 0,
            info: SubTreeInfo::from_in_mem(&root, generation),
            updates,
            generation,
        };
        Ok(updater.run(proof_reader)?.into_subtree())
    }

    fn run(self, proof_reader: &impl ProofRead) -> Result<InMemSubTreeInfo> {
        // Limit total tasks that are potentially sent to other threads.
        const MAX_PARALLELIZABLE_DEPTH: usize = 8;
        // No point to introduce Rayon overhead if work is small.
        const MIN_PARALLELIZABLE_SIZE: usize = 2;

        let generation = self.generation;
        let depth = self.depth;
        match self.maybe_end_recursion()? {
            MaybeEndRecursion::End(ended) => Ok(ended),
            MaybeEndRecursion::Continue(myself) => {
                let (left, right) = myself.into_children(proof_reader)?;
                let (left_ret, right_ret) = if depth <= MAX_PARALLELIZABLE_DEPTH
                    && left.updates.len() >= MIN_PARALLELIZABLE_SIZE
                    && right.updates.len() >= MIN_PARALLELIZABLE_SIZE
                {
                    POOL.join(|| left.run(proof_reader), || right.run(proof_reader))
                } else {
                    (left.run(proof_reader), right.run(proof_reader))
                };

                Ok(InMemSubTreeInfo::combine(left_ret?, right_ret?, generation))
            },
        }
    }

    fn maybe_end_recursion(self) -> Result<MaybeEndRecursion<InMemSubTreeInfo, Self>> {
        Ok(match self.updates.len() {
            0 => MaybeEndRecursion::End(self.info.materialize(self.generation)),
            1 => {
                let (key_to_update, update) = &self.updates[0];
                match &self.info {
                    SubTreeInfo::InMem(in_mem_info) => match in_mem_info {
                        InMemSubTreeInfo::Empty => match update {
                            Some(value) => {
                                MaybeEndRecursion::End(InMemSubTreeInfo::create_leaf_with_update(
                                    (*key_to_update.hash_ref(), *value.hash_ref()),
                                    self.generation,
                                ))
                            },
                            None => MaybeEndRecursion::End(self.info.materialize(self.generation)),
                        },
                        InMemSubTreeInfo::Leaf { key, .. } => match update {
                            Some(value) => MaybeEndRecursion::or(
                                key == key_to_update.hash_ref(),
                                InMemSubTreeInfo::create_leaf_with_update(
                                    (*key_to_update.hash_ref(), *value.hash_ref()),
                                    self.generation,
                                ),
                                self,
                            ),
                            None => {
                                if key == key_to_update.hash_ref() {
                                    MaybeEndRecursion::End(InMemSubTreeInfo::Empty)
                                } else {
                                    MaybeEndRecursion::End(self.info.materialize(self.generation))
                                }
                            },
                        },
                        _ => MaybeEndRecursion::Continue(self),
                    },
                    SubTreeInfo::Persisted(PersistedSubTreeInfo::Leaf { leaf }) => match update {
                        Some(value) => MaybeEndRecursion::or(
                            leaf.key() == key_to_update.hash_ref(),
                            InMemSubTreeInfo::create_leaf_with_update(
                                (*key_to_update.hash_ref(), *value.hash_ref()),
                                self.generation,
                            ),
                            self,
                        ),
                        None => {
                            if leaf.key() == key_to_update.hash_ref() {
                                MaybeEndRecursion::End(InMemSubTreeInfo::Empty)
                            } else {
                                MaybeEndRecursion::End(self.info.materialize(self.generation))
                            }
                        },
                    },
                    _ => MaybeEndRecursion::Continue(self),
                }
            },
            _ => MaybeEndRecursion::Continue(self),
        })
    }

    fn into_children(self, proof_reader: &impl ProofRead) -> Result<(Self, Self)> {
        let pivot = partition(self.updates, self.depth);
        let (left_updates, right_updates) = self.updates.split_at(pivot);
        let generation = self.generation;
        let (left_info, right_info) = self.info.into_children(
            self.updates[0].0.hash_ref(),
            self.depth,
            proof_reader,
            generation,
        )?;

        Ok((
            Self {
                depth: self.depth + 1,
                info: left_info,
                updates: left_updates,
                generation,
            },
            Self {
                depth: self.depth + 1,
                info: right_info,
                updates: right_updates,
                generation,
            },
        ))
    }
}

pub(crate) enum MaybeEndRecursion<A, B> {
    End(A),
    Continue(B),
}

impl<A, B> MaybeEndRecursion<A, B> {
    pub fn or(cond: bool, a: A, b: B) -> Self {
        if cond {
            MaybeEndRecursion::End(a)
        } else {
            MaybeEndRecursion::Continue(b)
        }
    }
}
