// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Binary search tree map (u64 → u64), vector-backed arena.
//!
//! Nodes are stored in a `Vec<Node>` with children represented as u64
//! indices (NULL_INDEX = u64::MAX for absent children). This layout mirrors
//! what's expressible in Move (no recursive types / Box).
//!
//! Native Rust reference plus the Move source (run through the pipeline /
//! MoveVM). Correctness is covered by `differential/programs/bst.move`, whose
//! `run_ops_checksum` entry generates a deterministic op stream in-Move.

use std::cmp::Ordering;

/// Canonical Move source — the same file the differential test drives.
pub const SOURCE: &str = include_str!("../../tests/test_cases/differential/programs/bst.move");

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

const NULL: usize = usize::MAX;

#[derive(Clone)]
struct Node {
    key: u64,
    value: u64,
    left: usize,  // index into nodes, NULL = absent
    right: usize, // index into nodes, NULL = absent
}

pub struct BstMap {
    nodes: Vec<Node>,
    root: usize,
    free_list: Vec<usize>,
}

impl BstMap {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        BstMap {
            nodes: Vec::new(),
            root: NULL,
            free_list: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: u64, value: u64) {
        if self.root == NULL {
            self.root = self.alloc_node(key, value);
            return;
        }

        let mut idx = self.root;
        loop {
            match key.cmp(&self.nodes[idx].key) {
                Ordering::Less => {
                    let left = self.nodes[idx].left;
                    if left == NULL {
                        self.nodes[idx].left = self.alloc_node(key, value);
                        return;
                    }
                    idx = left;
                },
                Ordering::Greater => {
                    let right = self.nodes[idx].right;
                    if right == NULL {
                        self.nodes[idx].right = self.alloc_node(key, value);
                        return;
                    }
                    idx = right;
                },
                Ordering::Equal => {
                    self.nodes[idx].value = value;
                    return;
                },
            }
        }
    }

    pub fn get(&self, key: u64) -> Option<u64> {
        let mut idx = self.root;
        while idx != NULL {
            let node = &self.nodes[idx];
            match key.cmp(&node.key) {
                Ordering::Less => idx = node.left,
                Ordering::Greater => idx = node.right,
                Ordering::Equal => return Some(node.value),
            }
        }
        None
    }

    pub fn remove(&mut self, key: u64) {
        let mut parent = NULL;
        let mut idx = self.root;
        while idx != NULL {
            match key.cmp(&self.nodes[idx].key) {
                Ordering::Equal => {
                    let replacement = self.remove_node(idx);
                    if parent == NULL {
                        self.root = replacement;
                    } else if self.nodes[parent].left == idx {
                        self.nodes[parent].left = replacement;
                    } else {
                        self.nodes[parent].right = replacement;
                    }
                    return;
                },
                Ordering::Less => {
                    parent = idx;
                    idx = self.nodes[idx].left;
                },
                Ordering::Greater => {
                    parent = idx;
                    idx = self.nodes[idx].right;
                },
            }
        }
    }

    /// Remove the node at `idx` and return the index of its replacement (or NULL).
    fn remove_node(&mut self, idx: usize) -> usize {
        let left = self.nodes[idx].left;
        let right = self.nodes[idx].right;

        // 0 or 1 child: promote the other child.
        if left == NULL {
            self.free_node(idx);
            return right;
        }
        if right == NULL {
            self.free_node(idx);
            return left;
        }

        // 2 children: replace with in-order successor (min of right subtree).
        if self.nodes[right].left == NULL {
            // Right child is the successor — just adopt left.
            self.nodes[right].left = left;
            self.free_node(idx);
            return right;
        }

        // Successor is deeper — walk left to find and detach it.
        let mut parent = right;
        let mut cur = self.nodes[right].left;
        while self.nodes[cur].left != NULL {
            parent = cur;
            cur = self.nodes[cur].left;
        }
        self.nodes[parent].left = self.nodes[cur].right; // detach cur
        self.nodes[cur].left = left; // cur takes idx's place
        self.nodes[cur].right = right;
        self.free_node(idx);
        cur
    }

    fn alloc_node(&mut self, key: u64, value: u64) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.nodes[idx] = Node {
                key,
                value,
                left: NULL,
                right: NULL,
            };
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Node {
                key,
                value,
                left: NULL,
                right: NULL,
            });
            idx
        }
    }

    fn free_node(&mut self, idx: usize) {
        self.free_list.push(idx);
    }
}

/// Native mirror of the Move `run_ops_checksum` entry: drive a deterministic
/// LCG-generated op stream (50% insert / 25% get / 25% remove) through the
/// BST, folding each `get` into a checksum. Used as the bench baseline.
pub fn native_run_ops_checksum(n_ops: u64, key_range: u64, seed: u64) -> u64 {
    // Checksum-fold modulus; matches the `CHECKSUM_MOD` Move const in bst.move.
    const CHECKSUM_MOD: u64 = 1_000_000_007;
    let mut bst = BstMap::new();
    let mut acc: u64 = 0;
    let mut x = seed % super::LCG_MOD;
    for _ in 0..n_ops {
        x = super::lcg_next(x);
        let op = x % 4;
        x = super::lcg_next(x);
        let key = x % key_range;
        x = super::lcg_next(x);
        let value = x;
        if op < 2 {
            bst.insert(key, value);
        } else if op == 2 {
            let contribution = match bst.get(key) {
                Some(found_value) => found_value + 1,
                None => 0,
            };
            acc = (acc * super::LCG_MOD + contribution) % CHECKSUM_MOD;
        } else {
            bst.remove(key);
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// Move bytecode (for the legacy MoveVM bench flavor)
// ---------------------------------------------------------------------------

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_bst() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}
