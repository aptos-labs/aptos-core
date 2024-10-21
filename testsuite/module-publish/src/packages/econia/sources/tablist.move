/// Tablist: a hybrid between a table and a doubly linked list.
///
/// Modeled off of what was previously `aptos_std::iterable_table.move`,
/// which had been removed from `aptos_std` as of the time of this
/// writing.
///
/// Accepts key-value pairs having key type `K` and value type `V`.
///
/// See `test_iterate()` and `test_iterate_remove()` for iteration
/// syntax.
///
/// # Complete docgen index
///
/// The below index is automatically generated from source code:
module econia::tablist {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_std::table_with_length::{Self, TableWithLength};
    use std::option::{Self, Option};

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// A tablist node, pointing to the previous and next nodes, if any.
    struct Node<
        K: copy + drop + store,
        V: store
    > has store {
        /// Value from a key-value pair.
        value: V,
        /// Key of previous tablist node, if any.
        previous: Option<K>,
        /// Key of next tablist node, if any.
        next: Option<K>
    }

    /// A hybrid between a table and a doubly linked list.
    struct Tablist<
        K: copy + drop + store,
        V: store
    > has store {
        /// All nodes in the tablist.
        table: TableWithLength<K, Node<K, V>>,
        /// Key of tablist head node, if any.
        head: Option<K>,
        /// Key of tablist tail node, if any.
        tail: Option<K>
    }

    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Attempting to destroy a tablist that is not empty.
    const E_DESTROY_NOT_EMPTY: u64 = 0;

