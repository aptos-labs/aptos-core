/// This module provides an implementation for an big ordered map.
/// Big means that it is stored across multiple resources, and doesn't have an
/// upper limit on number of elements it can contain.
///
/// Keys point to values, and each key in the map must be unique.
///
/// Currently, one implementation is provided - BPlusTreeMap, backed by a B+Tree,
/// with each node being a separate resource, internally containing OrderedMap.
///
/// BPlusTreeMap is chosen since the biggest (performance and gast)
/// costs are reading resources, and it:
/// * reduces number of resource accesses
/// * reduces number of rebalancing operations, and makes each rebalancing
///   operation touch only few resources
/// * it allows for parallelism for keys that are not close to each other,
///   once it contains enough keys
module aptos_std::big_ordered_map {
    use std::error;
    use std::vector;
    use std::option::{Self as option, Option};
    use std::bcs;
    use std::mem;
    use aptos_std::ordered_map::{Self, OrderedMap};
    use aptos_std::cmp;
    use aptos_std::storage_slots_allocator::{Self, StorageSlotsAllocator, StoredSlot, RefToSlot};
    use aptos_std::math64::{max, min};

    /// Map key already exists
    const EKEY_ALREADY_EXISTS: u64 = 1;
    /// Map key is not found
    const EKEY_NOT_FOUND: u64 = 2;
    // Trying to do an operation on an Iterator that would go out of bounds
    const EITER_OUT_OF_BOUNDS: u64 = 3;
    // The provided configuration parameter is invalid.
    const EINVALID_CONFIG_PARAMETER: u64 = 4;
    // Map isn't empty
    const EMAP_NOT_EMPTY: u64 = 5;
    // Trying to insert too large of an object into the mp.
    const EARGUMENT_BYTES_TOO_LARGE: u64 = 6;

    // Internal errors.
    const EINTERNAL_INVARIANT_BROKEN: u64 = 7;

    // Internal constants.

    const DEFAULT_TARGET_NODE_SIZE: u64 = 4096;
    const DEFAULT_INNER_MIN_DEGREE: u16 = 4;
    // We rely on 1 being valid size only for root node,
    // so this cannot be below 3 (unless that is changed)
    const DEFAULT_LEAF_MIN_DEGREE: u16 = 3;
    const MAX_DEGREE: u64 = 4096;

    const MAX_NODE_BYTES: u64 = 204800; // 200 KB, well bellow the max resource limit.

    /// A node of the BigOrderedMap.
    ///
    /// Inner node will have all children be Child::Inner, pointing to the child nodes.
    /// Leaf node will have all children be Child::Leaf.
    /// Basically - Leaf node is a single-resource OrderedMap, containing as much keys as can fit.
    /// So Leaf node contains multiple values, not just one.
    struct Node<K: store, V: store> has store {
        // Whether this node is a leaf node.
        is_leaf: bool,
        // The children of the nodes.
        // When node is inner node, K represents max_key within the child subtree, and values are Child::Inner.
        // When the node is leaf node, K represents key of the leaf, and values are Child::Leaf.
        children: OrderedMap<K, Child<V>>,
        // The node index of its previous node at the same level, or `null_ref()` if it doesn't have a previous node.
        prev: RefToSlot,
        // The node index of its next node at the same level, or `null_ref()` if it doesn't have a next node.
        next: RefToSlot,
    }

    /// The metadata of a child of a node.
    enum Child<V: store> has store {
        Inner {
            // The node index of it's child
            node_index: StoredSlot,
        },
        Leaf {
            // Value associated with the leaf node.
            value: V,
        }
    }

    /// An iterator to iterate all keys in the BigOrderedMap.
    enum Iterator<K> has drop {
        End,
        Some {
            /// The node index of the iterator pointing to.
            node_index: RefToSlot,

            /// Child iter it is pointing to
            child_iter: ordered_map::Iterator,

            /// `key` to which `(node_index, child_iter)` are pointing to
            /// cache to not require borrowing global resources to fetch again
            key: K,
        },
    }

    /// The BigOrderedMap data structure.
    enum BigOrderedMap<K: store, V: store> has store {
        BPlusTreeMap {
            root: Node<K, V>,
            /// The node index of the root node.
            root_index: RefToSlot,
            /// Mapping of node_index -> node.
            nodes: StorageSlotsAllocator<Node<K, V>>,
            /// The node index of the leftmost node.
            min_leaf_index: RefToSlot,
            /// The node index of the rightmost node.
            max_leaf_index: RefToSlot,

            /// Whether Key and Value have constant serialized size, and if so
            /// optimize out size checks on every insert, if so.
            constant_kv_size: bool,
            /// The max number of children an inner node can have.
            inner_max_degree: u16,
            /// The max number of children a leaf node can have.
            leaf_max_degree: u16,
        }
    }

    // ======================= Constructors && Destructors ====================

    /// Returns a new BigOrderedMap with the default configuration.
    public fun new<K: store, V: store>(): BigOrderedMap<K, V> {
        new_with_config(0, 0, false, 0)
    }

