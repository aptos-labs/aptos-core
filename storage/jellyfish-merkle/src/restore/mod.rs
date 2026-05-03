// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module implements the functionality to restore a `JellyfishMerkleTree` from small chunks
//! of states.

use crate::{
    node_type::{
        get_child_and_sibling_half_start, Child, Children, InternalNode, LeafNode, Node, NodeKey,
        NodeType,
    },
    NibbleExt, TreeReader, TreeWriter, ROOT_NIBBLE_HEIGHT,
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_logger::info;
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    nibble::{
        nibble_path::{NibbleIterator, NibblePath},
        ExpectNibble, Nibble,
    },
    proof::{SparseMerkleInternalNode, SparseMerkleLeafNode, SparseMerkleRangeProof},
    transaction::Version,
};
use once_cell::sync::Lazy;
use std::{
    cmp::Eq,
    collections::HashMap,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
};

static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(32)
        .thread_name(|index| format!("jmt_batch_{}", index))
        .build()
        .unwrap()
});

#[derive(Clone, Debug, Eq, PartialEq)]
enum ChildInfo<K> {
    /// This child is an internal node. The hash of the internal node is stored here if it is
    /// known, otherwise it is `None`. In the process of restoring a tree, we will only know the
    /// hash of an internal node after we see all the keys that share the same prefix.
    Internal {
        hash: Option<HashValue>,
        leaf_count: Option<usize>,
    },

    /// This child is a leaf node.
    Leaf(LeafNode<K>),
}

impl<K> ChildInfo<K>
where
    K: crate::Key + CryptoHash,
{
    /// Converts `self` to a child, assuming the hash is known if it's an internal node.
    fn into_child(self, version: Version) -> Child {
        match self {
            Self::Internal { hash, leaf_count } => Child::new(
                hash.expect("Must have been initialized."),
                version,
                NodeType::Internal {
                    leaf_count: leaf_count.expect("Must be complete already."),
                },
            ),
            Self::Leaf(node) => Child::new(node.hash(), version, NodeType::Leaf),
        }
    }
}

#[derive(Clone, Debug)]
struct InternalInfo<K> {
    /// The node key of this internal node.
    node_key: NodeKey,

    /// The existing children. Every time a child appears, the corresponding position will be set
    /// to `Some`.
    children: [Option<ChildInfo<K>>; 16],
}

impl<K> InternalInfo<K>
where
    K: crate::Key + CryptoHash,
{
    /// Creates an empty internal node with no children.
    fn new_empty(node_key: NodeKey) -> Self {
        Self {
            node_key,
            children: Default::default(),
        }
    }

    fn set_child(&mut self, index: usize, child_info: ChildInfo<K>) {
        self.children[index] = Some(child_info);
    }

    /// Converts `self` to an internal node, assuming all of its children are already known and
    /// fully initialized.
    fn into_internal_node(mut self, version: Version) -> (NodeKey, InternalNode) {
        let mut children = Vec::with_capacity(self.children.len());

        // Calling `into_iter` on an array is equivalent to calling `iter`:
        // https://github.com/rust-lang/rust/issues/25725. So we use `iter_mut` and `take`.
        for (index, child_info_option) in self.children.iter_mut().enumerate() {
            if let Some(child_info) = child_info_option.take() {
                children.push((index.expect_nibble(), child_info.into_child(version)));
            }
        }

        (
            self.node_key,
            InternalNode::new(Children::from_sorted(children)),
        )
    }
}

pub struct JellyfishMerkleRestore<K> {
    /// The underlying storage.
    store: Arc<dyn TreeWriter<K>>,
    reader: Option<Arc<dyn TreeReader<K> + Send + Sync>>,

    /// The version of the tree we are restoring.
    version: Version,

    /// The nodes we have partially restored. Each `partial_nodes[i-1]` is the parent of
    /// `partial_nodes[i]`. If a node `partial_nodes[i-1]` has multiple children, only the
    /// rightmost known child will appear here as `partial_nodes[i]`, because any other children on
    /// the left would have been frozen.
    ///
    /// At any point in time, the structure looks like the following:
    ///
    /// ```text
    /// +----+----+----+----+----+----+----+----+
    /// |    |    |    |    |    |    |    | C  |  partial_nodes[0]
    /// +----+----+----+----+----+----+----+----+
    ///   |         |              |
    ///   |         |              |
    ///   |         |              |
    ///   v         v              v
    /// Frozen    Frozen     +----+----+----+----+----+----+----+----+
    ///                      |    |    |    | B  |    |    | A  |    |  partial_nodes[1]
    ///                      +----+----+----+----+----+----+----+----+
    ///                             |         |
    ///                             |         |
    ///                             |         |
    ///                             v         v
    ///                            Frozen    Previously inserted state
    /// ```
    ///
    /// We insert the states from left to right. So if the next state appears at position `A`,
    /// it will cause the leaf at position `B` to be frozen. If it appears at position `B`, it
    /// might cause a few internal nodes to be created additionally. If it appears at position `C`,
    /// it will also cause `partial_nodes[1]` to be added to `frozen_nodes` as an internal node and
    /// be removed from `partial_nodes`.
    partial_nodes: Vec<InternalInfo<K>>,

