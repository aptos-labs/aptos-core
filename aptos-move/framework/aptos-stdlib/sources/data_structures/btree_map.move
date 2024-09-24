/// Type of large-scale search trees.
///
/// It internally uses BTree to organize the search tree data structure for keys. Comparing with
/// other common search trees like AVL or Red-black tree, a BTree node has more children, and packs
/// more metadata into one node, which is more disk friendly (and gas friendly).

module aptos_std::btree_map {
    use std::option::{Self, Option};
    use std::vector;
    use std::bcs;
    use aptos_std::cmp;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_std::math64::{max, min};

    // Internal errors.
    const E_INTERNAL: u64 = 0;
    // The tree is not empty, and cannot be destroyed.
    const E_TREE_NOT_EMPTY: u64 = 1;
    // The tree is too big for insertion.
    const E_TREE_TOO_BIG: u64 = 2;
    // The provided parameter is invalid.
    const E_INVALID_PARAMETER: u64 = 3;

    const NULL_INDEX: u64 = 0;
    const DEFAULT_TARGET_NODE_SIZE: u64 = 2048;
    const DEFAULT_INNER_MIN_DEGREE: u16 = 4;
    // We rely on 1 being valid size only for root node,
    // so this cannot be below 3 (unless that is changed)
    const DEFAULT_LEAF_MIN_DEGREE: u16 = 3;
    const MAX_DEGREE: u64 = 4096;

    /// A node of the BTreeMap.
    struct Node<K: store, V: store> has store {
        // Whether this node is a leaf node.
        is_leaf: bool,
        // The node index of its parent node, or NULL_INDEX if it doesn't have parent.
        parent: u64,
        // The children of the nodes. (When the node is leaf node, all keys of the node is stored in children.max_key)
        children: vector<Child<K, V>>,
        // The node index of its previous node at the same level, or NULL_INDEX if it doesn't have a previous node.
        prev: u64,
        // The node index of its next node at the same level, or NULL_INDEX if it doesn't have a next node.
        next: u64,
    }

    /// The metadata of a child of a node.
    enum Child<K: store, V: store> has store {
        Inner {
            // The max key of its child, or the key of the current node if it is a leaf node.
            max_key: K,
            // The node index of it's child, or NULL_INDEX if the current node is a leaf node.
            node_index: u64,
        },
        Leaf {
            // The max key of its child, or the key of the current node if it is a leaf node.
            max_key: K,

            value: V,
        }
    }

    /// An iterator to iterate all keys in the BTreeMap.
    enum Iterator<K> has copy, drop {
        End,
        Some {
            // The node index of the iterator pointing to.
            node_index: u64,
            // The child index of the iterator pointing to.
            child_index: u64,
            // The key of the iterator pointing to, not valid when the iterator is an end iterator.
            key: K,
        },
    }

