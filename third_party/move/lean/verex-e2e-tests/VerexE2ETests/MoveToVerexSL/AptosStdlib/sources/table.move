/// Type of large-scale storage tables.
/// source: https://github.com/move-language/move/blob/1b6b7513dcc1a5c866f178ca5c1e74beb2ce181e/language/extensions/move-table-extension/sources/Table.move#L1
///
/// It implements the Table type which supports individual table items to be represented by
/// separate global state items. The number of items and a unique handle are tracked on the table
/// struct itself, while the operations are implemented as native functions. No traversal is provided.

module aptos_std::table {
    friend aptos_std::table_with_length;
    friend aptos_std::storage_slots_allocator;

    /// Type of tables
    struct Table<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }

    /// Create a new Table.
    public fun new<K: copy + drop, V: store>(): Table<K, V> {
        Table {
            handle: new_table_handle<K, V>(),
        }
    }

    /// Add a new entry to the table. Aborts if an entry for this
    /// key already exists. The entry itself is not stored in the
    /// table, and cannot be discovered from it.
    public fun add<K: copy + drop, V>(self: &mut Table<K, V>, key: K, val: V) {
        add_box<K, V, Box<V>>(self, key, Box { val })
    }

    /// Acquire an immutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow<K: copy + drop, V>(self: &Table<K, V>, key: K): &V {
        &borrow_box<K, V, Box<V>>(self, key).val
    }

    /// Acquire an immutable reference to the value which `key` maps to.
    /// Returns specified default value if there is no entry for `key`.
    public fun borrow_with_default<K: copy + drop, V>(self: &Table<K, V>, key: K, default: &V): &V {
        if (!self.contains(copy key)) {
            default
        } else {
            self.borrow(copy key)
        }
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: copy + drop, V>(self: &mut Table<K, V>, key: K): &mut V {
        &mut borrow_box_mut<K, V, Box<V>>(self, key).val
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Insert the pair (`key`, `default`) first if there is no entry for `key`.
    public fun borrow_mut_with_default<K: copy + drop, V: drop>(self: &mut Table<K, V>, key: K, default: V): &mut V {
        if (!self.contains(copy key)) {
            self.add(copy key, default)
        };
        self.borrow_mut(key)
    }

    /// Insert the pair (`key`, `value`) if there is no entry for `key`.
    /// update the value of the entry for `key` to `value` otherwise
    public fun upsert<K: copy + drop, V: drop>(self: &mut Table<K, V>, key: K, value: V) {
        if (!self.contains(copy key)) {
            self.add(copy key, value)
        } else {
            let ref = self.borrow_mut(key);
            *ref = value;
        };
    }

    /// Remove from `self` and return the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: copy + drop, V>(self: &mut Table<K, V>, key: K): V {
        let Box { val } = remove_box<K, V, Box<V>>(self, key);
        val
    }

    /// Returns true iff `self` contains an entry for `key`.
    public fun contains<K: copy + drop, V>(self: &Table<K, V>, key: K): bool {
        contains_box<K, V, Box<V>>(self, key)
    }

    #[test_only]
    /// Testing only: allows to drop a table even if it is not empty.
    public fun drop_unchecked<K: copy + drop, V>(self: Table<K, V>) {
        drop_unchecked_box<K, V, Box<V>>(self)
    }

    /// Table cannot know if it is empty or not, so this method is not public,
    /// and can be used only in modules that know by themselves that table is empty.
    friend fun destroy_known_empty_unsafe<K: copy + drop, V>(self: Table<K, V>) {
        destroy_empty_box<K, V, Box<V>>(&self);
        drop_unchecked_box<K, V, Box<V>>(self)
    }

    #[test_only]
    struct TableHolder<phantom K: copy + drop, phantom V: drop> has key {
        t: Table<K, V>
    }

    #[test(account = @0x1)]
    fun test_upsert(account: signer) {
        let t = new<u64, u8>();
        let key: u64 = 111;
        let error_code: u64 = 1;
        assert!(!t.contains(key), error_code);
        t.upsert(key, 12);
        assert!(*t.borrow(key) == 12, error_code);
        t.upsert(key, 23);
        assert!(*t.borrow(key) == 23, error_code);

        move_to(&account, TableHolder { t });
    }

    #[test(account = @0x1)]
    fun test_borrow_with_default(account: signer) {
        let t = new<u64, u8>();
        let key: u64 = 100;
        let error_code: u64 = 1;
        assert!(!t.contains(key), error_code);
        assert!(*t.borrow_with_default(key, &12) == 12, error_code);
        t.add(key, 1);
        assert!(*t.borrow_with_default(key, &12) == 1, error_code);

        move_to(&account, TableHolder{ t });
    }

    // ======================================================================================================
    // Internal API

    /// Wrapper for values. Required for making values appear as resources in the implementation.
    struct Box<V> has key, drop, store {
        val: V
    }

    // Primitives which take as an additional type parameter `Box<V>`, so the implementation
    // can use this to determine serialization layout.
    native fun new_table_handle<K, V>(): address;

    native fun add_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>);

    native fun borrow_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): &Box<V>;

    native fun borrow_box_mut<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): &mut Box<V>;

    native fun contains_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): bool;

    native fun remove_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): Box<V>;

    native fun destroy_empty_box<K: copy + drop, V, B>(table: &Table<K, V>);

    native fun drop_unchecked_box<K: copy + drop, V, B>(table: Table<K, V>);
}
