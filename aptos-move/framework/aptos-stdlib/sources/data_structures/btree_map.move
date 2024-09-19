/// Type of large-scale search trees.
///
/// It internally uses BTree to organize the search tree data structure for keys. Comparing with
/// other common search trees like AVL or Red-black tree, a BTree node has more children, and packs
/// more metadata into one node, which is more disk friendly (and gas friendly).

module aptos_std::btree_map {
    use aptos_std::bucket_table::{Self, BucketTable};
    use std::option::{Self, Option};
    use std::vector;

    // Internal errors.
    const E_INTERNAL: u64 = 0;
    // The tree is not empty, and cannot be destroyed.
    const E_TREE_NOT_EMPTY: u64 = 1;
    // The tree is too big for insertion.
    const E_TREE_TOO_BIG: u64 = 2;
    // The provided parameter is invalid.
    const E_INVALID_PARAMETER: u64 = 3;

    const NULL_INDEX: u64 = 0;
    const DEFAULT_ORDER : u8 = 32;
    const MAX_NUM_NODES : u64 = 1048576;

    /// A node of the SmartTree.
    struct Node has drop, store {
        // Whether this node is a leaf node.
        is_leaf: bool,
        // The node index of its parent node, or NULL_INDEX if it doesn't have parent.
        parent: u64,
        // The children of the nodes. (When the node is leaf node, all keys of the node is stored in children.max_key)
        children: vector<Child>,
        // The node index of its previous node at the same level, or NULL_INDEX if it doesn't have a previous node.
        prev: u64,
        // The node index of its next node at the same level, or NULL_INDEX if it doesn't have a next node.
        next: u64,
    }

    /// The metadata of a child of a node.
    struct Child has copy, drop, store {
        // The max key of its child, or the key of the current node if it is a leaf node.
        max_key: u64,
        // The node index of it's child, or NULL_INDEX if the current node is a leaf node.
        node_index: u64,
    }

    /// An iterator to iterate all keys in the SmartTree.
    struct Iterator has copy, drop {
        // The node index of the iterator pointing to.
        node_index: u64,
        // The child index of the iterator pointing to.
        child_index: u64,
        // The key of the iterator pointing to, not valid when the iterator is an end iterator.
        key: u64,
    }

    /// The SmartTree data structure.
    struct SmartTree<V> has store {
        // The node index of the root node.
        root_index: u64,
        // Mapping of node_index -> node.
        nodes: BucketTable<u64, Node>,
        // Mapping of key -> value.
        entries: BucketTable<u64, V>,
        // The max number of children a node can have.
        order: u8,
        // The node index of the leftmost node.
        min_leaf_index: u64,
        // The node index of the rightmost node.
        max_leaf_index: u64,
    }

    /////////////////////////////////
    // Constructors && Destructors //
    /////////////////////////////////

    /// Returns a new SmartTree with the default configuration.
    public fun new<V: store>(): SmartTree<V> {
        new_with_order(DEFAULT_ORDER)
    }

    /// Returns a new SmartTree with the provided order (the maximum # of children a node can have).
    public fun new_with_order<V: store>(order: u8): SmartTree<V> {
        assert!(order >= 5, E_INVALID_PARAMETER);
        let root_node = new_node(/*is_leaf=*/true, /*parent=*/NULL_INDEX);
        let nodes = bucket_table::new(1);
        let root_index = 1;
        bucket_table::add(&mut nodes, root_index, root_node);
        SmartTree {
            root_index: root_index,
            nodes: nodes,
            entries: bucket_table::new(1),
            order: order,
            min_leaf_index: root_index,
            max_leaf_index: root_index,
        }
    }

    /// Destroys the tree if it's empty, otherwise aborts.
    public fun destroy_empty<V>(tree: SmartTree<V>) {
        let SmartTree { entries, nodes, root_index, order: _, min_leaf_index: _, max_leaf_index: _ } = tree;
        assert!(bucket_table::length(&entries) == 0, E_TREE_NOT_EMPTY);
        assert!(bucket_table::length(&nodes) == 1, E_TREE_NOT_EMPTY);
        bucket_table::remove(&mut nodes, &root_index);
        bucket_table::destroy_empty(nodes);
        bucket_table::destroy_empty(entries);
    }