    // Error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Add `key`-`value` pair to given `Tablist`, aborting if `key`
    /// already present.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun add<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref_mut: &mut Tablist<K, V>,
        key: K,
        value: V
    ) {
        let node = Node{value, previous: tablist_ref_mut.tail,
            next: option::none()}; // Wrap value in a node.
        // Add node to the inner table.
        table_with_length::add(&mut tablist_ref_mut.table, key, node);
        // If adding the first node in the tablist:
        if (option::is_none(&tablist_ref_mut.head)) {
            // Mark key as the new head.
            tablist_ref_mut.head = option::some(key);
        } else { // If adding node that is not first node in tablist:
            // Get the old tail node key.
            let old_tail = option::borrow(&tablist_ref_mut.tail);
            // Update the old tail node to have the new key as next.
            table_with_length::borrow_mut(
                &mut tablist_ref_mut.table, *old_tail).next =
                    option::some(key);
        };
        // Update the tablist tail to the new key.
        tablist_ref_mut.tail = option::some(key);
    }

    /// Return immutable reference to the value that `key` maps to,
    /// aborting if `key` is not in given `Tablist`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun borrow<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref: &Tablist<K, V>,
        key: K,
    ): &V {
        &table_with_length::borrow(&tablist_ref.table, key).value
    }

    /// Borrow the `Node` in the given `Tablist` having key, returning:
    ///
    /// * Immutable reference to corresponding value.
    /// * Key of previous `Node` in the `Tablist`, if any.
    /// * Key of next `Node` in the `Tablist`, if any.
    ///
    /// Aborts if there is no entry for `key`.
    ///
    /// # Testing
    ///
    /// * `test_iterate()`
    public fun borrow_iterable<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref: &Tablist<K, V>,
        key: K,
    ): (
        &V,
        Option<K>,
        Option<K>
    ) {
        let node_ref = // Borrow immutable reference to node having key.
            table_with_length::borrow(&tablist_ref.table, key);
        // Return corresponding fields.
        (&node_ref.value, node_ref.previous, node_ref.next)
    }

    /// Mutably borrow the `Node` in given `Tablist` having `key`,
    /// returning:
    ///
    /// * Mutable reference to corresponding value.
    /// * Key of previous `Node` in the `Tablist`, if any.
    /// * Key of next `Node` in the `Tablist`, if any.
    ///
    /// Aborts if there is no entry for `key`.
    ///
    /// # Testing
    ///
    /// * `test_iterate()`
    public fun borrow_iterable_mut<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref_mut: &mut Tablist<K, V>,
        key: K,
    ): (
        &mut V,
        Option<K>,
        Option<K>
    ) {
        // Borrow mutable reference to node having key.
        let node_ref_mut = table_with_length::borrow_mut(
            &mut tablist_ref_mut.table, key);
        // Return corresponding fields.
        (&mut node_ref_mut.value, node_ref_mut.previous, node_ref_mut.next)
    }

    /// Return mutable reference to the value that `key` maps to,
    /// aborting if `key` is not in given `Tablist`.
    ///
    /// Aborts if there is no entry for `key`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun borrow_mut<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref_mut: &mut Tablist<K, V>,
        key: K,
    ): &mut V {
        &mut table_with_length::borrow_mut(
            &mut tablist_ref_mut.table, key).value
    }

    /// Return `true` if given `Tablist` contains `key`, else `false`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun contains<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref: &Tablist<K, V>,
        key: K,
    ): bool {
        table_with_length::contains(&tablist_ref.table, key)
    }

    /// Destroy an empty `Tablist`, aborting if not empty.
    ///
    /// # Aborts
    ///
    /// * `E_DESTROY_NOT_EMPTY`: The tablist is not empty.
    ///
    /// # Testing
    ///
    /// * `test_destroy_empty_not_empty()`
    /// * `test_mixed()`
    public fun destroy_empty<
        K: copy + drop + store,
        V: store
    >(
        tablist: Tablist<K, V>
    ) {
        // Assert tablist is empty before attempting to unpack.
        assert!(is_empty(&tablist), E_DESTROY_NOT_EMPTY);
        // Unpack, destroying head and tail fields.
        let Tablist{table, head: _, tail: _} = tablist;
        // Destroy empty inner table.
        table_with_length::destroy_empty(table);
    }

    /// Return optional head key from given `Tablist`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun get_head_key<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref: &Tablist<K, V>
    ): Option<K> {
        tablist_ref.head
    }

    /// Return optional tail key in given `Tablist`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun get_tail_key<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref: &Tablist<K, V>
    ): Option<K> {
        tablist_ref.tail
    }

    /// Return number of elements in given `Tablist`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun length<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref: &Tablist<K, V>
    ): u64 {
        table_with_length::length(&tablist_ref.table)
    }

    /// Return an empty `Tablist`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun new<
        K: copy + drop + store,
        V: store
    >(): Tablist<K, V> {
        Tablist{
            table: table_with_length::new(),
            head: option::none(),
            tail: option::none()
        }
    }

    /// Return `true` if given `Tablist` is empty, else `false`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun is_empty<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref: &Tablist<K, V>
    ): bool {
        table_with_length::empty(&tablist_ref.table)
    }

    /// Remove `key` from given `Tablist`, returning the value `key`
    /// mapped to.
    ///
    /// See wrapped function `remove_iterable()`.
    ///
    /// Aborts if there is no entry for `key`.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun remove<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref_mut: &mut Tablist<K, V>,
        key: K
    ): V {
        // Get value via iterable removal.
        let (value, _, _) = remove_iterable(tablist_ref_mut, key);
        value // Return value.
    }

    /// Remove `key` from given `Tablist`, returning the value `key`
    /// mapped to, the previous key it mapped to (if any), and the
    /// next key it mapped to (if any).
    ///
    /// Aborts if there is no entry for `key`.
    ///
    /// # Testing
    ///
    /// * `test_iterate_remove()`
    public fun remove_iterable<
        K: copy + drop + store,
        V: store
    >(
        tablist_ref_mut: &mut Tablist<K, V>,
        key: K
    ): (
        V,
        Option<K>,
        Option<K>
    ) {
        // Unpack from inner table the node with the given key.
        let Node{value, previous, next} = table_with_length::remove(
            &mut tablist_ref_mut.table, key);
        // If the node was the head of the tablist:
        if (option::is_none(&previous)) { // If no previous node:
            // Set as the tablist head the node's next field.
            tablist_ref_mut.head = next;
        } else { // If node was not head of the tablist:
            // Update the node having the previous key to have as its
            // next field the next field of the removed node.
            table_with_length::borrow_mut(&mut tablist_ref_mut.table,
                *option::borrow(&previous)).next = next;
        };
        // If the node was the tail of the tablist:
        if (option::is_none(&next)) { // If no next node:
            // Set as the tablist tail the node's previous field.
            tablist_ref_mut.tail = previous;
        } else { // If node was not tail of tablist:
            // Update the node having the next key to have as its
            // previous field the previous field of the removed node.
            table_with_length::borrow_mut(&mut tablist_ref_mut.table,
                *option::borrow(&next)).previous = previous;
        };
        // Return node value, previous field, and next field.
        (value, previous, next)
    }

    /// Return a new `Tablist` containing `key`-`value` pair.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public fun singleton<
        K: copy + drop + store,
        V: store
    >(
        key: K,
        value: V
    ): Tablist<K, V> {
        let tablist = new<K, V>(); // Declare empty tablist
        add(&mut tablist, key, value); // Insert key-value pair.
        tablist // Return tablist.
    }

    // Public functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test]
    #[expected_failure(abort_code = E_DESTROY_NOT_EMPTY)]
    /// Verify failure for non-empty tablist destruction.
    fun test_destroy_empty_not_empty() {destroy_empty(singleton(0, 0));}

    #[test]
    /// Verify iteration in the following sequence:
    ///
    /// * Immutably, from head to tail.
    /// * Mutably, from tail to head.
    /// * Mutably, from head to tail.
    /// * Immutably, from tail to head.
    fun test_iterate(): Tablist<u64, u64> {
        let tablist = new(); // Declare new tablist.
        let i = 0; // Declare counter.
        while (i < 100) { // For 100 iterations:
            // Add key-value pair to table where key and value are both
            // the value of the counter.
            add(&mut tablist, i, i);
            i = i + 1; // Increment counter.
        };
        assert!(length(&tablist) == 100, 0); // Assert proper length.
        let key = get_head_key(&tablist); // Get head key.
        i = 0; // Re-init counter.
        // While keys left to iterate on, iterate from head to tail:
        while (option::is_some(&key)) {
            // Get value for key and next key in tablist.
            let (value_ref, _, next) =
                borrow_iterable(&tablist, *option::borrow(&key));
            // Assert key-value pair.
            assert!(*option::borrow(&key) == i, 0);
            assert!(*value_ref == i, 0);
            key = next; // Review the next key.
            i = i + 1; // Increment counter.
        };
        key = get_tail_key(&tablist); // Get tail key.
        i = 0; // Re-init counter
        // While keys left to iterate on, iterate from tail to head:
        while (option::is_some(&key)) {
            // Get value for key and previous key in tablist.
            let (value_ref_mut, previous, _) =
                borrow_iterable_mut(&mut tablist, *option::borrow(&key));
            // Assert key-value pair.
            assert!(*option::borrow(&key) == (99 - i), 0);
            assert!(*value_ref_mut == (99 - i), 0);
            // Mutate the value by adding 1.
            *value_ref_mut = *value_ref_mut + 1;
            key = previous; // Review the previous key.
            i = i + 1; // Increment counter.
        };
        let key = get_head_key(&tablist); // Get head key.
        i = 0; // Re-init counter.
        // While keys left to iterate on, iterate from head to tail:
        while (option::is_some(&key)) {
            // Get value for key and next key in tablist.
            let (value_ref_mut, _, next) =
                borrow_iterable_mut(&mut tablist, *option::borrow(&key));
            // Assert key-value pair.
            assert!(*option::borrow(&key) == i, 0);
            assert!(*value_ref_mut == i + 1, 0);
            // Mutate the value by adding 1.
            *value_ref_mut = *value_ref_mut + 1;
            key = next; // Review the next key.
            i = i + 1; // Increment counter.
        };
        key = get_tail_key(&tablist); // Get tail key.
        i = 0; // Re-init counter
        // While keys left to iterate on, iterate from tail to head:
        while (option::is_some(&key)) {
            // Get value for key and previous key in tablist.
            let (value_ref, previous, _) =
                borrow_iterable(&tablist, *option::borrow(&key));
            // Assert key-value pair.
            assert!(*option::borrow(&key) == (99 - i), 0);
            assert!(*value_ref == (99 - i) + 2, 0);
            key = previous; // Review the previous key.
            i = i + 1; // Increment counter.
        };
        tablist // Return tablist.
    }

    #[test]
    /// Verify iterated removal, first from head to tail, then from
    /// tail to head.
    fun test_iterate_remove() {
        let tablist = new(); // Declare new tablist.
        let i = 0; // Declare counter.
        while (i < 100) { // For 100 iterations:
            // Add key-value pair to tablist where key and value are
            // both the value of the counter.
            add(&mut tablist, i, i);
            i = i + 1; // Increment counter.
        };
        let key = get_head_key(&tablist); // Get head key.
        i = 0; // Re-initialize counter.
        // While keys left to iterate on:
        while (option::is_some(&key)) {
            // Get value for key and next key in tablist.
            let (value, _, next) = remove_iterable(
                &mut tablist, *option::borrow(&key));
            // Assert key-value pair.
            assert!(*option::borrow(&key) == i, 0);
            assert!(value == i, 0);
            key = next; // Review the next key.
            i = i + 1; // Increment counter
        };
        i = 0; // Re-initialize counter.
        while (i < 100) { // For 100 iterations:
            // Add key-value pair to tablist where key and value are
            // both the value of the counter.
            add(&mut tablist, i, i);
            i = i + 1; // Increment counter.
        };
        key = get_tail_key(&tablist); // Get tail key.
        i = 0; // Re-initialize counter.
        // While keys left to iterate on:
        while (option::is_some(&key)) {
            // Get value for key and previous key in tablist.
            let (value, previous, _) = remove_iterable(
                &mut tablist, *option::borrow(&key));
            // Assert key-value pair.
            assert!(*option::borrow(&key) == (99 - i), 0);
            assert!(value == (99 - i), 0);
            key = previous; // Review the previous key.
            i = i + 1; // Increment counter
        };
        destroy_empty(tablist); // Destroy empty tablist.
    }

    #[test]
    /// Verify assorted functionality, without iteration.
    fun test_mixed() {
        // Declare key-value pairs.
        let (key_0, value_0, key_1, value_1) = (1u8, true, 2u8, false);
        // Declare empty tablist.
        let empty_tablist = new<u8, bool>();
        // Assert state.
        assert!(length(&empty_tablist) == 0, 0);
        assert!(is_empty(&empty_tablist), 0);
        assert!(option::is_none(&get_head_key(&empty_tablist)), 0);
        assert!(option::is_none(&get_tail_key(&empty_tablist)), 0);
        assert!(!contains(&empty_tablist, key_0), 0);
        assert!(!contains(&empty_tablist, key_1), 0);
        // Destroy empty tablist.
        destroy_empty(empty_tablist);
        // Declare singleton.
        let tablist = singleton(key_0, value_0);
        // Assert state.
        assert!(length(&tablist) == 1, 0);
        assert!(!is_empty(&tablist), 0);
        assert!(*option::borrow(&get_head_key(&tablist)) == key_0, 0);
        assert!(*option::borrow(&get_tail_key(&tablist)) == key_0, 0);
        assert!(contains(&tablist, key_0), 0);
        assert!(!contains(&tablist, key_1), 0);
        assert!(*borrow(&tablist, key_0) == value_0, 0);
        // Mutate value.
        *borrow_mut(&mut tablist, key_0) = !value_0;
        // Assert mutation.
        assert!(*borrow(&tablist, key_0) == !value_0, 0);
        // Mutate value back.
        *borrow_mut(&mut tablist, key_0) = value_0;
        // Add another key-value pair.
        add(&mut tablist, key_1, value_1);
        // Assert state.
        assert!(length(&tablist) == 2, 0);
        assert!(!is_empty(&tablist), 0);
        assert!(*option::borrow(&get_head_key(&tablist)) == key_0, 0);
        assert!(*option::borrow(&get_tail_key(&tablist)) == key_1, 0);
        assert!(contains(&tablist, key_0), 0);
        assert!(contains(&tablist, key_1), 0);
        assert!(*borrow(&tablist, key_0) == value_0, 0);
        assert!(*borrow(&tablist, key_1) == value_1, 0);
        // Remove both nodes, asserting values.
        assert!(remove(&mut tablist, key_0) == value_0, 0);
        assert!(remove(&mut tablist, key_1) == value_1, 0);
        destroy_empty(tablist); // Destroy empty tablist.
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}