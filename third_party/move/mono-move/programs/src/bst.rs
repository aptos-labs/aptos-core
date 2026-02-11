// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Binary search tree map (u64 → u64), vector-backed arena.
//!
//! Nodes are stored in a `Vec<Node>` with children represented as u64
//! indices (NULL_INDEX = u64::MAX for absent children). This layout
//! mirrors what's expressible in Move (no recursive types / Box).
//!
//! Exercises vector indexing, conditional branching, and enum-like
//! option returns — representative of data-structure-heavy Move contracts.

use std::cmp::Ordering;

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

/// Generate a random sequence of BST operations as a flat `Vec<u64>` of
/// triples `[op, key, value, ...]`. Op codes: 0 = insert, 1 = get, 2 = remove.
/// Weights: 50% insert, 25% get, 25% remove.
pub fn generate_ops(n_ops: u64, key_range: u64, seed: u64) -> Vec<u64> {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    let mut ops = Vec::with_capacity(n_ops as usize * 3);
    for _ in 0..n_ops {
        let r: u32 = rng.gen_range(0, 100);
        if r < 50 {
            // insert(key, value)
            ops.push(0);
            ops.push(rng.gen_range(0, key_range));
            ops.push(rng.gen_range(0, u64::MAX));
        } else if r < 75 {
            // get(key)
            ops.push(1);
            ops.push(rng.gen_range(0, key_range));
            ops.push(0);
        } else {
            // remove(key)
            ops.push(2);
            ops.push(rng.gen_range(0, key_range));
            ops.push(0);
        }
    }
    ops
}

/// Execute a pre-generated ops sequence on the native Rust BST.
pub fn native_run_ops(ops: &[u64]) {
    native_run_ops_with_results(ops);
}

/// Execute ops and return get results: for each get operation, pushes
/// `(found, value)` where found=1 means key exists. Insert/remove ops
/// are executed but produce no output. Used to cross-check implementations.
pub fn native_run_ops_with_results(ops: &[u64]) -> Vec<(u64, u64)> {
    let mut bst = BstMap::new();
    let mut results = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        let op = ops[i];
        let key = ops[i + 1];
        let value = ops[i + 2];
        i += 3;
        match op {
            0 => bst.insert(key, value),
            1 => {
                let r = bst.get(key);
                results.push(match r {
                    Some(v) => (1, v),
                    None => (0, 0),
                });
            },
            2 => bst.remove(key),
            _ => {},
        }
    }
    results
}

// ---------------------------------------------------------------------------
// Micro-op
// ---------------------------------------------------------------------------