    ///////////////
    // Modifiers //
    ///////////////

    /// Inserts the key/value into the SmartTree.
    /// Aborts if the key is already in the tree.
    public fun insert<V>(tree: &mut SmartTree<V>, key: u64, value: V) {
        assert!(size(tree) < MAX_NUM_NODES, E_TREE_TOO_BIG);

        bucket_table::add(&mut tree.entries, key, value);

        let leaf = find_leaf(tree, key);

        if (leaf == NULL_INDEX) {
            // In this case, the key is greater than all keys in the tree.
            leaf = tree.max_leaf_index;
            let current = bucket_table::borrow(&tree.nodes, leaf).parent;
            while (current != NULL_INDEX) {
                let current_node = bucket_table::borrow_mut(&mut tree.nodes, current);
                let last_index = vector::length(&current_node.children) - 1;
                let last_element = vector::borrow_mut(&mut current_node.children, last_index);
                last_element.max_key = key;
                current = current_node.parent;
            }
        };

        insert_at(tree, leaf, new_child(key, NULL_INDEX));
    }

    /// If the key doesn't exist in the tree, inserts the key/value, and returns none.
    /// Otherwise updates the value under the given key, and returns the old value.
    public fun upsert<V>(tree: &mut SmartTree<V>, key: u64, value: V): Option<V> {
        if (!bucket_table::contains(&tree.entries, &key)) {
            insert(tree, key, value);
            return option::none()
        };

        let old_value = bucket_table::remove(&mut tree.entries, &key);
        bucket_table::add(&mut tree.entries, key, value);
        option::some(old_value)
    }

    /// Removes the entry from SmartTree and returns the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<V>(tree: &mut SmartTree<V>, key: u64): V {
        let value = bucket_table::remove(&mut tree.entries, &key);
        let leaf = find_leaf(tree, key);
        assert!(leaf != NULL_INDEX, E_INTERNAL);

        remove_at(tree, leaf, key);
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
    public fun is_begin_iter<V>(tree: &SmartTree<V>, iter: &Iterator): bool {
        (empty(tree) && iter.node_index == NULL_INDEX) || (iter.node_index == tree.min_leaf_index && iter.child_index == 0)
    }

    // Returns true iff the iterator is an end iterator.
    public fun is_end_iter<V>(_tree: &SmartTree<V>, iter: &Iterator): bool {
        iter.node_index == NULL_INDEX
    }

    /// Returns an iterator pointing to the first element that is greater or equal to the provided
    /// key, or an end iterator if such element doesn't exist.
    public fun lower_bound<V>(tree: &SmartTree<V>, key: u64): Iterator {
        let leaf = find_leaf(tree, key);
        if (leaf == NULL_INDEX) {
            return new_end_iter(tree)
        };

        let node = bucket_table::borrow(&tree.nodes, leaf);
        assert!(node.is_leaf, E_INTERNAL);

        let keys = &node.children;

        let len = vector::length(keys);

        let l = 0;
        let r = len;

        while (l != r) {
            let mid = l + (r - l) / 2;
            if (vector::borrow(keys, mid).max_key < key) {
                l = mid + 1;
            } else {
                r = mid;
            };
        };

        new_iter(leaf, l, vector::borrow(keys, l).max_key)
    }

    /// Returns an iterator pointing to the element that equals to the provided key, or an end
    /// iterator if the key is not found.
    public fun find<V>(tree: &SmartTree<V>, key: u64): Iterator {
        if (!bucket_table::contains(&tree.entries, &key)) {
            return new_end_iter(tree)
        };

        lower_bound(tree, key)
    }

    /// Returns true iff the key exists in the tree.
    public fun contains<V>(tree: &SmartTree<V>, key: u64): bool {
        bucket_table::contains(&tree.entries, &key)
    }

    /// Returns the key of the given iterator.
    public fun get_key(iter: Iterator): u64 {
        assert!(iter.node_index != NULL_INDEX, E_INVALID_PARAMETER);
        iter.key
    }

