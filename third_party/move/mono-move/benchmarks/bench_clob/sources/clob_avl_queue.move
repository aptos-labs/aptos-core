/// Bit-packed AVL queue: a self-balancing binary search tree on a 32-bit
/// insertion key, where every tree node owns a doubly linked list of
/// insertion values sharing that key.
///
/// Tree nodes and list nodes are each one `u128` in a `Table<u64, u128>`, so
/// every descent step is a table read followed by a shift and a mask to
/// recover the next node id. That is the point of the structure here: a
/// dependent load chain the VM cannot reorder or prefetch.
///
/// # Node ids
///
/// Node ids are 1-indexed and 14 bits wide, so id 0 can stand for null and at
/// most 16383 nodes of each kind can ever be allocated. Removed nodes are
/// pushed onto a free-list stack and reused rather than deallocated.
///
/// # Heights
///
/// A node stores the height of each child subtree plus one, with zero meaning
/// the child is absent. The height of a node is the larger of the two, so a
/// lone root has height zero. An AVL tree of at most 16383 nodes has height at
/// most 18, so a stored height never exceeds 18 and fits in 5 bits.
///
/// # Sort order
///
/// The tree is always ordered by ascending insertion key. The queue's sort
/// order decides only which end is the head: the leftmost tree node for an
/// ascending queue, the rightmost for a descending one. Within a tree node the
/// list always runs head to tail in insertion order, so equal keys break ties
/// by insertion count in both sort orders.
///
/// # Bit layouts
///
/// Queue header, `AvlQueue.bits`:
///
/// | Bit(s)  | Data                                        | Width |
/// |---------|---------------------------------------------|-------|
/// | 0-31    | Head insertion key                          | 32    |
/// | 32-45   | Head list node id                           | 14    |
/// | 46-77   | Tail insertion key                          | 32    |
/// | 78-91   | Tail list node id                           | 14    |
/// | 92-105  | Inactive tree node stack top                | 14    |
/// | 106-119 | Inactive list node stack top                | 14    |
/// | 120     | If set, ascending queue, else descending    | 1     |
///
/// The tree root does not fit alongside these and is a separate field.
///
/// Tree node:
///
/// | Bit(s)  | Data                                        | Width |
/// |---------|---------------------------------------------|-------|
/// | 0-31    | Insertion key                               | 32    |
/// | 32-45   | Parent node id                              | 14    |
/// | 46-59   | Left child node id                          | 14    |
/// | 60-73   | Right child node id                         | 14    |
/// | 74-87   | List head node id                           | 14    |
/// | 88-101  | List tail node id                           | 14    |
/// | 102-115 | Next inactive node id, when in stack        | 14    |
/// | 116-120 | Left height                                 | 5     |
/// | 121-125 | Right height                                | 5     |
///
/// Every field except the next inactive node id is ignored while the node sits
/// in the inactive stack.
///
/// List node:
///
/// | Bit(s)  | Data                                        | Width |
/// |---------|---------------------------------------------|-------|
/// | 0-13    | Previous node id                            | 14    |
/// | 14      | If set, previous id is a tree node id       | 1     |
/// | 15-28   | Next node id                                | 14    |
/// | 29      | If set, next id is a tree node id           | 1     |
///
/// A list node whose previous id is flagged as a tree node id is the head of
/// that tree node's list; one whose next id is flagged is the tail. That is
/// what lets a removal recover its tree node without a back pointer in every
/// list node, and what lets a walk step from one price level to the next.
///
/// Access key, returned by `insert` and consumed by `remove`:
///
/// | Bit(s)  | Data                                        | Width |
/// |---------|---------------------------------------------|-------|
/// | 0-31    | Insertion key                               | 32    |
/// | 32-45   | List node id                                | 14    |
/// | 46-59   | Tree node id                                | 14    |
/// | 60      | If set, ascending queue, else descending    | 1     |
///
/// An active list node id is never zero, so access key 0 is free to mean "no
/// such element" and is what the walk functions return at the end of the book.
///
/// Access keys are unique among live elements but not across time, since node
/// ids are reused. A caller that needs a stable identity concatenates its own
/// counter.
module bench::clob_avl_queue {
    use aptos_std::table::{Self, Table};

    /// Allocating another tree node would exceed the 14-bit id space.
    const E_TOO_MANY_TREE_NODES: u64 = 1;
    /// Allocating another list node would exceed the 14-bit id space.
    const E_TOO_MANY_LIST_NODES: u64 = 2;
    /// Insertion key does not fit in 32 bits.
    const E_INSERTION_KEY_TOO_LARGE: u64 = 3;
    /// Asked for the head of an empty queue.
    const E_EMPTY: u64 = 4;

    const ASCENDING: bool = true;
    const DESCENDING: bool = false;
    const LEFT: bool = true;
    const RIGHT: bool = false;

    /// Reserved node id standing for null.
    const NIL: u64 = 0;
    /// 14-bit node id mask.
    const HI_NODE_ID: u64 = 0x3fff;
    /// 32-bit insertion key mask.
    const HI_INSERTION_KEY: u64 = 0xffffffff;
    /// 5-bit height mask.
    const HI_HEIGHT: u64 = 0x1f;
    /// Single-bit mask, for the flag fields.
    const HI_BIT: u64 = 1;
    /// All bits of a `u128` set; xor against it to invert a mask.
    const HI_128: u128 = 0xffffffffffffffffffffffffffffffff;

    const N_NODES_MAX: u64 = 16383;
    /// Tallest an AVL tree of `N_NODES_MAX` nodes can get. Checked by the
    /// invariant assertions rather than enforced, since the node cap already
    /// bounds it.
    const MAX_HEIGHT: u64 = 18;

    // Queue header field offsets.
    const SHIFT_HEAD_KEY: u8 = 0;
    const SHIFT_HEAD_NODE: u8 = 32;
    const SHIFT_TAIL_KEY: u8 = 46;
    const SHIFT_TAIL_NODE: u8 = 78;
    const SHIFT_TREE_STACK_TOP: u8 = 92;
    const SHIFT_LIST_STACK_TOP: u8 = 106;
    const SHIFT_SORT_ORDER: u8 = 120;

    // Tree node field offsets.
    const SHIFT_KEY: u8 = 0;
    const SHIFT_PARENT: u8 = 32;
    const SHIFT_LEFT: u8 = 46;
    const SHIFT_RIGHT: u8 = 60;
    const SHIFT_LIST_HEAD: u8 = 74;
    const SHIFT_LIST_TAIL: u8 = 88;
    const SHIFT_NEXT_FREE: u8 = 102;
    const SHIFT_HEIGHT_LEFT: u8 = 116;
    const SHIFT_HEIGHT_RIGHT: u8 = 121;

    // List node field offsets.
    const SHIFT_LIST_PREV: u8 = 0;
    const SHIFT_LIST_PREV_TREE: u8 = 14;
    const SHIFT_LIST_NEXT: u8 = 15;
    const SHIFT_LIST_NEXT_TREE: u8 = 29;

    // Access key field offsets.
    const SHIFT_ACCESS_KEY: u8 = 0;
    const SHIFT_ACCESS_LIST_NODE: u8 = 32;
    const SHIFT_ACCESS_TREE_NODE: u8 = 46;
    const SHIFT_ACCESS_SORT_ORDER: u8 = 60;

    struct AvlQueue<V: store> has store {
        bits: u128,
        root: u64,
        tree_nodes: Table<u64, u128>,
        list_nodes: Table<u64, u128>,
        values: Table<u64, V>,
        /// Tree node ids ever handed out, active and inactive alike.
        n_tree_nodes: u64,
        /// List node ids ever handed out, active and inactive alike.
        n_list_nodes: u64,
    }

    // Bit field access.

    /// Read the `mask`-wide field at `shift`.
    fun get_bits(bits: u128, shift: u8, mask: u64): u64 {
        (((bits >> shift) & (mask as u128)) as u64)
    }

    /// Overwrite the `mask`-wide field at `shift`. Bits of `value` above the
    /// mask are dropped, so a field can never spill into its neighbours.
    fun set_bits(bits: u128, shift: u8, mask: u64, value: u64): u128 {
        let field = (mask as u128) << shift;
        (bits & (field ^ HI_128)) | (((value as u128) << shift) & field)
    }

    /// `get_bits` for the `u64`-wide access key.
    fun get_key_bits(bits: u64, shift: u8, mask: u64): u64 {
        (bits >> shift) & mask
    }

    fun pack_access_key(
        tree_node_id: u64, list_node_id: u64, key: u64, ascending: bool
    ): u64 {
        let sort_order = if (ascending) 1 else 0;
        ((key & HI_INSERTION_KEY) << SHIFT_ACCESS_KEY)
            | ((list_node_id & HI_NODE_ID) << SHIFT_ACCESS_LIST_NODE)
            | ((tree_node_id & HI_NODE_ID) << SHIFT_ACCESS_TREE_NODE)
            | (sort_order << SHIFT_ACCESS_SORT_ORDER)
    }

    public fun access_key_insertion_key(access_key: u64): u64 {
        get_key_bits(access_key, SHIFT_ACCESS_KEY, HI_INSERTION_KEY)
    }

    public fun access_key_list_node_id(access_key: u64): u64 {
        get_key_bits(access_key, SHIFT_ACCESS_LIST_NODE, HI_NODE_ID)
    }

    public fun access_key_tree_node_id(access_key: u64): u64 {
        get_key_bits(access_key, SHIFT_ACCESS_TREE_NODE, HI_NODE_ID)
    }

    public fun access_key_is_ascending(access_key: u64): bool {
        get_key_bits(access_key, SHIFT_ACCESS_SORT_ORDER, HI_BIT) == 1
    }

    // Construction and queries.

    public fun new<V: store>(ascending: bool): AvlQueue<V> {
        let sort_order = if (ascending) 1 else 0;
        AvlQueue {
            bits: set_bits(0, SHIFT_SORT_ORDER, HI_BIT, sort_order),
            root: NIL,
            tree_nodes: table::new(),
            list_nodes: table::new(),
            values: table::new(),
            n_tree_nodes: 0,
            n_list_nodes: 0,
        }
    }

    public fun is_ascending<V: store>(self: &AvlQueue<V>): bool {
        get_bits(self.bits, SHIFT_SORT_ORDER, HI_BIT) == 1
    }

    public fun is_empty<V: store>(self: &AvlQueue<V>): bool {
        self.root == NIL
    }

    /// Insertion key at the head, or zero when the queue is empty.
    public fun get_head_key<V: store>(self: &AvlQueue<V>): u64 {
        get_bits(self.bits, SHIFT_HEAD_KEY, HI_INSERTION_KEY)
    }

