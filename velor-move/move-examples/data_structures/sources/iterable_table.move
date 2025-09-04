module velor_std::iterable_table {
    use std::option::{Self, Option};
    use velor_std::table_with_length::{Self, TableWithLength};

    /// The iterable wrapper around value, points to previous and next key if any.
    struct IterableValue<K: copy + store + drop, V: store> has store {
        val: V,
        prev: Option<K>,
        next: Option<K>,
    }

    /// An iterable table implementation based on double linked list.
    struct IterableTable<K: copy + store + drop, V: store> has store {
        inner: TableWithLength<K, IterableValue<K, V>>,
        head: Option<K>,
        tail: Option<K>,
    }

    /// Regular table API.

    /// Create an empty table.
    public fun new<K: copy + store + drop, V: store>(): IterableTable<K, V> {
        IterableTable {
            inner: table_with_length::new(),
            head: option::none(),
            tail: option::none(),
        }
    }

    /// Destroy a table. The table must be empty to succeed.
    public fun destroy_empty<K: copy + store + drop, V: store>(table: IterableTable<K, V>) {
        assert!(empty(&table), 0);
        assert!(option::is_none(&table.head), 0);
        assert!(option::is_none(&table.tail), 0);
        let IterableTable {inner, head: _, tail: _} = table;
        table_with_length::destroy_empty(inner);
    }

    /// Add a new entry to the table. Aborts if an entry for this
    /// key already exists.
    public fun add<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K, val: V) {
        let wrapped_value = IterableValue {
            val,
            prev: table.tail,
            next: option::none(),
        };
        table_with_length::add(&mut table.inner, key, wrapped_value);
        if (option::is_some(&table.tail)) {
            let k = option::borrow(&table.tail);
            table_with_length::borrow_mut(&mut table.inner, *k).next = option::some(key);
        } else {
            table.head = option::some(key);
        };
        table.tail = option::some(key);
    }

    /// Remove from `table` and return the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K): V {
        let (val, _, _) = remove_iter(table, key);
        val
    }

    /// Acquire an immutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow<K: copy + store + drop, V: store>(table: &IterableTable<K, V>, key: K): &V {
        &table_with_length::borrow(&table.inner, key).val
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K): &mut V {
        &mut table_with_length::borrow_mut(&mut table.inner, key).val
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Insert the pair (`key`, `default`) first if there is no entry for `key`.
    public fun borrow_mut_with_default<K: copy + store + drop, V: store + drop>(table: &mut IterableTable<K, V>, key: K, default: V): &mut V {
        if (!contains(table, key)) {
            add(table, key, default)
        };
        borrow_mut(table, key)
    }

    /// Returns the length of the table, i.e. the number of entries.
    public fun length<K: copy + store + drop, V: store>(table: &IterableTable<K, V>): u64 {
        table_with_length::length(&table.inner)
    }

    /// Returns true if this table is empty.
    public fun empty<K: copy + store + drop, V: store>(table: &IterableTable<K, V>): bool {
        table_with_length::empty(&table.inner)
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K: copy + store + drop, V: store>(table: &IterableTable<K, V>, key: K): bool {
        table_with_length::contains(&table.inner, key)
    }

    /// Iterable API.

    /// Returns the key of the head for iteration.
    public fun head_key<K: copy + store + drop, V: store>(table: &IterableTable<K, V>): Option<K> {
        table.head
    }

    /// Returns the key of the tail for iteration.
    public fun tail_key<K: copy + store + drop, V: store>(table: &IterableTable<K, V>): Option<K> {
        table.tail
    }

    /// Acquire an immutable reference to the IterableValue which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_iter<K: copy + store + drop, V: store>(table: &IterableTable<K, V>, key: K): (&V, Option<K>, Option<K>) {
        let v = table_with_length::borrow(&table.inner, key);
        (&v.val, v.prev, v.next)
    }

    /// Acquire a mutable reference to the value and previous/next key which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_iter_mut<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K): (&mut V, Option<K>, Option<K>) {
        let v = table_with_length::borrow_mut(&mut table.inner, key);
        (&mut v.val, v.prev, v.next)
    }

    /// Remove from `table` and return the value and previous/next key which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove_iter<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K): (V, Option<K>, Option<K>) {
        let val = table_with_length::remove(&mut table.inner, copy key);
        if (option::contains(&table.tail, &key)) {
            table.tail = val.prev;
        };
        if (option::contains(&table.head, &key)) {
            table.head = val.next;
        };
        if (option::is_some(&val.prev)) {
            let key = option::borrow(&val.prev);
            table_with_length::borrow_mut(&mut table.inner, *key).next = val.next;
        };
        if (option::is_some(&val.next)) {
            let key = option::borrow(&val.next);
            table_with_length::borrow_mut(&mut table.inner, *key).prev = val.prev;
        };
        let IterableValue {val, prev, next} = val;
        (val, prev, next)
    }

    /// Remove all items from v2 and append to v1.
    public fun append<K: copy + store + drop, V: store>(v1: &mut IterableTable<K, V>, v2: &mut IterableTable<K, V>) {
        let key = head_key(v2);
        while (option::is_some(&key)) {
            let (val, _, next) = remove_iter(v2, *option::borrow(&key));
            add(v1, *option::borrow(&key), val);
            key = next;
        };
    }

    #[test]
    fun iterable_table_test() {
        let table = new();
        let i = 0;
        while (i < 100) {
            add(&mut table, i, i);
            i = i + 1;
        };
        assert!(length(&table) == 100, 0);
        i = 0;
        while (i < 100) {
            assert!(remove(&mut table, i) == i, 0);
            i = i + 2;
        };
        assert!(!empty(&table), 0);
        let key = head_key(&table);
        i = 1;
        while (option::is_some(&key)) {
            let (val, _, next) = borrow_iter(&table, *option::borrow(&key));
            assert!(*val == i, 0);
            key = next;
            i = i + 2;
        };
        assert!(i == 101, 0);
        let table2 = new();
        append(&mut table2, &mut table);
        destroy_empty(table);
        let key = tail_key(&table2);
        while (option::is_some(&key)) {
            let (val, prev, _) = remove_iter(&mut table2, *option::borrow(&key));
            assert!(val == *option::borrow(&key), 0);
            key = prev;
        };
        destroy_empty(table2);
    }
}