    /// Returns a reference to the element with its key, aborts if the key is not found.
    public fun borrow<V>(tree: &SmartTree<V>, key: u64): &V {
        bucket_table::borrow(&tree.entries, key)
    }

    /// Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.
    public fun borrow_mut<V>(tree: &mut SmartTree<V>, key: u64): &mut V {
        bucket_table::borrow_mut(&mut tree.entries, key)
    }

    /// Returns the number of elements in the SmartTree.
    public fun size<V>(tree: &SmartTree<V>): u64 {
        bucket_table::length(&tree.entries)
    }

    /// Returns true iff the SmartTree is empty.
    public fun empty<V>(tree: &SmartTree<V>): bool {
        bucket_table::length(&tree.entries) == 0
    }

    /// Return the begin iterator.
    public fun new_begin_iter<V>(tree: &SmartTree<V>): Iterator {
        if (empty(tree)) {
            return new_iter(NULL_INDEX, 0, 0)
        };

        let node = bucket_table::borrow(&tree.nodes, tree.min_leaf_index);
        let key = vector::borrow(&node.children, 0).max_key;

        new_iter(tree.min_leaf_index, 0, key)
    }

    /// Return the end iterator.
    public fun new_end_iter<V>(_tree: &SmartTree<V>): Iterator {
        new_iter(NULL_INDEX, 0, 0)
    }

    /// Returns the next iterator, or none if already at the end iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun next_iter<V>(tree: &SmartTree<V>, iter: Iterator): Option<Iterator> {
        let node_index = iter.node_index;
        if (node_index == NULL_INDEX) {
            return option::none()
        };

