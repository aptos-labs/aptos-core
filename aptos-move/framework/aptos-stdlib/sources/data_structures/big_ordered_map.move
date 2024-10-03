/// Type of large-scale search trees.
///
/// It internally uses BTree to organize the search tree data structure for keys. Comparing with
/// other common search trees like AVL or Red-black tree, a BTree node has more children, and packs
/// more metadata into one node, which is more disk friendly (and gas friendly).

module aptos_std::big_ordered_map {
    use std::option::{Self, Option};
    use std::bcs;
    use aptos_std::ordered_map::{Self, OrderedMap};
    use aptos_std::cmp;
    use aptos_std::storage_slots_allocator::{Self, StorageSlotsAllocator};
    use aptos_std::math64::{max, min};

    // Internal errors.
    const E_INTERNAL: u64 = 0;
    // The tree is not empty, and cannot be destroyed.
    const E_TREE_NOT_EMPTY: u64 = 1;
    // The tree is too big for insertion.
    const E_TREE_TOO_BIG: u64 = 2;
    // The provided parameter is invalid.
    const E_INVALID_PARAMETER: u64 = 3;

    const EKEY_ALREADY_EXISTS: u64 = 4;

    const NULL_INDEX: u64 = 0;
    const DEFAULT_TARGET_NODE_SIZE: u64 = 2048;
    const DEFAULT_INNER_MIN_DEGREE: u16 = 4;
    // We rely on 1 being valid size only for root node,
    // so this cannot be below 3 (unless that is changed)
    const DEFAULT_LEAF_MIN_DEGREE: u16 = 3;
    const MAX_DEGREE: u64 = 4096;

    /// A node of the BigOrderedMap.
    struct Node<K: store, V: store> has store {
        // Whether this node is a leaf node.
        is_leaf: bool,
        // The node index of its parent node, or NULL_INDEX if it doesn't have parent.
        parent: u64,
        // The children of the nodes.
        // When node is inner node, K represents max_key of the recursive children.
        // When the node is leaf node, K represents key of the leaf.
        children: OrderedMap<K, Child<V>>,
        // The node index of its previous node at the same level, or NULL_INDEX if it doesn't have a previous node.
        prev: u64,
        // The node index of its next node at the same level, or NULL_INDEX if it doesn't have a next node.
        next: u64,
    }

    /// The metadata of a child of a node.
    enum Child<V: store> has store {
        Inner {
            // The node index of it's child, or NULL_INDEX if the current node is a leaf node.
            node_index: u64,
        },
        Leaf {
            value: V,
        }
    }

    /// An iterator to iterate all keys in the BigOrderedMap.
    enum Iterator<K> has drop {
        End,
        Some {
            /// The node index of the iterator pointing to.
            node_index: u64,

            /// Child iter it is pointing to
            child_iter: ordered_map::Iterator,

            /// key (node_index, child_iter) are pointing to
            /// cache to not require borrowing global resources to fetch again
            key: K,
        },
    }

    /// The BigOrderedMap data structure.
    enum BigOrderedMap<K: store, V: store> has store {
        BPlusTreeMap {
            // The node index of the root node.
            root_index: u64,
            // Mapping of node_index -> node.
            nodes: StorageSlotsAllocator<Node<K, V>>,
            // The node index of the leftmost node.
            min_leaf_index: u64,
            // The node index of the rightmost node.
            max_leaf_index: u64,

            // The max number of children an inner node can have.
            inner_max_degree: u16,
            // The max number of children a leaf node can have.
            leaf_max_degree: u16,
        }
    }

    /////////////////////////////////
    // Constructors && Destructors //
    /////////////////////////////////

    /// Returns a new BigOrderedMap with the default configuration.
    public fun new<K: store, V: store>(): BigOrderedMap<K, V> {
        new_with_config(0, 0, false, 0)
    }

    /// Returns a new BigOrderedMap with the provided max degree consts (the maximum # of children a node can have).
    /// If 0 is passed, then it is dynamically computed based on size of first key and value.
    public fun new_with_config<K: store, V: store>(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool, num_to_preallocate: u64): BigOrderedMap<K, V> {
        assert!(inner_max_degree == 0 || inner_max_degree >= DEFAULT_INNER_MIN_DEGREE, E_INVALID_PARAMETER);
        assert!(leaf_max_degree == 0 || leaf_max_degree >= DEFAULT_LEAF_MIN_DEGREE, E_INVALID_PARAMETER);
        let nodes = if (reuse_slots) {
            storage_slots_allocator::new_reuse_storage_slots(num_to_preallocate)
        } else {
            assert!(num_to_preallocate == 0, E_INVALID_PARAMETER);
            storage_slots_allocator::new_storage_slots()
        };
        let root_index = nodes.add(new_node(/*is_leaf=*/true, /*parent=*/NULL_INDEX));
        BigOrderedMap::BPlusTreeMap {
            root_index: root_index,
            nodes: nodes,
            min_leaf_index: root_index,
            max_leaf_index: root_index,
            inner_max_degree: inner_max_degree,
            leaf_max_degree: leaf_max_degree
        }
    }