    /// Insertion key at the tail, or zero when the queue is empty.
    public fun get_tail_key<V: store>(self: &AvlQueue<V>): u64 {
        get_bits(self.bits, SHIFT_TAIL_KEY, HI_INSERTION_KEY)
    }

    /// Height of the tree. Zero both for an empty queue and for one holding a
    /// single insertion key, so callers that need to tell those apart check
    /// `is_empty` first.
    public fun get_height<V: store>(self: &AvlQueue<V>): u64 {
        if (self.root == NIL) return 0;
        let bits = *table::borrow(&self.tree_nodes, self.root);
        let left = get_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        let right = get_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
        if (left > right) left else right
    }

    public fun has_key<V: store>(self: &AvlQueue<V>, key: u64): bool {
        if (key > HI_INSERTION_KEY) return false;
        let (_, found, _) = search(self, key);
        found
    }

    public fun borrow<V: store>(self: &AvlQueue<V>, access_key: u64): &V {
        table::borrow(&self.values, access_key_list_node_id(access_key))
    }

    public fun borrow_mut<V: store>(
        self: &mut AvlQueue<V>, access_key: u64
    ): &mut V {
        table::borrow_mut(&mut self.values, access_key_list_node_id(access_key))
    }

    public fun borrow_head<V: store>(self: &AvlQueue<V>): &V {
        let list_node_id = get_bits(self.bits, SHIFT_HEAD_NODE, HI_NODE_ID);
        assert!(list_node_id != NIL, E_EMPTY);
        table::borrow(&self.values, list_node_id)
    }

    public fun borrow_head_mut<V: store>(self: &mut AvlQueue<V>): &mut V {
        let list_node_id = get_bits(self.bits, SHIFT_HEAD_NODE, HI_NODE_ID);
        assert!(list_node_id != NIL, E_EMPTY);
        table::borrow_mut(&mut self.values, list_node_id)
    }

    /// Access key of the head element, or zero when the queue is empty.
    public fun head_access_key<V: store>(self: &AvlQueue<V>): u64 {
        let list_node_id = get_bits(self.bits, SHIFT_HEAD_NODE, HI_NODE_ID);
        if (list_node_id == NIL) return 0;
        let ascending = is_ascending(self);
        let tree_node_id = extreme_tree_node(self, self.root, ascending);
        pack_access_key(
            tree_node_id,
            list_node_id,
            get_bits(self.bits, SHIFT_HEAD_KEY, HI_INSERTION_KEY),
            ascending,
        )
    }

    /// Access key of the element after `access_key` in sort order, or zero at
    /// the end of the queue.
    public fun next_access_key<V: store>(
        self: &AvlQueue<V>, access_key: u64
    ): u64 {
        let tree_node_id = access_key_tree_node_id(access_key);
        let list_node_id = access_key_list_node_id(access_key);
        let ascending = is_ascending(self);
        let list_bits = *table::borrow(&self.list_nodes, list_node_id);
        let next = get_bits(list_bits, SHIFT_LIST_NEXT, HI_NODE_ID);
        if (get_bits(list_bits, SHIFT_LIST_NEXT_TREE, HI_BIT) == 0) {
            let key = access_key_insertion_key(access_key);
            return pack_access_key(tree_node_id, next, key, ascending)
        };
        // At the list tail, so step to the neighbouring insertion key.
        let neighbour = if (ascending) {
            successor(self, tree_node_id)
        } else {
            predecessor(self, tree_node_id)
        };
        if (neighbour == NIL) return 0;
        let bits = *table::borrow(&self.tree_nodes, neighbour);
        pack_access_key(
            neighbour,
            get_bits(bits, SHIFT_LIST_HEAD, HI_NODE_ID),
            get_bits(bits, SHIFT_KEY, HI_INSERTION_KEY),
            ascending,
        )
    }

    // Insertion.

    /// Insert `value` under `key` and return its access key.
    public fun insert<V: store>(
        self: &mut AvlQueue<V>, key: u64, value: V
    ): u64 {
        assert!(key <= HI_INSERTION_KEY, E_INSERTION_KEY_TOO_LARGE);
        let (node_id, found, side) = search(self, key);
        let list_node_id = alloc_list_node(self);
        let tree_node_id;
        if (found) {
            tree_node_id = node_id;
            append_list_node(self, tree_node_id, list_node_id);
        } else {
            tree_node_id = alloc_tree_node(self);
            let bits = set_bits(0, SHIFT_KEY, HI_INSERTION_KEY, key);
            bits = set_bits(bits, SHIFT_PARENT, HI_NODE_ID, node_id);
            bits = set_bits(bits, SHIFT_LIST_HEAD, HI_NODE_ID, list_node_id);
            bits = set_bits(bits, SHIFT_LIST_TAIL, HI_NODE_ID, list_node_id);
            *table::borrow_mut(&mut self.tree_nodes, tree_node_id) = bits;
            // Sole list node, so both ends point back at the tree node.
            let list_bits =
                set_bits(0, SHIFT_LIST_PREV, HI_NODE_ID, tree_node_id);
            list_bits = set_bits(list_bits, SHIFT_LIST_PREV_TREE, HI_BIT, 1);
            list_bits =
                set_bits(list_bits, SHIFT_LIST_NEXT, HI_NODE_ID, tree_node_id);
            list_bits = set_bits(list_bits, SHIFT_LIST_NEXT_TREE, HI_BIT, 1);
            *table::borrow_mut(&mut self.list_nodes, list_node_id) = list_bits;
            hang(self, node_id, side, tree_node_id);
            retrace(self, node_id, side, true);
        };
        table::add(&mut self.values, list_node_id, value);
        insert_update_head_tail(self, key, list_node_id);
        pack_access_key(tree_node_id, list_node_id, key, is_ascending(self))
    }

    /// Descend from the root toward `key`. Returns the node the descent
    /// stopped at, whether that node holds `key`, and, when it does not, the
    /// side a node for `key` would hang on.
    fun search<V: store>(self: &AvlQueue<V>, key: u64): (u64, bool, bool) {
        let node_id = self.root;
        if (node_id == NIL) return (NIL, false, LEFT);
        loop {
            let bits = *table::borrow(&self.tree_nodes, node_id);
            let node_key = get_bits(bits, SHIFT_KEY, HI_INSERTION_KEY);
            if (key == node_key) return (node_id, true, LEFT);
            let side = if (key < node_key) LEFT else RIGHT;
            let shift = if (side == LEFT) SHIFT_LEFT else SHIFT_RIGHT;
            let child = get_bits(bits, shift, HI_NODE_ID);
            if (child == NIL) return (node_id, false, side);
            node_id = child;
        }
    }

    /// Append `list_node_id` to the tail of the list at `tree_node_id`.
    fun append_list_node<V: store>(
        self: &mut AvlQueue<V>, tree_node_id: u64, list_node_id: u64
    ) {
        let tree_bits = *table::borrow(&self.tree_nodes, tree_node_id);
        let old_tail = get_bits(tree_bits, SHIFT_LIST_TAIL, HI_NODE_ID);
        let bits = set_bits(0, SHIFT_LIST_PREV, HI_NODE_ID, old_tail);
        bits = set_bits(bits, SHIFT_LIST_NEXT, HI_NODE_ID, tree_node_id);
        bits = set_bits(bits, SHIFT_LIST_NEXT_TREE, HI_BIT, 1);
        *table::borrow_mut(&mut self.list_nodes, list_node_id) = bits;
        let tail_bits = *table::borrow(&self.list_nodes, old_tail);
        tail_bits =
            set_bits(tail_bits, SHIFT_LIST_NEXT, HI_NODE_ID, list_node_id);
        tail_bits = set_bits(tail_bits, SHIFT_LIST_NEXT_TREE, HI_BIT, 0);
        *table::borrow_mut(&mut self.list_nodes, old_tail) = tail_bits;
        *table::borrow_mut(&mut self.tree_nodes, tree_node_id) =
            set_bits(tree_bits, SHIFT_LIST_TAIL, HI_NODE_ID, list_node_id);
    }

    fun insert_update_head_tail<V: store>(
        self: &mut AvlQueue<V>, key: u64, list_node_id: u64
    ) {
        if (get_bits(self.bits, SHIFT_HEAD_NODE, HI_NODE_ID) == NIL) {
            set_head(self, key, list_node_id);
            set_tail(self, key, list_node_id);
            return
        };
        let ascending = is_ascending(self);
        let head_key = get_bits(self.bits, SHIFT_HEAD_KEY, HI_INSERTION_KEY);
        let beats_head =
            if (ascending) key < head_key else key > head_key;
        if (beats_head) set_head(self, key, list_node_id);
        // Ties go to the tail: within an insertion key the newest element is
        // last in sort order.
        let tail_key = get_bits(self.bits, SHIFT_TAIL_KEY, HI_INSERTION_KEY);
        let beats_tail =
            if (ascending) key >= tail_key else key <= tail_key;
        if (beats_tail) set_tail(self, key, list_node_id);
    }

    fun set_head<V: store>(
        self: &mut AvlQueue<V>, key: u64, list_node_id: u64
    ) {
        let bits = set_bits(self.bits, SHIFT_HEAD_KEY, HI_INSERTION_KEY, key);
        self.bits = set_bits(bits, SHIFT_HEAD_NODE, HI_NODE_ID, list_node_id);
    }

    fun set_tail<V: store>(
        self: &mut AvlQueue<V>, key: u64, list_node_id: u64
    ) {
        let bits = set_bits(self.bits, SHIFT_TAIL_KEY, HI_INSERTION_KEY, key);
        self.bits = set_bits(bits, SHIFT_TAIL_NODE, HI_NODE_ID, list_node_id);
    }

    // Removal.

    /// Remove and return the value at `access_key`.
    public fun remove<V: store>(self: &mut AvlQueue<V>, access_key: u64): V {
        remove_inner(
            self,
            access_key_tree_node_id(access_key),
            access_key_list_node_id(access_key),
        )
    }

    /// Remove and return the value at the head of the queue.
    public fun pop_head<V: store>(self: &mut AvlQueue<V>): V {
        let list_node_id = get_bits(self.bits, SHIFT_HEAD_NODE, HI_NODE_ID);
        assert!(list_node_id != NIL, E_EMPTY);
        let tree_node_id =
            extreme_tree_node(self, self.root, is_ascending(self));
        remove_inner(self, tree_node_id, list_node_id)
    }