        option::some(next_iter_or_die(tree, iter))
    }

    /// Returns the next iterator, aborts if already at the end iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun next_iter_or_die<V>(tree: &SmartTree<V>, iter: Iterator): Iterator {
        assert!(iter.node_index != NULL_INDEX, E_INVALID_PARAMETER);

        let node_index = iter.node_index;

        let node = bucket_table::borrow(&tree.nodes, node_index);
        iter.child_index = iter.child_index + 1;
        if (iter.child_index < vector::length(&node.children)) {
            iter.key = vector::borrow(&node.children, iter.child_index).max_key;
            return iter
        };

        let next_index = node.next;
        if (next_index != NULL_INDEX) {
            let next_node = bucket_table::borrow(&tree.nodes, next_index);
            iter.node_index = next_index;
            iter.child_index = 0;
            iter.key = vector::borrow(&next_node.children, 0).max_key;
            return iter
        };

        new_end_iter(tree)
    }

    /// Returns the previous iterator, or none if already at the begin iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun prev_iter<V>(tree: &SmartTree<V>, iter: Iterator): Option<Iterator> {
        if (iter.node_index == tree.min_leaf_index && iter.child_index == 0) {
            return option::none()
        };

        option::some(prev_iter_or_die(tree, iter))
    }

    /// Returns the previous iterator, aborts if already at the begin iterator.
    /// Requires the tree is not changed after the input iterator is generated.
    public fun prev_iter_or_die<V>(tree: &SmartTree<V>, iter: Iterator): Iterator {
        let node_index = iter.node_index;

        let prev_index;

        if (node_index == NULL_INDEX) {
            prev_index = tree.max_leaf_index;
        } else {
            let node = bucket_table::borrow(&tree.nodes, node_index);
            if (iter.child_index >= 1) {
                iter.child_index = iter.child_index - 1;
                iter.key = vector::borrow(&node.children, iter.child_index).max_key;
                return iter
            };
            prev_index = node.prev;
        };

        assert!(prev_index != NULL_INDEX, E_INTERNAL);

        let prev_node = bucket_table::borrow(&tree.nodes, prev_index);
        let len = vector::length(&prev_node.children);
        iter.node_index = prev_index;
        iter.child_index = len - 1;
        iter.key = vector::borrow(&prev_node.children, len - 1).max_key;
        iter
    }

    //////////////////////////////
    // Internal Implementations //
    //////////////////////////////

    fun new_node(is_leaf: bool, parent: u64): Node {
        Node {
            is_leaf: is_leaf,
            parent: parent,
            children: vector::empty(),
            prev: NULL_INDEX,
            next: NULL_INDEX,
        }
    }

    fun new_child(max_key: u64, node_index: u64): Child {
        Child {
            max_key: max_key,
            node_index: node_index,
        }
    }

    fun new_iter(node_index: u64, child_index: u64, key: u64): Iterator {
        Iterator {
            node_index: node_index,
            child_index: child_index,
            key: key,
        }
    }

    fun find_leaf<V>(tree: &SmartTree<V>, key: u64): u64 {
        let current = tree.root_index;
        while (current != NULL_INDEX) {
            let node = bucket_table::borrow(&tree.nodes, current);
            if (node.is_leaf) {
                return current
            };
            let len = vector::length(&node.children);
            if (vector::borrow(&node.children, len - 1).max_key < key) {
                return NULL_INDEX
            };

            let l = 0;
            let r = len;
            while (l != r) {
                let mid = l + (r - l) / 2;
                if (vector::borrow(&node.children, mid).max_key < key) {
                    l = mid + 1;
                } else {
                    r = mid;
                };
            };

            current = vector::borrow(&node.children, l).node_index;
        };

        NULL_INDEX
    }

    fun insert_at<V>(tree: &mut SmartTree<V>, node_index: u64, child: Child) {
        let node = bucket_table::remove(&mut tree.nodes, &node_index);
        let parent_index = node.parent;
        let children = &mut node.children;
        let is_leaf = &mut node.is_leaf;
        let next = &mut node.next;
        let prev = &mut node.prev;
        let current_size = vector::length(children);
        let key = child.max_key;

        if (current_size < (tree.order as u64)) {
            // Do not need to split.
            let i = current_size;
            vector::push_back(children, new_child(0, 0));
            while (i > 0) {
                let previous_child = vector::borrow(children, i - 1);
                if (previous_child.max_key < key) {
                    break
                };
                *vector::borrow_mut(children, i) = *previous_child;
                i = i - 1;
            };
            *vector::borrow_mut(children, i) = child;
            bucket_table::add(&mut tree.nodes, node_index, node);
            return
        };

        // # of children in the current node exceeds the threshold, need to split into two nodes.

        let target_size = ((tree.order as u64) + 1) / 2;

        let l = 0;
        let r = current_size;
        while (l != r) {
            let mid = l + (r - l) / 2;
            if (vector::borrow(children, mid).max_key < key) {
                l = mid + 1;
            } else {
                r = mid;
            };
        };

        let left_node_index = bucket_table::length(&tree.nodes) + 2;

        if (parent_index == NULL_INDEX) {
            // Splitting root now, need to create a new root.
            parent_index = bucket_table::length(&tree.nodes) + 3;
            node.parent = parent_index;

            tree.root_index = parent_index;
            let parent_node = new_node(/*is_leaf=*/false, /*parent=*/NULL_INDEX);
            let max_element = vector::borrow(children, current_size - 1).max_key;
            if (max_element < key) {
                max_element = key;
            };
            vector::push_back(&mut parent_node.children, new_child(max_element, node_index));
            bucket_table::add(&mut tree.nodes, parent_index, parent_node);
        };

        let right_node = new_node(*is_leaf, parent_index);

        right_node.next = *next;
        *next = node_index;
        right_node.prev = left_node_index;
        if (*prev != NULL_INDEX) {
            bucket_table::borrow_mut(&mut tree.nodes, *prev).next = left_node_index;
        };

        if (l < target_size) {
            let i = target_size - 1;
            while (i < current_size) {
                vector::push_back(&mut right_node.children, *vector::borrow(children, i));
                i = i + 1;
            };

            while (current_size > target_size) {
                vector::pop_back(children);
                current_size = current_size - 1;
            };

            i = target_size - 1;
            while (i > l) {
                *vector::borrow_mut(children, i) = *vector::borrow(children, i - 1);
                i = i - 1;
            };
            *vector::borrow_mut(children, l) = child;
        } else {
            let i = target_size;
            while (i < l) {
                vector::push_back(&mut right_node.children, *vector::borrow(children, i));
                i = i + 1;
            };
            vector::push_back(&mut right_node.children, child);
            while (i < current_size) {
                vector::push_back(&mut right_node.children, *vector::borrow(children, i));
                i = i + 1;
            };

            while (current_size > target_size) {
                vector::pop_back(children);
                current_size = current_size - 1;
            };
        };

        if (!*is_leaf) {
            let i = 0;
            while (i < target_size) {
                bucket_table::borrow_mut(&mut tree.nodes, vector::borrow(children, i).node_index).parent = left_node_index;
                i = i + 1;
            };
        };

        let split_key = vector::borrow(children, target_size - 1).max_key;

        bucket_table::add(&mut tree.nodes, left_node_index, node);
        bucket_table::add(&mut tree.nodes, node_index, right_node);
        if (node_index == tree.min_leaf_index) {
            tree.min_leaf_index = left_node_index;
        };
        insert_at(tree, parent_index, new_child(split_key, left_node_index));
    }

    fun update_key<V>(tree: &mut SmartTree<V>, node_index: u64, old_key: u64, new_key: u64) {
        if (node_index == NULL_INDEX) {
            return
        };

        let node = bucket_table::borrow_mut(&mut tree.nodes, node_index);
        let keys = &mut node.children;
        let current_size = vector::length(keys);

        let l = 0;
        let r = current_size;

        while (l != r) {
            let mid = l + (r - l) / 2;
            if (vector::borrow(keys, mid).max_key < old_key) {
                l = mid + 1;
            } else {
                r = mid;
            };
        };

        vector::borrow_mut(keys, l).max_key = new_key;
        move keys;

        if (l == current_size - 1) {
            update_key(tree, node.parent, old_key, new_key);
        };
    }

    fun remove_at<V>(tree: &mut SmartTree<V>, node_index: u64, key: u64) {
        let node = bucket_table::remove(&mut tree.nodes, &node_index);
        let prev = node.prev;
        let next = node.next;
        let parent = node.parent;
        let is_leaf = node.is_leaf;

        let children = &mut node.children;
        let current_size = vector::length(children);

        if (current_size == 1) {
            // Remove the only element at root node.
            assert!(node_index == tree.root_index, E_INTERNAL);
            assert!(key == vector::borrow(children, 0).max_key, E_INTERNAL);
            vector::pop_back(children);
            bucket_table::add(&mut tree.nodes, node_index, node);
            return
        };

        let l = 0;
        let r = current_size;

        while (l != r) {
            let mid = l + (r - l) / 2;
            if (vector::borrow(children, mid).max_key < key) {
                l = mid + 1;
            } else {
                r = mid;
            };
        };

        current_size = current_size - 1;

        if (l == current_size) {
            update_key(tree, parent, key, vector::borrow(children, l - 1).max_key);
        };

        while (l < current_size) {
            *vector::borrow_mut(children, l) = *vector::borrow(children, l + 1);
            l = l + 1;
        };
        vector::pop_back(children);

        if (current_size * 2 >= (tree.order as u64)) {
            bucket_table::add(&mut tree.nodes, node_index, node);
            return
        };

        if (node_index == tree.root_index) {
            if (current_size == 1 && !is_leaf) {
                tree.root_index = vector::borrow(children, 0).node_index;
                bucket_table::borrow_mut(&mut tree.nodes, tree.root_index).parent = NULL_INDEX;
            } else {
                bucket_table::add(&mut tree.nodes, node_index, node);
            };
            return
        };

        // Children size is below threshold.

        let brother_index = next;
        if (brother_index == NULL_INDEX || bucket_table::borrow(&tree.nodes, brother_index).parent != parent) {
            brother_index = prev;
        };
        let brother_node = bucket_table::remove(&mut tree.nodes, &brother_index);
        let brother_children = &mut brother_node.children;
        let brother_size = vector::length(brother_children);

        if ((brother_size - 1) * 2 >= (tree.order as u64)) {
            // The brother node has enough elements, borrow an element from the brother node.
            brother_size = brother_size - 1;
            if (brother_index == next) {
                let borrowed_element = *vector::borrow(brother_children, 0);
                vector::push_back(children, borrowed_element);
                if (borrowed_element.node_index != NULL_INDEX) {
                    bucket_table::borrow_mut(&mut tree.nodes, borrowed_element.node_index).parent = node_index;
                };
                let i = 0;
                while (i < brother_size) {
                    *vector::borrow_mut(brother_children, i) = *vector::borrow(brother_children, i + 1);
                    i = i + 1;
                };
                vector::pop_back(brother_children);
                update_key(tree, parent, vector::borrow(children, current_size - 2).max_key, borrowed_element.max_key);
            } else {
                let i = current_size;
                while (i > 0) {
                    *vector::borrow_mut(children, i) = *vector::borrow(children, i - 1);
                    i = i - 1;
                };
                let borrowed_element = vector::pop_back(brother_children);
                if (borrowed_element.node_index != NULL_INDEX) {
                    bucket_table::borrow_mut(&mut tree.nodes, borrowed_element.node_index).parent = node_index;
                };
                *vector::borrow_mut(children, 0) = borrowed_element;
                update_key(tree, parent, vector::borrow(children, 0).max_key, vector::borrow(brother_children, brother_size - 1).max_key);
            };

            bucket_table::add(&mut tree.nodes, node_index, node);
            bucket_table::add(&mut tree.nodes, brother_index, brother_node);
            return
        };

        // The brother node doesn't have enough elements to borrow, merge with the brother node.
        if (brother_index == next) {
            if (!is_leaf) {
                let len = vector::length(children);
                let i = 0;
                while (i < len) {
                    bucket_table::borrow_mut(&mut tree.nodes, vector::borrow(children, i).node_index).parent = brother_index;
                    i = i + 1;
                };
            };
            vector::append(children, brother_node.children);
            node.next = brother_node.next;
            let key_to_remove = vector::borrow(children, current_size - 1).max_key;

            move children;

            if (node.next != NULL_INDEX) {
                bucket_table::borrow_mut(&mut tree.nodes, node.next).prev = brother_index;
            };
            if (node.prev != NULL_INDEX) {
                bucket_table::borrow_mut(&mut tree.nodes, node.prev).next = brother_index;
            };

            bucket_table::add(&mut tree.nodes, brother_index, node);
            if (tree.min_leaf_index == node_index) {
                tree.min_leaf_index = brother_index;
            };

            if (parent != NULL_INDEX) {
                remove_at(tree, parent, key_to_remove);
            };
        } else {
            if (!is_leaf) {
                let len = vector::length(brother_children);
                let i = 0;
                while (i < len) {
                    bucket_table::borrow_mut(&mut tree.nodes, vector::borrow(brother_children, i).node_index).parent = node_index;
                    i = i + 1;
                };
            };
            vector::append(brother_children, node.children);
            brother_node.next = node.next;
            let key_to_remove = vector::borrow(brother_children, brother_size - 1).max_key;

            move brother_children;

            if (brother_node.next != NULL_INDEX) {
                bucket_table::borrow_mut(&mut tree.nodes, brother_node.next).prev = node_index;
            };
            if (brother_node.prev != NULL_INDEX) {
                bucket_table::borrow_mut(&mut tree.nodes, brother_node.prev).next = node_index;
            };

            bucket_table::add(&mut tree.nodes, node_index, brother_node);
            if (tree.min_leaf_index == brother_index) {
                tree.min_leaf_index = node_index;
            };

            if (parent != NULL_INDEX) {
                remove_at(tree, parent, key_to_remove);
            };
        }
    }

    ///////////
    // Tests //
    ///////////

    #[test_only]
    fun destroy<V: drop>(tree: SmartTree<V>) {
        let it = new_begin_iter(&tree);
        while (!is_end_iter(&tree, &it)) {
            remove(&mut tree, it.key);
            assert!(is_end_iter(&tree, &find(&tree, it.key)), E_INTERNAL);
            it = new_begin_iter(&tree);
            validate_tree(&tree);
        };

        destroy_empty(tree);
    }

    #[test_only]
    fun validate_iteration<V>(tree: &SmartTree<V>) {
        let expected_num_elements = bucket_table::length(&tree.entries);
        let num_elements = 0;
        let it = new_begin_iter(tree);
        while (!is_end_iter(tree, &it)) {
            num_elements = num_elements + 1;
            it = next_iter_or_die(tree, it);
        };
        assert!(num_elements == expected_num_elements, E_INTERNAL);

        let num_elements = 0;
        let it = new_end_iter(tree);
        while (!is_begin_iter(tree, &it)) {
            it = prev_iter_or_die(tree, it);
            num_elements = num_elements + 1;
        };
        assert!(num_elements == expected_num_elements, E_INTERNAL);

        let it = new_end_iter(tree);
        if (!is_begin_iter(tree, &it)) {
            it = prev_iter_or_die(tree, it);
            assert!(it.node_index == tree.max_leaf_index, E_INTERNAL);
        } else {
            assert!(expected_num_elements == 0, E_INTERNAL);
        };
    }

    #[test_only]
    fun validate_subtree<V>(tree: &SmartTree<V>, node_index: u64, expected_max_key: Option<u64>, expected_parent: u64) {
        let node = bucket_table::borrow(&tree.nodes, node_index);
        let len = vector::length(&node.children);
        assert!(len <= (tree.order as u64), E_INTERNAL);
        assert!(len * 2 >= (tree.order as u64) || node_index == tree.root_index, E_INTERNAL);

        assert!(node.parent == expected_parent, E_INTERNAL);

        let i = 1;
        while (i < len) {
            assert!(vector::borrow(&node.children, i).max_key > vector::borrow(&node.children, i - 1).max_key, E_INTERNAL);
            i = i + 1;
        };

        if (!node.is_leaf) {
            let i = 0;
            while (i < len) {
                let child = vector::borrow(&node.children, i);
                validate_subtree(tree, child.node_index, option::some(child.max_key), node_index);
                i = i + 1;
            };
        } else {
            let i = 0;
            while (i < len) {
                let child = vector::borrow(&node.children, i);
                assert!(child.node_index == NULL_INDEX, E_INTERNAL);
                i = i + 1;
            };
        };

        if (option::is_some(&expected_max_key)) {
            let expected_max_key = option::extract(&mut expected_max_key);
            assert!(expected_max_key == vector::borrow(&node.children, len - 1).max_key, E_INTERNAL);
        };
    }

    #[test_only]
    fun validate_tree<V>(tree: &SmartTree<V>) {
        validate_subtree(tree, tree.root_index, option::none(), NULL_INDEX);
        validate_iteration(tree);
    }

    #[test]
    fun test_smart_tree() {
        let tree = new_with_order(5);
        insert(&mut tree, 1, 1);
        insert(&mut tree, 2, 2);
        assert!(upsert(&mut tree, 3, 3) == option::none(), E_INTERNAL);
        insert(&mut tree, 4, 4);
        assert!(upsert(&mut tree, 4, 8) == option::some(4), E_INTERNAL);
        insert(&mut tree, 5, 5);
        insert(&mut tree, 6, 6);

        remove(&mut tree, 5);
        remove(&mut tree, 4);
        remove(&mut tree, 1);
        remove(&mut tree, 3);
        remove(&mut tree, 2);
        remove(&mut tree, 6);

        destroy_empty(tree);
    }

    #[test]
    fun test_iterator() {
        let tree = new_with_order(5);

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
        let tree = new_with_order(5);

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
        let tree = new_with_order(5);

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
    fun test_large_data_set_helper(order: u8) {
        let tree = new_with_order(order);
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
        test_large_data_set_helper(5);
    }

    #[test]
    fun test_large_data_set_order_6() {
        test_large_data_set_helper(6);
    }

    #[test]
    fun test_large_data_set_order_16() {
        test_large_data_set_helper(16);
    }

    #[test]
    fun test_large_data_set_order_31() {
        test_large_data_set_helper(31);
    }

    #[test]
    fun test_large_data_set_order_32() {
        test_large_data_set_helper(32);
    }
}