    /// The BTreeMap data structure.
    enum BTreeMap<K: store, V: store> has store {
        V1 {
            // The node index of the root node.
            root_index: u64,
            // Mapping of node_index -> node.
            nodes: TableWithLength<u64, Node<K, V>>,
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

    /// Returns a new BTreeMap with the default configuration.
    public fun new<K: store, V: store>(): BTreeMap<K, V> {
        new_with_config(0, 0)
    }

    /// Returns a new BTreeMap with the provided max degree consts (the maximum # of children a node can have).
    /// If 0 is passed, then it is dynamically computed based on size of first key and value.
    public fun new_with_config<K: store, V: store>(inner_max_degree: u16, leaf_max_degree: u16): BTreeMap<K, V> {
        assert!(inner_max_degree == 0 || inner_max_degree >= DEFAULT_INNER_MIN_DEGREE, E_INVALID_PARAMETER);
        assert!(leaf_max_degree == 0 || leaf_max_degree >= DEFAULT_LEAF_MIN_DEGREE, E_INVALID_PARAMETER);
        let root_node = new_node(/*is_leaf=*/true, /*parent=*/NULL_INDEX);
        let nodes = table_with_length::new();
        let root_index = 1;
        nodes.add(root_index, root_node);
        BTreeMap::V1 {
            root_index: root_index,
            nodes: nodes,
            min_leaf_index: root_index,
            max_leaf_index: root_index,
            inner_max_degree: inner_max_degree,
            leaf_max_degree: leaf_max_degree
        }
    }

    /// Destroys the tree if it's empty, otherwise aborts.
    public fun destroy_empty<K: store, V: store>(self: BTreeMap<K, V>) {
        let BTreeMap::V1 { nodes, root_index, min_leaf_index: _, max_leaf_index: _, inner_max_degree: _, leaf_max_degree: _ } = self;
        aptos_std::debug::print(&nodes);
        assert!(nodes.length() == 1, E_TREE_NOT_EMPTY);
        nodes.remove(root_index).destroy_empty_node();
        nodes.destroy_empty();
    }

    ///////////////
    // Modifiers //
    ///////////////

    fun init_max_degrees<K: store, V: store>(self: &mut BTreeMap<K, V>, key: &K, value: &V) {
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

    /// Inserts the key/value into the BTreeMap.
    /// Aborts if the key is already in the tree.
    public fun insert<K: drop + copy + store, V: store>(self: &mut BTreeMap<K, V>, key: K, value: V) {
        if (self.inner_max_degree == 0 || self.leaf_max_degree == 0) {
            self.init_max_degrees(&key, &value);
        };

        let leaf = self.find_leaf(key);

        if (leaf == NULL_INDEX) {
            // In this case, the key is greater than all keys in the tree.
            leaf = self.max_leaf_index;
            let current = self.nodes.borrow(leaf).parent;
            while (current != NULL_INDEX) {
                let current_node = self.nodes.borrow_mut(current);
                let last_index = current_node.children.length() - 1;
                let last_element = current_node.children.borrow_mut(last_index);
                last_element.max_key = key;
                current = current_node.parent;
            }
        };

        self.insert_at(leaf, new_leaf_child(key, value));
    }

    /// If the key doesn't exist in the tree, inserts the key/value, and returns none.
    /// Otherwise updates the value under the given key, and returns the old value.
    public fun upsert<K: drop + copy + store, V: store>(self: &mut BTreeMap<K, V>, key: K, value: V): Option<V> {
        if (self.inner_max_degree == 0 || self.leaf_max_degree == 0) {
            self.init_max_degrees(&key, &value);
        };

        let iter = self.find(key);
        if (is_end_iter(self, &iter)) {
            self.insert(key, value);
            return option::none()
        } else {
            let node = self.nodes.borrow_mut(iter.node_index);
            let children = &mut node.children;

            // Field swap doesn't compile.
            // let child = vector::borrow_mut(children, iter.child_index);
            // assert!(child.max_key == key, E_INTERNAL);
            // let old = child.value;
            // child.value = value;
            // option::some(old)

            let Child::Leaf {
                max_key: old_max_key,
                value: old_value,
            } = children.replace(iter.child_index, Child::Leaf { max_key: key, value: value });
            assert!(old_max_key == key, E_INTERNAL);
            option::some(old_value)
        }
    }

    /// Removes the entry from BTreeMap and returns the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: drop + copy + store, V: store>(self: &mut BTreeMap<K, V>, key: K): V {
        let iter = self.find(key);
        assert!(!is_end_iter(self, &iter), E_INTERNAL);

        let Child::Leaf {
            value,
            max_key: _,
        } = self.remove_at(iter.node_index, key);

        value
    }

    ///////////////
    // Accessors //
    ///////////////

    // Returns true iff the node_index is NULL_INDEX.
    public fun is_null_index(node_index: u64): bool {
        node_index == NULL_INDEX
    }

    // Returns true iff the iterator is a begin iterator.
    public fun is_begin_iter<K: store, V: store>(tree: &BTreeMap<K, V>, iter: &Iterator<K>): bool {
        if (iter is Iterator::End<K>) {
            empty(tree)
        } else {
            (iter.node_index == tree.min_leaf_index && iter.child_index == 0)
        }
    }

    // Returns true iff the iterator is an end iterator.
    public fun is_end_iter<K: store, V: store>(_tree: &BTreeMap<K, V>, iter: &Iterator<K>): bool {
        iter is Iterator::End<K>
    }

    /// Returns an iterator pointing to the first element that is greater or equal to the provided
    /// key, or an end iterator if such element doesn't exist.
    public fun lower_bound<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>, key: K): Iterator<K> {
        let leaf = self.find_leaf(key);
        if (leaf == NULL_INDEX) {
            return self.new_end_iter()
        };

        let node = self.nodes.borrow(leaf);
        assert!(node.is_leaf, E_INTERNAL);

        let keys = &node.children;

        let len = keys.length();

        let index = binary_search(key, keys, 0, len);
        if (index == len) {
            self.new_end_iter()
        } else {
            new_iter(leaf, index, keys.borrow(index).max_key)
        }
    }

    /// Returns an iterator pointing to the element that equals to the provided key, or an end
    /// iterator if the key is not found.
    public fun find<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>, key: K): Iterator<K> {
        let lower_bound = self.lower_bound(key);
        if (is_end_iter(self, &lower_bound)) {
            lower_bound
        } else if (lower_bound.key == key) {
            lower_bound
        } else {
            self.new_end_iter()
        }
    }