    /// Returns a new BigOrderedMap with the provided max degree consts (the maximum # of children a node can have).
    /// If 0 is passed, then it is dynamically computed based on size of first key and value.
    public fun new_with_config<K: store, V: store>(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool, num_to_preallocate: u32): BigOrderedMap<K, V> {
        assert!(inner_max_degree == 0 || inner_max_degree >= DEFAULT_INNER_MIN_DEGREE, error::invalid_argument(EINVALID_CONFIG_PARAMETER));
        assert!(leaf_max_degree == 0 || leaf_max_degree >= DEFAULT_LEAF_MIN_DEGREE, error::invalid_argument(EINVALID_CONFIG_PARAMETER));
        assert!(reuse_slots || num_to_preallocate == 0, error::invalid_argument(EINVALID_CONFIG_PARAMETER));

        let nodes = storage_slots_allocator::new(storage_slots_allocator::new_config(reuse_slots, num_to_preallocate));

        let root_ref = storage_slots_allocator::special_ref();
        let self = BigOrderedMap::BPlusTreeMap {
            root: new_node(/*is_leaf=*/true),
            root_index: root_ref,
            nodes: nodes,
            min_leaf_index: root_ref,
            max_leaf_index: root_ref,
            constant_kv_size: false,
            inner_max_degree: inner_max_degree,
            leaf_max_degree: leaf_max_degree
        };
        self.validate_static_size_and_init_max_degrees();
        self
    }

    /// Destroys the map if it's empty, otherwise aborts.
    public fun destroy_empty<K: store, V: store>(self: BigOrderedMap<K, V>) {
        let BigOrderedMap::BPlusTreeMap { root, nodes, root_index: _, min_leaf_index: _, max_leaf_index: _, constant_kv_size: _, inner_max_degree: _, leaf_max_degree: _ } = self;
        root.destroy_empty_node();
        nodes.destroy();
    }

    // ======================= Section with Modifiers =========================

