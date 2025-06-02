/// Extends Table and provides functions such as length and the ability to be destroyed

module aptos_std::table_with_length {
    use std::error;
    use aptos_std::table::{Self, Table};

    // native code raises this with error::invalid_arguments()
    const EALREADY_EXISTS: u64 = 100;
    // native code raises this with error::invalid_arguments()
    const ENOT_FOUND: u64 = 101;
    const ENOT_EMPTY: u64 = 102;

    /// Type of tables
    struct TableWithLength<phantom K: copy + drop, phantom V> has store {
        inner: Table<K, V>,
        length: u64,
    }

    /// Create a new Table.
    public fun new<K: copy + drop, V: store>(): TableWithLength<K, V> {
        TableWithLength {
            inner: table::new<K, V>(),
            length: 0,
        }
    }

    /// Destroy a table. The table must be empty to succeed.
    public fun destroy_empty<K: copy + drop, V>(self: TableWithLength<K, V>) {
        assert!(self.length == 0, error::invalid_state(ENOT_EMPTY));
        let TableWithLength { inner, length: _ } = self;
        inner.destroy_known_empty_unsafe()
    }

    /// Add a new entry to the table. Aborts if an entry for this
    /// key already exists. The entry itself is not stored in the
    /// table, and cannot be discovered from it.
    public fun add<K: copy + drop, V>(self: &mut TableWithLength<K, V>, key: K, val: V) {
        self.inner.add(key, val);
        self.length += 1;
    }

    /// Acquire an immutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow<K: copy + drop, V>(self: &TableWithLength<K, V>, key: K): &V {
        self.inner.borrow(key)
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: copy + drop, V>(self: &mut TableWithLength<K, V>, key: K): &mut V {
        self.inner.borrow_mut(key)
    }

    /// Returns the length of the table, i.e. the number of entries.
    public fun length<K: copy + drop, V>(self: &TableWithLength<K, V>): u64 {
        self.length
    }

    /// Returns true if this table is empty.
    public fun empty<K: copy + drop, V>(self: &TableWithLength<K, V>): bool {
        self.length == 0
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Insert the pair (`key`, `default`) first if there is no entry for `key`.
    public fun borrow_mut_with_default<K: copy + drop, V: drop>(self: &mut TableWithLength<K, V>, key: K, default: V): &mut V {
        if (self.inner.contains(key)) {
            self.inner.borrow_mut(key)
        } else {
            self.inner.add(key, default);
            self.length += 1;
            self.inner.borrow_mut(key)
        }
    }

    /// Insert the pair (`key`, `value`) if there is no entry for `key`.
    /// update the value of the entry for `key` to `value` otherwise
    public fun upsert<K: copy + drop, V: drop>(self: &mut TableWithLength<K, V>, key: K, value: V) {
        if (!self.inner.contains(key)) {
            self.add(copy key, value)
        } else {
            let ref = self.inner.borrow_mut(key);
            *ref = value;
        };
    }

    /// Remove from `table` and return the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: copy + drop, V>(self: &mut TableWithLength<K, V>, key: K): V {
        let val = self.inner.remove(key);
        self.length -= 1;
        val
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K: copy + drop, V>(self: &TableWithLength<K, V>, key: K): bool {
        self.inner.contains(key)
    }

    #[test_only]
    /// Drop table even if not empty, only when testing.
    public fun drop_unchecked<K: copy + drop, V>(self: TableWithLength<K, V>) {
        // Unpack table with length, dropping length count but not
        // inner table.
        let TableWithLength{inner, length: _} = self;
        inner.drop_unchecked(); // Drop inner table.
    }

    #[test]
    /// Verify test-only drop functionality.
    fun test_drop_unchecked() {
        let table = new<bool, bool>(); // Declare new table.
        table.add(true, false); // Add table entry.
        table.drop_unchecked(); // Drop table.
    }

    #[test]
    fun test_upsert() {
        let t = new<u8, u8>();
        // Table should not have key 0 yet
        assert!(!t.contains(0), 1);
        // This should insert key 0, with value 10, and length should be 1
        t.upsert(0, 10);
        // Ensure the value is correctly set to 10
        assert!(*t.borrow(0) == 10, 1);
        // Ensure the length is correctly set
        assert!(t.length() == 1, 1);
        // Lets upsert the value to something else, and verify it's correct
        t.upsert(0, 23);
        assert!(*t.borrow(0) == 23, 1);
        // Since key 0 already existed, the length should not have changed
        assert!(t.length() == 1, 1);
        // If we upsert a non-existing key, the length should increase
        t.upsert(1, 7);
        assert!(t.length() == 2, 1);

        t.remove(0);
        t.remove(1);
        t.destroy_empty();
    }
}