    /// Returns true iff the key exists in the tree.
    public fun contains<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>, key: K): bool {
        let lower_bound = self.lower_bound(key);
        if (is_end_iter(self, &lower_bound)) {
            false
        } else if (lower_bound.key == key) {
            true
        } else {
            false
        }
    }

    /// Returns the key of the given iterator.
    public fun get_key<K: copy>(iter: &Iterator<K>): K {
        assert!(!(iter is Iterator::End<K>), E_INVALID_PARAMETER);
        iter.key
    }

    /// Returns a reference to the element with its key, aborts if the key is not found.
    public fun borrow<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>, key: K): &V {
        let iter = self.find(key);

        assert!(is_end_iter(self, &iter), E_INVALID_PARAMETER);
        let children = &self.nodes.borrow(iter.node_index).children;
        &children.borrow(iter.child_index).value
    }

    /// Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.
    public fun borrow_mut<K: drop + copy + store, V: store>(self: &mut BTreeMap<K, V>, key: K): &mut V {
        let iter = self.find(key);

        assert!(is_end_iter(self, &iter), E_INVALID_PARAMETER);
        let children = &mut self.nodes.borrow_mut(iter.node_index).children;
        &mut children.borrow_mut(iter.child_index).value
    }

    /// Returns the number of elements in the BTreeMap.
    public fun size<K: store, V: store>(self: &BTreeMap<K, V>): u64 {
        self.size_for_node(self.root_index)
    }

    fun size_for_node<K: store, V: store>(self: &BTreeMap<K, V>, node_index: u64): u64 {
        let node = self.nodes.borrow(node_index);
        if (node.is_leaf) {
            node.children.length()
        } else {
            let size = 0;

            for (i in 0..node.children.length()) {
                size = size + self.size_for_node(node.children[i].node_index);
            };
            size
        }
    }

    #[test_only]
    fun print_tree<K: store, V: store>(self: &BTreeMap<K, V>) {
        aptos_std::debug::print(self);
        self.print_tree_for_node(self.root_index, 0);
    }

    #[test_only]
    fun print_tree_for_node<K: store, V: store>(self: &BTreeMap<K, V>, node_index: u64, level: u64) {
        let node = self.nodes.borrow(node_index);

        aptos_std::debug::print(&level);
        aptos_std::debug::print(node);

        if (!node.is_leaf) {
            for (i in 0..node.children.length()) {
                self.print_tree_for_node(node.children[i].node_index, level + 1);
            };
        };
    }

    /// Returns true iff the BTreeMap is empty.
    fun empty<K: store, V: store>(self: &BTreeMap<K, V>): bool {
        let node = self.nodes.borrow(self.min_leaf_index);

        node.children.is_empty()
    }

    /// Return the begin iterator.
    public fun new_begin_iter<K: copy + store, V: store>(self: &BTreeMap<K, V>): Iterator<K> {
        if (self.empty()) {
            return Iterator::End;
        };

        let node = self.nodes.borrow(self.min_leaf_index);
        let key = node.children.borrow(0).max_key;

        new_iter(self.min_leaf_index, 0, key)
    }

    /// Return the end iterator.
    public fun new_end_iter<K: copy + store, V: store>(self: &BTreeMap<K, V>): Iterator<K> {
        Iterator::End
    }

    /// Returns the next iterator, or none if already at the end iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun next_iter<K: drop + copy + store, V: store>(tree: &BTreeMap<K, V>, iter: Iterator<K>): Option<Iterator<K>> {
        if (iter is Iterator::End<K>) {
            option::none()
        } else {
            option::some(next_iter_or_die(tree, iter))
        }
    }

    /// Returns the next iterator, aborts if already at the end iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun next_iter_or_die<K: drop + copy + store, V: store>(tree: &BTreeMap<K, V>, iter: Iterator<K>): Iterator<K> {
        assert!(!(iter is Iterator::End<K>), E_INVALID_PARAMETER);

        let node_index = iter.node_index;

        let node = tree.nodes.borrow(node_index);
        iter.child_index = iter.child_index + 1;
        if (iter.child_index < node.children.length()) {
            iter.key = node.children.borrow(iter.child_index).max_key;
            return iter
        };

        let next_index = node.next;
        if (next_index != NULL_INDEX) {
            let next_node = tree.nodes.borrow(next_index);
            iter.node_index = next_index;
            iter.child_index = 0;
            iter.key = next_node.children.borrow(0).max_key;
            return iter
        };

        new_end_iter(tree)
    }

    /// Returns the previous iterator, or none if already at the begin iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun prev_iter<K: drop + copy + store, V: store>(tree: &BTreeMap<K, V>, iter: Iterator<K>): Option<Iterator<K>> {
        if (iter.node_index == tree.min_leaf_index && iter.child_index == 0) {
            return option::none()
        };

        option::some(prev_iter_or_die(tree, iter))
    }

    /// Returns the previous iterator, aborts if already at the begin iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun prev_iter_or_die<K: drop + copy + store, V: store>(tree: &BTreeMap<K, V>, iter: Iterator<K>): Iterator<K> {
        let prev_index = if (iter is Iterator::End<K>) {
            tree.max_leaf_index
        } else {
            let node_index = iter.node_index;
            let node = tree.nodes.borrow(node_index);
            if (iter.child_index >= 1) {
                iter.child_index = iter.child_index - 1;
                iter.key = node.children.borrow(iter.child_index).max_key;
                return iter
            };
            node.prev
        };

        assert!(prev_index != NULL_INDEX, E_INTERNAL);

        let prev_node = tree.nodes.borrow(prev_index);
        let len = prev_node.children.length();

        Iterator::Some {
            node_index: prev_index,
            child_index: len - 1,
            key: prev_node.children.borrow(len - 1).max_key,
        }
    }

    //////////////////////////////
    // Internal Implementations //
    //////////////////////////////

    fun destroy_inner_child<K: drop + store, V: store>(self: Child<K, V>) {
        let Child::Inner {
            max_key: _,
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
            children: vector::empty(),
            prev: NULL_INDEX,
            next: NULL_INDEX,
        }
    }

    fun new_node_with_children<K: store, V: store>(is_leaf: bool, parent: u64, children: vector<Child<K, V>>): Node<K, V> {
        Node {
            is_leaf: is_leaf,
            parent: parent,
            children: children,
            prev: NULL_INDEX,
            next: NULL_INDEX,
        }
    }

    fun new_inner_child<K: store, V: store>(max_key: K, node_index: u64): Child<K, V> {
        Child::Inner {
            max_key: max_key,
            node_index: node_index,
        }
    }

    fun new_leaf_child<K: store, V: store>(max_key: K, value: V): Child<K, V> {
        Child::Leaf {
            max_key: max_key,
            value: value,
        }
    }

    fun new_iter<K: store>(node_index: u64, child_index: u64, key: K): Iterator<K> {
        Iterator::Some {
            node_index: node_index,
            child_index: child_index,
            key: key,
        }
    }

    fun find_leaf<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>, key: K): u64 {
        let current = self.root_index;
        while (current != NULL_INDEX) {
            let node = self.nodes.borrow(current);
            if (node.is_leaf) {
                return current
            };
            let len = node.children.length();
            if (cmp::compare(&node.children.borrow(len - 1).max_key, &key).is_less_than()) {
                return NULL_INDEX
            };

            let index = binary_search(key, &node.children, 0, len);

            current = node.children.borrow(index).node_index;
        };

        NULL_INDEX
    }

    // return index of, or insert position.
    fun binary_search<K: drop + store, V: store>(key: K, children: &vector<Child<K, V>>, start: u64, end: u64): u64 {
        let l = start;
        let r = end;
        while (l != r) {
            let mid = l + (r - l) / 2;
            if (cmp::compare(&children.borrow(mid).max_key, &key).is_less_than()) {
                l = mid + 1;
            } else {
                r = mid;
            };
        };
        l
    }

    fun get_max_degree<K: store, V: store>(self: &BTreeMap<K, V>, leaf: bool): u64 {
        if (leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        }
    }

    fun insert_at<K: drop + copy + store, V: store>(self: &mut BTreeMap<K, V>, node_index: u64, child: Child<K, V>) {
        let current_size = {
            let node = self.nodes.borrow_mut(node_index);
            let children = &mut node.children;
            let current_size = children.length();
            let key = child.max_key;

            let max_degree = if (node.is_leaf) {
                self.leaf_max_degree as u64
            } else {
                self.inner_max_degree as u64
            };

            if (current_size < max_degree) {
                let index = binary_search(key, children, 0, current_size);
                assert!(index >= current_size || children[index].max_key != key, E_INTERNAL); // key cannot already be inside.
                children.insert(index, child);
                return
            };
            current_size
        };

        // # of children in the current node exceeds the threshold, need to split into two nodes.
        let node = table_with_length::remove(&mut self.nodes, node_index);
        let parent_index = node.parent;
        let is_leaf = &mut node.is_leaf;
        let next = &mut node.next;
        let prev = &mut node.prev;
        let children = &mut node.children;
        let key = child.max_key;

        let max_degree = if (*is_leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        };
        let target_size = (max_degree + 1) / 2;

        let l = binary_search(key, children, 0, current_size);

        let left_node_index = table_with_length::length(&self.nodes) + 2;

        if (parent_index == NULL_INDEX) {
            // Splitting root now, need to create a new root.
            parent_index = table_with_length::length(&self.nodes) + 3;
            node.parent = parent_index;

            self.root_index = parent_index;
            let parent_node = new_node(/*is_leaf=*/false, /*parent=*/NULL_INDEX);
            let max_element = children.borrow(current_size - 1).max_key;
            if (cmp::compare(&max_element, &key).is_less_than()) {
                max_element = key;
            };
            parent_node.children.push_back(new_inner_child(max_element, node_index));
            table_with_length::add(&mut self.nodes, parent_index, parent_node);
        };

        let new_node_children = if (l < target_size) {
            let new_node_children = children.split_off(target_size - 1);
            children.insert(l, child);
            new_node_children
        } else {
            children.insert(l, child);
            children.split_off(target_size)
        };

        assert!(children.length() <= max_degree, E_INTERNAL);
        assert!(new_node_children.length() <= max_degree, E_INTERNAL);

        let right_node = new_node_with_children(*is_leaf, parent_index, new_node_children);

        right_node.next = *next;
        *next = node_index;
        right_node.prev = left_node_index;
        if (*prev != NULL_INDEX) {
            self.nodes.borrow_mut(*prev).next = left_node_index;
        };

        if (!*is_leaf) {
            let i = 0;
            while (i < target_size) {
                self.nodes.borrow_mut(children.borrow(i).node_index).parent = left_node_index;
                i = i + 1;
            };
        };

        let split_key = children.borrow(target_size - 1).max_key;

        table_with_length::add(&mut self.nodes, left_node_index, node);
        table_with_length::add(&mut self.nodes, node_index, right_node);
        if (node_index == self.min_leaf_index) {
            self.min_leaf_index = left_node_index;
        };
        self.insert_at(parent_index, new_inner_child(split_key, left_node_index));
    }

    fun update_key<K: drop + copy + store, V: store>(self: &mut BTreeMap<K, V>, node_index: u64, old_key: K, new_key: K) {
        if (node_index == NULL_INDEX) {
            return
        };

        let node = self.nodes.borrow_mut(node_index);
        let keys = &mut node.children;
        let current_size = keys.length();

        let index = binary_search(old_key, keys, 0, current_size);

        keys.borrow_mut(index).max_key = new_key;
        move keys;

        if (index == current_size - 1) {
            self.update_key(node.parent, old_key, new_key);
        };
    }

    fun remove_at<K: drop + copy + store, V: store>(self: &mut BTreeMap<K, V>, node_index: u64, key: K): Child<K, V> {
        let (old_child, current_size) = {
            let node = self.nodes.borrow_mut(node_index);

            let children = &mut node.children;
            let current_size = children.length();

            if (current_size == 1 && node_index == self.root_index) {
                // Remove the only element at root node.
                // assert!(node_index == self.root_index, E_INTERNAL);
                assert!(key == children.borrow(0).max_key, E_INTERNAL);
                return children.pop_back();
            };

            let is_leaf = node.is_leaf;

            let index = binary_search(key, children, 0, current_size);

            assert!(index < current_size, E_INTERNAL);

            let max_key_updated = index == (current_size - 1);
            let old_child = children.remove(index);
            current_size = current_size - 1;

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
                        max_key: _,
                    } = children.pop_back();
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

                let new_max_key = children.borrow(current_size - 1).max_key;
                let parent = node.parent;

                self.update_key(parent, key, new_max_key);

                if (big_enough) {
                    return old_child;
                }
            };

            (old_child, current_size)
        };

        // We need to update tree beyond the current node

        let node = self.nodes.remove(node_index);

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
        let brother_node = self.nodes.remove(brother_index);
        let brother_children = &mut brother_node.children;
        let brother_size = brother_children.length();

        if ((brother_size - 1) * 2 >= max_degree) {
            // The brother node has enough elements, borrow an element from the brother node.
            brother_size = brother_size - 1;
            if (brother_index == next) {
                let borrowed_element = brother_children.remove(0);
                if (borrowed_element is Child::Inner<K, V>) {
                    self.nodes.borrow_mut(borrowed_element.node_index).parent = node_index;
                };
                let borrowed_max_key = borrowed_element.max_key;
                children.push_back(borrowed_element);
                current_size = current_size + 1;
                self.update_key(parent, children.borrow(current_size - 2).max_key, borrowed_max_key);
            } else {
                let borrowed_element = brother_children.pop_back();
                if (borrowed_element is Child::Inner<K, V>) {
                    self.nodes.borrow_mut(borrowed_element.node_index).parent = node_index;
                };
                children.insert(0, borrowed_element);
                // unused, so unnecessary to update
                // current_size = current_size + 1;
                self.update_key(parent, children.borrow(0).max_key, brother_children.borrow(brother_size - 1).max_key);
            };

            self.nodes.add(node_index, node);
            self.nodes.add(brother_index, brother_node);
            return old_child;
        };

        // The brother node doesn't have enough elements to borrow, merge with the brother node.
        if (brother_index == next) {
            if (!is_leaf) {
                let len = children.length();
                let i = 0;
                while (i < len) {
                    self.nodes.borrow_mut(children.borrow(i).node_index).parent = brother_index;
                    i = i + 1;
                };
            };
            let Node { children: brother_children, is_leaf: _, parent: _, prev: _, next: brother_next } = brother_node;
            children.append(brother_children);
            node.next = brother_next;
            let key_to_remove = children.borrow(current_size - 1).max_key;

            move children;

            if (node.next != NULL_INDEX) {
                self.nodes.borrow_mut(node.next).prev = brother_index;
            };
            if (node.prev != NULL_INDEX) {
                self.nodes.borrow_mut(node.prev).next = brother_index;
            };

            self.nodes.add(brother_index, node);
            if (self.min_leaf_index == node_index) {
                self.min_leaf_index = brother_index;
            };

            if (parent != NULL_INDEX) {
                destroy_inner_child(self.remove_at(parent, key_to_remove));
            };
        } else {
            if (!is_leaf) {
                let len = brother_children.length();
                let i = 0;
                while (i < len) {
                    self.nodes.borrow_mut(brother_children.borrow(i).node_index).parent = node_index;
                    i = i + 1;
                };
            };
            let Node { children: node_children, is_leaf: _, parent: _, prev: _, next: node_next } = node;
            brother_children.append(node_children);
            brother_node.next = node_next;
            let key_to_remove = brother_children.borrow(brother_size - 1).max_key;

            move brother_children;

            if (brother_node.next != NULL_INDEX) {
                self.nodes.borrow_mut(brother_node.next).prev = node_index;
            };
            if (brother_node.prev != NULL_INDEX) {
                self.nodes.borrow_mut(brother_node.prev).next = node_index;
            };

            self.nodes.add(node_index, brother_node);
            if (self.min_leaf_index == brother_index) {
                self.min_leaf_index = node_index;
            };

            if (parent != NULL_INDEX) {
                destroy_inner_child(self.remove_at(parent, key_to_remove));
            };
        };
        old_child
    }

    ///////////
    // Tests //
    ///////////

    #[test_only]
    fun destroy<K: drop + copy + store, V: drop + store>(self: BTreeMap<K, V>) {
        let it = new_begin_iter(&self);
        while (!is_end_iter(&self, &it)) {
            remove(&mut self, it.key);
            assert!(is_end_iter(&self, &find(&self, it.key)), E_INTERNAL);
            it = new_begin_iter(&self);
            self.validate_tree();
        };

        self.destroy_empty();
    }

    #[test_only]
    fun validate_iteration<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>) {
        let expected_num_elements = size(self);
        let num_elements = 0;
        let it = new_begin_iter(self);
        while (!is_end_iter(self, &it)) {
            num_elements = num_elements + 1;
            it = next_iter_or_die(self, it);
        };
        assert!(num_elements == expected_num_elements, E_INTERNAL);

        let num_elements = 0;
        let it = new_end_iter(self);
        while (!is_begin_iter(self, &it)) {
            it = prev_iter_or_die(self, it);
            num_elements = num_elements + 1;
        };
        assert!(num_elements == expected_num_elements, E_INTERNAL);

        let it = new_end_iter(self);
        if (!is_begin_iter(self, &it)) {
            it = prev_iter_or_die(self, it);
            assert!(it.node_index == self.max_leaf_index, E_INTERNAL);
        } else {
            assert!(expected_num_elements == 0, E_INTERNAL);
        };
    }

    #[test_only]
    fun validate_subtree<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>, node_index: u64, expected_max_key: Option<K>, expected_parent: u64) {
        let node = table_with_length::borrow(&self.nodes, node_index);
        let len = vector::length(&node.children);
        assert!(len <= self.get_max_degree(node.is_leaf), E_INTERNAL);

        if (node_index != self.root_index) {
            assert!(len >= 1, E_INTERNAL);
            assert!(len * 2 >= self.get_max_degree(node.is_leaf) || node_index == self.root_index, E_INTERNAL);
        };

        assert!(node.parent == expected_parent, E_INTERNAL);

        let i = 1;
        while (i < len) {
            assert!(cmp::compare(&node.children.borrow(i).max_key, &node.children.borrow(i - 1).max_key).is_greater_than(), E_INTERNAL);
            i = i + 1;
        };

        if (!node.is_leaf) {
            let i = 0;
            while (i < len) {
                let child = node.children.borrow(i);
                self.validate_subtree(child.node_index, option::some(child.max_key), node_index);
                i = i + 1;
            };
        } else {
            let i = 0;
            while (i < len) {
                let child = node.children.borrow(i);
                assert!((child is Child::Leaf<K, V>), E_INTERNAL);
                i = i + 1;
            };
        };

        if (option::is_some(&expected_max_key)) {
            let expected_max_key = option::extract(&mut expected_max_key);
            assert!(expected_max_key == node.children.borrow(len - 1).max_key, E_INTERNAL);
        };
    }

    #[test_only]
    fun validate_tree<K: drop + copy + store, V: store>(self: &BTreeMap<K, V>) {
        self.validate_subtree(self.root_index, option::none(), NULL_INDEX);
        self.validate_iteration();
    }

    #[test]
    fun test_smart_tree() {
        let tree = new_with_config(5, 3);
        print_tree(&tree);
        insert(&mut tree, 1, 1); print_tree(&tree);
        insert(&mut tree, 2, 2); print_tree(&tree);
        let r1 = upsert(&mut tree, 3, 3); print_tree(&tree);
        assert!(r1 == option::none(), E_INTERNAL);
        insert(&mut tree, 4, 4); print_tree(&tree);
        let r2 = upsert(&mut tree, 4, 8); print_tree(&tree);
        assert!(r2 == option::some(4), E_INTERNAL);
        insert(&mut tree, 5, 5); print_tree(&tree);
        insert(&mut tree, 6, 6); print_tree(&tree);

        remove(&mut tree, 5); print_tree(&tree);
        remove(&mut tree, 4); print_tree(&tree);
        remove(&mut tree, 1); print_tree(&tree);
        remove(&mut tree, 3); print_tree(&tree);
        remove(&mut tree, 2); print_tree(&tree);
        remove(&mut tree, 6); print_tree(&tree);

        destroy_empty(tree);
    }

    #[test]
    fun test_iterator() {
        let tree = new_with_config(5, 5);

        let data = vector[1, 7, 5, 8, 4, 2, 6, 3, 9, 0];
        while (vector::length(&data) != 0) {
            let element = vector::pop_back(&mut data);
            insert(&mut tree, element, element);
        };

        let it = new_begin_iter(&tree);

        let i = 0;
        while (!is_end_iter(&tree, &it)) {
            assert!(i == it.key, E_INTERNAL);
            i = i + 1;
            it = next_iter_or_die(&tree, it);
        };

        destroy(tree);
    }

    #[test]
    fun test_find() {
        let tree = new_with_config(5, 5);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 0, 10];

        let i = 0;
        let len = vector::length(&data);
        while (i < len) {
            let element = *vector::borrow(&data, i);
            insert(&mut tree, element, element);
            i = i + 1;
        };

        let i = 0;
        while (i < len) {
            let element = *vector::borrow(&data, i);
            let it = find(&tree, element);
            assert!(!is_end_iter(&tree, &it), E_INTERNAL);
            assert!(it.key == element, E_INTERNAL);
            i = i + 1;
        };

        assert!(is_end_iter(&tree, &find(&tree, 4)), E_INTERNAL);
        assert!(is_end_iter(&tree, &find(&tree, 9)), E_INTERNAL);

        destroy(tree);
    }

    #[test]
    fun test_lower_bound() {
        let tree = new_with_config(5, 5);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 12, 10];

        let i = 0;
        let len = vector::length(&data);
        while (i < len) {
            let element = *vector::borrow(&data, i);
            insert(&mut tree, element, element);
            i = i + 1;
        };

        let i = 0;
        while (i < len) {
            let element = *vector::borrow(&data, i);
            let it = lower_bound(&tree, element);
            assert!(!is_end_iter(&tree, &it), E_INTERNAL);
            assert!(it.key == element, E_INTERNAL);
            i = i + 1;
        };

        assert!(lower_bound(&tree, 0).key == 1, E_INTERNAL);
        assert!(lower_bound(&tree, 4).key == 5, E_INTERNAL);
        assert!(lower_bound(&tree, 9).key == 10, E_INTERNAL);
        assert!(is_end_iter(&tree, &lower_bound(&tree, 13)), E_INTERNAL);

        remove(&mut tree, 3);
        assert!(lower_bound(&tree, 3).key == 5, E_INTERNAL);
        remove(&mut tree, 5);
        assert!(lower_bound(&tree, 3).key == 6, E_INTERNAL);
        assert!(lower_bound(&tree, 4).key == 6, E_INTERNAL);

        destroy(tree);
    }

    #[test_only]
    fun test_large_data_set_helper(inner_max_degree: u16, leaf_max_degree: u16) {
        let tree = new_with_config(inner_max_degree, leaf_max_degree);
        let data = vector[383, 886, 777, 915, 793, 335, 386, 492, 649, 421, 362, 27, 690, 59, 763, 926, 540, 426, 172, 736, 211, 368, 567, 429, 782, 530, 862, 123, 67, 135, 929, 802, 22, 58, 69, 167, 393, 456, 11, 42, 229, 373, 421, 919, 784, 537, 198, 324, 315, 370, 413, 526, 91, 980, 956, 873, 862, 170, 996, 281, 305, 925, 84, 327, 336, 505, 846, 729, 313, 857, 124, 895, 582, 545, 814, 367, 434, 364, 43, 750, 87, 808, 276, 178, 788, 584, 403, 651, 754, 399, 932, 60, 676, 368, 739, 12, 226, 586, 94, 539, 795, 570, 434, 378, 467, 601, 97, 902, 317, 492, 652, 756, 301, 280, 286, 441, 865, 689, 444, 619, 440, 729, 31, 117, 97, 771, 481, 675, 709, 927, 567, 856, 497, 353, 586, 965, 306, 683, 219, 624, 528, 871, 732, 829, 503, 19, 270, 368, 708, 715, 340, 149, 796, 723, 618, 245, 846, 451, 921, 555, 379, 488, 764, 228, 841, 350, 193, 500, 34, 764, 124, 914, 987, 856, 743, 491, 227, 365, 859, 936, 432, 551, 437, 228, 275, 407, 474, 121, 858, 395, 29, 237, 235, 793, 818, 428, 143, 11, 928, 529];

        let shuffled_data = vector[895, 228, 530, 784, 624, 335, 729, 818, 373, 456, 914, 226, 368, 750, 428, 956, 437, 586, 763, 235, 567, 91, 829, 690, 434, 178, 584, 426, 228, 407, 237, 497, 764, 135, 124, 421, 537, 270, 11, 367, 378, 856, 529, 276, 729, 618, 929, 227, 149, 788, 925, 675, 121, 795, 306, 198, 421, 350, 555, 441, 403, 932, 368, 383, 928, 841, 440, 771, 364, 902, 301, 987, 467, 873, 921, 11, 365, 340, 739, 492, 540, 386, 919, 723, 539, 87, 12, 782, 324, 862, 689, 395, 488, 793, 709, 505, 582, 814, 245, 980, 936, 736, 619, 69, 370, 545, 764, 886, 305, 551, 19, 865, 229, 432, 29, 754, 34, 676, 43, 846, 451, 491, 871, 500, 915, 708, 586, 60, 280, 652, 327, 172, 856, 481, 796, 474, 219, 651, 170, 281, 84, 97, 715, 857, 353, 862, 393, 567, 368, 777, 97, 315, 526, 94, 31, 167, 123, 413, 503, 193, 808, 649, 143, 42, 444, 317, 67, 926, 434, 211, 379, 570, 683, 965, 732, 927, 429, 859, 313, 528, 996, 117, 492, 336, 22, 399, 275, 802, 743, 124, 846, 58, 858, 286, 756, 601, 27, 59, 362, 793];

        let i = 0;
        let len = vector::length(&data);
        while (i < len) {
            let element = *vector::borrow(&data, i);
            upsert(&mut tree, element, element);
            validate_tree(&tree);
            i = i + 1;
        };

        let i = 0;
        while (i < len) {
            let element = *vector::borrow(&shuffled_data, i);
            let it = find(&tree, element);
            assert!(!is_end_iter(&tree, &it), E_INTERNAL);
            assert!(it.key == element, E_INTERNAL);
            let it_next = lower_bound(&tree, element + 1);
            assert!(it_next == next_iter_or_die(&tree, it), E_INTERNAL);

            i = i + 1;
        };

        destroy(tree);
    }

    #[test]
    fun test_large_data_set_order_5() {
        test_large_data_set_helper(5, 5);
    }

    #[test]
    fun test_large_data_set_order_4_3() {
        test_large_data_set_helper(4, 3);
    }

    #[test]
    fun test_large_data_set_order_4_4() {
        test_large_data_set_helper(4, 4);
    }

    #[test]
    fun test_large_data_set_order_6() {
        test_large_data_set_helper(6, 6);
    }

    #[test]
    fun test_large_data_set_order_6_3() {
        test_large_data_set_helper(6, 3);
    }

    #[test]
    fun test_large_data_set_order_4_6() {
        test_large_data_set_helper(4, 6);
    }

    #[test]
    fun test_large_data_set_order_16() {
        test_large_data_set_helper(16, 16);
    }

    #[test]
    fun test_large_data_set_order_31() {
        test_large_data_set_helper(31, 31);
    }

    #[test]
    fun test_large_data_set_order_31_3() {
        test_large_data_set_helper(31, 3);
    }

    #[test]
    fun test_large_data_set_order_31_5() {
        test_large_data_set_helper(31, 5);
    }

    #[test]
    fun test_large_data_set_order_32() {
        test_large_data_set_helper(32, 32);
    }
}