    fun remove_inner<V: store>(
        self: &mut AvlQueue<V>, tree_node_id: u64, list_node_id: u64
    ): V {
        let value = table::remove(&mut self.values, list_node_id);
        let was_head =
            get_bits(self.bits, SHIFT_HEAD_NODE, HI_NODE_ID) == list_node_id;
        let was_tail =
            get_bits(self.bits, SHIFT_TAIL_NODE, HI_NODE_ID) == list_node_id;
        if (remove_list_node(self, list_node_id)) {
            remove_tree_node(self, tree_node_id);
        };
        free_list_node(self, list_node_id);
        if (was_head) remove_update_head(self);
        if (was_tail) remove_update_tail(self);
        value
    }

    /// Unlink `list_node_id` from its list. Returns true when it was the only
    /// node there, leaving its tree node empty.
    fun remove_list_node<V: store>(
        self: &mut AvlQueue<V>, list_node_id: u64
    ): bool {
        let bits = *table::borrow(&self.list_nodes, list_node_id);
        let prev = get_bits(bits, SHIFT_LIST_PREV, HI_NODE_ID);
        let next = get_bits(bits, SHIFT_LIST_NEXT, HI_NODE_ID);
        let prev_is_tree =
            get_bits(bits, SHIFT_LIST_PREV_TREE, HI_BIT) == 1;
        let next_is_tree =
            get_bits(bits, SHIFT_LIST_NEXT_TREE, HI_BIT) == 1;
        if (prev_is_tree && next_is_tree) return true;
        if (prev_is_tree) {
            let tree_bits = *table::borrow(&self.tree_nodes, prev);
            *table::borrow_mut(&mut self.tree_nodes, prev) =
                set_bits(tree_bits, SHIFT_LIST_HEAD, HI_NODE_ID, next);
            let next_bits = *table::borrow(&self.list_nodes, next);
            next_bits =
                set_bits(next_bits, SHIFT_LIST_PREV, HI_NODE_ID, prev);
            *table::borrow_mut(&mut self.list_nodes, next) =
                set_bits(next_bits, SHIFT_LIST_PREV_TREE, HI_BIT, 1);
        } else if (next_is_tree) {
            let tree_bits = *table::borrow(&self.tree_nodes, next);
            *table::borrow_mut(&mut self.tree_nodes, next) =
                set_bits(tree_bits, SHIFT_LIST_TAIL, HI_NODE_ID, prev);
            let prev_bits = *table::borrow(&self.list_nodes, prev);
            prev_bits =
                set_bits(prev_bits, SHIFT_LIST_NEXT, HI_NODE_ID, next);
            *table::borrow_mut(&mut self.list_nodes, prev) =
                set_bits(prev_bits, SHIFT_LIST_NEXT_TREE, HI_BIT, 1);
        } else {
            let prev_bits = *table::borrow(&self.list_nodes, prev);
            *table::borrow_mut(&mut self.list_nodes, prev) =
                set_bits(prev_bits, SHIFT_LIST_NEXT, HI_NODE_ID, next);
            let next_bits = *table::borrow(&self.list_nodes, next);
            *table::borrow_mut(&mut self.list_nodes, next) =
                set_bits(next_bits, SHIFT_LIST_PREV, HI_NODE_ID, prev);
        };
        false
    }

