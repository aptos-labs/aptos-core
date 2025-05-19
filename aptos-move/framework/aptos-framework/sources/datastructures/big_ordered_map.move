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
///
/// TODO: all iterator functions are public(friend) for now, so that they can be modified in a
/// backward incompatible way. Type is also named IteratorPtr, so that Iterator is free to use later.
/// They are waiting for Move improvement that will allow references to be part of the struct,
/// allowing cleaner iterator APIs.
module aptos_std::big_ordered_map {
    use std::error;
    use std::vector;
    use std::option::{Self as option, Option};
    use std::bcs;
    use aptos_std::ordered_map::{Self, OrderedMap};
    use aptos_std::cmp;
    use aptos_std::storage_slots_allocator::{Self, StorageSlotsAllocator, StoredSlot};
    use aptos_std::math64::{max, min};

    // Error constants shared with ordered_map (so try using same values)

    /// Map key already exists
    const EKEY_ALREADY_EXISTS: u64 = 1;
    /// Map key is not found
    const EKEY_NOT_FOUND: u64 = 2;
    /// Trying to do an operation on an IteratorPtr that would go out of bounds
    const EITER_OUT_OF_BOUNDS: u64 = 3;

    // Error constants specific to big_ordered_map

    /// The provided configuration parameter is invalid.
    const EINVALID_CONFIG_PARAMETER: u64 = 11;
    /// Map isn't empty
    const EMAP_NOT_EMPTY: u64 = 12;
    /// Trying to insert too large of an object into the map.
    const EARGUMENT_BYTES_TOO_LARGE: u64 = 13;
    /// borrow_mut requires that key and value types have constant size
    /// (otherwise it wouldn't be able to guarantee size requirements are not violated)
    /// Use remove() + add() combo instead.
    const EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE: u64 = 14;

    // Errors that should never be thrown

    /// Internal errors.
    const EINTERNAL_INVARIANT_BROKEN: u64 = 20;

    // Internal constants.

    const DEFAULT_TARGET_NODE_SIZE: u64 = 4096;
    const INNER_MIN_DEGREE: u16 = 4;
    // We rely on 1 being valid size only for root node,
    // so this cannot be below 3 (unless that is changed)
    const LEAF_MIN_DEGREE: u16 = 3;
    const MAX_DEGREE: u64 = 4096;

    const MAX_NODE_BYTES: u64 = 409600; // 400 KB, bellow the max resource limit.

    // Constants aligned with storage_slots_allocator
    const NULL_INDEX: u64 = 0;
    const ROOT_INDEX: u64 = 1;

    /// A node of the BigOrderedMap.
    ///
    /// Inner node will have all children be Child::Inner, pointing to the child nodes.
    /// Leaf node will have all children be Child::Leaf.
    /// Basically - Leaf node is a single-resource OrderedMap, containing as much key/value entries, as can fit.
    /// So Leaf node contains multiple values, not just one.
    enum Node<K: store, V: store> has store {
        V1 {
            // Whether this node is a leaf node.
            is_leaf: bool,
            // The children of the nodes.
            // When node is inner node, K represents max_key within the child subtree, and values are Child::Inner.
            // When the node is leaf node, K represents key of the leaf, and values are Child::Leaf.
            children: OrderedMap<K, Child<V>>,
            // The node index of its previous node at the same level, or `NULL_INDEX` if it doesn't have a previous node.
            prev: u64,
            // The node index of its next node at the same level, or `NULL_INDEX` if it doesn't have a next node.
            next: u64,
        }
    }

    /// Contents of a child node.
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
    ///
    /// TODO: Once fields can be (mutable) references, this class will be deprecated.
    enum IteratorPtr<K> has copy, drop {
        End,
        Some {
            /// The node index of the iterator pointing to.
            node_index: u64,

            /// Child iter it is pointing to
            child_iter: ordered_map::IteratorPtr,

            /// `key` to which `(node_index, child_iter)` are pointing to
            /// cache to not require borrowing global resources to fetch again
            key: K,
        },
    }

    /// The BigOrderedMap data structure.
    enum BigOrderedMap<K: store, V: store> has store {
        BPlusTreeMap {
            /// Root node. It is stored directly in the resource itself, unlike all other nodes.
            root: Node<K, V>,
            /// Storage of all non-root nodes. They are stored in separate storage slots.
            nodes: StorageSlotsAllocator<Node<K, V>>,
            /// The node index of the leftmost node.
            min_leaf_index: u64,
            /// The node index of the rightmost node.
            max_leaf_index: u64,

            /// Whether Key and Value have constant serialized size, and if so,
            /// optimize out size checks on every insert.
            constant_kv_size: bool,
            /// The max number of children an inner node can have.
            inner_max_degree: u16,
            /// The max number of children a leaf node can have.
            leaf_max_degree: u16,
        }
    }

    // ======================= Constructors && Destructors ====================

    /// Returns a new BigOrderedMap with the default configuration.
    /// Only allowed to be called with constant size types. For variable sized types,
    /// it is required to use new_with_config, to explicitly select automatic or specific degree selection.
    public fun new<K: store, V: store>(): BigOrderedMap<K, V> {
        // Use new_with_type_size_hints or new_with_config if your types have variable sizes.
        assert!(
            bcs::constant_serialized_size<K>().is_some() && bcs::constant_serialized_size<V>().is_some(),
            error::invalid_argument(EINVALID_CONFIG_PARAMETER)
        );

        new_with_config(0, 0, false)
    }


    /// Returns a new BigOrderedMap with with reusable storage slots.
    /// Only allowed to be called with constant size types. For variable sized types,
    /// it is required to use new_with_config, to explicitly select automatic or specific degree selection.
    public fun new_with_reusable<K: store, V: store>(): BigOrderedMap<K, V> {
        // Use new_with_type_size_hints or new_with_config if your types have variable sizes.
        assert!(
            bcs::constant_serialized_size<K>().is_some() && bcs::constant_serialized_size<V>().is_some(),
            error::invalid_argument(EINVALID_CONFIG_PARAMETER)
        );

        new_with_config(0, 0, true)
    }