    /// Inserts the key/value into the BigOrderedMap.
    /// Aborts if the key is already in the map.
    public fun add<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K, value: V) {
        self.add_or_upsert_impl(key, value, false).destroy_none()
    }

    /// If the key doesn't exist in the map, inserts the key/value, and returns none.
    /// Otherwise updates the value under the given key, and returns the old value.
    public fun upsert<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K, value: V): Option<V> {
        let result = self.add_or_upsert_impl(key, value, true);
        if (result.is_some()) {
            let Child::Leaf {
                value: old_value,
            } = result.destroy_some();
            option::some(old_value)
        } else {
            result.destroy_none();
            option::none()
        }
    }

    /// Removes the entry from BigOrderedMap and returns the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: &K): V {
        let path_to_leaf = self.find_leaf_with_path(key);

        assert!(!path_to_leaf.is_empty(), error::invalid_argument(EKEY_NOT_FOUND));

        let Child::Leaf {
            value,
        } = self.remove_at(path_to_leaf, key);

        value
    }

    // ============================= Accessors ================================

    /// Returns an iterator pointing to the first element that is greater or equal to the provided
    /// key, or an end iterator if such element doesn't exist.
    public fun lower_bound<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Iterator<K> {
        let leaf = self.find_leaf(key);
        if (leaf.ref_is_null()) {
            return self.new_end_iter()
        };

        let node = self.borrow_node(leaf);
        assert!(node.is_leaf, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let child_lower_bound = node.children.lower_bound(key);
        if (child_lower_bound.iter_is_end(&node.children)) {
            self.new_end_iter()
        } else {
            let iter_key = *child_lower_bound.iter_borrow_key(&node.children);
            new_iter(leaf, child_lower_bound, iter_key)
        }
    }

    /// Returns an iterator pointing to the element that equals to the provided key, or an end
    /// iterator if the key is not found.
    public fun find<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Iterator<K> {
        let lower_bound = self.lower_bound(key);
        if (lower_bound.iter_is_end(self)) {
            lower_bound
        } else if (&lower_bound.key == key) {
            lower_bound
        } else {
            self.new_end_iter()
        }
    }

    /// Returns true iff the key exists in the map.
    public fun contains<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): bool {
        let lower_bound = self.lower_bound(key);
        if (lower_bound.iter_is_end(self)) {
            false
        } else if (&lower_bound.key == key) {
            true
        } else {
            false
        }
    }

    /// Returns a reference to the element with its key, aborts if the key is not found.
    public fun borrow<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: K): &V {
        let iter = self.find(&key);

        assert!(iter.iter_is_end(self), error::invalid_argument(EKEY_NOT_FOUND));
        let children = &self.borrow_node(iter.node_index).children;
        &iter.child_iter.iter_borrow(children).value
    }

    // TODO: Add back for fixed sized structs only.
    // /// Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.
    // public fun borrow_mut<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K): &mut V {
    //     let iter = self.find(&key);
    //
    //     assert!(iter.iter_is_end(self), error::invalid_argument(EKEY_NOT_FOUND));
    //     let children = &mut self.nodes.borrow_mut(iter.node_index).children;
    //     &mut iter.child_iter.iter_borrow_mut(children).value
    // }

    // ========================= Iterator functions ===========================

    /// Return the begin iterator.
    public fun new_begin_iter<K: copy + store, V: store>(self: &BigOrderedMap<K, V>): Iterator<K> {
        if (self.is_empty()) {
            return Iterator::End;
        };

        let node = self.borrow_node(self.min_leaf_index);
        assert!(!node.children.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        let begin_child_iter = node.children.new_begin_iter();
        let begin_child_key = *begin_child_iter.iter_borrow_key(&node.children);
        new_iter(self.min_leaf_index, begin_child_iter, begin_child_key)
    }

    /// Return the end iterator.
    public fun new_end_iter<K: copy + store, V: store>(self: &BigOrderedMap<K, V>): Iterator<K> {
        Iterator::End
    }

    // Returns true iff the iterator is a begin iterator.
    public fun iter_is_begin<K: store, V: store>(self: &Iterator<K>, map: &BigOrderedMap<K, V>): bool {
        if (self is Iterator::End<K>) {
            map.is_empty()
        } else {
            (self.node_index == map.min_leaf_index && self.child_iter.iter_is_begin_from_non_empty())
        }
    }

    // Returns true iff the iterator is an end iterator.
    public fun iter_is_end<K: store, V: store>(self: &Iterator<K>, _map: &BigOrderedMap<K, V>): bool {
        self is Iterator::End<K>
    }

    /// Returns the key of the given iterator.
    public fun iter_get_key<K>(self: &Iterator<K>): &K {
        assert!(!(self is Iterator::End<K>), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        &self.key
    }

    /// Returns the next iterator, or none if already at the end iterator.
    /// Requires the map is not changed after the input iterator is generated.
    public fun iter_next<K: drop + copy + store, V: store>(self: Iterator<K>, map: &BigOrderedMap<K, V>): Iterator<K> {
        assert!(!(self is Iterator::End<K>), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let node_index = self.node_index;
        let node = map.borrow_node(node_index);

        let child_iter = self.child_iter.iter_next(&node.children);
        if (!child_iter.iter_is_end(&node.children)) {
            let iter_key = *child_iter.iter_borrow_key(&node.children);
            return new_iter(node_index, child_iter, iter_key);
        };

        let next_index = node.next;
        if (!next_index.ref_is_null()) {
            let next_node = map.borrow_node(next_index);

            let child_iter = next_node.children.new_begin_iter();
            assert!(!child_iter.iter_is_end(&next_node.children), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            let iter_key = *child_iter.iter_borrow_key(&next_node.children);
            return new_iter(next_index, child_iter, iter_key);
        };

        new_end_iter(map)
    }

    /// Returns the previous iterator, or none if already at the begin iterator.
    /// Requires the map is not changed after the input iterator is generated.
    public fun iter_prev<K: drop + copy + store, V: store>(self: Iterator<K>, map: &BigOrderedMap<K, V>): Iterator<K> {
        let prev_index = if (self is Iterator::End<K>) {
            map.max_leaf_index
        } else {
            let node_index = self.node_index;
            let node = map.borrow_node(node_index);

            if (!self.child_iter.iter_is_begin(&node.children)) {
                let child_iter = self.child_iter.iter_prev(&node.children);
                let key = *child_iter.iter_borrow_key(&node.children);
                return new_iter(node_index, child_iter, key);
            };
            node.prev
        };

        assert!(!prev_index.ref_is_null(), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let prev_node = map.borrow_node(prev_index);

        let prev_children = &prev_node.children;
        let child_iter = prev_children.new_end_iter().iter_prev(prev_children);
        let iter_key = *child_iter.iter_borrow_key(prev_children);
        new_iter(prev_index, child_iter, iter_key)
    }

    // ====================== Internal Implementations ========================

    inline fun borrow_node<K: store, V: store>(self: &BigOrderedMap<K, V>, node: RefToSlot): &Node<K, V> {
        if (self.root_index == node) {
            &self.root
        } else {
            self.nodes.borrow(node)
        }
    }

    inline fun borrow_node_mut<K: store, V: store>(self: &mut BigOrderedMap<K, V>, node: RefToSlot): &mut Node<K, V> {
        if (self.root_index == node) {
            &mut self.root
        } else {
            self.nodes.borrow_mut(node)
        }
    }

    fun add_or_upsert_impl<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K, value: V, allow_overwrite: bool): Option<Child<V>> {
        if (!self.constant_kv_size) {
            self.validate_dynamic_size_and_init_max_degrees(&key, &value);
        };

        let path_to_leaf = self.find_leaf_with_path(&key);

        if (path_to_leaf.is_empty()) {
            // In this case, the key is greater than all keys in the map.

            let current = self.root_index;

            loop {
                path_to_leaf.push_back(current);

                let current_node = self.borrow_node_mut(current);
                if (current_node.is_leaf) {
                    break;
                };
                let last_value = current_node.children.new_end_iter().iter_prev(&current_node.children).iter_remove(&mut current_node.children);
                current = last_value.node_index.stored_as_ref();
                current_node.children.add(key, last_value);
            };
        };

        // aptos_std::debug::print(&std::string::utf8(b"add_or_upsert_impl::path_to_leaf"));
        // aptos_std::debug::print(&path_to_leaf);
        self.add_at(path_to_leaf, key, new_leaf_child(value), allow_overwrite)
    }

    fun validate_dynamic_size_and_init_max_degrees<K: store, V: store>(self: &mut BigOrderedMap<K, V>, key: &K, value: &V) {
        let key_size = bcs::serialized_size(key);
        let value_size = bcs::serialized_size(value);
        self.validate_size_and_init_max_degrees(key_size, value_size)
    }

    fun validate_static_size_and_init_max_degrees<K: store, V: store>(self: &mut BigOrderedMap<K, V>) {
        let key_size = bcs::constant_serialized_size<K>();
        let value_size = bcs::constant_serialized_size<V>();

        if (key_size.is_some() && value_size.is_some()) {
            self.validate_size_and_init_max_degrees(key_size.destroy_some(), value_size.destroy_some());
            self.constant_kv_size = true;
        };
    }

    fun validate_size_and_init_max_degrees<K: store, V: store>(self: &mut BigOrderedMap<K, V>, key_size: u64, value_size: u64) {
        let entry_size = key_size + value_size;

        if (self.inner_max_degree == 0) {
            self.inner_max_degree = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / key_size), DEFAULT_INNER_MIN_DEGREE as u64) as u16;
        };

        if (self.leaf_max_degree == 0) {
            self.leaf_max_degree = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / entry_size), DEFAULT_LEAF_MIN_DEGREE as u64) as u16;
        };

        assert!(key_size * (self.inner_max_degree as u64) <= MAX_NODE_BYTES, error::invalid_argument(EARGUMENT_BYTES_TOO_LARGE));
        assert!(entry_size * (self.leaf_max_degree as u64) <= MAX_NODE_BYTES, error::invalid_argument(EARGUMENT_BYTES_TOO_LARGE));
    }

    fun destroy_inner_child<V: store>(self: Child<V>): StoredSlot {
        let Child::Inner {
            node_index,
        } = self;

        node_index
    }

    fun destroy_empty_node<K: store, V: store>(self: Node<K, V>) {
        let Node { children, is_leaf: _, prev: _, next: _ } = self;
        assert!(children.is_empty(), error::invalid_argument(EMAP_NOT_EMPTY));
        children.destroy_empty();
    }

    fun new_node<K: store, V: store>(is_leaf: bool): Node<K, V> {
        Node {
            is_leaf: is_leaf,
            children: ordered_map::new(),
            prev: storage_slots_allocator::null_ref(),
            next: storage_slots_allocator::null_ref(),
        }
    }

    fun new_node_with_children<K: store, V: store>(is_leaf: bool, children: OrderedMap<K, Child<V>>): Node<K, V> {
        Node {
            is_leaf: is_leaf,
            children: children,
            prev: storage_slots_allocator::null_ref(),
            next: storage_slots_allocator::null_ref(),
        }
    }

    fun new_inner_child<V: store>(node_index: StoredSlot): Child<V> {
        Child::Inner {
            node_index: node_index,
        }
    }

    fun new_leaf_child<V: store>(value: V): Child<V> {
        Child::Leaf {
            value: value,
        }
    }

    fun new_iter<K>(node_index: RefToSlot, child_iter: ordered_map::Iterator, key: K): Iterator<K> {
        Iterator::Some {
            node_index: node_index,
            child_iter: child_iter,
            key: key,
        }
    }

    fun find_leaf<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): RefToSlot {
        let current = self.root_index;
        while (!current.ref_is_null()) {
            let node = self.borrow_node(current);
            if (node.is_leaf) {
                return current;
            };
            let children = &node.children;
            let child_iter = children.lower_bound(key);
            if (child_iter.iter_is_end(children)) {
                return storage_slots_allocator::null_ref();
            } else {
                current = child_iter.iter_borrow(children).node_index.stored_as_ref();
            }
        };

        storage_slots_allocator::null_ref()
    }

    fun find_leaf_with_path<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): vector<RefToSlot> {
        let vec = vector::empty();

        let current = self.root_index;
        while (!current.ref_is_null()) {
            vec.push_back(current);

            let node = self.borrow_node(current);
            if (node.is_leaf) {
                return vec;
            };
            let children = &node.children;
            let child_iter = children.lower_bound(key);
            if (child_iter.iter_is_end(children)) {
                return vector::empty();
            } else {
                current = child_iter.iter_borrow(children).node_index.stored_as_ref();
            }
        };

        abort error::invalid_state(EINTERNAL_INVARIANT_BROKEN)
    }

    fun get_max_degree<K: store, V: store>(self: &BigOrderedMap<K, V>, leaf: bool): u64 {
        if (leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        }
    }

    fun add_at<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, path_to_node: vector<RefToSlot>, key: K, child: Child<V>, allow_overwrite: bool): Option<Child<V>> {
        let node_index = path_to_node.pop_back();
        {
            let node = self.borrow_node_mut(node_index);
            let children = &mut node.children;
            let current_size = children.length();

            let max_degree = if (node.is_leaf) {
                self.leaf_max_degree as u64
            } else {
                self.inner_max_degree as u64
            };

            if (current_size < max_degree) {
                let result = children.upsert(key, child);

                if (node.is_leaf) {
                    assert!(allow_overwrite || result.is_none(), error::invalid_argument(EKEY_ALREADY_EXISTS));
                    return result;
                } else {
                    assert!(!allow_overwrite && result.is_none(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
                    return result;
                };
            };

            if (allow_overwrite) {
                let iter = children.find(&key);
                if (!iter.iter_is_end(children)) {
                    return option::some(iter.iter_replace(children, child));
                }
            }
        };

        // # of children in the current node exceeds the threshold, need to split into two nodes.

        let (right_node_slot, node) = if (path_to_node.is_empty()) {
            // If we are at the root, we need to move root node to become a child and have a new root node.

            assert!(node_index == self.root_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            // aptos_std::debug::print(&std::string::utf8(b"changing root"));

            // Splitting root now, need to create a new root.
            // We keep root_index always the same
            let new_root_node = new_node<K, V>(/*is_leaf=*/false);

            let (replacement_node_stored_slot, replacement_node_slot) = self.nodes.reserve_slot();
            // aptos_std::debug::print(&replacement_node_slot);

            let root_children = &self.root.children;
            let max_element = *root_children.new_end_iter().iter_prev(root_children).iter_borrow_key(root_children);
            if (cmp::compare(&max_element, &key).is_less_than()) {
                max_element = key;
            };
            new_root_node.children.add(max_element, new_inner_child(replacement_node_stored_slot));

            // aptos_std::debug::print(&cur_node_slot);
            path_to_node.push_back(self.root_index);

            let node = mem::replace(&mut self.root, new_root_node);

            let replacement_ref = replacement_node_slot.reserved_as_ref();
            if (node.is_leaf) {
                self.min_leaf_index = replacement_ref;
                self.max_leaf_index = replacement_ref;
            };
            (replacement_node_slot, node)
        } else {
            let (cur_node_slot, node) = self.nodes.remove_and_reserve(node_index);
            (cur_node_slot, node)
        };

        // aptos_std::debug::print(&std::string::utf8(b"node that needs to be split"));
        // aptos_std::debug::print(&node);

        move node_index;
        let is_leaf = node.is_leaf;
        let children = &mut node.children;

        let right_node_ref = right_node_slot.reserved_as_ref();
        let next = &mut node.next;
        let prev = &mut node.prev;

        let max_degree = if (is_leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        };
        let target_size = (max_degree + 1) / 2;

        children.add(key, child);
        let new_node_children = children.trim(target_size);

        assert!(children.length() <= max_degree, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        assert!(new_node_children.length() <= max_degree, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let right_node = new_node_with_children(is_leaf, new_node_children);

        let (left_node_stored_slot, left_node_slot) = self.nodes.reserve_slot();
        let left_node_ref = left_node_stored_slot.stored_as_ref();
        right_node.next = *next;
        *next = right_node_ref;
        right_node.prev = left_node_ref;
        if (!prev.ref_is_null()) {
            self.nodes.borrow_mut(*prev).next = left_node_ref;
        };

        let split_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);

        // aptos_std::debug::print(&std::string::utf8(b"creating right node"));
        // aptos_std::debug::print(&right_node_slot);
        // aptos_std::debug::print(&right_node);

        // aptos_std::debug::print(&std::string::utf8(b"updating left node"));
        // aptos_std::debug::print(&left_node_slot);
        // aptos_std::debug::print(&node);

        self.nodes.fill_reserved_slot(left_node_slot, node);
        self.nodes.fill_reserved_slot(right_node_slot, right_node);

        if (right_node_ref == self.min_leaf_index) {
            self.min_leaf_index = left_node_ref;
        };
        self.add_at(path_to_node, split_key, new_inner_child(left_node_stored_slot), false).destroy_none();
        option::none()
    }

    fun update_key<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, path_to_node: vector<RefToSlot>, old_key: &K, new_key: K) {
        if (path_to_node.is_empty()) {
            return
        };

        let node_index = path_to_node.pop_back();
        let node = self.borrow_node_mut(node_index);
        let children = &mut node.children;
        children.replace_key_inplace(old_key, new_key);

        if (children.new_end_iter().iter_prev(children).iter_borrow_key(children) == &new_key) {
            self.update_key(path_to_node, old_key, new_key);
        };
    }

    fun remove_at<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, path_to_node: vector<RefToSlot>, key: &K): Child<V> {
        let node_index = path_to_node.pop_back();
        let old_child = {
            let node = self.borrow_node_mut(node_index);

            let children = &mut node.children;

            let is_leaf = node.is_leaf;

            let old_child = children.remove(key);
            if (path_to_node.is_empty()) {
                assert!(node_index == self.root_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

                if (!is_leaf && children.length() == 1) {
                    // promote only child to root, and drop current root.
                    // keep the root index the same.
                    let Child::Inner {
                        node_index: inner_child_index,
                    } = children.new_end_iter().iter_prev(children).iter_remove(children);
                    move children;
                    move node;

                    let inner_child = self.nodes.remove(inner_child_index);
                    if (inner_child.is_leaf) {
                        let root_ref = self.root_index;
                        self.min_leaf_index = root_ref;
                        self.max_leaf_index = root_ref;
                    };

                    mem::replace(&mut self.root, inner_child).destroy_empty_node();
                }; // else: nothing to change
                return old_child;
            };

            let max_degree = if (is_leaf) {
                self.leaf_max_degree as u64
            } else {
                self.inner_max_degree as u64
            };

            let current_size = children.length();
            let big_enough = current_size * 2 >= max_degree;

            let new_max_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
            let max_key_updated = cmp::compare(&new_max_key, key).is_less_than();
            if (!max_key_updated && big_enough) {
                return old_child;
            };

            if (max_key_updated) {
                assert!(current_size >= 1, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

                self.update_key(path_to_node, key, new_max_key);

                if (big_enough) {
                    return old_child;
                }
            };

            old_child
        };

        // Children size is below threshold, we need to rebalance

        let (node_slot, node) = self.nodes.remove_and_reserve(node_index);

        let is_leaf = node.is_leaf;
        let max_degree = self.get_max_degree(is_leaf);
        let prev = node.prev;
        let next = node.next;

        let brother_index = {
            let parent_children = &self.borrow_node(*path_to_node.borrow(path_to_node.length() - 1)).children;
            if (parent_children.new_end_iter().iter_prev(parent_children).iter_borrow(parent_children).node_index.stored_as_ref() == node_index) {
                prev
            } else {
                next
            }
        };

        let children = &mut node.children;
        let (brother_slot, brother_node) = self.nodes.remove_and_reserve(brother_index);

        let brother_children = &mut brother_node.children;

        if ((brother_children.length() - 1) * 2 >= max_degree) {
            // The brother node has enough elements, borrow an element from the brother node.
            if (brother_index == next) {
                let old_max_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
                let brother_begin_iter = brother_children.new_begin_iter();
                let borrowed_max_key = *brother_begin_iter.iter_borrow_key(brother_children);
                let borrowed_element = brother_begin_iter.iter_remove(brother_children);

                children.add(borrowed_max_key, borrowed_element);
                self.update_key(path_to_node, &old_max_key, borrowed_max_key);
            } else {
                let brother_end_iter = brother_children.new_end_iter().iter_prev(brother_children);
                let borrowed_max_key = *brother_end_iter.iter_borrow_key(brother_children);
                let borrowed_element = brother_end_iter.iter_remove(brother_children);

                children.add(borrowed_max_key, borrowed_element);
                self.update_key(path_to_node, &borrowed_max_key, *brother_children.new_end_iter().iter_prev(brother_children).iter_borrow_key(brother_children));
            };

            self.nodes.fill_reserved_slot(node_slot, node);
            self.nodes.fill_reserved_slot(brother_slot, brother_node);
            return old_child;
        };

        // The brother node doesn't have enough elements to borrow, merge with the brother node.
        if (brother_index == next) {
            let Node { children: brother_children, is_leaf: _, prev: _, next: brother_next } = brother_node;
            let key_to_remove = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
            children.append(brother_children);
            node.next = brother_next;

            move children;

            if (!node.next.ref_is_null()) {
                self.nodes.borrow_mut(node.next).prev = brother_index;
            };
            if (!node.prev.ref_is_null()) {
                self.nodes.borrow_mut(node.prev).next = brother_index;
            };

            self.nodes.fill_reserved_slot(brother_slot, node);

            if (self.min_leaf_index == node_index) {
                self.min_leaf_index = brother_index;
            };

            assert!(!path_to_node.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            let node_stored_slot = destroy_inner_child(self.remove_at(path_to_node, &key_to_remove));
            self.nodes.free_reserved_slot(node_slot, node_stored_slot);
        } else {
            let Node { children: node_children, is_leaf: _, prev: _, next: node_next } = node;
            let key_to_remove = *brother_children.new_end_iter().iter_prev(brother_children).iter_borrow_key(brother_children);
            brother_children.append(node_children);
            brother_node.next = node_next;

            move brother_children;

            if (!brother_node.next.ref_is_null()) {
                self.nodes.borrow_mut(brother_node.next).prev = node_index;
            };
            if (!brother_node.prev.ref_is_null()) {
                self.nodes.borrow_mut(brother_node.prev).next = node_index;
            };

            self.nodes.fill_reserved_slot(node_slot, brother_node);

            if (self.min_leaf_index == brother_index) {
                self.min_leaf_index = node_index;
            };

            assert!(!path_to_node.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            let node_stored_slot = destroy_inner_child(self.remove_at(path_to_node, &key_to_remove));
            self.nodes.free_reserved_slot(brother_slot, node_stored_slot);
        };
        old_child
    }

    /// Returns the number of elements in the BigOrderedMap.
    fun length<K: store, V: store>(self: &BigOrderedMap<K, V>): u64 {
        self.length_for_node(self.root_index)
    }

    fun length_for_node<K: store, V: store>(self: &BigOrderedMap<K, V>, node_index: RefToSlot): u64 {
        let node = self.borrow_node(node_index);
        if (node.is_leaf) {
            node.children.length()
        } else {
            let size = 0;

            node.children.for_each_ref(|_key, child| {
                size = size + self.length_for_node(child.node_index.stored_as_ref());
            });
            size
        }
    }

    /// Returns true iff the BigOrderedMap is empty.
    fun is_empty<K: store, V: store>(self: &BigOrderedMap<K, V>): bool {
        let node = self.borrow_node(self.min_leaf_index);

        node.children.is_empty()
    }

    // ============================= Tests ====================================

    #[test_only]
    fun print_map<K: store, V: store>(self: &BigOrderedMap<K, V>) {
        aptos_std::debug::print(&std::string::utf8(b"print map"));
        aptos_std::debug::print(self);
        self.print_map_for_node(self.root_index, 0);
    }

    #[test_only]
    fun print_map_for_node<K: store, V: store>(self: &BigOrderedMap<K, V>, node_index: RefToSlot, level: u64) {
        let node = self.borrow_node(node_index);

        aptos_std::debug::print(&level);
        aptos_std::debug::print(&node_index);
        aptos_std::debug::print(node);

        if (!node.is_leaf) {
            node.children.for_each_ref(|_key, node| {
                self.print_map_for_node(node.node_index.stored_as_ref(), level + 1);
            });
        };
    }

    #[test_only]
    fun destroy<K: drop + copy + store, V: drop + store>(self: BigOrderedMap<K, V>) {
        let it = new_begin_iter(&self);
        while (!it.iter_is_end(&self)) {
            remove(&mut self, it.iter_get_key());
            assert!(find(&self, it.iter_get_key()).iter_is_end(&self), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            it = new_begin_iter(&self);
            self.validate_map();
        };

        self.destroy_empty();
    }

    #[test_only]
    fun validate_iteration<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>) {
        let expected_num_elements = self.length();
        let num_elements = 0;
        let it = new_begin_iter(self);
        while (!it.iter_is_end(self)) {
            num_elements = num_elements + 1;
            it = it.iter_next(self);
        };

        assert!(num_elements == expected_num_elements, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let num_elements = 0;
        let it = new_end_iter(self);
        while (!it.iter_is_begin(self)) {
            it = it.iter_prev(self);
            num_elements = num_elements + 1;
        };
        assert!(num_elements == expected_num_elements, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let it = new_end_iter(self);
        if (!it.iter_is_begin(self)) {
            it = it.iter_prev(self);
            assert!(it.node_index == self.max_leaf_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        } else {
            assert!(expected_num_elements == 0, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };
    }

    #[test_only]
    fun validate_subtree<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, node_index: RefToSlot, expected_lower_bound_key: Option<K>, expected_max_key: Option<K>) {
        let node = self.borrow_node(node_index);
        let len = node.children.length();
        assert!(len <= self.get_max_degree(node.is_leaf), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        if (node_index != self.root_index) {
            assert!(len >= 1, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            assert!(len * 2 >= self.get_max_degree(node.is_leaf) || node_index == self.root_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };

        node.children.validate_ordered();

        let previous_max_key = expected_lower_bound_key;
        node.children.for_each_ref(|key: &K, child: &Child<V>| {
            if (!node.is_leaf) {
                self.validate_subtree(child.node_index.stored_as_ref(), previous_max_key, option::some(*key));
            } else {
                assert!((child is Child::Leaf<V>), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            };
            previous_max_key = option::some(*key);
        });

        if (option::is_some(&expected_max_key)) {
            let expected_max_key = option::extract(&mut expected_max_key);
            assert!(&expected_max_key == node.children.new_end_iter().iter_prev(&node.children).iter_borrow_key(&node.children), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };

        if (option::is_some(&expected_lower_bound_key)) {
            let expected_lower_bound_key = option::extract(&mut expected_lower_bound_key);
            assert!(cmp::compare(&expected_lower_bound_key, node.children.new_begin_iter().iter_borrow_key(&node.children)).is_less_than(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };
    }

    #[test_only]
    fun validate_map<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>) {
        self.validate_subtree(self.root_index, option::none(), option::none());
        self.validate_iteration();
    }

    #[test]
    fun test_small_example() {
        let map = new_with_config(5, 3, true, 2);
        map.print_map(); map.validate_map();
        add(&mut map, 1, 1); map.print_map(); map.validate_map();
        add(&mut map, 2, 2); map.print_map(); map.validate_map();
        let r1 = upsert(&mut map, 3, 3); map.print_map(); map.validate_map();
        assert!(r1 == option::none(), 1);
        add(&mut map, 4, 4); map.print_map(); map.validate_map();
        let r2 = upsert(&mut map, 4, 8); map.print_map(); map.validate_map();
        assert!(r2 == option::some(4), 2);
        add(&mut map, 5, 5); map.print_map(); map.validate_map();
        add(&mut map, 6, 6); map.print_map(); map.validate_map();

        remove(&mut map, &5); map.print_map(); map.validate_map();
        remove(&mut map, &4); map.print_map(); map.validate_map();
        remove(&mut map, &1); map.print_map(); map.validate_map();
        remove(&mut map, &3); map.print_map(); map.validate_map();
        remove(&mut map, &2); map.print_map(); map.validate_map();
        remove(&mut map, &6); map.print_map(); map.validate_map();

        destroy_empty(map);
    }

    #[test]
    fun test_deleting_and_creating_nodes() {
        let map = new_with_config(4, 3, true, 2);

        for (i in 0..50) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 0..40) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 50..100) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 50..90) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 100..150) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 100..150) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 40..50) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 90..100) {
            map.remove(&i);
            map.validate_map();
        };

        destroy_empty(map);
    }

    #[test]
    fun test_iterator() {
        let map = new_with_config(5, 5, true, 2);

        let data = vector[1, 7, 5, 8, 4, 2, 6, 3, 9, 0];
        while (data.length() != 0) {
            let element = data.pop_back();
            add(&mut map, element, element);
        };

        let it = new_begin_iter(&map);

        let i = 0;
        while (!it.iter_is_end(&map)) {
            assert!(i == it.key, i);
            i = i + 1;
            it = it.iter_next(&map);
        };

        destroy(map);
    }

    #[test]
    fun test_find() {
        let map = new_with_config(5, 5, true, 2);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 0, 10];

        let i = 0;
        let len = data.length();
        while (i < len) {
            let element = *data.borrow(i);
            map.add(element, element);
            i = i + 1;
        };

        let i = 0;
        while (i < len) {
            let element = data.borrow(i);
            let it = find(&map, element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.iter_get_key() == element, i);
            i = i + 1;
        };

        assert!(find(&map, &4).iter_is_end(&map), 0);
        assert!(find(&map, &9).iter_is_end(&map), 1);

        destroy(map);
    }

    #[test]
    fun test_lower_bound() {
        let map = new_with_config(5, 5, true, 2);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 12, 10];

        let i = 0;
        let len = data.length();
        while (i < len) {
            let element = *data.borrow(i);
            add(&mut map, element, element);
            i = i + 1;
        };

        let i = 0;
        while (i < len) {
            let element = *data.borrow(i);
            let it = lower_bound(&map, &element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.key == element, i);
            i = i + 1;
        };

        assert!(lower_bound(&map, &0).key == 1, 0);
        assert!(lower_bound(&map, &4).key == 5, 1);
        assert!(lower_bound(&map, &9).key == 10, 2);
        assert!(lower_bound(&map, &13).iter_is_end(&map), 3);

        remove(&mut map, &3);
        assert!(lower_bound(&map, &3).key == 5, 4);
        remove(&mut map, &5);
        assert!(lower_bound(&map, &3).key == 6, 5);
        assert!(lower_bound(&map, &4).key == 6, 6);

        destroy(map);
    }

    #[test_only]
    fun test_large_data_set_helper(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool) {
        use std::vector;

        let map = new_with_config(inner_max_degree, leaf_max_degree, reuse_slots, if (reuse_slots) {4} else {0});
        let data = ordered_map::large_dataset();
        let shuffled_data = ordered_map::large_dataset_shuffled();

        let len = data.length();
        for (i in 0..len) {
            let element = *data.borrow(i);
            map.upsert(element, element);
            map.validate_map();
        };

        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            let it = map.find(element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.iter_get_key() == element, i);

            // aptos_std::debug::print(&it);

            let it_next = it.iter_next(&map);
            let it_after = map.lower_bound(&(*element + 1));

            // aptos_std::debug::print(&it_next);
            // aptos_std::debug::print(&it_after);
            // aptos_std::debug::print(&std::string::utf8(b"bla"));

            assert!(it_next == it_after, i);
        };

        let removed = vector::empty();
        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            if (!removed.contains(element)) {
                removed.push_back(*element);
                map.remove(element);
                map.validate_map();
            } else {
                assert!(!map.contains(element));
            };
        };

        map.destroy_empty();
    }

    #[test]
    fun test_large_data_set_order_5_false() {
        test_large_data_set_helper(5, 5, false);
    }

    #[test]
    fun test_large_data_set_order_5_true() {
        test_large_data_set_helper(5, 5, true);
    }

    #[test]
    fun test_large_data_set_order_4_3_false() {
        test_large_data_set_helper(4, 3, false);
    }

    #[test]
    fun test_large_data_set_order_4_3_true() {
        test_large_data_set_helper(4, 3, true);
    }

    #[test]
    fun test_large_data_set_order_4_4_false() {
        test_large_data_set_helper(4, 4, false);
    }

    #[test]
    fun test_large_data_set_order_4_4_true() {
        test_large_data_set_helper(4, 4, true);
    }

    #[test]
    fun test_large_data_set_order_6_false() {
        test_large_data_set_helper(6, 6, false);
    }

    #[test]
    fun test_large_data_set_order_6_true() {
        test_large_data_set_helper(6, 6, true);
    }

    #[test]
    fun test_large_data_set_order_6_3_false() {
        test_large_data_set_helper(6, 3, false);
    }

    #[test]
    fun test_large_data_set_order_6_3_true() {
        test_large_data_set_helper(6, 3, true);
    }

    #[test]
    fun test_large_data_set_order_4_6_false() {
        test_large_data_set_helper(4, 6, false);
    }

    #[test]
    fun test_large_data_set_order_4_6_true() {
        test_large_data_set_helper(4, 6, true);
    }

    #[test]
    fun test_large_data_set_order_16_false() {
        test_large_data_set_helper(16, 16, false);
    }

    #[test]
    fun test_large_data_set_order_16_true() {
        test_large_data_set_helper(16, 16, true);
    }

    #[test]
    fun test_large_data_set_order_31_false() {
        test_large_data_set_helper(31, 31, false);
    }

    #[test]
    fun test_large_data_set_order_31_true() {
        test_large_data_set_helper(31, 31, true);
    }

    #[test]
    fun test_large_data_set_order_31_3_false() {
        test_large_data_set_helper(31, 3, false);
    }

    #[test]
    fun test_large_data_set_order_31_3_true() {
        test_large_data_set_helper(31, 3, true);
    }

    #[test]
    fun test_large_data_set_order_31_5_false() {
        test_large_data_set_helper(31, 5, false);
    }

    #[test]
    fun test_large_data_set_order_31_5_true() {
        test_large_data_set_helper(31, 5, true);
    }

    #[test]
    fun test_large_data_set_order_32_false() {
        test_large_data_set_helper(32, 32, false);
    }

    #[test]
    fun test_large_data_set_order_32_true() {
        test_large_data_set_helper(32, 32, true);
    }
}