/// BstMap is a heap-allocated struct with layout:
///   field 0: nodes (heap ptr to Vec<Node>)
///   field 1: free_list (heap ptr to Vec<u64>)
///   field 2: root (u64 index, NULL = absent)
///
/// Node layout (32 bytes inline in the nodes vector):
///   [0..8) key, [8..16) value, [16..24) left, [24..32) right
///
/// Functions (mirror the native Rust API — insert/get/remove take &mut/& BstMap):
///   0 — new(capacity) → bst_ref
///   1 — insert(bst_ref, key, value)
///   2 — get(bst_ref, key) → (tag, value)
///   3 — alloc_node(bst_ref, key, value) → idx       [internal]
///   4 — remove(bst_ref, key)
///   5 — remove_node(bst_ref, idx) → replacement     [internal]
///
/// Use `InterpreterContext::invoke(func_id)` to call each function
/// in sequence on the same heap.
#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_runtime::{
        CodeOffset as CO, FrameOffset as FO, Function, MicroOp as Op, MicroOp::*, ObjectDescriptor,
        FRAME_METADATA_SIZE,
    };

    const NULL: u64 = u64::MAX;
    const NODE_SIZE: u32 = 32;
    /// Descriptor index for trivial (no-pointer) vector elements.
    const DESC_TRIVIAL: u16 = 0;
    /// Descriptor index for the BstMap heap struct.
    const DESC_BST_MAP: u16 = 1;

    /// BstMap struct field offsets (within the struct payload).
    const BST_NODES: u32 = 0;
    const BST_FREE_LIST: u32 = 8;
    const BST_ROOT: u32 = 16;

    /// Function IDs for use with `invoke()`.
    pub const FN_NEW: usize = 0;
    pub const FN_INSERT: usize = 1;
    pub const FN_GET: usize = 2;
    pub const FN_REMOVE: usize = 4;

    pub fn program(capacity: u64) -> (Vec<Function>, Vec<ObjectDescriptor>) {
        let descriptors = vec![
            ObjectDescriptor::Trivial, // 0: node elements, free_list elements
            ObjectDescriptor::Struct {
                // 1: BstMap { nodes, free_list, root }
                size: 24,
                ref_offsets: vec![0, 8], // nodes and free_list are heap pointers
            },
        ];
        (
            vec![
                make_new(capacity), // 0
                make_insert(3),     // 1, calls alloc_node at 3
                make_get(),         // 2
                make_alloc_node(),  // 3
                make_remove(5),     // 4, calls remove_node at 5
                make_remove_node(), // 5
                make_run_ops(),     // 6
            ],
            descriptors,
        )
    }

    // =================================================================
    // Function 0 — new(capacity) → bst_ref
    //
    // Allocates a BstMap struct on the heap, creates the backing
    // vectors, and returns the struct pointer.
    //
    // Frame layout:
    //   [0] result: bst_ref   [8] nodes (temp)   [16] free_list (temp)
    // =================================================================
    fn make_new(capacity: u64) -> Function {
        let bst = 0u32;
        let nodes = 8u32;
        let free_list = 16u32;

        #[rustfmt::skip]
        let code = vec![
            VecNew { dst: FO(nodes), descriptor_id: DESC_TRIVIAL,
                     elem_size: NODE_SIZE, initial_capacity: capacity },           // 0
            VecNew { dst: FO(free_list), descriptor_id: DESC_TRIVIAL,
                     elem_size: 8, initial_capacity: 4 },                          // 1
            HeapNew { dst: FO(bst), descriptor_id: DESC_BST_MAP },                 // 2
            Op::struct_store8(FO(bst), BST_NODES, FO(nodes)),                 // 3
            Op::struct_store8(FO(bst), BST_FREE_LIST, FO(free_list)),         // 4
            HeapMoveToImm8 { heap_ptr: FO(bst),
                             offset: 8 + BST_ROOT, imm: NULL },                   // 5: STRUCT_DATA_OFFSET=8
            Return,                                                                // 6
        ];

        Function {
            code,
            args_size: 0,
            data_size: 24,
            extended_frame_size: 24 + FRAME_METADATA_SIZE,
            zero_locals: true,
            pointer_slots: vec![FO(bst), FO(nodes), FO(free_list)],
        }
    }

    // =================================================================
    // Function 2 — get(bst_ref, key) → (tag, value)
    //
    // Frame layout:
    //   [0]  bst_ref (ptr) / result: tag   [8] key / result: value
    //   [16] nodes (ptr)   [24] root   [32] idx
    //   [40] node (32B: key[40] val[48] left[56] right[64])
    // =================================================================
    fn make_get() -> Function {
        let bst = 0u32;
        let key = 8u32;
        let nodes = 16u32;
        let root = 24u32;
        let idx = 32u32;
        let node_key = 40u32;
        let node_val = 48u32;
        let node_left = 56u32;
        let node_right = 64u32;
        let tag = 0u32;
        let value = 8u32;

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue: load struct fields --
            Op::struct_load8(FO(bst), BST_NODES, FO(nodes)),                  // 0
            Op::struct_load8(FO(bst), BST_ROOT, FO(root)),                    // 1
            Move8 { dst: FO(idx), src: FO(root) },                                // 2: idx = root
            // -- LOOP (3) --
            JumpGreaterEqualU64Imm { target: CO(14), src: FO(idx), imm: NULL },   // 3: NULL? → NONE
            VecLoadElem { dst: FO(node_key), heap_ptr: FO(nodes),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 4: node = nodes[idx]
            JumpLessU64 { target: CO(10), lhs: FO(key), rhs: FO(node_key) },      // 5: key < node.key → LEFT
            JumpLessU64 { target: CO(12), lhs: FO(node_key), rhs: FO(key) },      // 6: node.key < key → RIGHT
            // EQUAL (7)
            StoreImm8 { dst: FO(tag), imm: 1 },                                   // 7
            Move8 { dst: FO(value), src: FO(node_val) },                           // 8
            Return,                                                                // 9
            // GO_LEFT (10)
            Move8 { dst: FO(idx), src: FO(node_left) },                           // 10
            Jump { target: CO(3) },                                                // 11
            // GO_RIGHT (12)
            Move8 { dst: FO(idx), src: FO(node_right) },                          // 12
            Jump { target: CO(3) },                                                // 13
            // NONE (14)
            StoreImm8 { dst: FO(tag), imm: 0 },                                   // 14
            StoreImm8 { dst: FO(value), imm: 0 },                                 // 15
            Return,                                                                // 16
        ];

        Function {
            code,
            args_size: 16,
            data_size: 72,
            extended_frame_size: 72 + FRAME_METADATA_SIZE,
            zero_locals: false,
            pointer_slots: vec![FO(bst), FO(nodes)],
        }
    }

    // =================================================================
    // Function 1 — insert(&mut bst, key, value)
    //
    // Mirrors the Rust insert method: each branch (root-null, left-null,
    // right-null, equal) is self-contained with its own early return.
    //
    // Frame layout:
    //   [0]  bst_ref (ptr)   [8] key   [16] value
    //   [24] nodes (ptr)   [32] root
    //   [40] idx   [48] node (32B: key[48] val[56] left[64] right[72])
    //   [80] metadata (24B)
    //   [104] callee: bst_ref  [112] callee: key  [120] callee: value
    // =================================================================
    fn make_insert(alloc_node_id: u32) -> Function {
        let meta = FRAME_METADATA_SIZE as u32;
        let bst = 0u32;
        let key = 8u32;
        let value = 16u32;
        let nodes = 24u32;
        let root = 32u32;
        let idx = 40u32;
        let node = 48u32;
        let node_key = 48u32;
        let node_val = 56u32;
        let node_left = 64u32;
        let node_right = 72u32;
        let data_size = 80u32;
        let c0 = data_size + meta; // 104
        let c1 = c0 + 8; // 112
        let c2 = c1 + 8; // 120

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue: load struct fields --
            Op::struct_load8(FO(bst), BST_NODES, FO(nodes)),                       // 0
            Op::struct_load8(FO(bst), BST_ROOT, FO(root)),                         // 1
            // -- if root != NULL → LOOP_SETUP; else fall through to INSERT_ROOT --
            JumpLessU64Imm { target: CO(9),
                             src: FO(root), imm: NULL },                           // 2: → LOOP_SETUP
            // -- INSERT_ROOT (3): bst.root = alloc_node(bst, key, value); return --
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 3
            Move8 { dst: FO(c1), src: FO(key) },                                  // 4
            Move8 { dst: FO(c2), src: FO(value) },                                // 5
            CallFunc { func_id: alloc_node_id },                                   // 6
            Op::struct_store8(FO(bst), BST_ROOT, FO(c0)),                          // 7
            Return,                                                                // 8
            // -- LOOP_SETUP (9) --
            Move8 { dst: FO(idx), src: FO(root) },                                // 9
            // -- LOOP (10): load node, 3-way compare --
            VecLoadElem { dst: FO(node), heap_ptr: FO(nodes),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 10
            // -- key < node_key? (GO_LEFT falls through, skip if >=) --
            JumpGreaterEqualU64 { target: CO(22),
                                  lhs: FO(key), rhs: FO(node_key) },              // 11: key >= node_key → NOT_LESS
            // GO_LEFT (12): key < node_key
            JumpLessU64Imm { target: CO(20),
                             src: FO(node_left), imm: NULL },                     // 12: left != NULL → CONTINUE_LEFT
            // INSERT_LEFT (13): node.left = alloc_node(...); store node; return
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 13
            Move8 { dst: FO(c1), src: FO(key) },                                  // 14
            Move8 { dst: FO(c2), src: FO(value) },                                // 15
            CallFunc { func_id: alloc_node_id },                                   // 16
            Move8 { dst: FO(node_left), src: FO(c0) },                            // 17
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(idx),
                           src: FO(node), elem_size: NODE_SIZE },                  // 18
            Return,                                                                // 19
            // CONTINUE_LEFT (20)
            Move8 { dst: FO(idx), src: FO(node_left) },                           // 20
            Jump { target: CO(10) },                                               // 21
            // -- NOT_LESS (22): key > node_key? (GO_RIGHT falls through, skip to EQUAL if >=) --
            JumpGreaterEqualU64 { target: CO(33),
                                  lhs: FO(node_key), rhs: FO(key) },              // 22: node_key >= key → EQUAL
            // GO_RIGHT (23): key > node_key
            JumpLessU64Imm { target: CO(31),
                             src: FO(node_right), imm: NULL },                    // 23: right != NULL → CONTINUE_RIGHT
            // INSERT_RIGHT (24): node.right = alloc_node(...); store node; return
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 24
            Move8 { dst: FO(c1), src: FO(key) },                                  // 25
            Move8 { dst: FO(c2), src: FO(value) },                                // 26
            CallFunc { func_id: alloc_node_id },                                   // 27
            Move8 { dst: FO(node_right), src: FO(c0) },                           // 28
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(idx),
                           src: FO(node), elem_size: NODE_SIZE },                  // 29
            Return,                                                                // 30
            // CONTINUE_RIGHT (31)
            Move8 { dst: FO(idx), src: FO(node_right) },                          // 31
            Jump { target: CO(10) },                                               // 32
            // -- EQUAL (33): node.value = value; store node; return --
            Move8 { dst: FO(node_val), src: FO(value) },                           // 33
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(idx),
                           src: FO(node), elem_size: NODE_SIZE },                  // 34
            Return,                                                                // 35
        ];

        Function {
            code,
            args_size: 24,
            data_size: data_size as usize,
            extended_frame_size: (c2 + 8) as usize,
            zero_locals: true,
            pointer_slots: vec![FO(bst), FO(nodes)],
        }
    }

    // =================================================================
    // Function 3 — alloc_node(&mut bst, key, value) → idx
    //
    // Loads nodes/free_list from the BstMap struct. If free_list is
    // non-empty, pops an index and overwrites that slot. Otherwise
    // appends to nodes. Returns the new node's index.
    //
    // Frame layout:
    //   [0]  bst_ref (ptr) / result: idx   [8] key   [16] value
    //   [24] nodes (ptr)   [32] free_list (ptr)
    //   [40] idx   [48] fl_len
    //   [56] new_node (32B: key[56] val[64] left[72] right[80])
    // =================================================================
    fn make_alloc_node() -> Function {
        let bst = 0u32;
        let key = 8u32;
        let value = 16u32;
        let nodes = 24u32;
        let free_list = 32u32;
        let idx = 40u32;
        let fl_len = 48u32;
        let new_node = 56u32;
        let new_node_key = 56u32;
        let new_node_val = 64u32;
        let new_node_left = 72u32;
        let new_node_right = 80u32;
        let result = 0u32;

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue: load struct fields --
            Op::struct_load8(FO(bst), BST_NODES, FO(nodes)),                       // 0
            Op::struct_load8(FO(bst), BST_FREE_LIST, FO(free_list)),               // 1
            // Build new_node = { key, value, NULL, NULL }
            Move8 { dst: FO(new_node_key), src: FO(key) },                        // 2
            Move8 { dst: FO(new_node_val), src: FO(value) },                      // 3
            StoreImm8 { dst: FO(new_node_left), imm: NULL },                      // 4
            StoreImm8 { dst: FO(new_node_right), imm: NULL },                     // 5
            // Check free_list
            VecLen { dst: FO(fl_len), heap_ptr: FO(free_list) },                   // 6
            JumpNotZeroU64 { target: CO(11), src: FO(fl_len) },                    // 7: → POP
            // PUSH path: idx = nodes.len(); nodes.push(new_node)
            VecLen { dst: FO(idx), heap_ptr: FO(nodes) },                          // 8
            VecPushBack { heap_ptr: FO(nodes), elem: FO(new_node),
                          elem_size: NODE_SIZE },                                  // 9
            Jump { target: CO(13) },                                               // 10: → DONE
            // POP path (11): idx = free_list.pop(); nodes[idx] = new_node
            VecPopBack { dst: FO(idx), heap_ptr: FO(free_list),
                         elem_size: 8 },                                           // 11
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(idx),
                           src: FO(new_node), elem_size: NODE_SIZE },              // 12
            // DONE (13)
            Move8 { dst: FO(result), src: FO(idx) },                               // 13
            Return,                                                                // 14
        ];

        Function {
            code,
            args_size: 24,
            data_size: 88,
            extended_frame_size: 88 + FRAME_METADATA_SIZE,
            zero_locals: true,
            pointer_slots: vec![FO(bst), FO(nodes), FO(free_list)],
        }
    }

    // =================================================================
    // Function 4 — remove(&mut bst, key)
    //
    // Searches for `key`, calls remove_node to detach the node, then
    // fixes up the parent pointer (or root).
    //
    // Frame layout:
    //   [0]  bst_ref (ptr)   [8] key
    //   [16] nodes (ptr)   [24] root
    //   [32] parent   [40] idx
    //   [48] node (32B: key[48] val[56] left[64] right[72])
    //   [80] metadata (24B)
    //   [104] callee: bst_ref / result  [112] callee: idx
    // =================================================================
    fn make_remove(remove_node_id: u32) -> Function {
        let meta = FRAME_METADATA_SIZE as u32;
        let bst = 0u32;
        let key = 8u32;
        let nodes = 16u32;
        let root = 24u32;
        let parent = 32u32;
        let idx = 40u32;
        let node = 48u32;
        let node_key = 48u32;
        let node_left = 64u32;
        let node_right = 72u32;
        let data_size = 80u32;
        let c0 = data_size + meta; // 104 — also holds replacement after CallFunc
        let c1 = c0 + 8; // 112

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue --
            Op::struct_load8(FO(bst), BST_NODES, FO(nodes)),                       // 0
            Op::struct_load8(FO(bst), BST_ROOT, FO(root)),                         // 1
            StoreImm8 { dst: FO(parent), imm: NULL },                              // 2
            Move8 { dst: FO(idx), src: FO(root) },                                // 3
            // -- LOOP (4): while idx != NULL --
            JumpGreaterEqualU64Imm { target: CO(26),
                                     src: FO(idx), imm: NULL },                   // 4: → DONE
            VecLoadElem { dst: FO(node), heap_ptr: FO(nodes),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 5
            // -- key < node_key? --
            JumpGreaterEqualU64 { target: CO(10),
                                  lhs: FO(key), rhs: FO(node_key) },              // 6: key >= → NOT_LESS
            // GO_LEFT: key < node_key
            Move8 { dst: FO(parent), src: FO(idx) },                              // 7
            Move8 { dst: FO(idx), src: FO(node_left) },                           // 8
            Jump { target: CO(4) },                                                // 9
            // -- NOT_LESS (10): key > node_key? --
            JumpGreaterEqualU64 { target: CO(14),
                                  lhs: FO(node_key), rhs: FO(key) },              // 10: node_key >= key → EQUAL
            // GO_RIGHT: key > node_key
            Move8 { dst: FO(parent), src: FO(idx) },                              // 11
            Move8 { dst: FO(idx), src: FO(node_right) },                          // 12
            Jump { target: CO(4) },                                                // 13
            // -- EQUAL (14): remove_node(bst, idx) → c0 holds replacement --
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 14
            Move8 { dst: FO(c1), src: FO(idx) },                                  // 15
            CallFunc { func_id: remove_node_id },                                  // 16
            // -- if parent != NULL → HAS_PARENT --
            JumpLessU64Imm { target: CO(20),
                             src: FO(parent), imm: NULL },                        // 17: → HAS_PARENT
            // UPDATE_ROOT: parent == NULL, bst.root = replacement (c0)
            Op::struct_store8(FO(bst), BST_ROOT, FO(c0)),                          // 18
            Return,                                                                // 19
            // -- HAS_PARENT (20) --
            VecLoadElem { dst: FO(node), heap_ptr: FO(nodes),
                          idx: FO(parent), elem_size: NODE_SIZE },                 // 20
            JumpNotEqualU64 { target: CO(24),
                              lhs: FO(node_left), rhs: FO(idx) },                 // 21: → UPDATE_RIGHT
            // UPDATE_LEFT: parent.left = replacement (c0)
            Move8 { dst: FO(node_left), src: FO(c0) },                            // 22
            Jump { target: CO(25) },                                               // 23: → STORE_PARENT
            // UPDATE_RIGHT (24): parent.right = replacement (c0)
            Move8 { dst: FO(node_right), src: FO(c0) },                           // 24
            // -- STORE_PARENT (25): shared epilogue --
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(parent),
                           src: FO(node), elem_size: NODE_SIZE },                  // 25
            Return,                                                                // 26
        ];

        Function {
            code,
            args_size: 16,
            data_size: data_size as usize,
            extended_frame_size: (c1 + 8) as usize,
            zero_locals: true,
            pointer_slots: vec![FO(bst), FO(nodes)],
        }
    }

    // =================================================================
    // Function 5 — remove_node(&mut bst, idx) → replacement
    //
    // Detaches the node at `idx` from the tree, pushes it to the free
    // list, and returns the index of its replacement (or NULL).
    //
    // Frame layout:
    //   [0]  bst_ref (ptr) / result   [8] idx
    //   [16] nodes (ptr)   [24] free_list (ptr)
    //   [32] left   [40] right
    //   [48] parent   [56] cur   [64] cur_right
    //   [72] scratch (32B: key[72] val[80] left[88] right[96])
    // =================================================================
    fn make_remove_node() -> Function {
        let bst = 0u32;
        let idx = 8u32;
        let nodes = 16u32;
        let free_list = 24u32;
        let left = 32u32;
        let right = 40u32;
        let parent = 48u32;
        let cur = 56u32;
        let cur_right = 64u32;
        let scratch = 72u32;
        let scratch_left = 88u32; // scratch + 16
        let scratch_right = 96u32; // scratch + 24
        let result = 0u32;

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue --
            Op::struct_load8(FO(bst), BST_NODES, FO(nodes)),                       // 0
            Op::struct_load8(FO(bst), BST_FREE_LIST, FO(free_list)),               // 1
            VecLoadElem { dst: FO(scratch), heap_ptr: FO(nodes),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 2
            Move8 { dst: FO(left), src: FO(scratch_left) },                        // 3
            Move8 { dst: FO(right), src: FO(scratch_right) },                      // 4
            // -- if left == NULL: free idx, return right --
            JumpLessU64Imm { target: CO(8),
                             src: FO(left), imm: NULL },                           // 5: left != NULL → HAS_LEFT
            Move8 { dst: FO(result), src: FO(right) },                            // 6
            Jump { target: CO(32) },                                               // 7: → FREE_RETURN
            // -- HAS_LEFT (8): if right == NULL: free idx, return left --
            JumpLessU64Imm { target: CO(11),
                             src: FO(right), imm: NULL },                          // 8: right != NULL → HAS_BOTH
            Move8 { dst: FO(result), src: FO(left) },                             // 9
            Jump { target: CO(32) },                                               // 10: → FREE_RETURN
            // -- HAS_BOTH (11): load nodes[right], check right.left --
            VecLoadElem { dst: FO(scratch), heap_ptr: FO(nodes),
                          idx: FO(right), elem_size: NODE_SIZE },                  // 11
            JumpLessU64Imm { target: CO(17),
                             src: FO(scratch_left), imm: NULL },                   // 12: right.left != NULL → DEEP
            // SIMPLE_SUCCESSOR (13): right.left == NULL, right adopts left
            Move8 { dst: FO(scratch_left), src: FO(left) },                       // 13
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(right),
                           src: FO(scratch), elem_size: NODE_SIZE },               // 14
            Move8 { dst: FO(result), src: FO(right) },                            // 15
            Jump { target: CO(32) },                                               // 16: → FREE_RETURN
            // -- DEEP_SUCCESSOR (17): walk left to find in-order successor --
            Move8 { dst: FO(parent), src: FO(right) },                            // 17
            Move8 { dst: FO(cur), src: FO(scratch_left) },                        // 18
            // WALK_LOOP (19)
            VecLoadElem { dst: FO(scratch), heap_ptr: FO(nodes),
                          idx: FO(cur), elem_size: NODE_SIZE },                    // 19
            JumpGreaterEqualU64Imm { target: CO(24),
                                     src: FO(scratch_left), imm: NULL },           // 20: cur.left == NULL → WALK_DONE
            Move8 { dst: FO(parent), src: FO(cur) },                              // 21
            Move8 { dst: FO(cur), src: FO(scratch_left) },                        // 22
            Jump { target: CO(19) },                                               // 23
            // -- WALK_DONE (24): scratch = nodes[cur] --
            // Save cur.right, then set cur's children to left/right from removed node
            Move8 { dst: FO(cur_right), src: FO(scratch_right) },                 // 24
            Move8 { dst: FO(scratch_left), src: FO(left) },                       // 25: cur.left = left
            Move8 { dst: FO(scratch_right), src: FO(right) },                     // 26: cur.right = right
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(cur),
                           src: FO(scratch), elem_size: NODE_SIZE },               // 27
            // Detach cur from parent: parent.left = cur_right
            VecLoadElem { dst: FO(scratch), heap_ptr: FO(nodes),
                          idx: FO(parent), elem_size: NODE_SIZE },                 // 28
            Move8 { dst: FO(scratch_left), src: FO(cur_right) },                  // 29
            VecStoreElem { heap_ptr: FO(nodes), idx: FO(parent),
                           src: FO(scratch), elem_size: NODE_SIZE },               // 30
            Move8 { dst: FO(result), src: FO(cur) },                              // 31
            // -- FREE_RETURN (32): free_list.push(idx); return --
            VecPushBack { heap_ptr: FO(free_list), elem: FO(idx),
                          elem_size: 8 },                                          // 32
            Return,                                                                // 33
        ];

        Function {
            code,
            args_size: 16,
            data_size: 104,
            extended_frame_size: 104 + FRAME_METADATA_SIZE,
            zero_locals: true,
            pointer_slots: vec![FO(bst), FO(nodes), FO(free_list)],
        }
    }

    // =================================================================
    // Function 6 — run_ops(ops_vec)
    //
    // Creates a new BST, then iterates the ops vector (flat triples of
    // [op, key, value, ...]) and dispatches to insert/get/remove.
    // Op codes: 0 = insert, 1 = get, 2 = remove.
    //
    // Frame layout:
    //   [0]  ops (ptr)
    //   [8]  bst (ptr)   [16] len   [24] i
    //   [32] op_code   [40] key   [48] value
    //   [56] metadata (24 bytes)
    //   [80] c0   [88] c1   [96] c2
    // =================================================================
    fn make_run_ops() -> Function {
        let meta = FRAME_METADATA_SIZE as u32;
        let ops = 0u32;
        let bst = 8u32;
        let len = 16u32;
        let i = 24u32;
        let op_code = 32u32;
        let key = 40u32;
        let value = 48u32;
        let data_size = 56u32;
        let c0 = data_size + meta; // 80
        let c1 = c0 + 8; // 88
        let c2 = c1 + 8; // 96

        #[rustfmt::skip]
        let code = vec![
            // -- Create BST --
            CallFunc { func_id: FN_NEW as u32 },                                  // 0
            Move8 { dst: FO(bst), src: FO(c0) },                                 // 1
            // -- Init loop --
            VecLen { dst: FO(len), heap_ptr: FO(ops) },                           // 2
            StoreImm8 { dst: FO(i), imm: 0 },                                    // 3
            // -- LOOP (4) --
            JumpGreaterEqualU64 { target: CO(27),
                                  lhs: FO(i), rhs: FO(len) },                    // 4: → DONE
            // Load triple
            VecLoadElem { dst: FO(op_code), heap_ptr: FO(ops),
                          idx: FO(i), elem_size: 8 },                            // 5
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                       // 6
            VecLoadElem { dst: FO(key), heap_ptr: FO(ops),
                          idx: FO(i), elem_size: 8 },                            // 7
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                       // 8
            VecLoadElem { dst: FO(value), heap_ptr: FO(ops),
                          idx: FO(i), elem_size: 8 },                            // 9
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                       // 10
            // -- Dispatch --
            JumpNotZeroU64 { target: CO(17), src: FO(op_code) },                 // 11: → CHECK_GET
            // INSERT (12)
            Move8 { dst: FO(c0), src: FO(bst) },                                 // 12
            Move8 { dst: FO(c1), src: FO(key) },                                 // 13
            Move8 { dst: FO(c2), src: FO(value) },                               // 14
            CallFunc { func_id: FN_INSERT as u32 },                               // 15
            Jump { target: CO(4) },                                               // 16
            // CHECK_GET (17)
            JumpGreaterEqualU64Imm { target: CO(23),
                                      src: FO(op_code), imm: 2 },               // 17: → REMOVE
            // GET (18)
            Move8 { dst: FO(c0), src: FO(bst) },                                 // 18
            Move8 { dst: FO(c1), src: FO(key) },                                 // 19
            CallFunc { func_id: FN_GET as u32 },                                  // 20
            Jump { target: CO(4) },                                               // 21
            // skip (22) — padding for alignment, shouldn't be reached
            Jump { target: CO(4) },                                               // 22
            // REMOVE (23)
            Move8 { dst: FO(c0), src: FO(bst) },                                 // 23
            Move8 { dst: FO(c1), src: FO(key) },                                 // 24
            CallFunc { func_id: FN_REMOVE as u32 },                               // 25
            Jump { target: CO(4) },                                               // 26
            // DONE (27)
            Return,                                                               // 27
        ];

        Function {
            code,
            args_size: 8,
            data_size: data_size as usize,
            extended_frame_size: (c2 + 8) as usize,
            zero_locals: true,
            pointer_slots: vec![FO(ops), FO(bst)],
        }
    }
}