    /// Destroys the tree if it's empty, otherwise aborts.
    public fun destroy_empty<K: store, V: store>(self: BigOrderedMap<K, V>) {
        let BigOrderedMap::BPlusTreeMap { nodes, root_index, min_leaf_index: _, max_leaf_index: _, inner_max_degree: _, leaf_max_degree: _ } = self;
        // aptos_std::debug::print(&nodes);
        nodes.remove(root_index).destroy_empty_node();
        nodes.destroy();
    }

    ///////////////
    // Modifiers //
    ///////////////

    /// Inserts the key/value into the BigOrderedMap.
    /// Aborts if the key is already in the tree.
    public fun add<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K, value: V) {
        self.add_or_upsert_impl(key, value, false).destroy_none()
    }

    /// If the key doesn't exist in the tree, inserts the key/value, and returns none.
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

    fun add_or_upsert_impl<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K, value: V, allow_overwrite: bool): Option<Child<V>> {
        if (self.inner_max_degree == 0 || self.leaf_max_degree == 0) {
            self.init_max_degrees(&key, &value);
        };

        let leaf = self.find_leaf(&key);

        if (leaf == NULL_INDEX) {
            // In this case, the key is greater than all keys in the tree.
            leaf = self.max_leaf_index;
            let current = self.nodes.borrow(leaf).parent;
            while (current != NULL_INDEX) {
                let current_node = self.nodes.borrow_mut(current);

                let last_value = current_node.children.new_end_iter().iter_prev(&current_node.children).iter_remove(&mut current_node.children);
                current_node.children.add(key, last_value);
                current = current_node.parent;
            }
        };

        self.add_at(leaf, key, new_leaf_child(value), allow_overwrite)
    }

    /// Removes the entry from BigOrderedMap and returns the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: &K): V {
        let iter = self.find(key);
        assert!(!is_end_iter(self, &iter), E_INTERNAL);

        let Child::Leaf {
            value,
        } = self.remove_at(iter.node_index, key);

        value
    }

    ///////////////
    // Accessors //
    ///////////////

    // Returns true iff the iterator is a begin iterator.
    public fun is_begin_iter<K: store, V: store>(tree: &BigOrderedMap<K, V>, iter: &Iterator<K>): bool {
        if (iter is Iterator::End<K>) {
            tree.is_empty()
        } else {
            (iter.node_index == tree.min_leaf_index && iter.child_iter.iter_is_begin_from_non_empty())
        }
    }

    // Returns true iff the iterator is an end iterator.
    public fun is_end_iter<K: store, V: store>(_tree: &BigOrderedMap<K, V>, iter: &Iterator<K>): bool {
        iter is Iterator::End<K>
    }

    /// Returns the key of the given iterator.
    public fun iter_get_key<K>(self: &Iterator<K>): &K {
        assert!(!(self is Iterator::End<K>), E_INVALID_PARAMETER);
        &self.key
    }

    /// Returns an iterator pointing to the first element that is greater or equal to the provided
    /// key, or an end iterator if such element doesn't exist.
    public fun lower_bound<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Iterator<K> {
        let leaf = self.find_leaf(key);
        if (leaf == NULL_INDEX) {
            return self.new_end_iter()
        };

        let node = self.nodes.borrow(leaf);
        assert!(node.is_leaf, E_INTERNAL);

        let child_lower_bound = node.children.lower_bound(key);
        if (child_lower_bound.iter_is_end()) {
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
        if (is_end_iter(self, &lower_bound)) {
            lower_bound
        } else if (&lower_bound.key == key) {
            lower_bound
        } else {
            self.new_end_iter()
        }
    }

    /// Returns true iff the key exists in the tree.
    public fun contains<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): bool {
        let lower_bound = self.lower_bound(key);
        if (is_end_iter(self, &lower_bound)) {
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

        assert!(is_end_iter(self, &iter), E_INVALID_PARAMETER);
        let children = &self.nodes.borrow(iter.node_index).children;
        &iter.child_iter.iter_borrow(children).value
    }

    /// Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.
    public fun borrow_mut<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K): &mut V {
        let iter = self.find(&key);

        assert!(is_end_iter(self, &iter), E_INVALID_PARAMETER);
        let children = &mut self.nodes.borrow_mut(iter.node_index).children;
        &mut iter.child_iter.iter_borrow_mut(children).value
    }

    /// Return the begin iterator.
    public fun new_begin_iter<K: copy + store, V: store>(self: &BigOrderedMap<K, V>): Iterator<K> {
        if (self.is_empty()) {
            return Iterator::End;
        };

        let node = self.nodes.borrow(self.min_leaf_index);
        assert!(!node.children.is_empty(), E_INTERNAL);
        let begin_child_iter = node.children.new_begin_iter();
        let begin_child_key = *begin_child_iter.iter_borrow_key(&node.children);
        new_iter(self.min_leaf_index, begin_child_iter, begin_child_key)
    }

    /// Return the end iterator.
    public fun new_end_iter<K: copy + store, V: store>(self: &BigOrderedMap<K, V>): Iterator<K> {
        Iterator::End
    }

    /// Returns the next iterator, or none if already at the end iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun next_iter<K: drop + copy + store, V: store>(tree: &BigOrderedMap<K, V>, iter: Iterator<K>): Iterator<K> {
        assert!(!(iter is Iterator::End<K>), E_INVALID_PARAMETER);

        let node_index = iter.node_index;
        let node = tree.nodes.borrow(node_index);

        let child_iter = iter.child_iter.iter_next(&node.children);
        if (!child_iter.iter_is_end()) {
            let iter_key = *child_iter.iter_borrow_key(&node.children);
            return new_iter(node_index, child_iter, iter_key);
        };

        let next_index = node.next;
        if (next_index != NULL_INDEX) {
            let next_node = tree.nodes.borrow(next_index);

            let child_iter = next_node.children.new_begin_iter();
            assert!(!iter.child_iter.iter_is_end(), E_INTERNAL);
            let iter_key = *child_iter.iter_borrow_key(&next_node.children);
            return new_iter(next_index, child_iter, iter_key);
        };

        new_end_iter(tree)
    }

    /// Returns the previous iterator, or none if already at the begin iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun prev_iter<K: drop + copy + store, V: store>(tree: &BigOrderedMap<K, V>, iter: Iterator<K>): Iterator<K> {
        let prev_index = if (iter is Iterator::End<K>) {
            tree.max_leaf_index
        } else {
            let node_index = iter.node_index;
            let node = tree.nodes.borrow(node_index);

            if (!iter.child_iter.iter_is_begin(&node.children)) {
                let child_iter = iter.child_iter.iter_prev(&node.children);
                let key = *child_iter.iter_borrow_key(&node.children);
                return new_iter(node_index, child_iter, key);
            };
            node.prev
        };

        assert!(prev_index != NULL_INDEX, E_INTERNAL);

        let prev_node = tree.nodes.borrow(prev_index);

        let prev_children = &prev_node.children;
        let child_iter = prev_children.new_end_iter().iter_prev(prev_children);
        let iter_key = *child_iter.iter_borrow_key(prev_children);
        new_iter(prev_index, child_iter, iter_key)
    }

    //////////////////////////////
    // Internal Implementations //
    //////////////////////////////

    fun init_max_degrees<K: store, V: store>(self: &mut BigOrderedMap<K, V>, key: &K, value: &V) {
        if (self.inner_max_degree == 0 || self.leaf_max_degree == 0) {
            let key_size = bcs::serialized_size(key);

            if (self.inner_max_degree == 0) {
                self.inner_max_degree = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / key_size), DEFAULT_INNER_MIN_DEGREE as u64) as u16;
            };

            if (self.leaf_max_degree == 0) {
                let value_size = bcs::serialized_size(value);
                self.leaf_max_degree = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / (key_size + value_size)), DEFAULT_LEAF_MIN_DEGREE as u64) as u16;
            };
        };
    }

    fun destroy_inner_child<V: store>(self: Child<V>) {
        let Child::Inner {
            node_index: _,
        } = self;
    }

    fun destroy_empty_node<K: store, V: store>(self: Node<K, V>) {
        let Node { children, is_leaf: _, parent: _, prev: _, next: _ } = self;
        assert!(children.is_empty(), E_TREE_NOT_EMPTY);
        children.destroy_empty();
    }

    fun new_node<K: store, V: store>(is_leaf: bool, parent: u64): Node<K, V> {
        Node {
            is_leaf: is_leaf,
            parent: parent,
            children: ordered_map::new(),
            prev: NULL_INDEX,
            next: NULL_INDEX,
        }
    }

    fun new_node_with_children<K: store, V: store>(is_leaf: bool, parent: u64, children: OrderedMap<K, Child<V>>): Node<K, V> {
        Node {
            is_leaf: is_leaf,
            parent: parent,
            children: children,
            prev: NULL_INDEX,
            next: NULL_INDEX,
        }
    }

    fun new_inner_child<V: store>(node_index: u64): Child<V> {
        Child::Inner {
            node_index: node_index,
        }
    }

    fun new_leaf_child<V: store>(value: V): Child<V> {
        Child::Leaf {
            value: value,
        }
    }

    fun new_iter<K>(node_index: u64, child_iter: ordered_map::Iterator, key: K): Iterator<K> {
        Iterator::Some {
            node_index: node_index,
            child_iter: child_iter,
            key: key,
        }
    }

    fun find_leaf<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): u64 {
        let current = self.root_index;
        while (current != NULL_INDEX) {
            let node = self.nodes.borrow(current);
            if (node.is_leaf) {
                return current
            };
            let children = &node.children;
            let child_iter = children.lower_bound(key);
            if (child_iter.iter_is_end()) {
                return NULL_INDEX;
            } else {
                current = child_iter.iter_borrow(children).node_index;
            }
        };

        NULL_INDEX
    }

    fun get_max_degree<K: store, V: store>(self: &BigOrderedMap<K, V>, leaf: bool): u64 {
        if (leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        }
    }

    fun add_at<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, node_index: u64, key: K, child: Child<V>, allow_overwrite: bool): Option<Child<V>> {
        {
            let node = self.nodes.borrow_mut(node_index);
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
                    assert!(allow_overwrite || result.is_none(), EKEY_ALREADY_EXISTS);
                    return result;
                } else {
                    assert!(!allow_overwrite && result.is_none(), E_INTERNAL);
                    return result;
                };
            };

            if (allow_overwrite) {
                let iter = children.find(&key);
                if (!iter.iter_is_end()) {
                    return option::some(iter.iter_replace(children, child));
                }
            }
        };

        // # of children in the current node exceeds the threshold, need to split into two nodes.
        let (right_node_slot, node) = self.nodes.remove_and_reserve(node_index);
        let parent_index = node.parent;
        let is_leaf = &mut node.is_leaf;
        let next = &mut node.next;
        let prev = &mut node.prev;
        let children = &mut node.children;

        if (parent_index == NULL_INDEX) {
            // Splitting root now, need to create a new root.
            let parent_node = new_node(/*is_leaf=*/false, /*parent=*/NULL_INDEX);
            let max_element = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
            if (cmp::compare(&max_element, &key).is_less_than()) {
                max_element = key;
            };
            parent_node.children.add(max_element, new_inner_child(node_index));

            parent_index = self.nodes.add(parent_node);
            node.parent = parent_index;

            self.root_index = parent_index;
        };

        let max_degree = if (*is_leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        };
        let target_size = (max_degree + 1) / 2;

        children.add(key, child);
        let new_node_children = children.split_off(target_size);

        assert!(children.length() <= max_degree, E_INTERNAL);
        assert!(new_node_children.length() <= max_degree, E_INTERNAL);

        let right_node = new_node_with_children(*is_leaf, parent_index, new_node_children);

        let left_node_slot = self.nodes.reserve_slot();
        let left_node_index = left_node_slot.get_index();
        right_node.next = *next;
        *next = node_index;
        right_node.prev = left_node_index;
        if (*prev != NULL_INDEX) {
            self.nodes.borrow_mut(*prev).next = left_node_index;
        };

        if (!*is_leaf) {
            children.for_each_ref(|_key, child| {
                self.nodes.borrow_mut(child.node_index).parent = left_node_index;
            });
        };

        let split_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);

        self.nodes.fill_reserved_slot(left_node_slot, node);
        self.nodes.fill_reserved_slot(right_node_slot, right_node);

        if (node_index == self.min_leaf_index) {
            self.min_leaf_index = left_node_index;
        };
        self.add_at(parent_index, split_key, new_inner_child(left_node_index), false).destroy_none();
        option::none()
    }

    fun update_key<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, node_index: u64, old_key: &K, new_key: K) {
        if (node_index == NULL_INDEX) {
            return
        };

        let node = self.nodes.borrow_mut(node_index);
        let children = &mut node.children;
        children.replace_key_inplace(old_key, new_key);

        if (children.new_end_iter().iter_prev(children).iter_borrow_key(children) == &new_key) {
            self.update_key(node.parent, old_key, new_key);
        };
    }

    fun remove_at<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, node_index: u64, key: &K): Child<V> {
        // aptos_std::debug::print(&std::string::utf8(b"remove_at"));
        // aptos_std::debug::print(&node_index);
        // aptos_std::debug::print(key);

        let old_child = {
            let node = self.nodes.borrow_mut(node_index);
            // aptos_std::debug::print(&std::string::utf8(b"node, borrowed"));
            // aptos_std::debug::print(node);

            let children = &mut node.children;
            let current_size = children.length();

            if (current_size == 1 && node_index == self.root_index) {
                // Remove the only element at root node.
                return children.remove(key);
            };

            let is_leaf = node.is_leaf;

            let old_child = children.remove(key);
            current_size = current_size - 1;

            let new_max_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
            let max_key_updated = cmp::compare(&new_max_key, key).is_less_than();

            let max_degree = if (node.is_leaf) {
                self.leaf_max_degree as u64
            } else {
                self.inner_max_degree as u64
            };

            let big_enough = current_size * 2 >= max_degree;
            if (!max_key_updated && big_enough) {
                return old_child;
            };

            if (!big_enough && node_index == self.root_index) {
                // promote only child to root, and drop current root.
                if (current_size == 1 && !is_leaf) {
                    let Child::Inner {
                        node_index: inner_child_index,
                    } = children.new_end_iter().iter_prev(children).iter_remove(children);
                    self.root_index = inner_child_index;
                    self.nodes.borrow_mut(self.root_index).parent = NULL_INDEX;
                    destroy_empty_node(self.nodes.remove(node_index));
                } else {
                    // nothing to change
                };
                return old_child;
            };

            if (max_key_updated) {
                assert!(current_size >= 1, E_INTERNAL);

                let parent = node.parent;

                self.update_key(parent, key, new_max_key);

                if (big_enough) {
                    return old_child;
                }
            };

            old_child
        };

        // We need to update tree beyond the current node

        let (node_slot, node) = self.nodes.remove_and_reserve(node_index);
        // aptos_std::debug::print(&std::string::utf8(b"node, removed and reserved"));
        // aptos_std::debug::print(&node);

        let prev = node.prev;
        let next = node.next;
        let parent = node.parent;
        let is_leaf = node.is_leaf;
        let max_degree = self.get_max_degree(is_leaf);

        let children = &mut node.children;

        // Children size is below threshold, we need to rebalance

        let brother_index = next;
        if (brother_index == NULL_INDEX || self.nodes.borrow(brother_index).parent != parent) {
            brother_index = prev;
        };
        let (brother_slot, brother_node) = self.nodes.remove_and_reserve(brother_index);
        // aptos_std::debug::print(&std::string::utf8(b"brother, removed and reserved"));
        // aptos_std::debug::print(&brother_node);

        let brother_children = &mut brother_node.children;

        if ((brother_children.length() - 1) * 2 >= max_degree) {
            // aptos_std::debug::print(&std::string::utf8(b"The brother node has enough elements, borrow an element from the brother node."));
            // The brother node has enough elements, borrow an element from the brother node.
            if (brother_index == next) {
                // aptos_std::debug::print(&std::string::utf8(b"brother_index == next. Moving from brother."));

                let old_max_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
                let brother_begin_iter = brother_children.new_begin_iter();
                let borrowed_max_key = *brother_begin_iter.iter_borrow_key(brother_children);
                let borrowed_element = brother_begin_iter.iter_remove(brother_children);
                if (borrowed_element is Child::Inner<V>) {
                    self.nodes.borrow_mut(borrowed_element.node_index).parent = node_index;
                };

                // aptos_std::debug::print(&borrowed_max_key);
                // aptos_std::debug::print(&old_max_key);

                children.add(borrowed_max_key, borrowed_element);
                self.update_key(parent, &old_max_key, borrowed_max_key);
            } else {
                // aptos_std::debug::print(&std::string::utf8(b"brother_index != next. Moving from brother"));

                let brother_end_iter = brother_children.new_end_iter().iter_prev(brother_children);
                let borrowed_max_key = *brother_end_iter.iter_borrow_key(brother_children);
                let borrowed_element = brother_end_iter.iter_remove(brother_children);

                if (borrowed_element is Child::Inner<V>) {
                    self.nodes.borrow_mut(borrowed_element.node_index).parent = node_index;
                };

                // aptos_std::debug::print(&borrowed_max_key);

                children.add(borrowed_max_key, borrowed_element);
                self.update_key(parent, &borrowed_max_key, *brother_children.new_end_iter().iter_prev(brother_children).iter_borrow_key(brother_children));
            };

            self.nodes.fill_reserved_slot(node_slot, node);
            self.nodes.fill_reserved_slot(brother_slot, brother_node);
            return old_child;
        };

        // aptos_std::debug::print(&std::string::utf8(b"The brother node doesn't have enough elements to borrow, merge with the brother node."));

        // The brother node doesn't have enough elements to borrow, merge with the brother node.
        if (brother_index == next) {
            // aptos_std::debug::print(&std::string::utf8(b"brother_index == next"));

            if (!is_leaf) {
                children.for_each_ref(|_key, child| {
                    self.nodes.borrow_mut(child.node_index).parent = brother_index;
                });
            };
            let Node { children: brother_children, is_leaf: _, parent: _, prev: _, next: brother_next } = brother_node;
            let key_to_remove = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
            children.append(brother_children);
            node.next = brother_next;

            move children;

            if (node.next != NULL_INDEX) {
                self.nodes.borrow_mut(node.next).prev = brother_index;
            };
            if (node.prev != NULL_INDEX) {
                self.nodes.borrow_mut(node.prev).next = brother_index;
            };

            // aptos_std::debug::print(&std::string::utf8(b"keeping node"));
            // aptos_std::debug::print(&brother_slot);
            // aptos_std::debug::print(&node);
            // aptos_std::debug::print(&std::string::utf8(b"freeing node"));
            // aptos_std::debug::print(&node_slot);

            self.nodes.fill_reserved_slot(brother_slot, node);
            self.nodes.free_reserved_slot(node_slot);
            if (self.min_leaf_index == node_index) {
                self.min_leaf_index = brother_index;
            };

            if (parent != NULL_INDEX) {
                destroy_inner_child(self.remove_at(parent, &key_to_remove));
            };
        } else {
            // aptos_std::debug::print(&std::string::utf8(b"brother_index != next"));

            if (!is_leaf) {
                brother_children.for_each_ref(|_key, child| {
                    self.nodes.borrow_mut(child.node_index).parent = node_index;
                });
            };

            let Node { children: node_children, is_leaf: _, parent: _, prev: _, next: node_next } = node;
            let key_to_remove = *brother_children.new_end_iter().iter_prev(brother_children).iter_borrow_key(brother_children);
            brother_children.append(node_children);
            brother_node.next = node_next;

            move brother_children;

            if (brother_node.next != NULL_INDEX) {
                self.nodes.borrow_mut(brother_node.next).prev = node_index;
            };
            if (brother_node.prev != NULL_INDEX) {
                self.nodes.borrow_mut(brother_node.prev).next = node_index;
            };

            // aptos_std::debug::print(&std::string::utf8(b"keeping node"));
            // aptos_std::debug::print(&node_slot);
            // aptos_std::debug::print(&brother_node);
            // aptos_std::debug::print(&std::string::utf8(b"freeing node"));
            // aptos_std::debug::print(&brother_slot);

            self.nodes.fill_reserved_slot(node_slot, brother_node);
            self.nodes.free_reserved_slot(brother_slot);
            if (self.min_leaf_index == brother_index) {
                self.min_leaf_index = node_index;
            };

            if (parent != NULL_INDEX) {
                destroy_inner_child(self.remove_at(parent, &key_to_remove));
            };
        };
        old_child
    }

    /// Returns the number of elements in the BigOrderedMap.
    fun length<K: store, V: store>(self: &BigOrderedMap<K, V>): u64 {
        self.length_for_node(self.root_index)
    }

    fun length_for_node<K: store, V: store>(self: &BigOrderedMap<K, V>, node_index: u64): u64 {
        let node = self.nodes.borrow(node_index);
        if (node.is_leaf) {
            node.children.length()
        } else {
            let size = 0;

            node.children.for_each_ref(|_key, child| {
                size = size + self.length_for_node(child.node_index);
            });
            size
        }
    }

    #[test_only]
    fun print_tree<K: store, V: store>(self: &BigOrderedMap<K, V>) {
        aptos_std::debug::print(self);
        self.print_tree_for_node(self.root_index, 0);
    }

    #[test_only]
    fun print_tree_for_node<K: store, V: store>(self: &BigOrderedMap<K, V>, node_index: u64, level: u64) {
        let node = self.nodes.borrow(node_index);

        aptos_std::debug::print(&level);
        aptos_std::debug::print(&node_index);
        aptos_std::debug::print(node);

        if (!node.is_leaf) {
            node.children.for_each_ref(|_key, node| {
                self.print_tree_for_node(node.node_index, level + 1);
            });
        };
    }

    /// Returns true iff the BigOrderedMap is empty.
    fun is_empty<K: store, V: store>(self: &BigOrderedMap<K, V>): bool {
        let node = self.nodes.borrow(self.min_leaf_index);

        node.children.is_empty()
    }

    ///////////
    // Tests //
    ///////////

    #[test_only]
    fun destroy<K: drop + copy + store, V: drop + store>(self: BigOrderedMap<K, V>) {
        let it = new_begin_iter(&self);
        while (!is_end_iter(&self, &it)) {
            remove(&mut self, it.iter_get_key());
            assert!(is_end_iter(&self, &find(&self, it.iter_get_key())), E_INTERNAL);
            it = new_begin_iter(&self);
            self.validate_tree();
        };

        self.destroy_empty();
    }

    #[test_only]
    fun validate_iteration<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>) {
        let expected_num_elements = self.length();
        let num_elements = 0;
        let it = new_begin_iter(self);
        while (!is_end_iter(self, &it)) {
            num_elements = num_elements + 1;
            it = next_iter(self, it);
        };
        assert!(num_elements == expected_num_elements, E_INTERNAL);

        let num_elements = 0;
        let it = new_end_iter(self);
        while (!is_begin_iter(self, &it)) {
            it = prev_iter(self, it);
            num_elements = num_elements + 1;
        };
        assert!(num_elements == expected_num_elements, E_INTERNAL);

        let it = new_end_iter(self);
        if (!is_begin_iter(self, &it)) {
            it = prev_iter(self, it);
            assert!(it.node_index == self.max_leaf_index, E_INTERNAL);
        } else {
            assert!(expected_num_elements == 0, E_INTERNAL);
        };
    }

    #[test_only]
    fun validate_subtree<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, node_index: u64, expected_lower_bound_key: Option<K>, expected_max_key: Option<K>, expected_parent: u64) {
        let node = self.nodes.borrow(node_index);
        let len = node.children.length();
        assert!(len <= self.get_max_degree(node.is_leaf), E_INTERNAL);

        if (node_index != self.root_index) {
            assert!(len >= 1, E_INTERNAL);
            assert!(len * 2 >= self.get_max_degree(node.is_leaf) || node_index == self.root_index, E_INTERNAL);
        };

        assert!(node.parent == expected_parent, E_INTERNAL);

        node.children.validate_ordered();

        let previous_max_key = expected_lower_bound_key;
        node.children.for_each_ref(|key: &K, child: &Child<V>| {
            if (!node.is_leaf) {
                self.validate_subtree(child.node_index, previous_max_key, option::some(*key), node_index);
            } else {
                assert!((child is Child::Leaf<V>), E_INTERNAL);
            };
            previous_max_key = option::some(*key);
        });

        if (option::is_some(&expected_max_key)) {
            let expected_max_key = option::extract(&mut expected_max_key);
            assert!(&expected_max_key == node.children.new_end_iter().iter_prev(&node.children).iter_borrow_key(&node.children), E_INTERNAL);
        };

        if (option::is_some(&expected_lower_bound_key)) {
            let expected_lower_bound_key = option::extract(&mut expected_lower_bound_key);
            assert!(cmp::compare(&expected_lower_bound_key, node.children.new_begin_iter().iter_borrow_key(&node.children)).is_less_than(), E_INTERNAL);
        };
    }

    #[test_only]
    fun validate_tree<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>) {
        self.validate_subtree(self.root_index, option::none(), option::none(), NULL_INDEX);
        self.validate_iteration();
    }

    #[test]
    fun test_smart_tree() {
        let map = new_with_config(5, 3, true, 2);
        map.print_tree(); map.validate_tree();
        add(&mut map, 1, 1); map.print_tree(); map.validate_tree();
        add(&mut map, 2, 2); map.print_tree(); map.validate_tree();
        let r1 = upsert(&mut map, 3, 3); map.print_tree(); map.validate_tree();
        assert!(r1 == option::none(), E_INTERNAL);
        add(&mut map, 4, 4); map.print_tree(); map.validate_tree();
        let r2 = upsert(&mut map, 4, 8); map.print_tree(); map.validate_tree();
        assert!(r2 == option::some(4), E_INTERNAL);
        add(&mut map, 5, 5); map.print_tree(); map.validate_tree();
        add(&mut map, 6, 6); map.print_tree(); map.validate_tree();

        remove(&mut map, &5); map.print_tree(); map.validate_tree();
        remove(&mut map, &4); map.print_tree(); map.validate_tree();
        remove(&mut map, &1); map.print_tree(); map.validate_tree();
        remove(&mut map, &3); map.print_tree(); map.validate_tree();
        remove(&mut map, &2); map.print_tree(); map.validate_tree();
        remove(&mut map, &6); map.print_tree(); map.validate_tree();

        destroy_empty(map);
    }

    #[test]
    fun test_deleting_and_creating_nodes() {
        let map = new_with_config(4, 3, true, 2);

        for (i in 0..50) {
            map.upsert(i, i);
            map.validate_tree();
        };

        for (i in 0..40) {
            map.remove(&i);
            map.validate_tree();
        };

        for (i in 50..100) {
            map.upsert(i, i);
            map.validate_tree();
        };

        for (i in 50..90) {
            map.remove(&i);
            map.validate_tree();
        };

        for (i in 100..150) {
            map.upsert(i, i);
            map.validate_tree();
        };

        for (i in 100..150) {
            map.remove(&i);
            map.validate_tree();
        };

        for (i in 40..50) {
            map.remove(&i);
            map.validate_tree();
        };

        for (i in 90..100) {
            map.remove(&i);
            map.validate_tree();
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
        while (!is_end_iter(&map, &it)) {
            assert!(i == it.key, E_INTERNAL);
            i = i + 1;
            it = next_iter(&map, it);
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
            assert!(!is_end_iter(&map, &it), E_INTERNAL);
            assert!(it.iter_get_key() == element, E_INTERNAL);
            i = i + 1;
        };

        assert!(is_end_iter(&map, &find(&map, &4)), E_INTERNAL);
        assert!(is_end_iter(&map, &find(&map, &9)), E_INTERNAL);

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
            assert!(!is_end_iter(&map, &it), E_INTERNAL);
            assert!(it.key == element, E_INTERNAL);
            i = i + 1;
        };

        assert!(lower_bound(&map, &0).key == 1, E_INTERNAL);
        assert!(lower_bound(&map, &4).key == 5, E_INTERNAL);
        assert!(lower_bound(&map, &9).key == 10, E_INTERNAL);
        assert!(is_end_iter(&map, &lower_bound(&map, &13)), E_INTERNAL);

        remove(&mut map, &3);
        assert!(lower_bound(&map, &3).key == 5, E_INTERNAL);
        remove(&mut map, &5);
        assert!(lower_bound(&map, &3).key == 6, E_INTERNAL);
        assert!(lower_bound(&map, &4).key == 6, E_INTERNAL);

        destroy(map);
    }

    #[test_only]
    fun test_large_data_set_helper(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool) {
        let map = new_with_config(inner_max_degree, leaf_max_degree, reuse_slots, if (reuse_slots) {4} else {0});
        let data = ordered_map::large_dataset();
        let shuffled_data = ordered_map::large_dataset_shuffled();

        let i = 0;
        let len = data.length();
        while (i < len) {
            let element = *data.borrow(i);
            map.upsert(element, element);
            map.validate_tree();
            i = i + 1;
        };

        let i = 0;
        while (i < len) {
            let element = shuffled_data.borrow(i);
            let it = map.find(element);
            assert!(!is_end_iter(&map, &it), E_INTERNAL);
            assert!(it.iter_get_key() == element, E_INTERNAL);

            // aptos_std::debug::print(&it);

            let it_next = next_iter(&map, it);
            let it_after = map.lower_bound(&(*element + 1));

            // aptos_std::debug::print(&it_next);
            // aptos_std::debug::print(&it_after);
            // aptos_std::debug::print(&std::string::utf8(b"bla"));

            assert!(it_next == it_after, E_INTERNAL);

            i = i + 1;
        };


        let i = 0;
        while (i < len) {
            let element = shuffled_data.borrow(i);
            map.remove(element);
            map.validate_map();
            i = i + 1;
        };

        map.destroy_empty();
    }

    #[test]
    fun test_large_data_set_order_5() {
        test_large_data_set_helper(5, 5, false);
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
    fun test_large_data_set_order_4_4() {
        test_large_data_set_helper(4, 4, false);
        test_large_data_set_helper(4, 4, true);
    }

    #[test]
    fun test_large_data_set_order_6() {
        test_large_data_set_helper(6, 6, false);
        test_large_data_set_helper(6, 6, true);
    }

    #[test]
    fun test_large_data_set_order_6_3() {
        test_large_data_set_helper(6, 3, false);
        test_large_data_set_helper(6, 3, true);
    }

    #[test]
    fun test_large_data_set_order_4_6() {
        test_large_data_set_helper(4, 6, false);
        test_large_data_set_helper(4, 6, true);
    }

    #[test]
    fun test_large_data_set_order_16() {
        test_large_data_set_helper(16, 16, false);
        test_large_data_set_helper(16, 16, true);
    }

    #[test]
    fun test_large_data_set_order_31() {
        test_large_data_set_helper(31, 31, false);
        test_large_data_set_helper(31, 31, true);
    }

    #[test]
    fun test_large_data_set_order_31_3() {
        test_large_data_set_helper(31, 3, false);
        test_large_data_set_helper(31, 3, true);
    }

    #[test]
    fun test_large_data_set_order_31_5() {
        test_large_data_set_helper(31, 5, false);
        test_large_data_set_helper(31, 5, true);
    }

    #[test]
    fun test_large_data_set_order_32() {
        test_large_data_set_helper(32, 32, false);
        test_large_data_set_helper(32, 32, true);
    }
}
