// Vector-arena binary search tree map (u64 -> u64). `run_ops_checksum`
// generates a deterministic LCG op stream and folds the `get` results into a
// checksum, returning a single primitive.

// RUN: publish
module 0x1::bst {
    use std::vector;

    const NULL: u64 = 18446744073709551615; // u64::MAX

    // LCG parameters; mirrored by `lcg_next`/`LCG_MOD` in src/programs/mod.rs.
    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;
    // Checksum-fold modulus; mirrored by `CHECKSUM_MOD` in src/programs/bst.rs.
    const CHECKSUM_MOD: u64 = 1000000007;

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
        // `length(..) != 0` rather than `vector::is_empty` — the latter is a
        // Move-implemented stdlib fn (a runtime call into 0x1::vector), whereas
        // the ops used here all lower to intrinsic vector instructions.
        if (vector::length(&bst.free_list) != 0) {
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

    /// Generate `n_ops` operations via an LCG (50% insert / 25% get / 25%
    /// remove), run them, and fold each `get` result into a checksum.
    public fun run_ops_checksum(n_ops: u64, key_range: u64, seed: u64): u64 {
        let bst = new();
        let acc = 0;
        let x = seed % LCG_MOD;
        let c = 0;
        while (c < n_ops) {
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let op = x % 4;
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let key = x % key_range;
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let value = x;
            if (op < 2) {
                insert(&mut bst, key, value);
            } else if (op == 2) {
                let (found, found_value) = get(&bst, key);
                let contribution = if (found) { found_value + 1 } else { 0 };
                acc = (acc * LCG_MOD + contribution) % CHECKSUM_MOD;
            } else {
                remove(&mut bst, key);
            };
            c = c + 1;
        };
        acc
    }
}

// RUN: execute 0x1::bst::run_ops_checksum --args 100, 50, 42
// CHECK: results: 487220948
// RUN: execute 0x1::bst::run_ops_checksum --args 500, 100, 7
// CHECK: results: 295054122
// RUN: execute 0x1::bst::run_ops_checksum --args 2000, 500, 9
// CHECK: results: 389078507