#[cfg(feature = "micro-op")]
pub use micro_op::{program as micro_op_bst, FN_GET, FN_INSERT, FN_NEW, FN_REMOVE};

// ---------------------------------------------------------------------------
// Move bytecode
// ---------------------------------------------------------------------------

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use move_binary_format::file_format::CompiledModule;

    pub const SOURCE: &str = "
module 0x1::bst {
    use std::vector;

    const NULL: u64 = 18446744073709551615; // u64::MAX

    struct Node has copy, drop {
        key: u64,
        value: u64,
        left: u64,
        right: u64,
    }

    struct BstMap has drop {
        nodes: vector<Node>,
        root: u64,
        free_list: vector<u64>,
    }

    public fun new(): BstMap {
        BstMap {
            nodes: vector::empty<Node>(),
            root: NULL,
            free_list: vector::empty<u64>(),
        }
    }

    public fun insert(bst: &mut BstMap, key: u64, value: u64) {
        if (bst.root == NULL) {
            bst.root = alloc_node(bst, key, value);
            return
        };

        let idx = bst.root;
        loop {
            let node_key = vector::borrow(&bst.nodes, idx).key;
            if (key < node_key) {
                let left = vector::borrow(&bst.nodes, idx).left;
                if (left == NULL) {
                    let new_idx = alloc_node(bst, key, value);
                    vector::borrow_mut(&mut bst.nodes, idx).left = new_idx;
                    return
                };
                idx = left;
            } else if (key > node_key) {
                let right = vector::borrow(&bst.nodes, idx).right;
                if (right == NULL) {
                    let new_idx = alloc_node(bst, key, value);
                    vector::borrow_mut(&mut bst.nodes, idx).right = new_idx;
                    return
                };
                idx = right;
            } else {
                vector::borrow_mut(&mut bst.nodes, idx).value = value;
                return
            }
        }
    }

    public fun get(bst: &BstMap, key: u64): (bool, u64) {
        let idx = bst.root;
        while (idx != NULL) {
            let node = vector::borrow(&bst.nodes, idx);
            if (key < node.key) {
                idx = node.left;
            } else if (key > node.key) {
                idx = node.right;
            } else {
                return (true, node.value)
            }
        };
        (false, 0)
    }

    public fun remove(bst: &mut BstMap, key: u64) {
        let parent = NULL;
        let idx = bst.root;
        while (idx != NULL) {
            let node_key = vector::borrow(&bst.nodes, idx).key;
            if (key < node_key) {
                parent = idx;
                idx = vector::borrow(&bst.nodes, idx).left;
            } else if (key > node_key) {
                parent = idx;
                idx = vector::borrow(&bst.nodes, idx).right;
            } else {
                let replacement = remove_node(bst, idx);
                if (parent == NULL) {
                    bst.root = replacement;
                } else if (vector::borrow(&bst.nodes, parent).left == idx) {
                    vector::borrow_mut(&mut bst.nodes, parent).left = replacement;
                } else {
                    vector::borrow_mut(&mut bst.nodes, parent).right = replacement;
                };
                return
            }
        }
    }

    fun remove_node(bst: &mut BstMap, idx: u64): u64 {
        let left = vector::borrow(&bst.nodes, idx).left;
        let right = vector::borrow(&bst.nodes, idx).right;

        if (left == NULL) {
            free_node(bst, idx);
            return right
        };
        if (right == NULL) {
            free_node(bst, idx);
            return left
        };

        // 2 children: find in-order successor (min of right subtree).
        if (vector::borrow(&bst.nodes, right).left == NULL) {
            // Right child is the successor.
            vector::borrow_mut(&mut bst.nodes, right).left = left;
            free_node(bst, idx);
            return right
        };

        // Successor is deeper — walk left.
        let parent = right;
        let cur = vector::borrow(&bst.nodes, right).left;
        while (vector::borrow(&bst.nodes, cur).left != NULL) {
            parent = cur;
            cur = vector::borrow(&bst.nodes, cur).left;
        };
        let cur_right = vector::borrow(&bst.nodes, cur).right;
        vector::borrow_mut(&mut bst.nodes, parent).left = cur_right;
        let cur_node = vector::borrow_mut(&mut bst.nodes, cur);
        cur_node.left = left;
        cur_node.right = right;
        free_node(bst, idx);
        cur
    }

    fun alloc_node(bst: &mut BstMap, key: u64, value: u64): u64 {
        let node = Node { key, value, left: NULL, right: NULL };
        if (!vector::is_empty(&bst.free_list)) {
            let idx = vector::pop_back(&mut bst.free_list);
            *vector::borrow_mut(&mut bst.nodes, idx) = node;
            idx
        } else {
            let idx = vector::length(&bst.nodes);
            vector::push_back(&mut bst.nodes, node);
            idx
        }
    }

    fun free_node(bst: &mut BstMap, idx: u64) {
        vector::push_back(&mut bst.free_list, idx);
    }

    public fun run_ops(ops: vector<u64>) {
        let bst = new();
        let len = vector::length(&ops);
        let i = 0;
        while (i < len) {
            let op = *vector::borrow(&ops, i);
            let key = *vector::borrow(&ops, i + 1);
            let value = *vector::borrow(&ops, i + 2);
            i = i + 3;
            if (op == 0) {
                insert(&mut bst, key, value);
            } else if (op == 1) {
                get(&bst, key);
            } else {
                remove(&mut bst, key);
            };
        };
    }
}
";

    pub fn program() -> CompiledModule {
        crate::compile_move_source_with_deps(SOURCE, &[crate::MOVE_STDLIB_DIR])
    }

    /// Compile the stdlib vector module for publishing alongside the BST module.
    /// Needed because vector<Node> (struct-typed) generates runtime dependencies.
    pub fn stdlib_vector() -> CompiledModule {
        crate::compile_move_source_with_deps(
            &std::fs::read_to_string(
                std::path::Path::new(crate::MOVE_STDLIB_DIR).join("vector.move"),
            )
            .expect("failed to read vector.move"),
            &[],
        )
    }
}

#[cfg(feature = "move-bytecode")]
pub use move_bytecode::{program as move_bytecode_bst, stdlib_vector as move_stdlib_vector};