    /// Returns a new BigOrderedMap, configured based on passed key and value serialized size hints.
    public fun new_with_type_size_hints<K: store, V: store>(avg_key_bytes: u64, max_key_bytes: u64, avg_value_bytes: u64, max_value_bytes: u64): BigOrderedMap<K, V> {
        assert!(avg_key_bytes <= max_key_bytes, error::invalid_argument(EINVALID_CONFIG_PARAMETER));
        assert!(avg_value_bytes <= max_value_bytes, error::invalid_argument(EINVALID_CONFIG_PARAMETER));

        let inner_max_degree_from_avg = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / avg_key_bytes), INNER_MIN_DEGREE as u64);
        let inner_max_degree_from_max = MAX_NODE_BYTES / max_key_bytes;
        assert!(inner_max_degree_from_max >= (INNER_MIN_DEGREE as u64), error::invalid_argument(EINVALID_CONFIG_PARAMETER));

        let avg_entry_size = avg_key_bytes + avg_value_bytes;
        let max_entry_size = max_key_bytes + max_value_bytes;

        let leaf_max_degree_from_avg = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / avg_entry_size), LEAF_MIN_DEGREE as u64);
        let leaf_max_degree_from_max = MAX_NODE_BYTES / max_entry_size;
        assert!(leaf_max_degree_from_max >= (INNER_MIN_DEGREE as u64), error::invalid_argument(EINVALID_CONFIG_PARAMETER));

        new_with_config(
            min(inner_max_degree_from_avg, inner_max_degree_from_max) as u16,
            min(leaf_max_degree_from_avg, leaf_max_degree_from_max) as u16,
            false,
        )
    }

    /// Returns a new BigOrderedMap with the provided max degree consts (the maximum # of children a node can have, both inner and leaf).
    /// If 0 is passed, then it is dynamically computed based on size of first key and value.
    ///
    /// Sizes of all elements must respect (or their additions will be rejected):
    ///   `key_size * inner_max_degree <= MAX_NODE_BYTES`
    ///   `entry_size * leaf_max_degree <= MAX_NODE_BYTES`
    /// If keys or values have variable size, and first element could be non-representative in size (i.e. smaller than future ones),
    /// it is important to compute and pass inner_max_degree and leaf_max_degree based on the largest element you want to be able to insert.
    ///
    /// `reuse_slots` means that removing elements from the map doesn't free the storage slots and returns the refund.
    /// Together with `allocate_spare_slots`, it allows to preallocate slots and have inserts have predictable gas costs.
    /// (otherwise, inserts that require map to add new nodes, cost significantly more, compared to the rest)
    public fun new_with_config<K: store, V: store>(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool): BigOrderedMap<K, V> {
        assert!(inner_max_degree == 0 || (inner_max_degree >= INNER_MIN_DEGREE && (inner_max_degree as u64) <= MAX_DEGREE), error::invalid_argument(EINVALID_CONFIG_PARAMETER));
        assert!(leaf_max_degree == 0 || (leaf_max_degree >= LEAF_MIN_DEGREE && (leaf_max_degree as u64) <= MAX_DEGREE), error::invalid_argument(EINVALID_CONFIG_PARAMETER));

        // Assert that storage_slots_allocator special indices are aligned:
        assert!(storage_slots_allocator::is_null_index(NULL_INDEX), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        assert!(storage_slots_allocator::is_special_unused_index(ROOT_INDEX), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let nodes = storage_slots_allocator::new(reuse_slots);

        let self = BigOrderedMap::BPlusTreeMap {
            root: new_node(/*is_leaf=*/true),
            nodes: nodes,
            min_leaf_index: ROOT_INDEX,
            max_leaf_index: ROOT_INDEX,
            constant_kv_size: false, // Will be initialized in validate_static_size_and_init_max_degrees below.
            inner_max_degree: inner_max_degree,
            leaf_max_degree: leaf_max_degree
        };
        self.validate_static_size_and_init_max_degrees();
        self
    }

    /// Create a BigOrderedMap from a vector of keys and values, with default configuration.
    /// Aborts with EKEY_ALREADY_EXISTS if duplicate keys are passed in.
    public fun new_from<K: drop + copy + store, V: store>(keys: vector<K>, values: vector<V>): BigOrderedMap<K, V> {
        let map = new();
        map.add_all(keys, values);
        map
    }

    /// Destroys the map if it's empty, otherwise aborts.
    public fun destroy_empty<K: store, V: store>(self: BigOrderedMap<K, V>) {
        let BigOrderedMap::BPlusTreeMap { root, nodes, min_leaf_index: _, max_leaf_index: _, constant_kv_size: _, inner_max_degree: _, leaf_max_degree: _ } = self;
        root.destroy_empty_node();
        // If root node is empty, then we know that no storage slots are used,
        // and so we can safely destroy all nodes.
        nodes.destroy_empty();
    }

    /// Map was created with reuse_slots=true, you can allocate spare slots, to pay storage fee now, to
    /// allow future insertions to not require any storage slot creation - making their gas more predictable
    /// and better bounded/fair.
    /// (otherwsie, unlucky inserts create new storage slots and are charge more for it)
    public fun allocate_spare_slots<K: store, V: store>(self: &mut BigOrderedMap<K, V>, num_to_allocate: u64) {
        self.nodes.allocate_spare_slots(num_to_allocate)
    }

    /// Returns true iff the BigOrderedMap is empty.
    public fun is_empty<K: store, V: store>(self: &BigOrderedMap<K, V>): bool {
        let node = self.borrow_node(self.min_leaf_index);
        node.children.is_empty()
    }

    /// Returns the number of elements in the BigOrderedMap.
    /// This is an expensive function, as it goes through all the leaves to compute it.
    public fun compute_length<K: store, V: store>(self: &BigOrderedMap<K, V>): u64 {
        let size = 0;
        self.for_each_leaf_node_ref(|node| {
            size += node.children.length();
        });
        size
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
        // Optimize case where only root node exists
        // (optimizes out borrowing and path creation in `find_leaf_path`)
        if (self.root.is_leaf) {
            let Child::Leaf {
                value,
            } = self.root.children.remove(key);
            return value;
        };

        let path_to_leaf = self.find_leaf_path(key);

        assert!(!path_to_leaf.is_empty(), error::invalid_argument(EKEY_NOT_FOUND));

        let Child::Leaf {
            value,
        } = self.remove_at(path_to_leaf, key);
        value
    }

    /// Add multiple key/value pairs to the map. The keys must not already exist.
    /// Aborts with EKEY_ALREADY_EXISTS if key already exist, or duplicate keys are passed in.
    public fun add_all<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, keys: vector<K>, values: vector<V>) {
        // TODO: Can be optimized, both in insertion order (largest first, then from smallest),
        // as well as on initializing inner_max_degree/leaf_max_degree better
        keys.zip(values, |key, value| {
            self.add(key, value);
        });
    }

    public fun pop_front<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>): (K, V) {
        let it = self.new_begin_iter();
        let k = *it.iter_borrow_key();
        let v = self.remove(&k);
        (k, v)
    }

    public fun pop_back<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>): (K, V) {
        let it = self.new_end_iter().iter_prev(self);
        let k = *it.iter_borrow_key();
        let v = self.remove(&k);
        (k, v)
    }

    // ============================= Accessors ================================

    /// Returns an iterator pointing to the first element that is greater or equal to the provided
    /// key, or an end iterator if such element doesn't exist.
    public(friend) fun lower_bound<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): IteratorPtr<K> {
        let leaf = self.find_leaf(key);
        if (leaf == NULL_INDEX) {
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
    public(friend) fun find<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): IteratorPtr<K> {
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
    public fun borrow<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): &V {
        let iter = self.find(key);
        assert!(!iter.iter_is_end(self), error::invalid_argument(EKEY_NOT_FOUND));

        iter.iter_borrow(self)
    }

    public fun get<K: drop + copy + store, V: copy + store>(self: &BigOrderedMap<K, V>, key: &K): Option<V> {
        let iter = self.find(key);
        if (iter.iter_is_end(self)) {
            option::none()
        } else {
            option::some(*iter.iter_borrow(self))
        }
    }

    /// Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.
    /// Aborts with EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE if KV size doesn't have constant size,
    /// because if it doesn't we cannot assert invariants on the size.
    /// In case of variable size, use either `borrow`, `copy` then `upsert`, or `remove` and `add` instead of mutable borrow.
    public fun borrow_mut<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: &K): &mut V {
        let iter = self.find(key);
        assert!(!iter.iter_is_end(self), error::invalid_argument(EKEY_NOT_FOUND));
        iter.iter_borrow_mut(self)
    }
    public fun borrow_front<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>): (K, &V) {
        let it = self.new_begin_iter();
        let key = *it.iter_borrow_key();
        (key, it.iter_borrow(self))
    }

    public fun borrow_back<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>): (K, &V) {
        let it = self.new_end_iter().iter_prev(self);
        let key = *it.iter_borrow_key();
        (key, it.iter_borrow(self))
    }

    public fun prev_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Option<K> {
        let it = self.lower_bound(key);
        if (it.iter_is_begin(self)) {
            option::none()
        } else {
            option::some(*it.iter_prev(self).iter_borrow_key())
        }
    }

    public fun next_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Option<K> {
        let it = self.lower_bound(key);
        if (it.iter_is_end(self)) {
            option::none()
        } else {
            let cur_key = it.iter_borrow_key();
            if (key == cur_key) {
                let it = it.iter_next(self);
                if (it.iter_is_end(self)) {
                    option::none()
                } else {
                    option::some(*it.iter_borrow_key())
                }
            } else {
                option::some(*cur_key)
            }
        }
    }

    // =========================== Views and Traversals ==============================

    /// Convert a BigOrderedMap to an OrderedMap, which is supposed to be called mostly by view functions to get an atomic
    /// view of the whole map.
    /// Disclaimer: This function may be costly as the BigOrderedMap may be huge in size. Use it at your own discretion.
    public fun to_ordered_map<K: drop + copy + store, V: copy + store>(self: &BigOrderedMap<K, V>): OrderedMap<K, V> {
        let result = ordered_map::new();
        self.for_each_ref_friend(|k, v| {
            result.new_end_iter().iter_add(&mut result, *k, *v);
        });
        result
    }

    /// Get all keys.
    ///
    /// For a large enough BigOrderedMap this function will fail due to execution gas limits,
    /// use iterartor or next_key/prev_key to iterate over across portion of the map.
    public fun keys<K: store + copy + drop, V: store + copy>(self: &BigOrderedMap<K, V>): vector<K> {
        let result = vector[];
        self.for_each_ref_friend(|k, _v| {
            result.push_back(*k);
        });
        result
    }

    /// Apply the function to each element in the vector, consuming it, leaving the map empty.
    ///
    /// Current implementation is O(n * log(n)). After function values will be optimized
    /// to O(n).
    public inline fun for_each_and_clear<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, f: |K, V|) {
        // TODO - this can be done more efficiently, by destroying the leaves directly
        // but that requires more complicated code and testing.
        while (!self.is_empty()) {
            let (k, v) = self.pop_front();
            f(k, v);
        };
    }

    /// Apply the function to each element in the vector, consuming it, and consuming the map
    ///
    /// Current implementation is O(n * log(n)). After function values will be optimized
    /// to O(n).
    public inline fun for_each<K: drop + copy + store, V: store>(self: BigOrderedMap<K, V>, f: |K, V|) {
        // TODO - this can be done more efficiently, by destroying the leaves directly
        // but that requires more complicated code and testing.
        self.for_each_and_clear(|k, v| f(k, v));
        self.destroy_empty()
    }

    /// Apply the function to a reference of each element in the vector.
    ///
    /// Current implementation is O(n * log(n)). After function values will be optimized
    /// to O(n).
    public inline fun for_each_ref<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, f: |&K, &V|) {
        // This implementation is innefficient: O(log(n)) for next_key / borrow lookups every time,
        // but is the only one available through the public API.
        if (!self.is_empty()) {
            let (k, v) = self.borrow_front();
            f(&k, v);

            let cur_k = self.next_key(&k);
            while (cur_k.is_some()) {
                let k = cur_k.destroy_some();
                f(&k, self.borrow(&k));

                cur_k = self.next_key(&k);
            };
        };

        // TODO use this more efficient implementation when function values are enabled.
        // self.for_each_leaf_node_ref(|node| {
        //     node.children.for_each_ref(|k: &K, v: &Child<V>| {
        //         f(k, &v.value);
        //     });
        // })
    }

    // TODO: Temporary friend implementaiton, until for_each_ref can be made efficient.
    public(friend) inline fun for_each_ref_friend<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, f: |&K, &V|) {
        self.for_each_leaf_node_ref(|node| {
            node.children.for_each_ref_friend(|k: &K, v: &Child<V>| {
                f(k, &v.value);
            });
        })
    }

    /// Apply the function to a mutable reference of each key-value pair in the map.
    ///
    /// Current implementation is O(n * log(n)). After function values will be optimized
    /// to O(n).
    public inline fun for_each_mut<K: copy + drop + store, V: store>(self: &mut BigOrderedMap<K, V>, f: |&K, &mut V|) {
        // This implementation is innefficient: O(log(n)) for next_key / borrow lookups every time,
        // but is the only one available through the public API.
        if (!self.is_empty()) {
            let (k, _v) = self.borrow_front();

            let done = false;
            while (!done) {
                f(&k, self.borrow_mut(&k));

                let cur_k = self.next_key(&k);
                if (cur_k.is_some()) {
                    k = cur_k.destroy_some();
                } else {
                    done = true;
                }
            };
        };

        // TODO: if we make iterator api public update to:
        // let iter = self.new_begin_iter();
        // while (!iter.iter_is_end(self)) {
        //     let key = *iter.iter_borrow_key(self);
        //     f(key, iter.iter_borrow_mut(self));
        //     iter = iter.iter_next(self);
        // }
    }

    /// Destroy a map, by destroying elements individually.
    ///
    /// Current implementation is O(n * log(n)). After function values will be optimized
    /// to O(n).
    public inline fun destroy<K: drop + copy + store, V: store>(self: BigOrderedMap<K, V>, dv: |V|) {
        self.for_each(|_k, v| {
            dv(v);
        });
    }

    // ========================= IteratorPtr functions ===========================

    /// Returns the begin iterator.
    public(friend) fun new_begin_iter<K: copy + store, V: store>(self: &BigOrderedMap<K, V>): IteratorPtr<K> {
        if (self.is_empty()) {
            return IteratorPtr::End;
        };

        let node = self.borrow_node(self.min_leaf_index);
        assert!(!node.children.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        let begin_child_iter = node.children.new_begin_iter();
        let begin_child_key = *begin_child_iter.iter_borrow_key(&node.children);
        new_iter(self.min_leaf_index, begin_child_iter, begin_child_key)
    }

    /// Returns the end iterator.
    public(friend) fun new_end_iter<K: copy + store, V: store>(self: &BigOrderedMap<K, V>): IteratorPtr<K> {
        IteratorPtr::End
    }

    // Returns true iff the iterator is a begin iterator.
    public(friend) fun iter_is_begin<K: store, V: store>(self: &IteratorPtr<K>, map: &BigOrderedMap<K, V>): bool {
        if (self is IteratorPtr::End<K>) {
            map.is_empty()
        } else {
            (self.node_index == map.min_leaf_index && self.child_iter.iter_is_begin_from_non_empty())
        }
    }

    // Returns true iff the iterator is an end iterator.
    public(friend) fun iter_is_end<K: store, V: store>(self: &IteratorPtr<K>, _map: &BigOrderedMap<K, V>): bool {
        self is IteratorPtr::End<K>
    }

    /// Borrows the key given iterator points to.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_borrow_key<K>(self: &IteratorPtr<K>): &K {
        assert!(!(self is IteratorPtr::End<K>), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        &self.key
    }

    /// Borrows the value given iterator points to.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_borrow<K: drop + store, V: store>(self: IteratorPtr<K>, map: &BigOrderedMap<K, V>): &V {
        assert!(!self.iter_is_end(map), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        let IteratorPtr::Some { node_index, child_iter, key: _ } = self;
        let children = &map.borrow_node(node_index).children;
        &child_iter.iter_borrow(children).value
    }

    /// Mutably borrows the value iterator points to.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Aborts with EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE if KV size doesn't have constant size,
    /// because if it doesn't we cannot assert invariants on the size.
    /// In case of variable size, use either `borrow`, `copy` then `upsert`, or `remove` and `add` instead of mutable borrow.
    ///
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_borrow_mut<K: drop + store, V: store>(self: IteratorPtr<K>, map: &mut BigOrderedMap<K, V>): &mut V {
        assert!(map.constant_kv_size, error::invalid_argument(EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE));
        assert!(!self.iter_is_end(map), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        let IteratorPtr::Some { node_index, child_iter, key: _ } = self;
        let children = &mut map.borrow_node_mut(node_index).children;
        &mut child_iter.iter_borrow_mut(children).value
    }

    /// Returns the next iterator.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Requires the map is not changed after the input iterator is generated.
    public(friend) fun iter_next<K: drop + copy + store, V: store>(self: IteratorPtr<K>, map: &BigOrderedMap<K, V>): IteratorPtr<K> {
        assert!(!(self is IteratorPtr::End<K>), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let node_index = self.node_index;
        let node = map.borrow_node(node_index);

        let child_iter = self.child_iter.iter_next(&node.children);
        if (!child_iter.iter_is_end(&node.children)) {
            // next is in the same leaf node
            let iter_key = *child_iter.iter_borrow_key(&node.children);
            return new_iter(node_index, child_iter, iter_key);
        };

        // next is in a different leaf node
        let next_index = node.next;
        if (next_index != NULL_INDEX) {
            let next_node = map.borrow_node(next_index);

            let child_iter = next_node.children.new_begin_iter();
            assert!(!child_iter.iter_is_end(&next_node.children), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            let iter_key = *child_iter.iter_borrow_key(&next_node.children);
            return new_iter(next_index, child_iter, iter_key);
        };

        map.new_end_iter()
    }

    /// Returns the previous iterator.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the beginning.
    /// Requires the map is not changed after the input iterator is generated.
    public(friend) fun iter_prev<K: drop + copy + store, V: store>(self: IteratorPtr<K>, map: &BigOrderedMap<K, V>): IteratorPtr<K> {
        let prev_index = if (self is IteratorPtr::End<K>) {
            map.max_leaf_index
        } else {
            let node_index = self.node_index;
            let node = map.borrow_node(node_index);

            if (!self.child_iter.iter_is_begin(&node.children)) {
                // next is in the same leaf node
                let child_iter = self.child_iter.iter_prev(&node.children);
                let key = *child_iter.iter_borrow_key(&node.children);
                return new_iter(node_index, child_iter, key);
            };
            node.prev
        };

        assert!(prev_index != NULL_INDEX, error::invalid_argument(EITER_OUT_OF_BOUNDS));

        // next is in a different leaf node
        let prev_node = map.borrow_node(prev_index);

        let prev_children = &prev_node.children;
        let child_iter = prev_children.new_end_iter().iter_prev(prev_children);
        let iter_key = *child_iter.iter_borrow_key(prev_children);
        new_iter(prev_index, child_iter, iter_key)
    }

    // ====================== Internal Implementations ========================

    inline fun for_each_leaf_node_ref<K: store, V: store>(self: &BigOrderedMap<K, V>, f: |&Node<K, V>|) {
        let cur_node_index = self.min_leaf_index;

        while (cur_node_index != NULL_INDEX) {
            let node = self.borrow_node(cur_node_index);
            f(node);
            cur_node_index = node.next;
        }
    }

    /// Borrow a node, given an index. Works for both root (i.e. inline) node and separately stored nodes
    inline fun borrow_node<K: store, V: store>(self: &BigOrderedMap<K, V>, node_index: u64): &Node<K, V> {
        if (node_index == ROOT_INDEX) {
            &self.root
        } else {
            self.nodes.borrow(node_index)
        }
    }

    /// Borrow a node mutably, given an index. Works for both root (i.e. inline) node and separately stored nodes
    inline fun borrow_node_mut<K: store, V: store>(self: &mut BigOrderedMap<K, V>, node_index: u64): &mut Node<K, V> {
        if (node_index == ROOT_INDEX) {
            &mut self.root
        } else {
            self.nodes.borrow_mut(node_index)
        }
    }

    fun add_or_upsert_impl<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, key: K, value: V, allow_overwrite: bool): Option<Child<V>> {
        if (!self.constant_kv_size) {
            self.validate_dynamic_size_and_init_max_degrees(&key, &value);
        };

        // Optimize case where only root node exists
        // (optimizes out borrowing and path creation in `find_leaf_path`)
        if (self.root.is_leaf) {
            let children = &mut self.root.children;
            let degree = children.length();

            if (degree < (self.leaf_max_degree as u64)) {
                let result = children.upsert(key, new_leaf_child(value));
                assert!(allow_overwrite || result.is_none(), error::invalid_argument(EKEY_ALREADY_EXISTS));
                return result;
            };
        };

        let path_to_leaf = self.find_leaf_path(&key);

        if (path_to_leaf.is_empty()) {
            // In this case, the key is greater than all keys in the map.
            // So we need to update `key` in the pointers to the last (rightmost) child
            // on every level, to maintain the invariant of `add_at`
            // we also create a path_to_leaf to the rightmost leaf.
            let current = ROOT_INDEX;

            loop {
                path_to_leaf.push_back(current);

                let current_node = self.borrow_node_mut(current);
                if (current_node.is_leaf) {
                    break;
                };
                let last_value = current_node.children.new_end_iter().iter_prev(&current_node.children).iter_remove(&mut current_node.children);
                current = last_value.node_index.stored_to_index();
                current_node.children.add(key, last_value);
            };
        };

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
            self.inner_max_degree = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / key_size), INNER_MIN_DEGREE as u64) as u16;
        };

        if (self.leaf_max_degree == 0) {
            self.leaf_max_degree = max(min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / entry_size), LEAF_MIN_DEGREE as u64) as u16;
        };

        // Make sure that no nodes can exceed the upper size limit.
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
        let Node::V1 { children, is_leaf: _, prev: _, next: _ } = self;
        assert!(children.is_empty(), error::invalid_argument(EMAP_NOT_EMPTY));
        children.destroy_empty();
    }

    fun new_node<K: store, V: store>(is_leaf: bool): Node<K, V> {
        Node::V1 {
            is_leaf: is_leaf,
            children: ordered_map::new(),
            prev: NULL_INDEX,
            next: NULL_INDEX,
        }
    }

    fun new_node_with_children<K: store, V: store>(is_leaf: bool, children: OrderedMap<K, Child<V>>): Node<K, V> {
        Node::V1 {
            is_leaf: is_leaf,
            children: children,
            prev: NULL_INDEX,
            next: NULL_INDEX,
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

    fun new_iter<K>(node_index: u64, child_iter: ordered_map::IteratorPtr, key: K): IteratorPtr<K> {
        IteratorPtr::Some {
            node_index: node_index,
            child_iter: child_iter,
            key: key,
        }
    }

    /// Find leaf where the given key would fall in.
    /// So the largest leaf with its `max_key <= key`.
    /// return NULL_INDEX if `key` is larger than any key currently stored in the map.
    fun find_leaf<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): u64 {
        let current = ROOT_INDEX;
        loop {
            let node = self.borrow_node(current);
            if (node.is_leaf) {
                return current;
            };
            let children = &node.children;
            let child_iter = children.lower_bound(key);
            if (child_iter.iter_is_end(children)) {
                return NULL_INDEX;
            } else {
                current = child_iter.iter_borrow(children).node_index.stored_to_index();
            };
        }
    }

    /// Find leaf where the given key would fall in.
    /// So the largest leaf with it's `max_key <= key`.
    /// Returns the path from root to that leaf (including the leaf itself)
    /// Returns empty path if `key` is larger than any key currently stored in the map.
    fun find_leaf_path<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): vector<u64> {
        let vec = vector::empty();

        let current = ROOT_INDEX;
        loop {
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
                current = child_iter.iter_borrow(children).node_index.stored_to_index();
            };
        }
    }

    fun get_max_degree<K: store, V: store>(self: &BigOrderedMap<K, V>, leaf: bool): u64 {
        if (leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        }
    }

    fun replace_root<K: store, V: store>(self: &mut BigOrderedMap<K, V>, new_root: Node<K, V>): Node<K, V> {
        // TODO: once mem::replace is made public/released, update to:
        // mem::replace(&mut self.root, new_root_node)

        let root = &mut self.root;
        let tmp_is_leaf = root.is_leaf;
        root.is_leaf = new_root.is_leaf;
        new_root.is_leaf = tmp_is_leaf;

        assert!(root.prev == NULL_INDEX, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        assert!(root.next == NULL_INDEX, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        assert!(new_root.prev == NULL_INDEX, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        assert!(new_root.next == NULL_INDEX, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        // let tmp_prev = root.prev;
        // root.prev = new_root.prev;
        // new_root.prev = tmp_prev;

        // let tmp_next = root.next;
        // root.next = new_root.next;
        // new_root.next = tmp_next;

        let tmp_children = root.children.trim(0);
        root.children.append_disjoint(new_root.children.trim(0));
        new_root.children.append_disjoint(tmp_children);

        new_root
    }

    /// Add a given child to a given node (last in the `path_to_node`), and update/rebalance the tree as necessary.
    /// It is required that `key` pointers to the child node, on the `path_to_node` are greater or equal to the given key.
    /// That means if we are adding a `key` larger than any currently existing in the map - we needed
    /// to update `key` pointers on the `path_to_node` to include it, before calling this method.
    ///
    /// Returns Child previously associated with the given key.
    /// If `allow_overwrite` is not set, function will abort if `key` is already present.
    fun add_at<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, path_to_node: vector<u64>, key: K, child: Child<V>, allow_overwrite: bool): Option<Child<V>> {
        // Last node in the path is one where we need to add the child to.
        let node_index = path_to_node.pop_back();
        {
            // First check if we can perform this operation, without changing structure of the tree (i.e. without adding any nodes).

            // For that we can just borrow the single node
            let node = self.borrow_node_mut(node_index);
            let children = &mut node.children;
            let degree = children.length();

            // Compute directly, as we cannot use get_max_degree(), as self is already mutably borrowed.
            let max_degree = if (node.is_leaf) {
                self.leaf_max_degree as u64
            } else {
                self.inner_max_degree as u64
            };

            if (degree < max_degree) {
                // Adding a child to a current node doesn't exceed the size, so we can just do that.
                let old_child = children.upsert(key, child);

                if (node.is_leaf) {
                    assert!(allow_overwrite || old_child.is_none(), error::invalid_argument(EKEY_ALREADY_EXISTS));
                    return old_child;
                } else {
                    assert!(!allow_overwrite && old_child.is_none(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
                    return old_child;
                };
            };

            // If we cannot add more nodes without exceeding the size,
            // but node with `key` already exists, we either need to replace or abort.
            let iter = children.find(&key);
            if (!iter.iter_is_end(children)) {
                assert!(node.is_leaf, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
                assert!(allow_overwrite, error::invalid_argument(EKEY_ALREADY_EXISTS));

                return option::some(iter.iter_replace(children, child));
            }
        };

        // # of children in the current node exceeds the threshold, need to split into two nodes.

        // If we are at the root, we need to move root node to become a child and have a new root node,
        // in order to be able to split the node on the level it is.
        let (reserved_slot, node) = if (node_index == ROOT_INDEX) {
            assert!(path_to_node.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

            // Splitting root now, need to create a new root.
            // Since root is stored direclty in the resource, we will swap-in the new node there.
            let new_root_node = new_node<K, V>(/*is_leaf=*/false);

            // Reserve a slot where the current root will be moved to.
            let (replacement_node_slot, replacement_node_reserved_slot) = self.nodes.reserve_slot();

            let max_key = {
                let root_children = &self.root.children;
                let max_key = *root_children.new_end_iter().iter_prev(root_children).iter_borrow_key(root_children);
                // need to check if key is largest, as invariant is that "parent's pointers" have been updated,
                // but key itself can be larger than all previous ones.
                if (cmp::compare(&max_key, &key).is_lt()) {
                    max_key = key;
                };
                max_key
            };
            // New root will have start with a single child - the existing root (which will be at replacement location).
            new_root_node.children.add(max_key, new_inner_child(replacement_node_slot));
            let node = self.replace_root(new_root_node);

            // we moved the currently processing node one level down, so we need to update the path
            path_to_node.push_back(ROOT_INDEX);

            let replacement_index = replacement_node_reserved_slot.reserved_to_index();
            if (node.is_leaf) {
                // replacement node is the only leaf, so we update the pointers:
                self.min_leaf_index = replacement_index;
                self.max_leaf_index = replacement_index;
            };
            (replacement_node_reserved_slot, node)
        } else {
            // In order to work on multiple nodes at the same time, we cannot borrow_mut, and need to be
            // remove_and_reserve existing node.
            let (cur_node_reserved_slot, node) = self.nodes.remove_and_reserve(node_index);
            (cur_node_reserved_slot, node)
        };

        // move node_index out of scope, to make sure we don't accidentally access it, as we are done with it.
        // (i.e. we should be using `reserved_slot` instead).
        move node_index;

        // Now we can perform the split at the current level, as we know we are not at the root level.
        assert!(!path_to_node.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        // Parent has a reference under max key to the current node, so existing index
        // needs to be the right node.
        // Since ordered_map::trim moves from the end (i.e. smaller keys stay),
        // we are going to put the contents of the current node on the left side,
        // and create a new right node.
        // So if we had before (node_index, node), we will change that to end up having:
        // (new_left_node_index, node trimmed off) and (node_index, new node with trimmed off children)
        //
        // So let's rename variables cleanly:
        let right_node_reserved_slot = reserved_slot;
        let left_node = node;

        let is_leaf = left_node.is_leaf;
        let left_children = &mut left_node.children;

        let right_node_index = right_node_reserved_slot.reserved_to_index();
        let left_next = &mut left_node.next;
        let left_prev = &mut left_node.prev;

        // Compute directly, as we cannot use get_max_degree(), as self is already mutably borrowed.
        let max_degree = if (is_leaf) {
            self.leaf_max_degree as u64
        } else {
            self.inner_max_degree as u64
        };
        // compute the target size for the left node:
        let target_size = (max_degree + 1) / 2;

        // Add child (which will exceed the size), and then trim off to create two sets of children of correct sizes.
        left_children.add(key, child);
        let right_node_children = left_children.trim(target_size);

        assert!(left_children.length() <= max_degree, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        assert!(right_node_children.length() <= max_degree, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let right_node = new_node_with_children(is_leaf, right_node_children);

        let (left_node_slot, left_node_reserved_slot) = self.nodes.reserve_slot();
        let left_node_index = left_node_slot.stored_to_index();

        // right nodes next is the node that was next of the left (previous) node, and next of left node is the right node.
        right_node.next = *left_next;
        *left_next = right_node_index;

        // right node's prev becomes current left node
        right_node.prev = left_node_index;
        // Since the previously used index is going to the right node, `prev` pointer of the next node is correct,
        // and we need to update next pointer of the previous node (if exists)
        if (*left_prev != NULL_INDEX) {
            self.nodes.borrow_mut(*left_prev).next = left_node_index;
            assert!(right_node_index != self.min_leaf_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        } else if (right_node_index == self.min_leaf_index) {
            // Otherwise, if we were the smallest node on the level. if this is the leaf level, update the pointer.
            assert!(is_leaf, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            self.min_leaf_index = left_node_index;
        };

        // Largest left key is the split key.
        let max_left_key = *left_children.new_end_iter().iter_prev(left_children).iter_borrow_key(left_children);

        self.nodes.fill_reserved_slot(left_node_reserved_slot, left_node);
        self.nodes.fill_reserved_slot(right_node_reserved_slot, right_node);

        // Add new Child (i.e. pointer to the left node) in the parent.
        self.add_at(path_to_node, max_left_key, new_inner_child(left_node_slot), false).destroy_none();
        option::none()
    }

    /// Given a path to node (excluding the node itself), which is currently stored under "old_key", update "old_key" to "new_key".
    fun update_key<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, path_to_node: vector<u64>, old_key: &K, new_key: K) {
        while (!path_to_node.is_empty()) {
            let node_index = path_to_node.pop_back();
            let node = self.borrow_node_mut(node_index);
            let children = &mut node.children;
            children.replace_key_inplace(old_key, new_key);

            // If we were not updating the largest child, we don't need to continue.
            if (children.new_end_iter().iter_prev(children).iter_borrow_key(children) != &new_key) {
                return
            };
        }
    }

    fun remove_at<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>, path_to_node: vector<u64>, key: &K): Child<V> {
        // Last node in the path is one where we need to remove the child from.
        let node_index = path_to_node.pop_back();
        let old_child = {
            // First check if we can perform this operation, without changing structure of the tree (i.e. without rebalancing any nodes).

            // For that we can just borrow the single node
            let node = self.borrow_node_mut(node_index);

            let children = &mut node.children;
            let is_leaf = node.is_leaf;

            let old_child = children.remove(key);
            if (node_index == ROOT_INDEX) {
                // If current node is root, lower limit of max_degree/2 nodes doesn't apply.
                // So we can adjust internally

                assert!(path_to_node.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

                if (!is_leaf && children.length() == 1) {
                    // If root is not leaf, but has a single child, promote only child to root,
                    // and drop current root. Since root is stored directly in the resource, we
                    // "move" the child into the root.

                    let Child::Inner {
                        node_index: inner_child_index,
                    } = children.new_end_iter().iter_prev(children).iter_remove(children);

                    let inner_child = self.nodes.remove(inner_child_index);
                    if (inner_child.is_leaf) {
                        self.min_leaf_index = ROOT_INDEX;
                        self.max_leaf_index = ROOT_INDEX;
                    };

                    self.replace_root(inner_child).destroy_empty_node();
                };
                return old_child;
            };

            // Compute directly, as we cannot use get_max_degree(), as self is already mutably borrowed.
            let max_degree = if (is_leaf) {
                self.leaf_max_degree as u64
            } else {
                self.inner_max_degree as u64
            };
            let degree = children.length();

            // See if the node is big enough, or we need to merge it with another node on this level.
            let big_enough = degree * 2 >= max_degree;

            let new_max_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);

            // See if max key was updated for the current node, and if so - update it on the path.
            let max_key_updated = cmp::compare(&new_max_key, key).is_lt();
            if (max_key_updated) {
                assert!(degree >= 1, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

                self.update_key(path_to_node, key, new_max_key);
            };

            // If node is big enough after removal, we are done.
            if (big_enough) {
                return old_child;
            };

            old_child
        };

        // Children size is below threshold, we need to rebalance with a neighbor on the same level.

        // In order to work on multiple nodes at the same time, we cannot borrow_mut, and need to be
        // remove_and_reserve existing node.
        let (node_slot, node) = self.nodes.remove_and_reserve(node_index);

        let is_leaf = node.is_leaf;
        let max_degree = self.get_max_degree(is_leaf);
        let prev = node.prev;
        let next = node.next;

        // index of the node we will rebalance with.
        let sibling_index = {
            let parent_children = &self.borrow_node(*path_to_node.borrow(path_to_node.length() - 1)).children;
            assert!(parent_children.length() >= 2, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            // If we are the largest node from the parent, we merge with the `prev`
            // (which is then guaranteed to have the same parent, as any node has >1 children),
            // otherwise we merge with `next`.
            if (parent_children.new_end_iter().iter_prev(parent_children).iter_borrow(parent_children).node_index.stored_to_index() == node_index) {
                prev
            } else {
                next
            }
        };

        let children = &mut node.children;

        let (sibling_slot, sibling_node) = self.nodes.remove_and_reserve(sibling_index);
        assert!(is_leaf == sibling_node.is_leaf, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        let sibling_children = &mut sibling_node.children;

        if ((sibling_children.length() - 1) * 2 >= max_degree) {
            // The sibling node has enough elements, we can just borrow an element from the sibling node.
            if (sibling_index == next) {
                // if sibling is the node with larger keys, we remove a child from the start
                let old_max_key = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
                let sibling_begin_iter = sibling_children.new_begin_iter();
                let borrowed_max_key = *sibling_begin_iter.iter_borrow_key(sibling_children);
                let borrowed_element = sibling_begin_iter.iter_remove(sibling_children);

                children.new_end_iter().iter_add(children, borrowed_max_key, borrowed_element);

                // max_key of the current node changed, so update
                self.update_key(path_to_node, &old_max_key, borrowed_max_key);
            } else {
                // if sibling is the node with smaller keys, we remove a child from the end
                let sibling_end_iter = sibling_children.new_end_iter().iter_prev(sibling_children);
                let borrowed_max_key = *sibling_end_iter.iter_borrow_key(sibling_children);
                let borrowed_element = sibling_end_iter.iter_remove(sibling_children);

                children.add(borrowed_max_key, borrowed_element);

                // max_key of the sibling node changed, so update
                self.update_key(path_to_node, &borrowed_max_key, *sibling_children.new_end_iter().iter_prev(sibling_children).iter_borrow_key(sibling_children));
            };

            self.nodes.fill_reserved_slot(node_slot, node);
            self.nodes.fill_reserved_slot(sibling_slot, sibling_node);
            return old_child;
        };

        // The sibling node doesn't have enough elements to borrow, merge with the sibling node.
        // Keep the slot of the node with larger keys of the two, to not require updating key on the parent nodes.
        // But append to the node with smaller keys, as ordered_map::append is more efficient when adding to the end.
        let (key_to_remove, reserved_slot_to_remove) = if (sibling_index == next) {
            // destroying larger sibling node, keeping sibling_slot.
            let Node::V1 { children: sibling_children, is_leaf: _, prev: _, next: sibling_next } = sibling_node;
            let key_to_remove = *children.new_end_iter().iter_prev(children).iter_borrow_key(children);
            children.append_disjoint(sibling_children);
            node.next = sibling_next;

            if (node.next != NULL_INDEX) {
                assert!(self.nodes.borrow_mut(node.next).prev == sibling_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            };

            // we are removing node_index, which previous's node's next was pointing to,
            // so update the pointer
            if (node.prev != NULL_INDEX) {
                self.nodes.borrow_mut(node.prev).next = sibling_index;
            };
            // Otherwise, we were the smallest node on the level. if this is the leaf level, update the pointer.
            if (self.min_leaf_index == node_index) {
                assert!(is_leaf, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
                self.min_leaf_index = sibling_index;
            };

            self.nodes.fill_reserved_slot(sibling_slot, node);

            (key_to_remove, node_slot)
        } else {
            // destroying larger current node, keeping node_slot
            let Node::V1 { children: node_children, is_leaf: _, prev: _, next: node_next } = node;
            let key_to_remove = *sibling_children.new_end_iter().iter_prev(sibling_children).iter_borrow_key(sibling_children);
            sibling_children.append_disjoint(node_children);
            sibling_node.next = node_next;

            if (sibling_node.next != NULL_INDEX) {
                assert!(self.nodes.borrow_mut(sibling_node.next).prev == node_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            };
            // we are removing sibling node_index, which previous's node's next was pointing to,
            // so update the pointer
            if (sibling_node.prev != NULL_INDEX) {
                self.nodes.borrow_mut(sibling_node.prev).next = node_index;
            };
            // Otherwise, sibling was the smallest node on the level. if this is the leaf level, update the pointer.
            if (self.min_leaf_index == sibling_index) {
                assert!(is_leaf, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
                self.min_leaf_index = node_index;
            };

            self.nodes.fill_reserved_slot(node_slot, sibling_node);

            (key_to_remove, sibling_slot)
        };

        assert!(!path_to_node.is_empty(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        let slot_to_remove = self.remove_at(path_to_node, &key_to_remove).destroy_inner_child();
        self.nodes.free_reserved_slot(reserved_slot_to_remove, slot_to_remove);

        old_child
    }

    // ===== spec ===========

    spec module {
        pragma verify = false;
    }

    // recursive functions need to be marked opaque

    spec add_at {
        pragma opaque;
    }

    spec remove_at {
        pragma opaque;
    }

    // ============================= Tests ====================================

    #[test_only]
    fun print_map<K: store, V: store>(self: &BigOrderedMap<K, V>) {
        // uncomment to debug:
        // aptos_std::debug::print(&std::string::utf8(b"print map"));
        // aptos_std::debug::print(self);
        // self.print_map_for_node(ROOT_INDEX, 0);
    }

    #[test_only]
    fun print_map_for_node<K: store + copy + drop, V: store>(self: &BigOrderedMap<K, V>, node_index: u64, level: u64) {
        let node = self.borrow_node(node_index);

        aptos_std::debug::print(&level);
        aptos_std::debug::print(&node_index);
        aptos_std::debug::print(node);

        if (!node.is_leaf) {
            node.children.for_each_ref_friend(|_key, node| {
                self.print_map_for_node(node.node_index.stored_to_index(), level + 1);
            });
        };
    }

    #[test_only]
    fun destroy_and_validate<K: drop + copy + store, V: drop + store>(self: BigOrderedMap<K, V>) {
        let it = self.new_begin_iter();
        while (!it.iter_is_end(&self)) {
            self.remove(it.iter_borrow_key());
            assert!(self.find(it.iter_borrow_key()).iter_is_end(&self), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            it = self.new_begin_iter();
            self.validate_map();
        };

        self.destroy_empty();
    }

    #[test_only]
    fun validate_iteration<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>) {
        let expected_num_elements = self.compute_length();
        let num_elements = 0;
        let it = self.new_begin_iter();
        while (!it.iter_is_end(self)) {
            num_elements += 1;
            it = it.iter_next(self);
        };

        assert!(num_elements == expected_num_elements, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let num_elements = 0;
        let it = self.new_end_iter();
        while (!it.iter_is_begin(self)) {
            it = it.iter_prev(self);
            num_elements += 1;
        };
        assert!(num_elements == expected_num_elements, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        let it = self.new_end_iter();
        if (!it.iter_is_begin(self)) {
            it = it.iter_prev(self);
            assert!(it.node_index == self.max_leaf_index, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        } else {
            assert!(expected_num_elements == 0, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };
    }

    #[test_only]
    fun validate_subtree<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, node_index: u64, expected_lower_bound_key: Option<K>, expected_max_key: Option<K>) {
        let node = self.borrow_node(node_index);
        let len = node.children.length();
        assert!(len <= self.get_max_degree(node.is_leaf), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));

        if (node_index != ROOT_INDEX) {
            assert!(len >= 1, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            assert!(len * 2 >= self.get_max_degree(node.is_leaf) || node_index == ROOT_INDEX, error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };

        node.children.validate_ordered();

        let previous_max_key = expected_lower_bound_key;
        node.children.for_each_ref_friend(|key: &K, child: &Child<V>| {
            if (!node.is_leaf) {
                self.validate_subtree(child.node_index.stored_to_index(), previous_max_key, option::some(*key));
            } else {
                assert!((child is Child::Leaf<V>), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
            };
            previous_max_key = option::some(*key);
        });

        if (expected_max_key.is_some()) {
            let expected_max_key = expected_max_key.extract();
            assert!(&expected_max_key == node.children.new_end_iter().iter_prev(&node.children).iter_borrow_key(&node.children), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };

        if (expected_lower_bound_key.is_some()) {
            let expected_lower_bound_key = expected_lower_bound_key.extract();
            assert!(cmp::compare(&expected_lower_bound_key, node.children.new_begin_iter().iter_borrow_key(&node.children)).is_lt(), error::invalid_state(EINTERNAL_INVARIANT_BROKEN));
        };
    }

    #[test_only]
    fun validate_map<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>) {
        self.validate_subtree(ROOT_INDEX, option::none(), option::none());
        self.validate_iteration();
    }

    #[test]
    fun test_small_example() {
        let map = new_with_config(5, 3, true);
        map.allocate_spare_slots(2);
        map.print_map(); map.validate_map();
        map.add(1, 1); map.print_map(); map.validate_map();
        map.add(2, 2); map.print_map(); map.validate_map();
        let r1 = map.upsert(3, 3); map.print_map(); map.validate_map();
        assert!(r1 == option::none(), 1);
        map.add(4, 4); map.print_map(); map.validate_map();
        let r2 = map.upsert(4, 8); map.print_map(); map.validate_map();
        assert!(r2 == option::some(4), 2);
        map.add(5, 5); map.print_map(); map.validate_map();
        map.add(6, 6); map.print_map(); map.validate_map();

        let expected_keys = vector[1, 2, 3, 4, 5, 6];
        let expected_values = vector[1, 2, 3, 8, 5, 6];

        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(k == expected_keys.borrow(index), *k + 100);
            assert!(v == expected_values.borrow(index), *k + 200);
            index += 1;
        });

        let index = 0;
        map.for_each_ref_friend(|k, v| {
            assert!(k == expected_keys.borrow(index), *k + 100);
            assert!(v == expected_values.borrow(index), *k + 200);
            index += 1;
        });

        expected_keys.zip(expected_values, |key, value| {
            assert!(map.borrow(&key) == &value, key + 300);
            assert!(map.borrow_mut(&key) == &value, key + 400);
        });

        map.remove(&5); map.print_map(); map.validate_map();
        map.remove(&4); map.print_map(); map.validate_map();
        map.remove(&1); map.print_map(); map.validate_map();
        map.remove(&3); map.print_map(); map.validate_map();
        map.remove(&2); map.print_map(); map.validate_map();
        map.remove(&6); map.print_map(); map.validate_map();

        map.destroy_empty();
    }

    #[test]
    fun test_for_each() {
        let map = new_with_config<u64, u64>(4, 3, false);
        map.add_all(vector[1, 3, 6, 2, 9, 5, 7, 4, 8], vector[1, 3, 6, 2, 9, 5, 7, 4, 8]);

        let expected = vector[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let index = 0;
        map.for_each(|k, v| {
            assert!(k == expected[index], k + 100);
            assert!(v == expected[index], k + 200);
            index += 1;
        });
    }

    #[test]
    fun test_for_each_ref() {
        let map = new_with_config<u64, u64>(4, 3, false);
        map.add_all(vector[1, 3, 6, 2, 9, 5, 7, 4, 8], vector[1, 3, 6, 2, 9, 5, 7, 4, 8]);

        let expected = vector[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(*k == expected[index], *k + 100);
            assert!(*v == expected[index], *k + 200);
            index += 1;
        });

        map.destroy(|_v| {});
    }

    #[test]
    fun test_for_each_variants() {
        let keys = vector[1, 3, 5];
        let values = vector[10, 30, 50];
        let map = new_from(keys, values);

        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(keys[index] == *k);
            assert!(values[index] == *v);
            index += 1;
        });

        let index = 0;
        map.for_each_mut(|k, v| {
            assert!(keys[index] == *k);
            assert!(values[index] == *v);
            *v += 1;
            index += 1;
        });

        let index = 0;
        map.for_each(|k, v| {
            assert!(keys[index] == k);
            assert!(values[index] + 1 == v);
            index += 1;
        });
    }

    #[test]
    fun test_variable_size() {
        let map = new_with_config<vector<u64>, vector<u64>>(0, 0, false);
        map.print_map(); map.validate_map();
        map.add(vector[1], vector[1]); map.print_map(); map.validate_map();
        map.add(vector[2], vector[2]); map.print_map(); map.validate_map();
        let r1 = map.upsert(vector[3], vector[3]); map.print_map(); map.validate_map();
        assert!(r1 == option::none(), 1);
        map.add(vector[4], vector[4]); map.print_map(); map.validate_map();
        let r2 = map.upsert(vector[4], vector[8, 8, 8]); map.print_map(); map.validate_map();
        assert!(r2 == option::some(vector[4]), 2);
        map.add(vector[5], vector[5]); map.print_map(); map.validate_map();
        map.add(vector[6], vector[6]); map.print_map(); map.validate_map();

        vector[1, 2, 3, 4, 5, 6].zip(vector[1, 2, 3, 8, 5, 6], |key, value| {
            assert!(map.borrow(&vector[key])[0] == value, key + 100);
        });

        map.remove(&vector[5]); map.print_map(); map.validate_map();
        map.remove(&vector[4]); map.print_map(); map.validate_map();
        map.remove(&vector[1]); map.print_map(); map.validate_map();
        map.remove(&vector[3]); map.print_map(); map.validate_map();
        map.remove(&vector[2]); map.print_map(); map.validate_map();
        map.remove(&vector[6]); map.print_map(); map.validate_map();

        map.destroy_empty();
    }
    #[test]
    fun test_deleting_and_creating_nodes() {
        let map = new_with_config(4, 3, true);
        map.allocate_spare_slots(2);

        for (i in 0..25) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 0..20) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 25..50) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 25..45) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 50..75) {
            map.upsert(i, i);
            map.validate_map();
        };

        for (i in 50..75) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 20..25) {
            map.remove(&i);
            map.validate_map();
        };

        for (i in 45..50) {
            map.remove(&i);
            map.validate_map();
        };

        map.destroy_empty();
    }

    #[test]
    fun test_iterator() {
        let map = new_with_config(5, 5, true);
        map.allocate_spare_slots(2);

        let data = vector[1, 7, 5, 8, 4, 2, 6, 3, 9, 0];
        while (data.length() != 0) {
            let element = data.pop_back();
            map.add(element, element);
        };

        let it = map.new_begin_iter();

        let i = 0;
        while (!it.iter_is_end(&map)) {
            assert!(i == it.key, i);
            assert!(it.iter_borrow(&map) == &i, i);
            assert!(it.iter_borrow_mut(&mut map) == &i, i);
            i += 1;
            it = it.iter_next(&map);
        };

        map.destroy(|_v| {});
    }

    #[test]
    fun test_find() {
        let map = new_with_config(5, 5, true);
        map.allocate_spare_slots(2);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 0, 10];
        map.add_all(data, data);

        let i = 0;
        while (i < data.length()) {
            let element = data.borrow(i);
            let it = map.find(element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.iter_borrow_key() == element, i);
            i += 1;
        };

        assert!(map.find(&4).iter_is_end(&map), 0);
        assert!(map.find(&9).iter_is_end(&map), 1);

        map.destroy(|_v| {});
    }

    #[test]
    fun test_lower_bound() {
        let map = new_with_config(5, 5, true);
        map.allocate_spare_slots(2);

        let data = vector[11, 1, 7, 5, 8, 2, 6, 3, 12, 10];
        map.add_all(data, data);

        let i = 0;
        while (i < data.length()) {
            let element = *data.borrow(i);
            let it = map.lower_bound(&element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.key == element, i);
            i += 1;
        };

        assert!(map.lower_bound(&0).key == 1, 0);
        assert!(map.lower_bound(&4).key == 5, 1);
        assert!(map.lower_bound(&9).key == 10, 2);
        assert!(map.lower_bound(&13).iter_is_end(&map), 3);

        map.remove(&3);
        assert!(map.lower_bound(&3).key == 5, 4);
        map.remove(&5);
        assert!(map.lower_bound(&3).key == 6, 5);
        assert!(map.lower_bound(&4).key == 6, 6);

        map.destroy(|_v| {});
    }

    #[test]
    fun test_contains() {
        let map = new_with_config(4, 3, false);
        let data = vector[3, 1, 9, 7, 5];
        map.add_all(vector[3, 1, 9, 7, 5], vector[3, 1, 9, 7, 5]);

        data.for_each_ref(|i| assert!(map.contains(i), *i));

        let missing = vector[0, 2, 4, 6, 8, 10];
        missing.for_each_ref(|i| assert!(!map.contains(i), *i));

        map.destroy(|_v| {});
    }

    #[test]
    fun test_non_iterator_ordering() {
        let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
        assert!(map.prev_key(&1).is_none(), 1);
        assert!(map.next_key(&1) == option::some(2), 1);

        assert!(map.prev_key(&2) == option::some(1), 2);
        assert!(map.next_key(&2) == option::some(3), 3);

        assert!(map.prev_key(&3) == option::some(2), 4);
        assert!(map.next_key(&3).is_none(), 5);

        let (front_k, front_v) = map.borrow_front();
        assert!(front_k == 1, 6);
        assert!(front_v == &10, 7);

        let (back_k, back_v) = map.borrow_back();
        assert!(back_k == 3, 8);
        assert!(back_v == &30, 9);

        let (front_k, front_v) = map.pop_front();
        assert!(front_k == 1, 10);
        assert!(front_v == 10, 11);

        let (back_k, back_v) = map.pop_back();
        assert!(back_k == 3, 12);
        assert!(back_v == 30, 13);

        map.destroy(|_v| {});
    }

    #[test]
    #[expected_failure(abort_code = 0x1000B, location = Self)] /// EINVALID_CONFIG_PARAMETER
    fun test_inner_max_degree_too_large() {
        let map = new_with_config<u8, u8>(4097, 0, false);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000B, location = Self)] /// EINVALID_CONFIG_PARAMETER
    fun test_inner_max_degree_too_small() {
        let map = new_with_config<u8, u8>(3, 0, false);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000B, location = Self)] /// EINVALID_CONFIG_PARAMETER
    fun test_leaf_max_degree_too_small() {
        let map = new_with_config<u8, u8>(0, 2, false);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_abort_add_existing_value() {
        let map = new_from(vector[1], vector[1]);
        map.add(1, 2);
        map.destroy_and_validate();
    }

    #[test_only]
    fun vector_range(from: u64, to: u64): vector<u64> {
        let result = vector[];
        for (i in from..to) {
            result.push_back(i);
        };
        result
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_abort_add_existing_value_to_non_leaf() {
        let map = new_with_config(4, 4, false);
        map.add_all(vector_range(1, 10), vector_range(1, 10));
        map.add(3, 3);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = aptos_std::ordered_map)] /// EKEY_NOT_FOUND
    fun test_abort_remove_missing_value() {
        let map = new_from(vector[1], vector[1]);
        map.remove(&2);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = aptos_std::ordered_map)] /// EKEY_NOT_FOUND
    fun test_abort_remove_missing_value_to_non_leaf() {
        let map = new_with_config(4, 4, false);
        map.add_all(vector_range(1, 10), vector_range(1, 10));
        map.remove(&4);
        map.remove(&4);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_abort_remove_largest_missing_value_to_non_leaf() {
        let map = new_with_config(4, 4, false);
        map.add_all(vector_range(1, 10), vector_range(1, 10));
        map.remove(&11);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_abort_borrow_missing() {
        let map = new_from(vector[1], vector[1]);
        map.borrow(&2);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_abort_borrow_mut_missing() {
        let map = new_from(vector[1], vector[1]);
        map.borrow_mut(&2);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000E, location = Self)] /// EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE
    fun test_abort_borrow_mut_requires_constant_kv_size() {
        let map = new_with_config(0, 0, false);
        map.add(1, vector[1]);
        map.borrow_mut(&1);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_iter_borrow_key_missing() {
        let map = new_from(vector[1], vector[1]);
        map.new_end_iter().iter_borrow_key();
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_iter_borrow_missing() {
        let map = new_from(vector[1], vector[1]);
        map.new_end_iter().iter_borrow(&map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_iter_borrow_mut_missing() {
        let map = new_from(vector[1], vector[1]);
        map.new_end_iter().iter_borrow_mut(&mut map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000E, location = Self)] /// EBORROW_MUT_REQUIRES_CONSTANT_KV_SIZE
    fun test_abort_iter_borrow_mut_requires_constant_kv_size() {
        let map = new_with_config(0, 0, false);
        map.add(1, vector[1]);
        map.new_begin_iter().iter_borrow_mut(&mut map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_end_iter_next() {
        let map = new_from(vector[1, 2, 3], vector[1, 2, 3]);
        map.new_end_iter().iter_next(&map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    fun test_abort_begin_iter_prev() {
        let map = new_from(vector[1, 2, 3], vector[1, 2, 3]);
        map.new_begin_iter().iter_prev(&map);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000C, location = Self)] /// EMAP_NOT_EMPTY
    fun test_abort_fail_to_destroy_non_empty() {
        let map = new_from(vector[1], vector[1]);
        map.destroy_empty();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000D, location = Self)] /// EARGUMENT_BYTES_TOO_LARGE
    fun test_adding_key_too_large() {
        let map = new_with_config(0, 0, false);
        map.add(vector[1], 1);
        map.add(vector_range(0, 143), 1);
        map.destroy_and_validate();
    }

    #[test]
    #[expected_failure(abort_code = 0x1000D, location = Self)] /// EARGUMENT_BYTES_TOO_LARGE
    fun test_adding_value_too_large() {
        let map = new_with_config(0, 0, false);
        map.add(1, vector[1]);
        map.add(2, vector_range(0, 268));
        map.destroy_and_validate();
    }

    #[test_only]
    inline fun comparison_test(repeats: u64, inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool, next_1: ||u64, next_2: ||u64) {
        let big_map = new_with_config(inner_max_degree, leaf_max_degree, reuse_slots);
        if (reuse_slots) {
            big_map.allocate_spare_slots(4);
        };
        let small_map = ordered_map::new();
        for (i in 0..repeats) {
            let is_insert = if (2 * i < repeats) {
                i % 3 != 2
            } else {
                i % 3 == 0
            };
            if (is_insert) {
                let v = next_1();
                assert!(big_map.upsert(v, v) == small_map.upsert(v, v), i);
            } else {
                let v = next_2();
                assert!(big_map.remove(&v) == small_map.remove(&v), i);
            };
            if ((i + 1) % 50 == 0) {
                big_map.validate_map();

                let big_iter = big_map.new_begin_iter();
                let small_iter = small_map.new_begin_iter();
                while (!big_iter.iter_is_end(&big_map) || !small_iter.iter_is_end(&small_map)) {
                    assert!(big_iter.iter_borrow_key() == small_iter.iter_borrow_key(&small_map), i);
                    assert!(big_iter.iter_borrow(&big_map) == small_iter.iter_borrow(&small_map), i);
                    big_iter = big_iter.iter_next(&big_map);
                    small_iter = small_iter.iter_next(&small_map);
                };
            };
        };
        big_map.destroy_and_validate();
    }

    #[test_only]
    const OFFSET: u64 = 270001;
    #[test_only]
    const MOD: u64 = 1000000;

    #[test]
    fun test_comparison_random() {
        let x = 1234;
        let y = 1234;
        comparison_test(500, 5, 5, false,
            || {
                x += OFFSET;
                if (x > MOD) { x -= MOD};
                x
            },
            || {
                y += OFFSET;
                if (y > MOD) { y -= MOD};
                y
            },
        );
    }

    #[test]
    fun test_comparison_increasing() {
        let x = 0;
        let y = 0;
        comparison_test(500, 5, 5, false,
            || {
                x += 1;
                x
            },
            || {
                y += 1;
                y
            },
        );
    }

    #[test]
    fun test_comparison_decreasing() {
        let x = 100000;
        let y = 100000;
        comparison_test(500, 5, 5, false,
            || {
                x -= 1;
                x
            },
            || {
                y -= 1;
                y
            },
        );
    }

    #[test_only]
    fun test_large_data_set_helper(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool) {
        use std::vector;

        let map = new_with_config(inner_max_degree, leaf_max_degree, reuse_slots);
        if (reuse_slots) {
            map.allocate_spare_slots(4);
        };
        let data = ordered_map::large_dataset();
        let shuffled_data = ordered_map::large_dataset_shuffled();

        let len = data.length();
        for (i in 0..len) {
            let element = data[i];
            map.upsert(element, element);
            if (i % 7 == 0) {
                map.validate_map();
            }
        };

        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            let it = map.find(element);
            assert!(!it.iter_is_end(&map), i);
            assert!(it.iter_borrow_key() == element, i);

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
                if (i % 7 == 1) {
                    map.validate_map();

                }
            } else {
                assert!(!map.contains(element));
            };
        };

        map.destroy_empty();
    }

    // Currently ignored long / more extensive tests.

    // #[test]
    // fun test_large_data_set_order_5_false() {
    //     test_large_data_set_helper(5, 5, false);
    // }

    // #[test]
    // fun test_large_data_set_order_5_true() {
    //     test_large_data_set_helper(5, 5, true);
    // }

    // #[test]
    // fun test_large_data_set_order_4_3_false() {
    //     test_large_data_set_helper(4, 3, false);
    // }

    // #[test]
    // fun test_large_data_set_order_4_3_true() {
    //     test_large_data_set_helper(4, 3, true);
    // }

    // #[test]
    // fun test_large_data_set_order_4_4_false() {
    //     test_large_data_set_helper(4, 4, false);
    // }

    // #[test]
    // fun test_large_data_set_order_4_4_true() {
    //     test_large_data_set_helper(4, 4, true);
    // }

    // #[test]
    // fun test_large_data_set_order_6_false() {
    //     test_large_data_set_helper(6, 6, false);
    // }

    // #[test]
    // fun test_large_data_set_order_6_true() {
    //     test_large_data_set_helper(6, 6, true);
    // }

    // #[test]
    // fun test_large_data_set_order_6_3_false() {
    //     test_large_data_set_helper(6, 3, false);
    // }

    #[test]
    fun test_large_data_set_order_6_3_true() {
        test_large_data_set_helper(6, 3, true);
    }

    #[test]
    fun test_large_data_set_order_4_6_false() {
        test_large_data_set_helper(4, 6, false);
    }

    // #[test]
    // fun test_large_data_set_order_4_6_true() {
    //     test_large_data_set_helper(4, 6, true);
    // }

    // #[test]
    // fun test_large_data_set_order_16_false() {
    //     test_large_data_set_helper(16, 16, false);
    // }

    // #[test]
    // fun test_large_data_set_order_16_true() {
    //     test_large_data_set_helper(16, 16, true);
    // }

    // #[test]
    // fun test_large_data_set_order_31_false() {
    //     test_large_data_set_helper(31, 31, false);
    // }

    // #[test]
    // fun test_large_data_set_order_31_true() {
    //     test_large_data_set_helper(31, 31, true);
    // }

    // #[test]
    // fun test_large_data_set_order_31_3_false() {
    //     test_large_data_set_helper(31, 3, false);
    // }

    // #[test]
    // fun test_large_data_set_order_31_3_true() {
    //     test_large_data_set_helper(31, 3, true);
    // }

    // #[test]
    // fun test_large_data_set_order_31_5_false() {
    //     test_large_data_set_helper(31, 5, false);
    // }

    // #[test]
    // fun test_large_data_set_order_31_5_true() {
    //     test_large_data_set_helper(31, 5, true);
    // }

    // #[test]
    // fun test_large_data_set_order_32_false() {
    //     test_large_data_set_helper(32, 32, false);
    // }

    // #[test]
    // fun test_large_data_set_order_32_true() {
    //     test_large_data_set_helper(32, 32, true);
    // }
}