    /// The nodes that have been fully restored and are ready to be written to storage.
    frozen_nodes: HashMap<NodeKey, Node<K>>,

    /// The most recently added leaf. This is used to ensure the keys come in increasing
    /// order and do proof verification.
    previous_leaf: Option<LeafNode<K>>,

    /// The number of keys we have received since the most recent restart.
    num_keys_received: u64,

    /// When the restoration process finishes, we expect the tree to have this root hash.
    expected_root_hash: HashValue,

    /// Already finished, deem all chunks overlap.
    finished: bool,

    async_commit: bool,
    async_commit_result: Option<Receiver<Result<()>>>,
}

impl<K> JellyfishMerkleRestore<K>
where
    K: crate::Key + CryptoHash + 'static,
{
    pub fn new<D: 'static + TreeReader<K> + TreeWriter<K>>(
        store: Arc<D>,
        version: Version,
        expected_root_hash: HashValue,
        async_commit: bool,
    ) -> Result<Self> {
        let tree_reader: Arc<dyn TreeReader<K> + Send + Sync> = store.clone();
        let tree_writer: Arc<dyn TreeWriter<K>> = store;
        let (finished, partial_nodes, previous_leaf) = if let Some(root_node) =
            tree_reader.get_node_option(&NodeKey::new_empty_path(version), "restore")?
        {
            info!("Previous restore is complete, checking root hash.");
            ensure!(
                root_node.hash() == expected_root_hash,
                "Previous completed restore has root hash {}, expecting {}",
                root_node.hash(),
                expected_root_hash,
            );
            (true, vec![], None)
        } else if let Some((node_key, leaf_node)) = tree_reader.get_rightmost_leaf(version)? {
            // If the system crashed in the middle of the previous restoration attempt, we need
            // to recover the partial nodes to the state right before the crash.
            (
                false,
                Self::recover_partial_nodes(tree_reader.as_ref(), version, node_key)?,
                Some(leaf_node),
            )
        } else {
            (
                false,
                vec![InternalInfo::new_empty(NodeKey::new_empty_path(version))],
                None,
            )
        };

        Ok(Self {
            store: tree_writer,
            reader: Some(tree_reader),
            version,
            partial_nodes,
            frozen_nodes: HashMap::new(),
            previous_leaf,
            num_keys_received: 0,
            expected_root_hash,
            finished,
            async_commit,
            async_commit_result: None,
        })
    }

    pub fn new_overwrite<D: 'static + TreeWriter<K>>(
        store: Arc<D>,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Self> {
        Ok(Self {
            store,
            reader: None,
            version,
            partial_nodes: vec![InternalInfo::new_empty(NodeKey::new_empty_path(version))],
            frozen_nodes: HashMap::new(),
            previous_leaf: None,
            num_keys_received: 0,
            expected_root_hash,
            finished: false,
            async_commit: false,
            async_commit_result: None,
        })
    }

    pub fn previous_key_hash(&self) -> Option<HashValue> {
        if self.finished {
            // Hack: prevent any chunk to be added.
            Some(HashValue::new([0xFF; HashValue::LENGTH]))
        } else {
            self.previous_leaf.as_ref().map(|leaf| *leaf.account_key())
        }
    }

    /// Recovers partial nodes from storage. We do this by looking at all the ancestors of the
    /// rightmost leaf. The ones do not exist in storage are the partial nodes.
    fn recover_partial_nodes(
        store: &dyn TreeReader<K>,
        version: Version,
        rightmost_leaf_node_key: NodeKey,
    ) -> Result<Vec<InternalInfo<K>>> {
        ensure!(
            !rightmost_leaf_node_key.nibble_path().is_empty(),
            "Root node would not be written until entire restoration process has completed \
             successfully.",
        );

        // Start from the parent of the rightmost leaf. If this internal node exists in storage, it
        // is not a partial node. Go to the parent node and repeat until we see a node that does
        // not exist. This node and all its ancestors will be the partial nodes.
        let mut node_key = rightmost_leaf_node_key.gen_parent_node_key();
        while store.get_node_option(&node_key, "restore")?.is_some() {
            node_key = node_key.gen_parent_node_key();
        }

        // Next we reconstruct all the partial nodes up to the root node, starting from the bottom.
        // For all of them, we scan all its possible child positions and see if there is one at
        // each position. If the node is not the bottom one, there is additionally a partial node
        // child at the position `previous_child_index`.
        let mut partial_nodes = vec![];
        // Initialize `previous_child_index` to `None` for the first iteration of the loop so the
        // code below treats it differently.
        let mut previous_child_index = None;

        loop {
            let mut internal_info = InternalInfo::new_empty(node_key.clone());

            for i in 0..previous_child_index.unwrap_or(16) {
                let child_node_key = node_key.gen_child_node_key(version, (i as u8).into());
                if let Some(node) = store.get_node_option(&child_node_key, "restore")? {
                    let child_info = match node {
                        Node::Internal(internal_node) => ChildInfo::Internal {
                            hash: Some(internal_node.hash()),
                            leaf_count: Some(internal_node.leaf_count()),
                        },
                        Node::Leaf(leaf_node) => ChildInfo::Leaf(leaf_node),
                        Node::Null => unreachable!("Child cannot be Null"),
                    };
                    internal_info.set_child(i, child_info);
                }
            }

            // If this is not the lowest partial node, it will have a partial node child at
            // `previous_child_index`. Set the hash of this child to `None` because it is a
            // partial node and we do not know its hash yet. For the lowest partial node, we just
            // find all its known children from storage in the loop above.
            if let Some(index) = previous_child_index {
                internal_info.set_child(index, ChildInfo::Internal {
                    hash: None,
                    leaf_count: None,
                });
            }

            partial_nodes.push(internal_info);
            if node_key.nibble_path().is_empty() {
                break;
            }
            previous_child_index = node_key.nibble_path().last().map(|x| u8::from(x) as usize);
            node_key = node_key.gen_parent_node_key();
        }

        partial_nodes.reverse();
        Ok(partial_nodes)
    }

    /// Builds the partial tree in memory from `chunk` and verifies the entire chunk against
    /// the expected root and the JMT-stored leaves. Does not touch storage.
    ///
    /// `kv_progress` is the caller's last-written key hash for the dependent KV side (or
    /// `None` if nothing is written yet). When the JMT restore is ahead of the KV — typically
    /// after a partial-crash recovery where the previous chunk's tree commit landed but the
    /// KV write didn't — `kv_progress < previous_leaf.account_key()`, and the chunk arrives
    /// with a prefix of leaves that the JMT already stores. This function then walks the JMT
    /// in `(kv_progress, previous_leaf]` and pairs each stored leaf with the chunk's prefix,
    /// catching wrong values *and* missing keys (left-edge or internal gaps) before any KV
    /// write is allowed to land. To opt out (e.g., `TreeOnly` mode where there is no
    /// dependent KV), pass `kv_progress` equal to `previous_leaf.account_key()` — the walk
    /// then has an empty range and is skipped.
    ///
    /// Returns `Ok(true)` when there are frozen nodes pending — the caller must follow up
    /// with [`Self::commit_prepared`]. Returns `Ok(false)` when the chunk added nothing new
    /// (already finished, empty, or fully covered by `previous_leaf`); no commit is needed.
    pub fn prepare_chunk(
        &mut self,
        mut chunk: Vec<(&K, HashValue)>,
        proof: SparseMerkleRangeProof,
        kv_progress: Option<HashValue>,
    ) -> Result<bool> {
        if self.finished {
            info!("State snapshot restore already finished, ignoring entire chunk.");
            return Ok(false);
        }
        if chunk.is_empty() {
            return Ok(false);
        }

        let jmt_progress = self.previous_leaf.as_ref().map(|l| *l.account_key());
        let prefix_len = match jmt_progress {
            Some(jmt) => chunk.iter().take_while(|(k, _)| k.hash() <= jmt).count(),
            None => 0,
        };

        // When the dependent KV is behind the JMT, the chunk's prefix must exactly match the
        // JMT's leaves in `(kv_progress, walk_upper]`, where `walk_upper` is the highest key
        // the chunk claims to cover that the JMT also has stored — i.e.,
        // `min(chunk.last_key, jmt_progress)`. The merge-walk catches both wrong values
        // (Bug 1) and missing keys at the left edge or between prefix entries (Bug 2).
        let need_prefix_check = match (kv_progress, jmt_progress) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(kv), Some(jmt)) => kv < jmt,
        };
        if need_prefix_check {
            let jmt = jmt_progress.expect("jmt_progress is Some when need_prefix_check is true");
            let chunk_last = chunk
                .last()
                .expect("chunk is non-empty (early return above)")
                .0
                .hash();
            let walk_upper = std::cmp::min(chunk_last, jmt);
            self.wait_for_async_commit()?;
            self.verify_prefix_against_storage(&chunk[..prefix_len], kv_progress, walk_upper)?;
        }

        let suffix = chunk.split_off(prefix_len);
        if suffix.is_empty() {
            return Ok(false);
        }

        for (key, value_hash) in suffix {
            let hashed_key = key.hash();
            if let Some(ref prev_leaf) = self.previous_leaf {
                ensure!(
                    &hashed_key > prev_leaf.account_key(),
                    "State keys must come in increasing order.",
                );
            }
            self.previous_leaf.replace(LeafNode::new(
                hashed_key,
                value_hash,
                (key.clone(), self.version),
            ));
            self.add_one(key, value_hash);
            self.num_keys_received += 1;
        }

        // After this returns Ok, the chunk is proven against the expected root hash and it
        // is safe for the caller to commit dependent state (e.g. KV writes) in parallel with
        // `commit_prepared`.
        self.verify(proof)?;

        Ok(true)
    }

    /// Walks the JMT leaves in `(exclusive_lower, inclusive_upper]` in increasing key order
    /// and pairs them 1-to-1 with `prefix`. Errors on any divergence: a JMT leaf with no
    /// matching chunk entry (chunk skipped a key — gap), a chunk entry that doesn't match
    /// the next JMT leaf (out of order, extra entry, or missing key in JMT), a value-hash
    /// mismatch, or a leftover chunk entry past the upper bound.
    fn verify_prefix_against_storage(
        &self,
        prefix: &[(&K, HashValue)],
        exclusive_lower: Option<HashValue>,
        inclusive_upper: HashValue,
    ) -> Result<()> {
        let reader = self.reader.as_ref().ok_or_else(|| {
            AptosDbError::Other("Cannot verify chunk prefix without a tree reader.".into())
        })?;

        let mut prefix_iter = prefix.iter();
        self.walk_leaves(
            reader.as_ref(),
            exclusive_lower,
            inclusive_upper,
            |leaf: LeafNode<K>| match prefix_iter.next() {
                None => Err(AptosDbError::Other(format!(
                    "Chunk is missing key {} that exists in restored JMT.",
                    leaf.account_key(),
                ))),
                Some((key, value_hash)) => {
                    ensure!(
                        key.hash() == *leaf.account_key(),
                        "Chunk key {} does not match next restored JMT leaf {}.",
                        key.hash(),
                        leaf.account_key(),
                    );
                    ensure!(
                        *value_hash == leaf.value_hash(),
                        "Restored JMT value hash mismatch for key {}: chunk has {}, JMT has {}.",
                        leaf.account_key(),
                        value_hash,
                        leaf.value_hash(),
                    );
                    Ok(())
                },
            },
        )?;

        if let Some((extra_key, _)) = prefix_iter.next() {
            return Err(AptosDbError::Other(format!(
                "Chunk has prefix entry {} not found in restored JMT.",
                extra_key.hash(),
            )));
        }

        Ok(())
    }

    /// Walks the leaves of the in-progress tree whose keys fall in
    /// `(exclusive_lower, inclusive_upper]`, in increasing key order, invoking `visit` for
    /// each. The traversal uses `self.partial_nodes` for the rightmost in-memory path
    /// (whose root may not be in storage yet) and descends into storage only for frozen
    /// subtrees on the left. Subtrees fully outside the query range are pruned by nibble
    /// path so the cost is `O(yielded_leaves + log n)` storage reads.
    fn walk_leaves<F>(
        &self,
        reader: &(dyn TreeReader<K> + Send + Sync),
        exclusive_lower: Option<HashValue>,
        inclusive_upper: HashValue,
        mut visit: F,
    ) -> Result<()>
    where
        F: FnMut(LeafNode<K>) -> Result<()>,
    {
        self.walk_partial(reader, 0, exclusive_lower, inclusive_upper, &mut visit)
    }

    fn walk_partial<F>(
        &self,
        reader: &(dyn TreeReader<K> + Send + Sync),
        depth: usize,
        exclusive_lower: Option<HashValue>,
        inclusive_upper: HashValue,
        visit: &mut F,
    ) -> Result<()>
    where
        F: FnMut(LeafNode<K>) -> Result<()>,
    {
        let info = &self.partial_nodes[depth];
        for (child_index, child_opt) in info.children.iter().enumerate() {
            let child = match child_opt {
                Some(c) => c,
                None => continue,
            };
            let child_nibble: Nibble = (child_index as u8).into();
            let child_node_key = info.node_key.gen_child_node_key(self.version, child_nibble);
            let (sub_min, sub_max) = Self::subtree_key_range(child_node_key.nibble_path());
            let above_lower = exclusive_lower.map_or(true, |l| sub_max > l);
            let below_upper = sub_min <= inclusive_upper;
            if !(above_lower && below_upper) {
                continue;
            }
            match child {
                ChildInfo::Leaf(leaf) => {
                    let key = *leaf.account_key();
                    let above_lower_leaf = exclusive_lower.map_or(true, |l| key > l);
                    if above_lower_leaf && key <= inclusive_upper {
                        visit(leaf.clone())?;
                    }
                },
                ChildInfo::Internal { hash: Some(_), .. } => {
                    // Frozen — fully in storage; descend from this child's node key.
                    Self::walk_subtree(
                        reader,
                        self.version,
                        child_node_key,
                        exclusive_lower,
                        inclusive_upper,
                        visit,
                    )?;
                },
                ChildInfo::Internal { hash: None, .. } => {
                    // Partial — points to the next level of `partial_nodes`.
                    self.walk_partial(reader, depth + 1, exclusive_lower, inclusive_upper, visit)?;
                },
            }
        }
        Ok(())
    }

    fn walk_subtree<F>(
        reader: &(dyn TreeReader<K> + Send + Sync),
        version: Version,
        node_key: NodeKey,
        exclusive_lower: Option<HashValue>,
        inclusive_upper: HashValue,
        visit: &mut F,
    ) -> Result<()>
    where
        F: FnMut(LeafNode<K>) -> Result<()>,
    {
        let node = match reader.get_node_option(&node_key, "walk_leaves")? {
            Some(n) => n,
            None => return Ok(()),
        };
        match node {
            Node::Null => Ok(()),
            Node::Leaf(leaf) => {
                let key = *leaf.account_key();
                let above_lower = exclusive_lower.map_or(true, |l| key > l);
                if above_lower && key <= inclusive_upper {
                    visit(leaf)?;
                }
                Ok(())
            },
            Node::Internal(internal) => {
                for (nibble, _) in internal.children_sorted() {
                    let child_key = node_key.gen_child_node_key(version, *nibble);
                    let (sub_min, sub_max) = Self::subtree_key_range(child_key.nibble_path());
                    let above_lower = exclusive_lower.map_or(true, |l| sub_max > l);
                    let below_upper = sub_min <= inclusive_upper;
                    if above_lower && below_upper {
                        Self::walk_subtree(
                            reader,
                            version,
                            child_key,
                            exclusive_lower,
                            inclusive_upper,
                            visit,
                        )?;
                    }
                }
                Ok(())
            },
        }
    }

    /// Returns the smallest and largest possible leaf keys in the subtree rooted at
    /// `nibble_path` — i.e., the path padded with all-zero / all-`F` suffix bytes.
    fn subtree_key_range(nibble_path: &NibblePath) -> (HashValue, HashValue) {
        let mut min_bytes = [0u8; HashValue::LENGTH];
        let mut max_bytes = [0xFFu8; HashValue::LENGTH];
        for (i, nibble) in nibble_path.nibbles().enumerate() {
            let byte_idx = i / 2;
            let n = u8::from(nibble);
            if i % 2 == 0 {
                min_bytes[byte_idx] = n << 4;
                max_bytes[byte_idx] = (n << 4) | 0x0F;
            } else {
                min_bytes[byte_idx] = (min_bytes[byte_idx] & 0xF0) | n;
                max_bytes[byte_idx] = (max_bytes[byte_idx] & 0xF0) | n;
            }
        }
        (HashValue::new(min_bytes), HashValue::new(max_bytes))
    }

    /// Writes the frozen nodes accumulated by [`Self::prepare_chunk`] to storage. With
    /// `async_commit`, the write is dispatched to `IO_POOL` and drained on the next call.
    pub fn commit_prepared(&mut self) -> Result<()> {
        if self.async_commit {
            self.wait_for_async_commit()?;
            let (tx, rx) = channel();
            self.async_commit_result = Some(rx);

            let mut frozen_nodes = HashMap::new();
            std::mem::swap(&mut frozen_nodes, &mut self.frozen_nodes);
            let store = self.store.clone();

            IO_POOL.spawn(move || {
                let res = store.write_node_batch(&frozen_nodes);
                tx.send(res).unwrap();
            });
        } else {
            self.store.write_node_batch(&self.frozen_nodes)?;
            self.frozen_nodes.clear();
        }

        Ok(())
    }

    /// Restores one state.
    fn add_one(&mut self, new_key: &K, new_value_hash: HashValue) {
        let new_hashed_key = new_key.hash();
        let nibble_path = NibblePath::new_even(new_hashed_key.to_vec());
        let mut nibbles = nibble_path.nibbles();

        for i in 0..ROOT_NIBBLE_HEIGHT {
            let child_index = u8::from(nibbles.next().expect("This nibble must exist.")) as usize;

            assert!(i < self.partial_nodes.len());
            match self.partial_nodes[i].children[child_index] {
                Some(ref child_info) => {
                    // If there exists an internal node at this position, we just continue the loop
                    // with the next nibble. Here we deal with the leaf case.
                    if let ChildInfo::Leaf(node) = child_info {
                        assert_eq!(
                            i,
                            self.partial_nodes.len() - 1,
                            "If we see a leaf, there will be no more partial internal nodes on \
                             lower level, since they would have been frozen.",
                        );

                        let existing_leaf = node.clone();
                        self.insert_at_leaf(
                            child_index,
                            existing_leaf,
                            new_key,
                            new_value_hash,
                            nibbles,
                        );
                        break;
                    }
                },
                None => {
                    // This means that we are going to put a leaf in this position. For all the
                    // descendants on the left, they are now frozen.
                    self.freeze(i + 1);

                    // Mark this position as a leaf child.
                    self.partial_nodes[i].set_child(
                        child_index,
                        ChildInfo::Leaf(LeafNode::new(
                            new_hashed_key,
                            new_value_hash,
                            (new_key.clone(), self.version),
                        )),
                    );

                    // We do not add this leaf node to self.frozen_nodes because we don't know its
                    // node key yet. We will know its node key when the next state comes.
                    break;
                },
            }
        }
    }

    /// Inserts a new state at the position of the existing leaf node. We may need to create
    /// multiple internal nodes depending on the length of the common prefix of the existing key
    /// and the new key.
    fn insert_at_leaf(
        &mut self,
        child_index: usize,
        existing_leaf: LeafNode<K>,
        new_key: &K,
        new_value_hash: HashValue,
        mut remaining_nibbles: NibbleIterator,
    ) {
        let num_existing_partial_nodes = self.partial_nodes.len();

        // The node at this position becomes an internal node. Since we may insert more nodes at
        // this position in the future, we do not know its hash yet.
        self.partial_nodes[num_existing_partial_nodes - 1].set_child(
            child_index,
            ChildInfo::Internal {
                hash: None,
                leaf_count: None,
            },
        );

        // Next we build the new internal nodes from top to bottom. All these internal node except
        // the bottom one will now have a single internal node child.
        let new_hashed_key = CryptoHash::hash(new_key);
        let common_prefix_len = existing_leaf
            .account_key()
            .common_prefix_nibbles_len(new_hashed_key);
        for _ in num_existing_partial_nodes..common_prefix_len {
            let visited_nibbles = remaining_nibbles.visited_nibbles().collect();
            let next_nibble = remaining_nibbles.next().expect("This nibble must exist.");
            let new_node_key = NodeKey::new(self.version, visited_nibbles);

            let mut internal_info = InternalInfo::new_empty(new_node_key);
            internal_info.set_child(u8::from(next_nibble) as usize, ChildInfo::Internal {
                hash: None,
                leaf_count: None,
            });
            self.partial_nodes.push(internal_info);
        }

        // The last internal node will have two leaf node children.
        let visited_nibbles = remaining_nibbles.visited_nibbles().collect();
        let new_node_key = NodeKey::new(self.version, visited_nibbles);
        let mut internal_info = InternalInfo::new_empty(new_node_key);

        // Next we put the existing leaf as a child of this internal node.
        let existing_child_index = existing_leaf.account_key().get_nibble(common_prefix_len);
        internal_info.set_child(
            u8::from(existing_child_index) as usize,
            ChildInfo::Leaf(existing_leaf),
        );

        // Do not set the new child for now. We always call `freeze` first, then set the new child
        // later, because this way it's easier in `freeze` to find the correct leaf to freeze --
        // it's always the rightmost leaf on the lowest level.
        self.partial_nodes.push(internal_info);
        self.freeze(self.partial_nodes.len());

        // Now we set the new child.
        let new_child_index = new_hashed_key.get_nibble(common_prefix_len);
        assert!(
            new_child_index > existing_child_index,
            "New leaf must be on the right.",
        );
        self.partial_nodes
            .last_mut()
            .expect("This node must exist.")
            .set_child(
                u8::from(new_child_index) as usize,
                ChildInfo::Leaf(LeafNode::new(
                    new_hashed_key,
                    new_value_hash,
                    (new_key.clone(), self.version),
                )),
            );
    }

    /// Puts the nodes that will not be changed later in `self.frozen_nodes`.
    fn freeze(&mut self, num_remaining_partial_nodes: usize) {
        self.freeze_previous_leaf();
        self.freeze_internal_nodes(num_remaining_partial_nodes);
    }

    /// Freezes the previously added leaf node. It should always be the rightmost leaf node on the
    /// lowest level, inserted in the previous `add_one` call.
    fn freeze_previous_leaf(&mut self) {
        // If this is the very first key, there is no previous leaf to freeze.
        if self.num_keys_received == 0 {
            return;
        }

        let last_node = self
            .partial_nodes
            .last()
            .expect("Must have at least one partial node.");
        let rightmost_child_index = last_node
            .children
            .iter()
            .rposition(|x| x.is_some())
            .expect("Must have at least one child.");

        match last_node.children[rightmost_child_index] {
            Some(ChildInfo::Leaf(ref node)) => {
                let child_node_key = last_node
                    .node_key
                    .gen_child_node_key(self.version, (rightmost_child_index as u8).into());
                self.frozen_nodes
                    .insert(child_node_key, node.clone().into());
            },
            _ => panic!("Must have at least one child and must not have further internal nodes."),
        }
    }

    /// Freeze extra internal nodes. Only `num_remaining_nodes` partial internal nodes will be kept
    /// and the ones on the lower level will be frozen.
    fn freeze_internal_nodes(&mut self, num_remaining_nodes: usize) {
        while self.partial_nodes.len() > num_remaining_nodes {
            let last_node = self.partial_nodes.pop().expect("This node must exist.");
            let (node_key, internal_node) = last_node.into_internal_node(self.version);
            // Keep the hash of this node before moving it into `frozen_nodes`, so we can update
            // its parent later.
            let node_hash = internal_node.hash();
            let node_leaf_count = internal_node.leaf_count();
            self.frozen_nodes.insert(node_key, internal_node.into());

            // Now that we have computed the hash of the internal node above, we will also update
            // its parent unless it is root node.
            if let Some(parent_node) = self.partial_nodes.last_mut() {
                // This internal node must be the rightmost child of its parent at the moment.
                let rightmost_child_index = parent_node
                    .children
                    .iter()
                    .rposition(|x| x.is_some())
                    .expect("Must have at least one child.");

                match parent_node.children[rightmost_child_index] {
                    Some(ChildInfo::Internal {
                        ref mut hash,
                        ref mut leaf_count,
                    }) => {
                        assert_eq!(hash.replace(node_hash), None);
                        assert_eq!(leaf_count.replace(node_leaf_count), None);
                    },
                    _ => panic!(
                        "Must have at least one child and the rightmost child must not be a leaf."
                    ),
                }
            }
        }
    }

    /// Verifies that all states that have been added so far (from the leftmost one to
    /// `self.previous_leaf`) are correct, i.e., we are able to construct `self.expected_root_hash`
    /// by combining all existing states and `proof`.
    #[allow(clippy::collapsible_if)]
    fn verify(&self, proof: SparseMerkleRangeProof) -> Result<()> {
        let previous_leaf = self
            .previous_leaf
            .as_ref()
            .expect("The previous leaf must exist.");

        let previous_key = previous_leaf.account_key();
        // If we have all siblings on the path from root to `previous_key`, we should be able to
        // compute the root hash. The siblings on the right are already in the proof. Now we
        // compute the siblings on the left side, which represent all the states that have ever
        // been added.
        let mut left_siblings = vec![];

        // The following process might add some extra placeholder siblings on the left, but it is
        // nontrivial to determine when the loop should stop. So instead we just add these
        // siblings for now and get rid of them in the next step.
        let mut num_visited_right_siblings = 0;
        for (i, bit) in previous_key.iter_bits().enumerate() {
            if bit {
                // This node is a right child and there should be a sibling on the left.
                let sibling = if i >= self.partial_nodes.len() * 4 {
                    *SPARSE_MERKLE_PLACEHOLDER_HASH
                } else {
                    Self::compute_left_sibling(
                        &self.partial_nodes[i / 4],
                        previous_key.get_nibble(i / 4),
                        (3 - i % 4) as u8,
                    )
                };
                left_siblings.push(sibling);
            } else {
                // This node is a left child and there should be a sibling on the right.
                num_visited_right_siblings += 1;
            }
        }
        ensure!(
            num_visited_right_siblings >= proof.right_siblings().len(),
            "Too many right siblings in the proof.",
        );

        // Now we remove any extra placeholder siblings at the bottom. We keep removing the last
        // sibling if 1) it's a placeholder 2) it's a sibling on the left.
        for bit in previous_key.iter_bits().rev() {
            if bit {
                if *left_siblings.last().expect("This sibling must exist.")
                    == *SPARSE_MERKLE_PLACEHOLDER_HASH
                {
                    left_siblings.pop();
                } else {
                    break;
                }
            } else if num_visited_right_siblings > proof.right_siblings().len() {
                num_visited_right_siblings -= 1;
            } else {
                break;
            }
        }

        // Left siblings must use the same ordering as the right siblings in the proof
        left_siblings.reverse();

        // Verify the proof now that we have all the siblings
        proof
            .verify(
                self.expected_root_hash,
                SparseMerkleLeafNode::new(*previous_key, previous_leaf.value_hash()),
                left_siblings,
            )
            .map_err(Into::into)
    }

    /// Computes the sibling on the left for the `n`-th child.
    fn compute_left_sibling(partial_node: &InternalInfo<K>, n: Nibble, height: u8) -> HashValue {
        assert!(height < 4);
        let width = 1usize << height;
        let start = get_child_and_sibling_half_start(n, height).1 as usize;
        Self::compute_left_sibling_impl(&partial_node.children[start..start + width]).0
    }

    /// Returns the hash for given portion of the subtree and whether this part is a leaf node.
    fn compute_left_sibling_impl(children: &[Option<ChildInfo<K>>]) -> (HashValue, bool) {
        assert!(!children.is_empty());

        let num_children = children.len();
        assert!(num_children.is_power_of_two());

        if num_children == 1 {
            match &children[0] {
                Some(ChildInfo::Internal { hash, .. }) => {
                    (*hash.as_ref().expect("The hash must be known."), false)
                },
                Some(ChildInfo::Leaf(node)) => (node.hash(), true),
                None => (*SPARSE_MERKLE_PLACEHOLDER_HASH, true),
            }
        } else {
            let (left_hash, left_is_leaf) =
                Self::compute_left_sibling_impl(&children[..num_children / 2]);
            let (right_hash, right_is_leaf) =
                Self::compute_left_sibling_impl(&children[num_children / 2..]);

            if left_hash == *SPARSE_MERKLE_PLACEHOLDER_HASH && right_is_leaf {
                (right_hash, true)
            } else if left_is_leaf && right_hash == *SPARSE_MERKLE_PLACEHOLDER_HASH {
                (left_hash, true)
            } else {
                (
                    SparseMerkleInternalNode::new(left_hash, right_hash).hash(),
                    false,
                )
            }
        }
    }

    pub fn wait_for_async_commit(&mut self) -> Result<()> {
        if let Some(rx) = self.async_commit_result.take() {
            rx.recv()??;
        }
        Ok(())
    }

    /// Finishes the restoration process. This tells the code that there is no more state,
    /// otherwise we can not freeze the rightmost leaf and its ancestors.
    pub fn finish_impl(mut self) -> Result<()> {
        self.wait_for_async_commit()?;
        // Deal with the special case when the entire tree has a single leaf or null node.
        if self.partial_nodes.len() == 1 {
            let mut num_children = 0;
            let mut leaf = None;
            for i in 0..16 {
                if let Some(ref child_info) = self.partial_nodes[0].children[i] {
                    num_children += 1;
                    if let ChildInfo::Leaf(node) = child_info {
                        leaf = Some(node.clone());
                    }
                }
            }

            match num_children {
                0 => {
                    let node_key = NodeKey::new_empty_path(self.version);
                    assert!(self.frozen_nodes.is_empty());
                    self.frozen_nodes.insert(node_key, Node::Null);
                    self.store.write_node_batch(&self.frozen_nodes)?;
                    return Ok(());
                },
                1 => {
                    if let Some(node) = leaf {
                        let node_key = NodeKey::new_empty_path(self.version);
                        assert!(self.frozen_nodes.is_empty());
                        self.frozen_nodes.insert(node_key, node.into());
                        self.store.write_node_batch(&self.frozen_nodes)?;
                        return Ok(());
                    }
                },
                _ => (),
            }
        }

        self.freeze(0);
        self.store.write_node_batch(&self.frozen_nodes)?;
        Ok(())
    }
}

impl<K> Drop for JellyfishMerkleRestore<K> {
    fn drop(&mut self) {
        if let Some(rx) = self.async_commit_result.take() {
            rx.recv().unwrap().unwrap();
        }
    }
}