    /// Splice `node_id` out of the tree and retrace from where the height
    /// dropped.
    fun remove_tree_node<V: store>(self: &mut AvlQueue<V>, node_id: u64) {
        let bits = *table::borrow(&self.tree_nodes, node_id);
        let left = get_bits(bits, SHIFT_LEFT, HI_NODE_ID);
        let right = get_bits(bits, SHIFT_RIGHT, HI_NODE_ID);
        let parent = get_bits(bits, SHIFT_PARENT, HI_NODE_ID);
        // Read the side before any link is rewritten.
        let side = if (parent == NIL) LEFT else side_of(self, parent, node_id);
        if (left == NIL || right == NIL) {
            let child = if (left == NIL) right else left;
            hang(self, parent, side, child);
            if (child != NIL) set_parent(self, child, parent);
            retrace(self, parent, side, false);
        } else {
            // Two children: the in-order successor takes this node's place.
            let successor_id = right;
            let s_bits = *table::borrow(&self.tree_nodes, successor_id);
            let s_left = get_bits(s_bits, SHIFT_LEFT, HI_NODE_ID);
            while (s_left != NIL) {
                successor_id = s_left;
                s_bits = *table::borrow(&self.tree_nodes, successor_id);
                s_left = get_bits(s_bits, SHIFT_LEFT, HI_NODE_ID);
            };
            let retrace_from;
            let retrace_side;
            if (successor_id == right) {
                // The successor keeps its own right subtree, which just moved
                // up a level.
                retrace_from = successor_id;
                retrace_side = RIGHT;
            } else {
                let s_parent = get_bits(s_bits, SHIFT_PARENT, HI_NODE_ID);
                let s_right = get_bits(s_bits, SHIFT_RIGHT, HI_NODE_ID);
                hang(self, s_parent, LEFT, s_right);
                if (s_right != NIL) set_parent(self, s_right, s_parent);
                s_bits = set_bits(s_bits, SHIFT_RIGHT, HI_NODE_ID, right);
                set_parent(self, right, successor_id);
                retrace_from = s_parent;
                retrace_side = LEFT;
            };
            s_bits = set_bits(s_bits, SHIFT_LEFT, HI_NODE_ID, left);
            s_bits = set_bits(s_bits, SHIFT_PARENT, HI_NODE_ID, parent);
            s_bits = set_bits(
                s_bits,
                SHIFT_HEIGHT_LEFT,
                HI_HEIGHT,
                get_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT),
            );
            s_bits = set_bits(
                s_bits,
                SHIFT_HEIGHT_RIGHT,
                HI_HEIGHT,
                get_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT),
            );
            *table::borrow_mut(&mut self.tree_nodes, successor_id) = s_bits;
            set_parent(self, left, successor_id);
            hang(self, parent, side, successor_id);
            retrace(self, retrace_from, retrace_side, false);
        };
        free_tree_node(self, node_id);
    }

    /// Re-derive the head from the tree, after the old head was removed.
    fun remove_update_head<V: store>(self: &mut AvlQueue<V>) {
        if (self.root == NIL) {
            set_head(self, 0, NIL);
            return
        };
        let node_id = extreme_tree_node(self, self.root, is_ascending(self));
        let bits = *table::borrow(&self.tree_nodes, node_id);
        set_head(
            self,
            get_bits(bits, SHIFT_KEY, HI_INSERTION_KEY),
            get_bits(bits, SHIFT_LIST_HEAD, HI_NODE_ID),
        );
    }

    /// Re-derive the tail from the tree, after the old tail was removed.
    fun remove_update_tail<V: store>(self: &mut AvlQueue<V>) {
        if (self.root == NIL) {
            set_tail(self, 0, NIL);
            return
        };
        let node_id = extreme_tree_node(self, self.root, !is_ascending(self));
        let bits = *table::borrow(&self.tree_nodes, node_id);
        set_tail(
            self,
            get_bits(bits, SHIFT_KEY, HI_INSERTION_KEY),
            get_bits(bits, SHIFT_LIST_TAIL, HI_NODE_ID),
        );
    }

    // Tree structure helpers.

    /// Point `parent`'s child link on `side` at `node_id`, or set the root
    /// when there is no parent.
    fun hang<V: store>(
        self: &mut AvlQueue<V>, parent: u64, side: bool, node_id: u64
    ) {
        if (parent == NIL) {
            self.root = node_id;
        } else {
            let bits = *table::borrow(&self.tree_nodes, parent);
            let shift = if (side == LEFT) SHIFT_LEFT else SHIFT_RIGHT;
            *table::borrow_mut(&mut self.tree_nodes, parent) =
                set_bits(bits, shift, HI_NODE_ID, node_id);
        }
    }

    fun set_parent<V: store>(
        self: &mut AvlQueue<V>, node_id: u64, parent: u64
    ) {
        let bits = *table::borrow(&self.tree_nodes, node_id);
        *table::borrow_mut(&mut self.tree_nodes, node_id) =
            set_bits(bits, SHIFT_PARENT, HI_NODE_ID, parent);
    }

    fun side_of<V: store>(
        self: &AvlQueue<V>, parent: u64, node_id: u64
    ): bool {
        let bits = *table::borrow(&self.tree_nodes, parent);
        if (get_bits(bits, SHIFT_LEFT, HI_NODE_ID) == node_id) LEFT else RIGHT
    }

    /// Walk `node_id` all the way left when `leftward`, else all the way
    /// right.
    fun extreme_tree_node<V: store>(
        self: &AvlQueue<V>, node_id: u64, leftward: bool
    ): u64 {
        let shift = if (leftward) SHIFT_LEFT else SHIFT_RIGHT;
        loop {
            let bits = *table::borrow(&self.tree_nodes, node_id);
            let child = get_bits(bits, shift, HI_NODE_ID);
            if (child == NIL) return node_id;
            node_id = child;
        }
    }

    fun successor<V: store>(self: &AvlQueue<V>, node_id: u64): u64 {
        let bits = *table::borrow(&self.tree_nodes, node_id);
        let right = get_bits(bits, SHIFT_RIGHT, HI_NODE_ID);
        if (right != NIL) return extreme_tree_node(self, right, true);
        // No right subtree, so climb until the path turns right.
        let child = node_id;
        let parent = get_bits(bits, SHIFT_PARENT, HI_NODE_ID);
        while (parent != NIL) {
            let parent_bits = *table::borrow(&self.tree_nodes, parent);
            if (get_bits(parent_bits, SHIFT_LEFT, HI_NODE_ID) == child) {
                return parent
            };
            child = parent;
            parent = get_bits(parent_bits, SHIFT_PARENT, HI_NODE_ID);
        };
        NIL
    }

    fun predecessor<V: store>(self: &AvlQueue<V>, node_id: u64): u64 {
        let bits = *table::borrow(&self.tree_nodes, node_id);
        let left = get_bits(bits, SHIFT_LEFT, HI_NODE_ID);
        if (left != NIL) return extreme_tree_node(self, left, false);
        let child = node_id;
        let parent = get_bits(bits, SHIFT_PARENT, HI_NODE_ID);
        while (parent != NIL) {
            let parent_bits = *table::borrow(&self.tree_nodes, parent);
            if (get_bits(parent_bits, SHIFT_RIGHT, HI_NODE_ID) == child) {
                return parent
            };
            child = parent;
            parent = get_bits(parent_bits, SHIFT_PARENT, HI_NODE_ID);
        };
        NIL
    }

    // Rebalancing.

    /// Walk up from `node_id`, whose subtree on `side` just changed height by
    /// one, updating heights and rotating where the invariant broke. Stops as
    /// soon as a subtree's height comes out unchanged.
    fun retrace<V: store>(
        self: &mut AvlQueue<V>, node_id: u64, side: bool, increment: bool
    ) {
        while (node_id != NIL) {
            let bits = *table::borrow(&self.tree_nodes, node_id);
            let height_left = get_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
            let height_right = get_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
            let old_height =
                if (height_left > height_right) height_left else height_right;
            if (side == LEFT) {
                height_left =
                    if (increment) height_left + 1 else height_left - 1;
            } else {
                height_right =
                    if (increment) height_right + 1 else height_right - 1;
            };
            let parent = get_bits(bits, SHIFT_PARENT, HI_NODE_ID);
            let subtree_root;
            let new_height;
            if (height_left > height_right + 1
                || height_right > height_left + 1) {
                let hung_side = side_of_hung(self, parent, node_id);
                (subtree_root, new_height) =
                    rebalance(self, node_id, bits, height_left, height_right);
                hang(self, parent, hung_side, subtree_root);
                set_parent(self, subtree_root, parent);
            } else {
                bits = set_bits(
                    bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, height_left);
                bits = set_bits(
                    bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, height_right);
                *table::borrow_mut(&mut self.tree_nodes, node_id) = bits;
                subtree_root = node_id;
                new_height =
                    if (height_left > height_right) height_left
                    else height_right;
            };
            if (new_height == old_height || parent == NIL) return;
            increment = new_height > old_height;
            side = side_of(self, parent, subtree_root);
            node_id = parent;
        }
    }

    /// Side `node_id` hangs on under `parent`, tolerating a null parent so
    /// `retrace` can re-hang a rotated subtree at the root.
    fun side_of_hung<V: store>(
        self: &AvlQueue<V>, parent: u64, node_id: u64
    ): bool {
        if (parent == NIL) LEFT else side_of(self, parent, node_id)
    }

    /// Rotate the subtree at `node_id`, whose child heights differ by two,
    /// and return its new root and that root's height. The caller re-hangs
    /// the returned root under the old parent.
    fun rebalance<V: store>(
        self: &mut AvlQueue<V>,
        node_id: u64,
        bits: u128,
        height_left: u64,
        height_right: u64,
    ): (u64, u64) {
        if (height_left > height_right) {
            let child = get_bits(bits, SHIFT_LEFT, HI_NODE_ID);
            let child_bits = *table::borrow(&self.tree_nodes, child);
            let inner = get_bits(child_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT)
                >= get_bits(child_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
            if (inner) {
                rotate_right(
                    self, node_id, bits, height_right, child, child_bits)
            } else {
                rotate_left_right(
                    self, node_id, bits, height_right, child, child_bits)
            }
        } else {
            let child = get_bits(bits, SHIFT_RIGHT, HI_NODE_ID);
            let child_bits = *table::borrow(&self.tree_nodes, child);
            let inner = get_bits(child_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT)
                >= get_bits(child_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
            if (inner) {
                rotate_left(
                    self, node_id, bits, height_left, child, child_bits)
            } else {
                rotate_right_left(
                    self, node_id, bits, height_left, child, child_bits)
            }
        }
    }

    /// Left-left case. `child` becomes the subtree root and adopts `node_id`
    /// as its right child, which takes over `child`'s old right subtree.
    fun rotate_right<V: store>(
        self: &mut AvlQueue<V>,
        node_id: u64,
        bits: u128,
        height_right: u64,
        child: u64,
        child_bits: u128,
    ): (u64, u64) {
        let inner = get_bits(child_bits, SHIFT_RIGHT, HI_NODE_ID);
        let inner_height = get_bits(child_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
        bits = set_bits(bits, SHIFT_LEFT, HI_NODE_ID, inner);
        bits = set_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, inner_height);
        bits = set_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, height_right);
        bits = set_bits(bits, SHIFT_PARENT, HI_NODE_ID, child);
        *table::borrow_mut(&mut self.tree_nodes, node_id) = bits;
        if (inner != NIL) set_parent(self, inner, node_id);
        let node_height =
            if (inner_height > height_right) inner_height else height_right;
        let child_height_left =
            get_bits(child_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        child_bits = set_bits(child_bits, SHIFT_RIGHT, HI_NODE_ID, node_id);
        child_bits = set_bits(
            child_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, node_height + 1);
        *table::borrow_mut(&mut self.tree_nodes, child) = child_bits;
        let height =
            if (child_height_left > node_height + 1) child_height_left
            else node_height + 1;
        (child, height)
    }

    /// Right-right case, the mirror of `rotate_right`.
    fun rotate_left<V: store>(
        self: &mut AvlQueue<V>,
        node_id: u64,
        bits: u128,
        height_left: u64,
        child: u64,
        child_bits: u128,
    ): (u64, u64) {
        let inner = get_bits(child_bits, SHIFT_LEFT, HI_NODE_ID);
        let inner_height = get_bits(child_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        bits = set_bits(bits, SHIFT_RIGHT, HI_NODE_ID, inner);
        bits = set_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, inner_height);
        bits = set_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, height_left);
        bits = set_bits(bits, SHIFT_PARENT, HI_NODE_ID, child);
        *table::borrow_mut(&mut self.tree_nodes, node_id) = bits;
        if (inner != NIL) set_parent(self, inner, node_id);
        let node_height =
            if (inner_height > height_left) inner_height else height_left;
        let child_height_right =
            get_bits(child_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
        child_bits = set_bits(child_bits, SHIFT_LEFT, HI_NODE_ID, node_id);
        child_bits = set_bits(
            child_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, node_height + 1);
        *table::borrow_mut(&mut self.tree_nodes, child) = child_bits;
        let height =
            if (child_height_right > node_height + 1) child_height_right
            else node_height + 1;
        (child, height)
    }

    /// Left-right case. The left child's right child rises to the subtree
    /// root, taking the left child on its left and `node_id` on its right.
    fun rotate_left_right<V: store>(
        self: &mut AvlQueue<V>,
        node_id: u64,
        bits: u128,
        height_right: u64,
        child: u64,
        child_bits: u128,
    ): (u64, u64) {
        let grandchild = get_bits(child_bits, SHIFT_RIGHT, HI_NODE_ID);
        let g_bits = *table::borrow(&self.tree_nodes, grandchild);
        let g_left = get_bits(g_bits, SHIFT_LEFT, HI_NODE_ID);
        let g_left_height = get_bits(g_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        let g_right = get_bits(g_bits, SHIFT_RIGHT, HI_NODE_ID);
        let g_right_height = get_bits(g_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);

        let child_height_left =
            get_bits(child_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        child_bits = set_bits(child_bits, SHIFT_RIGHT, HI_NODE_ID, g_left);
        child_bits = set_bits(
            child_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, g_left_height);
        child_bits =
            set_bits(child_bits, SHIFT_PARENT, HI_NODE_ID, grandchild);
        *table::borrow_mut(&mut self.tree_nodes, child) = child_bits;
        let child_height =
            if (child_height_left > g_left_height) child_height_left
            else g_left_height;

        bits = set_bits(bits, SHIFT_LEFT, HI_NODE_ID, g_right);
        bits = set_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, g_right_height);
        bits = set_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, height_right);
        bits = set_bits(bits, SHIFT_PARENT, HI_NODE_ID, grandchild);
        *table::borrow_mut(&mut self.tree_nodes, node_id) = bits;
        let node_height =
            if (g_right_height > height_right) g_right_height
            else height_right;

        g_bits = set_bits(g_bits, SHIFT_LEFT, HI_NODE_ID, child);
        g_bits = set_bits(g_bits, SHIFT_RIGHT, HI_NODE_ID, node_id);
        g_bits = set_bits(
            g_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, child_height + 1);
        g_bits = set_bits(
            g_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, node_height + 1);
        *table::borrow_mut(&mut self.tree_nodes, grandchild) = g_bits;

        if (g_left != NIL) set_parent(self, g_left, child);
        if (g_right != NIL) set_parent(self, g_right, node_id);
        let height =
            if (child_height > node_height) child_height + 1
            else node_height + 1;
        (grandchild, height)
    }

    /// Right-left case, the mirror of `rotate_left_right`.
    fun rotate_right_left<V: store>(
        self: &mut AvlQueue<V>,
        node_id: u64,
        bits: u128,
        height_left: u64,
        child: u64,
        child_bits: u128,
    ): (u64, u64) {
        let grandchild = get_bits(child_bits, SHIFT_LEFT, HI_NODE_ID);
        let g_bits = *table::borrow(&self.tree_nodes, grandchild);
        let g_left = get_bits(g_bits, SHIFT_LEFT, HI_NODE_ID);
        let g_left_height = get_bits(g_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        let g_right = get_bits(g_bits, SHIFT_RIGHT, HI_NODE_ID);
        let g_right_height = get_bits(g_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);

        let child_height_right =
            get_bits(child_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
        child_bits = set_bits(child_bits, SHIFT_LEFT, HI_NODE_ID, g_right);
        child_bits = set_bits(
            child_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, g_right_height);
        child_bits =
            set_bits(child_bits, SHIFT_PARENT, HI_NODE_ID, grandchild);
        *table::borrow_mut(&mut self.tree_nodes, child) = child_bits;
        let child_height =
            if (child_height_right > g_right_height) child_height_right
            else g_right_height;

        bits = set_bits(bits, SHIFT_RIGHT, HI_NODE_ID, g_left);
        bits = set_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, g_left_height);
        bits = set_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, height_left);
        bits = set_bits(bits, SHIFT_PARENT, HI_NODE_ID, grandchild);
        *table::borrow_mut(&mut self.tree_nodes, node_id) = bits;
        let node_height =
            if (g_left_height > height_left) g_left_height else height_left;

        g_bits = set_bits(g_bits, SHIFT_RIGHT, HI_NODE_ID, child);
        g_bits = set_bits(g_bits, SHIFT_LEFT, HI_NODE_ID, node_id);
        g_bits = set_bits(
            g_bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT, child_height + 1);
        g_bits = set_bits(
            g_bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT, node_height + 1);
        *table::borrow_mut(&mut self.tree_nodes, grandchild) = g_bits;

        if (g_right != NIL) set_parent(self, g_right, child);
        if (g_left != NIL) set_parent(self, g_left, node_id);
        let height =
            if (child_height > node_height) child_height + 1
            else node_height + 1;
        (grandchild, height)
    }

    // Node provisioning.

    fun alloc_tree_node<V: store>(self: &mut AvlQueue<V>): u64 {
        let top = get_bits(self.bits, SHIFT_TREE_STACK_TOP, HI_NODE_ID);
        if (top != NIL) {
            let next = get_bits(
                *table::borrow(&self.tree_nodes, top),
                SHIFT_NEXT_FREE,
                HI_NODE_ID,
            );
            self.bits =
                set_bits(self.bits, SHIFT_TREE_STACK_TOP, HI_NODE_ID, next);
            top
        } else {
            let node_id = self.n_tree_nodes + 1;
            assert!(node_id <= N_NODES_MAX, E_TOO_MANY_TREE_NODES);
            self.n_tree_nodes = node_id;
            table::add(&mut self.tree_nodes, node_id, 0);
            node_id
        }
    }

    fun free_tree_node<V: store>(self: &mut AvlQueue<V>, node_id: u64) {
        let top = get_bits(self.bits, SHIFT_TREE_STACK_TOP, HI_NODE_ID);
        *table::borrow_mut(&mut self.tree_nodes, node_id) =
            set_bits(0, SHIFT_NEXT_FREE, HI_NODE_ID, top);
        self.bits =
            set_bits(self.bits, SHIFT_TREE_STACK_TOP, HI_NODE_ID, node_id);
    }

    fun alloc_list_node<V: store>(self: &mut AvlQueue<V>): u64 {
        let top = get_bits(self.bits, SHIFT_LIST_STACK_TOP, HI_NODE_ID);
        if (top != NIL) {
            let next = get_bits(
                *table::borrow(&self.list_nodes, top),
                SHIFT_LIST_NEXT,
                HI_NODE_ID,
            );
            self.bits =
                set_bits(self.bits, SHIFT_LIST_STACK_TOP, HI_NODE_ID, next);
            top
        } else {
            let node_id = self.n_list_nodes + 1;
            assert!(node_id <= N_NODES_MAX, E_TOO_MANY_LIST_NODES);
            self.n_list_nodes = node_id;
            table::add(&mut self.list_nodes, node_id, 0);
            node_id
        }
    }

    fun free_list_node<V: store>(self: &mut AvlQueue<V>, node_id: u64) {
        let top = get_bits(self.bits, SHIFT_LIST_STACK_TOP, HI_NODE_ID);
        *table::borrow_mut(&mut self.list_nodes, node_id) =
            set_bits(0, SHIFT_LIST_NEXT, HI_NODE_ID, top);
        self.bits =
            set_bits(self.bits, SHIFT_LIST_STACK_TOP, HI_NODE_ID, node_id);
    }

    // Tests.

    #[test_only]
    use std::vector;

    #[test_only]
    const LCG_MUL: u64 = 1103515245;
    #[test_only]
    const LCG_INC: u64 = 12345;
    #[test_only]
    const LCG_MOD: u64 = 1000003;

    #[test_only]
    const E_CHECK_FIELD: u64 = 100;
    #[test_only]
    const E_CHECK_PARENT: u64 = 101;
    #[test_only]
    const E_CHECK_HEIGHT: u64 = 102;
    #[test_only]
    const E_CHECK_BALANCE: u64 = 103;
    #[test_only]
    const E_CHECK_ORDER: u64 = 104;
    #[test_only]
    const E_CHECK_LIST: u64 = 105;
    #[test_only]
    const E_CHECK_STACK: u64 = 106;
    #[test_only]
    const E_CHECK_HEAD_TAIL: u64 = 107;
    #[test_only]
    const E_CHECK_SHAPE: u64 = 108;
    #[test_only]
    const E_CHECK_VALUE: u64 = 109;

    #[test_only]
    public fun drop_unchecked<V: store>(self: AvlQueue<V>) {
        let AvlQueue {
            bits: _,
            root: _,
            tree_nodes,
            list_nodes,
            values,
            n_tree_nodes: _,
            n_list_nodes: _,
        } = self;
        table::drop_unchecked(tree_nodes);
        table::drop_unchecked(list_nodes);
        table::drop_unchecked(values);
    }

    // Bit packing.

    #[test_only]
    fun assert_field_value(shift: u8, mask: u64, value: u64) {
        let bits = set_bits(0, shift, mask, value);
        assert!(get_bits(bits, shift, mask) == value, E_CHECK_FIELD);
        // Nothing outside the field was set.
        assert!(bits == ((value as u128) << shift), E_CHECK_FIELD);
        // Writing into an all-ones word clears only this field.
        let field = (mask as u128) << shift;
        let bits = set_bits(HI_128, shift, mask, value);
        assert!(get_bits(bits, shift, mask) == value, E_CHECK_FIELD);
        assert!((bits | field) == HI_128, E_CHECK_FIELD);
    }

    #[test_only]
    fun assert_field(shift: u8, mask: u64) {
        assert_field_value(shift, mask, 0);
        assert_field_value(shift, mask, 1);
        assert_field_value(shift, mask, mask);
    }

    #[test]
    fun test_field_tree_key() { assert_field(SHIFT_KEY, HI_INSERTION_KEY); }

    #[test]
    fun test_field_tree_parent() { assert_field(SHIFT_PARENT, HI_NODE_ID); }

    #[test]
    fun test_field_tree_left() { assert_field(SHIFT_LEFT, HI_NODE_ID); }

    #[test]
    fun test_field_tree_right() { assert_field(SHIFT_RIGHT, HI_NODE_ID); }

    #[test]
    fun test_field_tree_list_head() {
        assert_field(SHIFT_LIST_HEAD, HI_NODE_ID);
    }

    #[test]
    fun test_field_tree_list_tail() {
        assert_field(SHIFT_LIST_TAIL, HI_NODE_ID);
    }

    #[test]
    fun test_field_tree_next_free() {
        assert_field(SHIFT_NEXT_FREE, HI_NODE_ID);
    }

    #[test]
    fun test_field_tree_height_left() {
        assert_field(SHIFT_HEIGHT_LEFT, HI_HEIGHT);
    }

    #[test]
    fun test_field_tree_height_right() {
        assert_field(SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
    }

    #[test]
    fun test_field_list_prev() { assert_field(SHIFT_LIST_PREV, HI_NODE_ID); }

    #[test]
    fun test_field_list_prev_tree() {
        assert_field(SHIFT_LIST_PREV_TREE, HI_BIT);
    }

    #[test]
    fun test_field_list_next() { assert_field(SHIFT_LIST_NEXT, HI_NODE_ID); }

    #[test]
    fun test_field_list_next_tree() {
        assert_field(SHIFT_LIST_NEXT_TREE, HI_BIT);
    }

    #[test]
    fun test_field_header_head_key() {
        assert_field(SHIFT_HEAD_KEY, HI_INSERTION_KEY);
    }

    #[test]
    fun test_field_header_head_node() {
        assert_field(SHIFT_HEAD_NODE, HI_NODE_ID);
    }

    #[test]
    fun test_field_header_tail_key() {
        assert_field(SHIFT_TAIL_KEY, HI_INSERTION_KEY);
    }

    #[test]
    fun test_field_header_tail_node() {
        assert_field(SHIFT_TAIL_NODE, HI_NODE_ID);
    }

    #[test]
    fun test_field_header_tree_stack_top() {
        assert_field(SHIFT_TREE_STACK_TOP, HI_NODE_ID);
    }

    #[test]
    fun test_field_header_list_stack_top() {
        assert_field(SHIFT_LIST_STACK_TOP, HI_NODE_ID);
    }

    #[test]
    fun test_field_header_sort_order() {
        assert_field(SHIFT_SORT_ORDER, HI_BIT);
    }

    #[test_only]
    fun assert_access_key_value(
        tree_node_id: u64, list_node_id: u64, key: u64, ascending: bool
    ) {
        let access_key =
            pack_access_key(tree_node_id, list_node_id, key, ascending);
        assert!(access_key_tree_node_id(access_key) == tree_node_id,
            E_CHECK_FIELD);
        assert!(access_key_list_node_id(access_key) == list_node_id,
            E_CHECK_FIELD);
        assert!(access_key_insertion_key(access_key) == key, E_CHECK_FIELD);
        assert!(access_key_is_ascending(access_key) == ascending,
            E_CHECK_FIELD);
    }

    #[test]
    fun test_field_access_key() {
        assert_access_key_value(0, 0, 0, DESCENDING);
        assert_access_key_value(1, 1, 1, ASCENDING);
        assert_access_key_value(
            HI_NODE_ID, HI_NODE_ID, HI_INSERTION_KEY, ASCENDING);
        assert_access_key_value(
            HI_NODE_ID, HI_NODE_ID, HI_INSERTION_KEY, DESCENDING);
        // Each field is independent of the others.
        assert_access_key_value(HI_NODE_ID, 0, 0, DESCENDING);
        assert_access_key_value(0, HI_NODE_ID, 0, DESCENDING);
        assert_access_key_value(0, 0, HI_INSERTION_KEY, DESCENDING);
    }

    // Invariants.

    // Height the parent should store for `node_id`, the number of active
    // tree nodes below and including it, and the number of active list
    // nodes they own. Appends the subtree's insertion keys to `keys` in
    // order.
    #[test_only]
    fun check_subtree<V: store>(
        self: &AvlQueue<V>, node_id: u64, parent: u64, keys: &mut vector<u64>
    ): (u64, u64, u64) {
        if (node_id == NIL) return (0, 0, 0);
        let bits = *table::borrow(&self.tree_nodes, node_id);
        assert!(get_bits(bits, SHIFT_PARENT, HI_NODE_ID) == parent,
            E_CHECK_PARENT);
        let left = get_bits(bits, SHIFT_LEFT, HI_NODE_ID);
        let right = get_bits(bits, SHIFT_RIGHT, HI_NODE_ID);
        let (left_height, left_trees, left_lists) =
            check_subtree(self, left, node_id, keys);
        vector::push_back(keys, get_bits(bits, SHIFT_KEY, HI_INSERTION_KEY));
        let lists = check_list(self, node_id, bits);
        let (right_height, right_trees, right_lists) =
            check_subtree(self, right, node_id, keys);
        let stored_left = get_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        let stored_right = get_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
        assert!(stored_left == left_height, E_CHECK_HEIGHT);
        assert!(stored_right == right_height, E_CHECK_HEIGHT);
        let diff =
            if (stored_left > stored_right) stored_left - stored_right
            else stored_right - stored_left;
        assert!(diff <= 1, E_CHECK_BALANCE);
        let height =
            (if (stored_left > stored_right) stored_left else stored_right) + 1;
        (height, left_trees + right_trees + 1, left_lists + right_lists + lists)
    }

    // Walk the list at `node_id` head to tail, checking that every back
    // link matches and that both ends are flagged as tree node ids.
    #[test_only]
    fun check_list<V: store>(
        self: &AvlQueue<V>, node_id: u64, tree_bits: u128
    ): u64 {
        let head = get_bits(tree_bits, SHIFT_LIST_HEAD, HI_NODE_ID);
        let tail = get_bits(tree_bits, SHIFT_LIST_TAIL, HI_NODE_ID);
        assert!(head != NIL && tail != NIL, E_CHECK_LIST);
        let count = 0;
        let prev = node_id;
        let prev_is_tree = true;
        let list_node_id = head;
        loop {
            let bits = *table::borrow(&self.list_nodes, list_node_id);
            assert!(get_bits(bits, SHIFT_LIST_PREV, HI_NODE_ID) == prev,
                E_CHECK_LIST);
            assert!(
                (get_bits(bits, SHIFT_LIST_PREV_TREE, HI_BIT) == 1)
                    == prev_is_tree,
                E_CHECK_LIST,
            );
            assert!(table::contains(&self.values, list_node_id), E_CHECK_LIST);
            count = count + 1;
            assert!(count <= self.n_list_nodes, E_CHECK_LIST);
            let next = get_bits(bits, SHIFT_LIST_NEXT, HI_NODE_ID);
            if (get_bits(bits, SHIFT_LIST_NEXT_TREE, HI_BIT) == 1) {
                assert!(next == node_id, E_CHECK_LIST);
                assert!(list_node_id == tail, E_CHECK_LIST);
                break
            };
            prev = list_node_id;
            prev_is_tree = false;
            list_node_id = next;
        };
        count
    }

    // Length of the free-list stack rooted at `top`, whose link field sits
    // at `shift`. Bounded by `limit` so a cycle fails rather than hangs.
    #[test_only]
    fun stack_length(
        nodes: &Table<u64, u128>, top: u64, shift: u8, limit: u64
    ): u64 {
        let count = 0;
        while (top != NIL) {
            top = get_bits(*table::borrow(nodes, top), shift, HI_NODE_ID);
            count = count + 1;
            assert!(count <= limit, E_CHECK_STACK);
        };
        count
    }

    #[test_only]
    fun check_head_tail<V: store>(self: &AvlQueue<V>, n_tree_active: u64) {
        let head_node = get_bits(self.bits, SHIFT_HEAD_NODE, HI_NODE_ID);
        let tail_node = get_bits(self.bits, SHIFT_TAIL_NODE, HI_NODE_ID);
        if (n_tree_active == 0) {
            assert!(head_node == NIL, E_CHECK_HEAD_TAIL);
            assert!(tail_node == NIL, E_CHECK_HEAD_TAIL);
            return
        };
        let ascending = is_ascending(self);
        let bits = *table::borrow(
            &self.tree_nodes, extreme_tree_node(self, self.root, ascending));
        assert!(head_node == get_bits(bits, SHIFT_LIST_HEAD, HI_NODE_ID),
            E_CHECK_HEAD_TAIL);
        assert!(
            get_bits(self.bits, SHIFT_HEAD_KEY, HI_INSERTION_KEY)
                == get_bits(bits, SHIFT_KEY, HI_INSERTION_KEY),
            E_CHECK_HEAD_TAIL,
        );
        let bits = *table::borrow(
            &self.tree_nodes, extreme_tree_node(self, self.root, !ascending));
        assert!(tail_node == get_bits(bits, SHIFT_LIST_TAIL, HI_NODE_ID),
            E_CHECK_HEAD_TAIL);
        assert!(
            get_bits(self.bits, SHIFT_TAIL_KEY, HI_INSERTION_KEY)
                == get_bits(bits, SHIFT_KEY, HI_INSERTION_KEY),
            E_CHECK_HEAD_TAIL,
        );
    }

    #[test_only]
    fun assert_invariants<V: store>(self: &AvlQueue<V>) {
        let keys = vector::empty<u64>();
        let (_, n_tree_active, n_list_active) =
            check_subtree(self, self.root, NIL, &mut keys);
        let n_keys = vector::length(&keys);
        let i = 1;
        while (i < n_keys) {
            assert!(
                *vector::borrow(&keys, i - 1) < *vector::borrow(&keys, i),
                E_CHECK_ORDER,
            );
            i = i + 1;
        };
        let n_tree_free = stack_length(
            &self.tree_nodes,
            get_bits(self.bits, SHIFT_TREE_STACK_TOP, HI_NODE_ID),
            SHIFT_NEXT_FREE,
            self.n_tree_nodes,
        );
        let n_list_free = stack_length(
            &self.list_nodes,
            get_bits(self.bits, SHIFT_LIST_STACK_TOP, HI_NODE_ID),
            SHIFT_LIST_NEXT,
            self.n_list_nodes,
        );
        assert!(n_tree_active + n_tree_free == self.n_tree_nodes,
            E_CHECK_STACK);
        assert!(n_list_active + n_list_free == self.n_list_nodes,
            E_CHECK_STACK);
        assert!(get_height(self) <= MAX_HEIGHT, E_CHECK_HEIGHT);
        check_head_tail(self, n_tree_active);
    }

    // Stored left and right heights of the node holding `key`.
    #[test_only]
    fun heights_of<V: store>(self: &AvlQueue<V>, key: u64): (u64, u64) {
        let (node_id, found, _) = search(self, key);
        assert!(found, E_CHECK_SHAPE);
        let bits = *table::borrow(&self.tree_nodes, node_id);
        (
            get_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT),
            get_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT),
        )
    }

    #[test_only]
    fun borrow_tail<V: store>(self: &AvlQueue<V>): &V {
        let tail = get_bits(self.bits, SHIFT_TAIL_NODE, HI_NODE_ID);
        assert!(tail != NIL, E_EMPTY);
        table::borrow(&self.values, tail)
    }

    // Access key of the element at `index` in the list under `key`, counting
    // from the list head.
    #[test_only]
    fun access_key_at<V: store>(
        self: &AvlQueue<V>, key: u64, index: u64
    ): u64 {
        let (node_id, found, _) = search(self, key);
        assert!(found, E_CHECK_SHAPE);
        let list_node_id = get_bits(
            *table::borrow(&self.tree_nodes, node_id),
            SHIFT_LIST_HEAD,
            HI_NODE_ID,
        );
        while (index > 0) {
            let bits = *table::borrow(&self.list_nodes, list_node_id);
            assert!(get_bits(bits, SHIFT_LIST_NEXT_TREE, HI_BIT) == 0,
                E_CHECK_SHAPE);
            list_node_id = get_bits(bits, SHIFT_LIST_NEXT, HI_NODE_ID);
            index = index - 1;
        };
        pack_access_key(node_id, list_node_id, key, is_ascending(self))
    }

    #[test_only]
    fun remove_at<V: store>(
        self: &mut AvlQueue<V>, key: u64, index: u64
    ): V {
        let access_key = access_key_at(self, key, index);
        remove(self, access_key)
    }

    // Rotations.

    #[test_only]
    fun key_of<V: store>(self: &AvlQueue<V>, node_id: u64): u64 {
        get_bits(
            *table::borrow(&self.tree_nodes, node_id),
            SHIFT_KEY,
            HI_INSERTION_KEY,
        )
    }

    // Assert the children of the node holding `key`, with 0 standing for an
    // absent child. The rotation tests use nonzero keys throughout.
    #[test_only]
    fun assert_children<V: store>(
        self: &AvlQueue<V>, node_id: u64, left_key: u64, right_key: u64
    ) {
        let bits = *table::borrow(&self.tree_nodes, node_id);
        let left = get_bits(bits, SHIFT_LEFT, HI_NODE_ID);
        let right = get_bits(bits, SHIFT_RIGHT, HI_NODE_ID);
        assert!((if (left == NIL) 0 else key_of(self, left)) == left_key,
            E_CHECK_SHAPE);
        assert!((if (right == NIL) 0 else key_of(self, right)) == right_key,
            E_CHECK_SHAPE);
    }

    #[test_only]
    fun insert_keys(queue: &mut AvlQueue<u64>, keys: vector<u64>) {
        let i = 0;
        let n = vector::length(&keys);
        while (i < n) {
            let key = *vector::borrow(&keys, i);
            insert(queue, key, key);
            assert_invariants(queue);
            i = i + 1;
        }
    }

    #[test_only]
    fun keys_vector(a: u64, b: u64, c: u64, d: u64): vector<u64> {
        let keys = vector::empty<u64>();
        vector::push_back(&mut keys, a);
        vector::push_back(&mut keys, b);
        vector::push_back(&mut keys, c);
        if (d != 0) vector::push_back(&mut keys, d);
        keys
    }

    // Remove the sole element under `key`.
    #[test_only]
    fun remove_key(queue: &mut AvlQueue<u64>, key: u64) {
        let (node_id, found, _) = search(queue, key);
        assert!(found, E_CHECK_SHAPE);
        let list_node_id = get_bits(
            *table::borrow(&queue.tree_nodes, node_id),
            SHIFT_LIST_HEAD,
            HI_NODE_ID,
        );
        let access_key =
            pack_access_key(node_id, list_node_id, key, is_ascending(queue));
        let value = remove(queue, access_key);
        assert!(value == key, E_CHECK_VALUE);
        assert_invariants(queue);
    }

    #[test]
    fun test_rotate_right_on_insert() {
        // Descending inserts make the root left-heavy over a left-heavy
        // child.
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(30, 20, 10, 0));
        assert!(key_of(&queue, queue.root) == 20, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 10, 30);
        drop_unchecked(queue);
    }

    #[test]
    fun test_rotate_left_on_insert() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(10, 20, 30, 0));
        assert!(key_of(&queue, queue.root) == 20, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 10, 30);
        drop_unchecked(queue);
    }

    #[test]
    fun test_rotate_left_right_on_insert() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(30, 10, 20, 0));
        assert!(key_of(&queue, queue.root) == 20, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 10, 30);
        drop_unchecked(queue);
    }

    #[test]
    fun test_rotate_right_left_on_insert() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(10, 30, 20, 0));
        assert!(key_of(&queue, queue.root) == 20, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 10, 30);
        drop_unchecked(queue);
    }

    #[test]
    fun test_rotate_right_on_remove() {
        // Removing 40 leaves 30 left-heavy over a left-heavy child.
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(30, 40, 20, 10));
        remove_key(&mut queue, 40);
        assert!(key_of(&queue, queue.root) == 20, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 10, 30);
        drop_unchecked(queue);
    }

    #[test]
    fun test_rotate_left_on_remove() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(20, 10, 30, 40));
        remove_key(&mut queue, 10);
        assert!(key_of(&queue, queue.root) == 30, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 20, 40);
        drop_unchecked(queue);
    }

    #[test]
    fun test_rotate_left_right_on_remove() {
        // Removing 40 leaves 30 left-heavy over a right-heavy child.
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(30, 40, 10, 20));
        remove_key(&mut queue, 40);
        assert!(key_of(&queue, queue.root) == 20, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 10, 30);
        drop_unchecked(queue);
    }

    #[test]
    fun test_rotate_right_left_on_remove() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(20, 10, 40, 30));
        remove_key(&mut queue, 10);
        assert!(key_of(&queue, queue.root) == 30, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 20, 40);
        drop_unchecked(queue);
    }

    // Queue behaviour.

    #[test]
    fun test_empty_queue() {
        let queue = new<u64>(ASCENDING);
        assert!(is_empty(&queue), E_CHECK_SHAPE);
        assert!(is_ascending(&queue), E_CHECK_SHAPE);
        assert!(!has_key(&queue, 7), E_CHECK_SHAPE);
        assert!(head_access_key(&queue) == 0, E_CHECK_SHAPE);
        assert_invariants(&queue);
        drop_unchecked(queue);
        let queue = new<u64>(DESCENDING);
        assert!(!is_ascending(&queue), E_CHECK_SHAPE);
        drop_unchecked(queue);
    }

    #[test]
    #[expected_failure(abort_code = E_EMPTY, location = Self)]
    fun test_pop_head_empty() {
        let queue = new<u64>(ASCENDING);
        pop_head(&mut queue);
        drop_unchecked(queue);
    }

    #[test]
    #[expected_failure(abort_code = E_INSERTION_KEY_TOO_LARGE, location = Self)]
    fun test_insert_key_too_large() {
        let queue = new<u64>(ASCENDING);
        insert(&mut queue, HI_INSERTION_KEY + 1, 0);
        drop_unchecked(queue);
    }

    #[test]
    fun test_pop_head_ascending() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(50, 20, 80, 10));
        insert_keys(&mut queue, keys_vector(70, 30, 60, 40));
        assert!(get_head_key(&queue) == 10, E_CHECK_ORDER);
        assert!(get_tail_key(&queue) == 80, E_CHECK_ORDER);
        let expected = 10;
        while (!is_empty(&queue)) {
            assert!(get_head_key(&queue) == expected, E_CHECK_ORDER);
            assert!(*borrow_head(&queue) == expected, E_CHECK_VALUE);
            assert!(pop_head(&mut queue) == expected, E_CHECK_VALUE);
            assert_invariants(&queue);
            expected = expected + 10;
        };
        assert!(expected == 90, E_CHECK_ORDER);
        drop_unchecked(queue);
    }

    #[test]
    fun test_pop_head_descending() {
        let queue = new<u64>(DESCENDING);
        insert_keys(&mut queue, keys_vector(50, 20, 80, 10));
        insert_keys(&mut queue, keys_vector(70, 30, 60, 40));
        assert!(get_head_key(&queue) == 80, E_CHECK_ORDER);
        assert!(get_tail_key(&queue) == 10, E_CHECK_ORDER);
        let expected = 80;
        while (!is_empty(&queue)) {
            assert!(get_head_key(&queue) == expected, E_CHECK_ORDER);
            assert!(pop_head(&mut queue) == expected, E_CHECK_VALUE);
            assert_invariants(&queue);
            expected = expected - 10;
        };
        assert!(expected == 0, E_CHECK_ORDER);
        drop_unchecked(queue);
    }

    // Equal keys dequeue in insertion order in both sort orders, and the
    // walk visits every element exactly once.
    #[test]
    fun test_duplicate_keys_and_walk() {
        let queue = new<u64>(ASCENDING);
        insert(&mut queue, 5, 100);
        insert(&mut queue, 3, 200);
        insert(&mut queue, 5, 101);
        insert(&mut queue, 3, 201);
        insert(&mut queue, 5, 102);
        assert_invariants(&queue);
        // Walk order: 3 then 5, each list head to tail.
        let access_key = head_access_key(&queue);
        let seen = vector::empty<u64>();
        while (access_key != 0) {
            vector::push_back(&mut seen, *borrow(&queue, access_key));
            access_key = next_access_key(&queue, access_key);
        };
        assert!(vector::length(&seen) == 5, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 0) == 200, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 1) == 201, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 2) == 100, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 3) == 101, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 4) == 102, E_CHECK_ORDER);
        assert!(pop_head(&mut queue) == 200, E_CHECK_ORDER);
        assert!(pop_head(&mut queue) == 201, E_CHECK_ORDER);
        assert!(pop_head(&mut queue) == 100, E_CHECK_ORDER);
        assert_invariants(&queue);
        drop_unchecked(queue);
    }

    #[test]
    fun test_walk_descending() {
        let queue = new<u64>(DESCENDING);
        insert(&mut queue, 5, 100);
        insert(&mut queue, 3, 200);
        insert(&mut queue, 5, 101);
        insert(&mut queue, 9, 300);
        let access_key = head_access_key(&queue);
        let seen = vector::empty<u64>();
        while (access_key != 0) {
            vector::push_back(&mut seen, *borrow(&queue, access_key));
            access_key = next_access_key(&queue, access_key);
        };
        assert!(vector::length(&seen) == 4, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 0) == 300, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 1) == 100, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 2) == 101, E_CHECK_ORDER);
        assert!(*vector::borrow(&seen, 3) == 200, E_CHECK_ORDER);
        drop_unchecked(queue);
    }

    // Removing from the middle of a level leaves the rest of the level and
    // the tree node intact.
    #[test]
    fun test_remove_middle_of_level() {
        let queue = new<u64>(ASCENDING);
        insert(&mut queue, 7, 10);
        let middle = insert(&mut queue, 7, 11);
        insert(&mut queue, 7, 12);
        insert(&mut queue, 9, 20);
        assert!(remove(&mut queue, middle) == 11, E_CHECK_VALUE);
        assert_invariants(&queue);
        assert!(has_key(&queue, 7), E_CHECK_SHAPE);
        assert!(pop_head(&mut queue) == 10, E_CHECK_ORDER);
        assert!(pop_head(&mut queue) == 12, E_CHECK_ORDER);
        assert!(pop_head(&mut queue) == 20, E_CHECK_ORDER);
        assert!(!has_key(&queue, 7), E_CHECK_SHAPE);
        drop_unchecked(queue);
    }

    // Freed node ids come back out of the stacks rather than growing the
    // tables.
    #[test]
    fun test_node_reuse() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(1, 2, 3, 4));
        assert!(queue.n_tree_nodes == 4, E_CHECK_STACK);
        assert!(queue.n_list_nodes == 4, E_CHECK_STACK);
        remove_key(&mut queue, 1);
        remove_key(&mut queue, 3);
        insert_keys(&mut queue, keys_vector(5, 6, 7, 0));
        assert!(queue.n_tree_nodes == 5, E_CHECK_STACK);
        assert!(queue.n_list_nodes == 5, E_CHECK_STACK);
        drop_unchecked(queue);
    }

    #[test]
    fun test_borrow_mut() {
        let queue = new<u64>(ASCENDING);
        let access_key = insert(&mut queue, 4, 1);
        insert(&mut queue, 8, 2);
        *borrow_mut(&mut queue, access_key) = 99;
        assert!(*borrow(&queue, access_key) == 99, E_CHECK_VALUE);
        *borrow_head_mut(&mut queue) = 77;
        assert!(*borrow_head(&queue) == 77, E_CHECK_VALUE);
        drop_unchecked(queue);
    }

    // A long strictly ascending run is the worst case for a plain BST; the
    // AVL invariant must hold it to logarithmic height.
    #[test]
    fun test_sorted_insert_stays_balanced() {
        let queue = new<u64>(ASCENDING);
        let i = 0;
        while (i < 64) {
            insert(&mut queue, i, i);
            i = i + 1;
        };
        assert_invariants(&queue);
        let bits = *table::borrow(&queue.tree_nodes, queue.root);
        let left = get_bits(bits, SHIFT_HEIGHT_LEFT, HI_HEIGHT);
        let right = get_bits(bits, SHIFT_HEIGHT_RIGHT, HI_HEIGHT);
        let height = if (left > right) left else right;
        // 64 nodes fit in at most 8 levels, so the stored height is at most 8.
        assert!(height <= 8, E_CHECK_HEIGHT);
        drop_unchecked(queue);
    }

    // Fuzzing.

    // Run an LCG-driven insert/remove stream, asserting the AVL invariants
    // after every step, then drain the queue and check sort order.
    #[test_only]
    fun fuzz(ascending: bool, n_ops: u64, key_range: u64, seed: u64) {
        let queue = new<u64>(ascending);
        let access_keys = vector::empty<u64>();
        let insertion_keys = vector::empty<u64>();
        let x = seed % LCG_MOD;
        let i = 0;
        while (i < n_ops) {
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let op = x % 3;
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let n_live = vector::length(&access_keys);
            if (op < 2 || n_live == 0) {
                let key = x % key_range;
                let access_key = insert(&mut queue, key, i);
                assert!(access_key_insertion_key(access_key) == key,
                    E_CHECK_VALUE);
                assert!(access_key_is_ascending(access_key) == ascending,
                    E_CHECK_VALUE);
                assert!(has_key(&queue, key), E_CHECK_SHAPE);
                vector::push_back(&mut access_keys, access_key);
                vector::push_back(&mut insertion_keys, key);
            } else {
                let idx = x % n_live;
                let access_key = *vector::borrow(&access_keys, idx);
                assert!(remove(&mut queue, access_key) < i, E_CHECK_VALUE);
                vector::swap_remove(&mut access_keys, idx);
                vector::swap_remove(&mut insertion_keys, idx);
            };
            assert_invariants(&queue);
            i = i + 1;
        };
        let n_live = vector::length(&access_keys);
        let prev_key = 0;
        let prev_value = 0;
        let j = 0;
        while (j < n_live) {
            let key = get_head_key(&queue);
            let value = pop_head(&mut queue);
            if (j > 0) {
                if (ascending) {
                    assert!(key >= prev_key, E_CHECK_ORDER);
                } else {
                    assert!(key <= prev_key, E_CHECK_ORDER);
                };
                // Ties break by insertion order, and values are the
                // increasing op index.
                if (key == prev_key) {
                    assert!(value > prev_value, E_CHECK_ORDER);
                };
            };
            prev_key = key;
            prev_value = value;
            assert_invariants(&queue);
            j = j + 1;
        };
        assert!(is_empty(&queue), E_CHECK_SHAPE);
        assert!(get_bits(queue.bits, SHIFT_HEAD_NODE, HI_NODE_ID) == NIL,
            E_CHECK_HEAD_TAIL);
        drop_unchecked(queue);
    }

    #[test]
    fun test_fuzz_ascending() { fuzz(ASCENDING, 300, 64, 42); }

    #[test]
    fun test_fuzz_descending() { fuzz(DESCENDING, 300, 64, 7); }

    // A narrow key range forces many equal keys, so most of the work lands
    // on the list nodes rather than the tree.
    #[test]
    fun test_fuzz_duplicate_heavy() { fuzz(ASCENDING, 300, 6, 99); }

    // A wide key range keeps keys mostly distinct, so the tree stays deep
    // and rotations are frequent.
    #[test]
    fun test_fuzz_wide_keys() { fuzz(DESCENDING, 300, 100000, 1234); }

    // Parity with Econia's AVL queue. Each test below reproduces a worked
    // example or a numeric fact from the reference documentation and asserts
    // that this implementation lands in the same state. See README.md.

    // The reference caps node ids at 14 bits, insertion keys at 32, and stored
    // heights at 5, with a node count of 16383 and a tree height of 18.
    #[test]
    fun test_parity_caps() {
        assert!(N_NODES_MAX == 16383, E_CHECK_FIELD);
        assert!(HI_NODE_ID == 0x3fff, E_CHECK_FIELD);
        assert!(N_NODES_MAX == HI_NODE_ID, E_CHECK_FIELD);
        assert!(HI_INSERTION_KEY == 0xffffffff, E_CHECK_FIELD);
        assert!(HI_HEIGHT == 0x1f, E_CHECK_FIELD);
        assert!(MAX_HEIGHT == 18, E_CHECK_FIELD);
        // Five bits must hold the tallest stored height.
        assert!(MAX_HEIGHT <= HI_HEIGHT, E_CHECK_FIELD);
        assert!(ASCENDING == true && DESCENDING == false, E_CHECK_FIELD);
        assert!(LEFT == true && RIGHT == false, E_CHECK_FIELD);
        assert!(NIL == 0, E_CHECK_FIELD);
    }

    // Reference height progression: a lone root is height 0, then inserting 5,
    // 3 and 1 under root 4 takes the tree to 1, 1 and 2.
    #[test]
    fun test_parity_height_progression() {
        let queue = new<u64>(ASCENDING);
        assert!(get_height(&queue) == 0, E_CHECK_HEIGHT);
        insert(&mut queue, 4, 4);
        assert!(get_height(&queue) == 0, E_CHECK_HEIGHT);
        insert(&mut queue, 5, 5);
        assert!(get_height(&queue) == 1, E_CHECK_HEIGHT);
        insert(&mut queue, 3, 3);
        assert!(get_height(&queue) == 1, E_CHECK_HEIGHT);
        insert(&mut queue, 1, 1);
        assert!(get_height(&queue) == 2, E_CHECK_HEIGHT);
        assert_invariants(&queue);
        drop_unchecked(queue);
    }

    // Reference height table for the tree rooted at 2 with children 1 and 3 and
    // a right grandchild 4: per-node left and right heights of (0, 0), (1, 2),
    // (0, 1) and (0, 0).
    #[test]
    fun test_parity_height_table() {
        let queue = new<u64>(ASCENDING);
        insert_keys(&mut queue, keys_vector(2, 1, 3, 4));
        assert!(key_of(&queue, queue.root) == 2, E_CHECK_SHAPE);
        assert!(get_height(&queue) == 2, E_CHECK_HEIGHT);
        let (left, right) = heights_of(&queue, 1);
        assert!(left == 0 && right == 0, E_CHECK_HEIGHT);
        let (left, right) = heights_of(&queue, 2);
        assert!(left == 1 && right == 2, E_CHECK_HEIGHT);
        let (left, right) = heights_of(&queue, 3);
        assert!(left == 0 && right == 1, E_CHECK_HEIGHT);
        let (left, right) = heights_of(&queue, 4);
        assert!(left == 0 && right == 0, E_CHECK_HEIGHT);
        drop_unchecked(queue);
    }

    // Reference insertion sequence: (3, 9), (4, 8), (5, 7), (3, 6), (5, 5).
    // The third insertion rotates 4 to the root, and the duplicates append to
    // the tails of their lists.
    #[test]
    fun test_parity_insert_sequence() {
        let queue = new<u64>(ASCENDING);
        insert(&mut queue, 3, 9);
        assert!(key_of(&queue, queue.root) == 3, E_CHECK_SHAPE);
        insert(&mut queue, 4, 8);
        assert!(key_of(&queue, queue.root) == 3, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 0, 4);
        insert(&mut queue, 5, 7);
        assert!(key_of(&queue, queue.root) == 4, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 3, 5);
        insert(&mut queue, 3, 6);
        insert(&mut queue, 5, 5);
        assert!(key_of(&queue, queue.root) == 4, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 3, 5);
        assert_invariants(&queue);
        // Key 3 holds [9, 6] and key 5 holds [7, 5], head to tail.
        assert!(*borrow(&queue, access_key_at(&queue, 3, 0)) == 9,
            E_CHECK_VALUE);
        assert!(*borrow(&queue, access_key_at(&queue, 3, 1)) == 6,
            E_CHECK_VALUE);
        assert!(*borrow(&queue, access_key_at(&queue, 5, 0)) == 7,
            E_CHECK_VALUE);
        assert!(*borrow(&queue, access_key_at(&queue, 5, 1)) == 5,
            E_CHECK_VALUE);
        assert!(pop_head(&mut queue) == 9, E_CHECK_VALUE);
        assert!(pop_head(&mut queue) == 6, E_CHECK_VALUE);
        assert!(pop_head(&mut queue) == 8, E_CHECK_VALUE);
        assert!(pop_head(&mut queue) == 7, E_CHECK_VALUE);
        assert!(pop_head(&mut queue) == 5, E_CHECK_VALUE);
        assert!(is_empty(&queue), E_CHECK_SHAPE);
        drop_unchecked(queue);
    }

    // The reference removal diagram: key 2 holding [3, 4] with key 1 holding
    // [5, 6] as its left child.
    #[test_only]
    fun parity_removal_queue(ascending: bool): AvlQueue<u64> {
        let queue = new<u64>(ascending);
        insert(&mut queue, 2, 3);
        insert(&mut queue, 2, 4);
        insert(&mut queue, 1, 5);
        insert(&mut queue, 1, 6);
        assert!(key_of(&queue, queue.root) == 2, E_CHECK_SHAPE);
        assert_children(&queue, queue.root, 1, 0);
        assert_invariants(&queue);
        queue
    }

    // Case 1: removing 5 then 6 from an ascending queue walks the head to 6
    // and then to 3.
    #[test]
    fun test_parity_remove_ascending_head() {
        let queue = parity_removal_queue(ASCENDING);
        assert!(*borrow_head(&queue) == 5, E_CHECK_VALUE);
        remove_at(&mut queue, 1, 0);
        assert!(*borrow_head(&queue) == 6, E_CHECK_VALUE);
        assert_invariants(&queue);
        remove_at(&mut queue, 1, 0);
        assert!(*borrow_head(&queue) == 3, E_CHECK_VALUE);
        assert!(get_head_key(&queue) == 2, E_CHECK_HEAD_TAIL);
        assert_invariants(&queue);
        drop_unchecked(queue);
    }

    // Case 2: removing 4 then 3 from an ascending queue walks the tail to 3
    // and then to 6.
    #[test]
    fun test_parity_remove_ascending_tail() {
        let queue = parity_removal_queue(ASCENDING);
        assert!(*borrow_tail(&queue) == 4, E_CHECK_VALUE);
        remove_at(&mut queue, 2, 1);
        assert!(*borrow_tail(&queue) == 3, E_CHECK_VALUE);
        assert_invariants(&queue);
        remove_at(&mut queue, 2, 0);
        assert!(*borrow_tail(&queue) == 6, E_CHECK_VALUE);
        assert!(get_tail_key(&queue) == 1, E_CHECK_HEAD_TAIL);
        assert_invariants(&queue);
        drop_unchecked(queue);
    }

    // Case 3: removing 3 then 4 from a descending queue walks the head to 4
    // and then to 5.
    #[test]
    fun test_parity_remove_descending_head() {
        let queue = parity_removal_queue(DESCENDING);
        assert!(*borrow_head(&queue) == 3, E_CHECK_VALUE);
        remove_at(&mut queue, 2, 0);
        assert!(*borrow_head(&queue) == 4, E_CHECK_VALUE);
        assert_invariants(&queue);
        remove_at(&mut queue, 2, 0);
        assert!(*borrow_head(&queue) == 5, E_CHECK_VALUE);
        assert!(get_head_key(&queue) == 1, E_CHECK_HEAD_TAIL);
        assert_invariants(&queue);
        drop_unchecked(queue);
    }

    // Case 4: removing 6 then 5 from a descending queue walks the tail to 5
    // and then to 4.
    #[test]
    fun test_parity_remove_descending_tail() {
        let queue = parity_removal_queue(DESCENDING);
        assert!(*borrow_tail(&queue) == 6, E_CHECK_VALUE);
        remove_at(&mut queue, 1, 1);
        assert!(*borrow_tail(&queue) == 5, E_CHECK_VALUE);
        assert_invariants(&queue);
        remove_at(&mut queue, 1, 0);
        assert!(*borrow_tail(&queue) == 4, E_CHECK_VALUE);
        assert!(get_tail_key(&queue) == 2, E_CHECK_HEAD_TAIL);
        assert_invariants(&queue);
        drop_unchecked(queue);
    }

    // Removing from the middle of a list leaves the head, the tail and the
    // tree node alone, which is the reference's mid-list removal case.
    #[test]
    fun test_parity_remove_mid_list() {
        let queue = new<u64>(ASCENDING);
        insert(&mut queue, 7, 1);
        insert(&mut queue, 7, 2);
        insert(&mut queue, 7, 3);
        remove_at(&mut queue, 7, 1);
        assert!(*borrow_head(&queue) == 1, E_CHECK_VALUE);
        assert!(*borrow_tail(&queue) == 3, E_CHECK_VALUE);
        assert!(has_key(&queue, 7), E_CHECK_SHAPE);
        assert_invariants(&queue);
        assert!(pop_head(&mut queue) == 1, E_CHECK_VALUE);
        assert!(pop_head(&mut queue) == 3, E_CHECK_VALUE);
        drop_unchecked(queue);
    }

    // The reference reuses node ids from a stack, so an access key issued
    // twice can name two different elements over time.
    #[test]
    fun test_parity_access_key_reuse() {
        let queue = new<u64>(ASCENDING);
        let first = insert(&mut queue, 11, 100);
        remove(&mut queue, first);
        let second = insert(&mut queue, 11, 200);
        assert!(first == second, E_CHECK_SHAPE);
        assert!(*borrow(&queue, second) == 200, E_CHECK_VALUE);
        drop_unchecked(queue);
    }
}
