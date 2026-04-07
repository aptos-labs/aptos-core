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
    use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
    use mono_move_core::{
        CodeOffset as CO, DescriptorId, FrameOffset as FO, Function, MicroOp as Op, MicroOp::*,
        FRAME_METADATA_SIZE,
    };
    use mono_move_runtime::ObjectDescriptor;

    const NULL: u64 = u64::MAX;
    const NODE_SIZE: u32 = 32;
    /// Descriptor index for trivial (no-pointer) vector elements.
    const DESC_TRIVIAL: DescriptorId = DescriptorId(0);
    /// Descriptor index for the BstMap heap struct.
    const DESC_BST_MAP: DescriptorId = DescriptorId(1);

    /// BstMap struct field offsets (within the struct payload).
    const BST_NODES: u32 = 0;
    const BST_FREE_LIST: u32 = 8;
    const BST_ROOT: u32 = 16;

    /// Function IDs for use with `invoke()`.
    pub const FN_NEW: usize = 0;
    pub const FN_INSERT: usize = 1;
    pub const FN_GET: usize = 2;
    pub const FN_REMOVE: usize = 4;

    pub fn program() -> (
        Vec<Option<ExecutableArenaPtr<Function>>>,
        Vec<ObjectDescriptor>,
        ExecutableArena,
    ) {
        let arena = ExecutableArena::new();
        let descriptors = vec![
            ObjectDescriptor::Trivial, // 0: node elements, free_list elements
            ObjectDescriptor::Struct {
                // 1: BstMap { nodes, free_list, root }
                size: 24,
                pointer_offsets: vec![0, 8], // nodes and free_list are heap pointers
            },
        ];
        (
            vec![
                Some(make_new(&arena)),         // 0
                Some(make_insert(&arena, 3)),   // 1, calls alloc_node at 3
                Some(make_get(&arena)),         // 2
                Some(make_alloc_node(&arena)),  // 3
                Some(make_remove(&arena, 5)),   // 4, calls remove_node at 5
                Some(make_remove_node(&arena)), // 5
                Some(make_run_ops(&arena)),     // 6
            ],
            descriptors,
            arena,
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
    fn make_new(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
        let bst = 0u32;
        let nodes = 8u32;
        let free_list = 16u32;

        #[rustfmt::skip]
        let code = vec![
            VecNew { dst: FO(nodes) },                                             // 0
            VecNew { dst: FO(free_list) },                                         // 1
            HeapNew { dst: FO(bst), descriptor_id: DESC_BST_MAP },                 // 2
            Op::struct_store8(FO(bst), BST_NODES, FO(nodes)),                 // 3
            Op::struct_store8(FO(bst), BST_FREE_LIST, FO(free_list)),         // 4
            HeapMoveToImm8 { heap_ptr: FO(bst),
                             offset: 8 + BST_ROOT, imm: NULL },                   // 5: STRUCT_DATA_OFFSET=8
            Return,                                                                // 6
        ];

        let code = arena.alloc_slice_fill_iter(code);
        let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(bst), FO(nodes), FO(free_list)]);
        arena.alloc(Function {
            name: GlobalArenaPtr::from_static("new"),
            code,
            args_size: 0,
            args_and_locals_size: 24,
            extended_frame_size: 24 + FRAME_METADATA_SIZE,
            zero_frame: true,
            pointer_offsets,
        })
    }

    // =================================================================
    // Function 2 — get(bst_ref, key) → (tag, value)
    //
    // Frame layout:
    //   [0]  bst (ptr) / result: tag   [8] key / result: value
    //   [16] bst_ref (16B fat ptr)   [32] nodes_ref (16B fat ptr)
    //   [48] root   [56] idx
    //   [64] node (32B: key[64] val[72] left[80] right[88])
    // =================================================================
    fn make_get(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
        let bst = 0u32;
        let key = 8u32;
        let bst_ref = 16u32;
        let nodes_ref = 32u32;
        let root = 48u32;
        let idx = 56u32;
        let node_key = 64u32;
        let node_val = 72u32;
        let node_left = 80u32;
        let node_right = 88u32;
        let tag = 0u32;
        let value = 8u32;

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue: borrow struct fields --
            SlotBorrow { dst: FO(bst_ref), local: FO(bst) },                      // 0
            Op::struct_borrow(FO(bst_ref), BST_NODES, FO(nodes_ref)),             // 1
            Op::struct_load8(FO(bst), BST_ROOT, FO(root)),                        // 2
            Move8 { dst: FO(idx), src: FO(root) },                                // 3: idx = root
            // -- LOOP (4) --
            JumpGreaterEqualU64Imm { target: CO(15), src: FO(idx), imm: NULL },   // 4: NULL? → NONE
            VecLoadElem { dst: FO(node_key), vec_ref: FO(nodes_ref),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 5: node = nodes[idx]
            JumpLessU64 { target: CO(11), lhs: FO(key), rhs: FO(node_key) },      // 6: key < node.key → LEFT
            JumpLessU64 { target: CO(13), lhs: FO(node_key), rhs: FO(key) },      // 7: node.key < key → RIGHT
            // EQUAL (8)
            StoreImm8 { dst: FO(tag), imm: 1 },                                   // 8
            Move8 { dst: FO(value), src: FO(node_val) },                           // 9
            Return,                                                                // 10
            // GO_LEFT (11)
            Move8 { dst: FO(idx), src: FO(node_left) },                           // 11
            Jump { target: CO(4) },                                                // 12
            // GO_RIGHT (13)
            Move8 { dst: FO(idx), src: FO(node_right) },                          // 13
            Jump { target: CO(4) },                                                // 14
            // NONE (15)
            StoreImm8 { dst: FO(tag), imm: 0 },                                   // 15
            StoreImm8 { dst: FO(value), imm: 0 },                                 // 16
            Return,                                                                // 17
        ];

        let code = arena.alloc_slice_fill_iter(code);
        let pointer_offsets =
            arena.alloc_slice_fill_iter(vec![FO(bst), FO(bst_ref), FO(nodes_ref)]);
        arena.alloc(Function {
            name: GlobalArenaPtr::from_static("get"),
            code,
            args_size: 16,
            args_and_locals_size: 96,
            extended_frame_size: 96 + FRAME_METADATA_SIZE,
            zero_frame: false,
            pointer_offsets,
        })
    }

    // =================================================================
    // Function 1 — insert(&mut bst, key, value)
    //
    // Mirrors the Rust insert method: each branch (root-null, left-null,
    // right-null, equal) is self-contained with its own early return.
    //
    // Frame layout:
    //   [0]  bst (ptr)   [8] key   [16] value
    //   [24] bst_ref (16B fat ptr)   [40] nodes_ref (16B fat ptr)
    //   [56] root   [64] idx
    //   [72] node (32B: key[72] val[80] left[88] right[96])
    //   [104] metadata (24B)
    //   [128] callee: bst  [136] callee: key  [144] callee: value
    // =================================================================
    fn make_insert(arena: &ExecutableArena, alloc_node_id: u32) -> ExecutableArenaPtr<Function> {
        let meta = FRAME_METADATA_SIZE as u32;
        let bst = 0u32;
        let key = 8u32;
        let value = 16u32;
        let bst_ref = 24u32;
        let nodes_ref = 40u32;
        let root = 56u32;
        let idx = 64u32;
        let node = 72u32;
        let node_key = 72u32;
        let node_val = 80u32;
        let node_left = 88u32;
        let node_right = 96u32;
        let args_and_locals_size = 104u32;
        let c0 = args_and_locals_size + meta; // 128
        let c1 = c0 + 8; // 136
        let c2 = c1 + 8; // 144

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue: borrow struct fields --
            SlotBorrow { dst: FO(bst_ref), local: FO(bst) },                      // 0
            Op::struct_borrow(FO(bst_ref), BST_NODES, FO(nodes_ref)),              // 1
            Op::struct_load8(FO(bst), BST_ROOT, FO(root)),                         // 2
            // -- if root != NULL → LOOP_SETUP; else fall through to INSERT_ROOT --
            JumpLessU64Imm { target: CO(10),
                             src: FO(root), imm: NULL },                           // 3: → LOOP_SETUP
            // -- INSERT_ROOT (4): bst.root = alloc_node(bst, key, value); return --
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 4
            Move8 { dst: FO(c1), src: FO(key) },                                  // 5
            Move8 { dst: FO(c2), src: FO(value) },                                // 6
            CallFunc { func_id: alloc_node_id },                                   // 7
            Op::struct_store8(FO(bst), BST_ROOT, FO(c0)),                          // 8
            Return,                                                                // 9
            // -- LOOP_SETUP (10) --
            Move8 { dst: FO(idx), src: FO(root) },                                // 10
            // -- LOOP (11): load node, 3-way compare --
            VecLoadElem { dst: FO(node), vec_ref: FO(nodes_ref),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 11
            // -- key < node_key? (GO_LEFT falls through, skip if >=) --
            JumpGreaterEqualU64 { target: CO(23),
                                  lhs: FO(key), rhs: FO(node_key) },              // 12: key >= node_key → NOT_LESS
            // GO_LEFT (13): key < node_key
            JumpLessU64Imm { target: CO(21),
                             src: FO(node_left), imm: NULL },                     // 13: left != NULL → CONTINUE_LEFT
            // INSERT_LEFT (14): node.left = alloc_node(...); store node; return
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 14
            Move8 { dst: FO(c1), src: FO(key) },                                  // 15
            Move8 { dst: FO(c2), src: FO(value) },                                // 16
            CallFunc { func_id: alloc_node_id },                                   // 17
            Move8 { dst: FO(node_left), src: FO(c0) },                            // 18
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(idx),
                           src: FO(node), elem_size: NODE_SIZE },                  // 19
            Return,                                                                // 20
            // CONTINUE_LEFT (21)
            Move8 { dst: FO(idx), src: FO(node_left) },                           // 21
            Jump { target: CO(11) },                                               // 22
            // -- NOT_LESS (23): key > node_key? --
            JumpGreaterEqualU64 { target: CO(34),
                                  lhs: FO(node_key), rhs: FO(key) },              // 23: node_key >= key → EQUAL
            // GO_RIGHT (24): key > node_key
            JumpLessU64Imm { target: CO(32),
                             src: FO(node_right), imm: NULL },                    // 24: right != NULL → CONTINUE_RIGHT
            // INSERT_RIGHT (25): node.right = alloc_node(...); store node; return
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 25
            Move8 { dst: FO(c1), src: FO(key) },                                  // 26
            Move8 { dst: FO(c2), src: FO(value) },                                // 27
            CallFunc { func_id: alloc_node_id },                                   // 28
            Move8 { dst: FO(node_right), src: FO(c0) },                           // 29
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(idx),
                           src: FO(node), elem_size: NODE_SIZE },                  // 30
            Return,                                                                // 31
            // CONTINUE_RIGHT (32)
            Move8 { dst: FO(idx), src: FO(node_right) },                          // 32
            Jump { target: CO(11) },                                               // 33
            // -- EQUAL (34): node.value = value; store node; return --
            Move8 { dst: FO(node_val), src: FO(value) },                           // 34
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(idx),
                           src: FO(node), elem_size: NODE_SIZE },                  // 35
            Return,                                                                // 36
        ];

        let code = arena.alloc_slice_fill_iter(code);
        let pointer_offsets =
            arena.alloc_slice_fill_iter(vec![FO(bst), FO(bst_ref), FO(nodes_ref)]);
        arena.alloc(Function {
            name: GlobalArenaPtr::from_static("insert"),
            code,
            args_size: 24,
            args_and_locals_size: args_and_locals_size as usize,
            extended_frame_size: (c2 + 8) as usize,
            zero_frame: true,
            pointer_offsets,
        })
    }

    // =================================================================
    // Function 3 — alloc_node(&mut bst, key, value) → idx
    //
    // Borrows nodes/free_list from the BstMap struct via fat pointer
    // references. If free_list is non-empty, pops an index and
    // overwrites that slot. Otherwise appends to nodes. VecPushBack
    // writes updated pointers back through the references.
    //
    // Frame layout:
    //   [0]  bst (ptr) / result: idx   [8] key   [16] value
    //   [24] bst_ref (16B fat ptr)   [40] nodes_ref (16B fat ptr)
    //   [56] free_list_ref (16B fat ptr)
    //   [72] idx   [80] fl_len
    //   [88] new_node (32B: key[88] val[96] left[104] right[112])
    // =================================================================
    fn make_alloc_node(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
        let bst = 0u32;
        let key = 8u32;
        let value = 16u32;
        let bst_ref = 24u32;
        let nodes_ref = 40u32;
        let free_list_ref = 56u32;
        let idx = 72u32;
        let fl_len = 80u32;
        let new_node = 88u32;
        let new_node_key = 88u32;
        let new_node_val = 96u32;
        let new_node_left = 104u32;
        let new_node_right = 112u32;
        let result = 0u32;

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue: borrow struct fields --
            SlotBorrow { dst: FO(bst_ref), local: FO(bst) },                      // 0
            Op::struct_borrow(FO(bst_ref), BST_NODES, FO(nodes_ref)),              // 1
            Op::struct_borrow(FO(bst_ref), BST_FREE_LIST, FO(free_list_ref)),      // 2
            // Build new_node = { key, value, NULL, NULL }
            Move8 { dst: FO(new_node_key), src: FO(key) },                        // 3
            Move8 { dst: FO(new_node_val), src: FO(value) },                      // 4
            StoreImm8 { dst: FO(new_node_left), imm: NULL },                      // 5
            StoreImm8 { dst: FO(new_node_right), imm: NULL },                     // 6
            // Check free_list
            VecLen { dst: FO(fl_len), vec_ref: FO(free_list_ref) },                // 7
            JumpNotZeroU64 { target: CO(12), src: FO(fl_len) },                    // 8: → POP
            // PUSH path: idx = nodes.len(); nodes.push(new_node)
            VecLen { dst: FO(idx), vec_ref: FO(nodes_ref) },                       // 9
            VecPushBack { vec_ref: FO(nodes_ref), elem: FO(new_node),
                          elem_size: NODE_SIZE, descriptor_id: DESC_TRIVIAL },     // 10
            Jump { target: CO(14) },                                               // 11: → DONE
            // POP path (12): idx = free_list.pop(); nodes[idx] = new_node
            VecPopBack { dst: FO(idx), vec_ref: FO(free_list_ref),
                         elem_size: 8 },                                           // 12
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(idx),
                           src: FO(new_node), elem_size: NODE_SIZE },              // 13
            // DONE (14)
            Move8 { dst: FO(result), src: FO(idx) },                               // 14
            Return,                                                                // 15
        ];

        let code = arena.alloc_slice_fill_iter(code);
        let pointer_offsets = arena.alloc_slice_fill_iter(vec![
            FO(bst),
            FO(bst_ref),
            FO(nodes_ref),
            FO(free_list_ref),
        ]);
        arena.alloc(Function {
            name: GlobalArenaPtr::from_static("alloc_node"),
            code,
            args_size: 24,
            args_and_locals_size: 120,
            extended_frame_size: 120 + FRAME_METADATA_SIZE,
            zero_frame: true,
            pointer_offsets,
        })
    }

    // =================================================================
    // Function 4 — remove(&mut bst, key)
    //
    // Searches for `key`, calls remove_node to detach the node, then
    // fixes up the parent pointer (or root).
    //
    // Frame layout:
    //   [0]  bst (ptr)   [8] key
    //   [16] bst_ref (16B fat ptr)   [32] nodes_ref (16B fat ptr)
    //   [48] root   [56] parent   [64] idx
    //   [72] node (32B: key[72] val[80] left[88] right[96])
    //   [104] metadata (24B)
    //   [128] callee: bst / result  [136] callee: idx
    // =================================================================
    fn make_remove(arena: &ExecutableArena, remove_node_id: u32) -> ExecutableArenaPtr<Function> {
        let meta = FRAME_METADATA_SIZE as u32;
        let bst = 0u32;
        let key = 8u32;
        let bst_ref = 16u32;
        let nodes_ref = 32u32;
        let root = 48u32;
        let parent = 56u32;
        let idx = 64u32;
        let node = 72u32;
        let node_key = 72u32;
        let node_left = 88u32;
        let node_right = 96u32;
        let args_and_locals_size = 104u32;
        let c0 = args_and_locals_size + meta; // 128 — also holds replacement after CallFunc
        let c1 = c0 + 8; // 136

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue --
            SlotBorrow { dst: FO(bst_ref), local: FO(bst) },                      // 0
            Op::struct_borrow(FO(bst_ref), BST_NODES, FO(nodes_ref)),              // 1
            Op::struct_load8(FO(bst), BST_ROOT, FO(root)),                         // 2
            StoreImm8 { dst: FO(parent), imm: NULL },                              // 3
            Move8 { dst: FO(idx), src: FO(root) },                                // 4
            // -- LOOP (5): while idx != NULL --
            JumpGreaterEqualU64Imm { target: CO(27),
                                     src: FO(idx), imm: NULL },                   // 5: → DONE
            VecLoadElem { dst: FO(node), vec_ref: FO(nodes_ref),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 6
            // -- key < node_key? --
            JumpGreaterEqualU64 { target: CO(11),
                                  lhs: FO(key), rhs: FO(node_key) },              // 7: key >= → NOT_LESS
            // GO_LEFT: key < node_key
            Move8 { dst: FO(parent), src: FO(idx) },                              // 8
            Move8 { dst: FO(idx), src: FO(node_left) },                           // 9
            Jump { target: CO(5) },                                                // 10
            // -- NOT_LESS (11): key > node_key? --
            JumpGreaterEqualU64 { target: CO(15),
                                  lhs: FO(node_key), rhs: FO(key) },              // 11: node_key >= key → EQUAL
            // GO_RIGHT: key > node_key
            Move8 { dst: FO(parent), src: FO(idx) },                              // 12
            Move8 { dst: FO(idx), src: FO(node_right) },                          // 13
            Jump { target: CO(5) },                                                // 14
            // -- EQUAL (15): remove_node(bst, idx) → c0 holds replacement --
            Move8 { dst: FO(c0), src: FO(bst) },                                  // 15
            Move8 { dst: FO(c1), src: FO(idx) },                                  // 16
            CallFunc { func_id: remove_node_id },                                  // 17
            // -- if parent != NULL → HAS_PARENT --
            JumpLessU64Imm { target: CO(21),
                             src: FO(parent), imm: NULL },                        // 18: → HAS_PARENT
            // UPDATE_ROOT: parent == NULL, bst.root = replacement (c0)
            Op::struct_store8(FO(bst), BST_ROOT, FO(c0)),                          // 19
            Return,                                                                // 20
            // -- HAS_PARENT (21) --
            VecLoadElem { dst: FO(node), vec_ref: FO(nodes_ref),
                          idx: FO(parent), elem_size: NODE_SIZE },                 // 21
            JumpNotEqualU64 { target: CO(25),
                              lhs: FO(node_left), rhs: FO(idx) },                 // 22: → UPDATE_RIGHT
            // UPDATE_LEFT: parent.left = replacement (c0)
            Move8 { dst: FO(node_left), src: FO(c0) },                            // 23
            Jump { target: CO(26) },                                               // 24: → STORE_PARENT
            // UPDATE_RIGHT (25): parent.right = replacement (c0)
            Move8 { dst: FO(node_right), src: FO(c0) },                           // 25
            // -- STORE_PARENT (26): shared epilogue --
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(parent),
                           src: FO(node), elem_size: NODE_SIZE },                  // 26
            Return,                                                                // 27
        ];

        let code = arena.alloc_slice_fill_iter(code);
        let pointer_offsets =
            arena.alloc_slice_fill_iter(vec![FO(bst), FO(bst_ref), FO(nodes_ref)]);
        arena.alloc(Function {
            name: GlobalArenaPtr::from_static("remove"),
            code,
            args_size: 16,
            args_and_locals_size: args_and_locals_size as usize,
            extended_frame_size: (c1 + 8) as usize,
            zero_frame: true,
            pointer_offsets,
        })
    }

    // =================================================================
    // Function 5 — remove_node(&mut bst, idx) → replacement
    //
    // Detaches the node at `idx` from the tree, pushes it to the free
    // list, and returns the index of its replacement (or NULL).
    // Uses fat pointer references into the BstMap struct so that
    // VecPushBack can write back through the reference.
    //
    // Frame layout:
    //   [0]  bst (ptr) / result   [8] idx
    //   [16] bst_ref (16B fat ptr)   [32] nodes_ref (16B fat ptr)
    //   [48] free_list_ref (16B fat ptr)
    //   [64] left   [72] right
    //   [80] parent   [88] cur   [96] cur_right
    //   [104] scratch (32B: key[104] val[112] left[120] right[128])
    // =================================================================
    fn make_remove_node(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
        let bst = 0u32;
        let idx = 8u32;
        let bst_ref = 16u32;
        let nodes_ref = 32u32;
        let free_list_ref = 48u32;
        let left = 64u32;
        let right = 72u32;
        let parent = 80u32;
        let cur = 88u32;
        let cur_right = 96u32;
        let scratch = 104u32;
        let scratch_left = 120u32; // scratch + 16
        let scratch_right = 128u32; // scratch + 24
        let result = 0u32;

        #[rustfmt::skip]
        let code = vec![
            // -- Prologue --
            SlotBorrow { dst: FO(bst_ref), local: FO(bst) },                      // 0
            Op::struct_borrow(FO(bst_ref), BST_NODES, FO(nodes_ref)),              // 1
            Op::struct_borrow(FO(bst_ref), BST_FREE_LIST, FO(free_list_ref)),      // 2
            VecLoadElem { dst: FO(scratch), vec_ref: FO(nodes_ref),
                          idx: FO(idx), elem_size: NODE_SIZE },                    // 3
            Move8 { dst: FO(left), src: FO(scratch_left) },                        // 4
            Move8 { dst: FO(right), src: FO(scratch_right) },                      // 5
            // -- if left == NULL: free idx, return right --
            JumpLessU64Imm { target: CO(9),
                             src: FO(left), imm: NULL },                           // 6: left != NULL → HAS_LEFT
            Move8 { dst: FO(result), src: FO(right) },                            // 7
            Jump { target: CO(33) },                                               // 8: → FREE_RETURN
            // -- HAS_LEFT (9): if right == NULL: free idx, return left --
            JumpLessU64Imm { target: CO(12),
                             src: FO(right), imm: NULL },                          // 9: right != NULL → HAS_BOTH
            Move8 { dst: FO(result), src: FO(left) },                             // 10
            Jump { target: CO(33) },                                               // 11: → FREE_RETURN
            // -- HAS_BOTH (12): load nodes[right], check right.left --
            VecLoadElem { dst: FO(scratch), vec_ref: FO(nodes_ref),
                          idx: FO(right), elem_size: NODE_SIZE },                  // 12
            JumpLessU64Imm { target: CO(18),
                             src: FO(scratch_left), imm: NULL },                   // 13: right.left != NULL → DEEP
            // SIMPLE_SUCCESSOR (14): right.left == NULL, right adopts left
            Move8 { dst: FO(scratch_left), src: FO(left) },                       // 14
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(right),
                           src: FO(scratch), elem_size: NODE_SIZE },               // 15
            Move8 { dst: FO(result), src: FO(right) },                            // 16
            Jump { target: CO(33) },                                               // 17: → FREE_RETURN
            // -- DEEP_SUCCESSOR (18): walk left to find in-order successor --
            Move8 { dst: FO(parent), src: FO(right) },                            // 18
            Move8 { dst: FO(cur), src: FO(scratch_left) },                        // 19
            // WALK_LOOP (20)
            VecLoadElem { dst: FO(scratch), vec_ref: FO(nodes_ref),
                          idx: FO(cur), elem_size: NODE_SIZE },                    // 20
            JumpGreaterEqualU64Imm { target: CO(25),
                                     src: FO(scratch_left), imm: NULL },           // 21: cur.left == NULL → WALK_DONE
            Move8 { dst: FO(parent), src: FO(cur) },                              // 22
            Move8 { dst: FO(cur), src: FO(scratch_left) },                        // 23
            Jump { target: CO(20) },                                               // 24
            // -- WALK_DONE (25): scratch = nodes[cur] --
            // Save cur.right, then set cur's children to left/right from removed node
            Move8 { dst: FO(cur_right), src: FO(scratch_right) },                 // 25
            Move8 { dst: FO(scratch_left), src: FO(left) },                       // 26: cur.left = left
            Move8 { dst: FO(scratch_right), src: FO(right) },                     // 27: cur.right = right
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(cur),
                           src: FO(scratch), elem_size: NODE_SIZE },               // 28
            // Detach cur from parent: parent.left = cur_right
            VecLoadElem { dst: FO(scratch), vec_ref: FO(nodes_ref),
                          idx: FO(parent), elem_size: NODE_SIZE },                 // 29
            Move8 { dst: FO(scratch_left), src: FO(cur_right) },                  // 30
            VecStoreElem { vec_ref: FO(nodes_ref), idx: FO(parent),
                           src: FO(scratch), elem_size: NODE_SIZE },               // 31
            Move8 { dst: FO(result), src: FO(cur) },                              // 32
            // -- FREE_RETURN (33): free_list.push(idx); return --
            VecPushBack { vec_ref: FO(free_list_ref), elem: FO(idx),
                          elem_size: 8, descriptor_id: DESC_TRIVIAL },             // 33
            Return,                                                                // 34
        ];

        let code = arena.alloc_slice_fill_iter(code);
        let pointer_offsets = arena.alloc_slice_fill_iter(vec![
            FO(bst),
            FO(bst_ref),
            FO(nodes_ref),
            FO(free_list_ref),
        ]);
        arena.alloc(Function {
            name: GlobalArenaPtr::from_static("remove_node"),
            code,
            args_size: 16,
            args_and_locals_size: 136,
            extended_frame_size: 136 + FRAME_METADATA_SIZE,
            zero_frame: true,
            pointer_offsets,
        })
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
    //   [56] ops_ref (16B fat ptr)
    //   [72] metadata (24 bytes)
    //   [96] c0   [104] c1   [112] c2
    // =================================================================
    fn make_run_ops(arena: &ExecutableArena) -> ExecutableArenaPtr<Function> {
        let meta = FRAME_METADATA_SIZE as u32;
        let ops = 0u32;
        let bst = 8u32;
        let len = 16u32;
        let i = 24u32;
        let op_code = 32u32;
        let key = 40u32;
        let value = 48u32;
        let ops_ref = 56u32;
        let args_and_locals_size = 72u32;
        let c0 = args_and_locals_size + meta; // 96
        let c1 = c0 + 8; // 104
        let c2 = c1 + 8; // 112

        #[rustfmt::skip]
        let code = vec![
            // -- Create BST --
            CallFunc { func_id: FN_NEW as u32 },                                  // 0
            Move8 { dst: FO(bst), src: FO(c0) },                                 // 1
            // -- Init loop --
            SlotBorrow { dst: FO(ops_ref), local: FO(ops) },                      // 2
            VecLen { dst: FO(len), vec_ref: FO(ops_ref) },                        // 3
            StoreImm8 { dst: FO(i), imm: 0 },                                    // 4
            // -- LOOP (5) --
            JumpGreaterEqualU64 { target: CO(28),
                                  lhs: FO(i), rhs: FO(len) },                    // 5: → DONE
            // Load triple
            VecLoadElem { dst: FO(op_code), vec_ref: FO(ops_ref),
                          idx: FO(i), elem_size: 8 },                            // 6
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                       // 7
            VecLoadElem { dst: FO(key), vec_ref: FO(ops_ref),
                          idx: FO(i), elem_size: 8 },                            // 8
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                       // 9
            VecLoadElem { dst: FO(value), vec_ref: FO(ops_ref),
                          idx: FO(i), elem_size: 8 },                            // 10
            AddU64Imm { dst: FO(i), src: FO(i), imm: 1 },                       // 11
            // -- Dispatch --
            JumpNotZeroU64 { target: CO(18), src: FO(op_code) },                 // 12: → CHECK_GET
            // INSERT (13)
            Move8 { dst: FO(c0), src: FO(bst) },                                 // 13
            Move8 { dst: FO(c1), src: FO(key) },                                 // 14
            Move8 { dst: FO(c2), src: FO(value) },                               // 15
            CallFunc { func_id: FN_INSERT as u32 },                               // 16
            Jump { target: CO(5) },                                               // 17
            // CHECK_GET (18)
            JumpGreaterEqualU64Imm { target: CO(24),
                                      src: FO(op_code), imm: 2 },               // 18: → REMOVE
            // GET (19)
            Move8 { dst: FO(c0), src: FO(bst) },                                 // 19
            Move8 { dst: FO(c1), src: FO(key) },                                 // 20
            CallFunc { func_id: FN_GET as u32 },                                  // 21
            Jump { target: CO(5) },                                               // 22
            // skip (23) — padding for alignment, shouldn't be reached
            Jump { target: CO(5) },                                               // 23
            // REMOVE (24)
            Move8 { dst: FO(c0), src: FO(bst) },                                 // 24
            Move8 { dst: FO(c1), src: FO(key) },                                 // 25
            CallFunc { func_id: FN_REMOVE as u32 },                               // 26
            Jump { target: CO(5) },                                               // 27
            // DONE (28)
            Return,                                                               // 28
        ];

        let code = arena.alloc_slice_fill_iter(code);
        let pointer_offsets = arena.alloc_slice_fill_iter(vec![FO(ops), FO(bst), FO(ops_ref)]);
        arena.alloc(Function {
            name: GlobalArenaPtr::from_static("run_ops"),
            code,
            args_size: 8,
            args_and_locals_size: args_and_locals_size as usize,
            extended_frame_size: (c2 + 8) as usize,
            zero_frame: true,
            pointer_offsets,
        })
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
