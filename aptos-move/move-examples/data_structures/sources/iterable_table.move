module aptos_std::iterable_table {
    use std::option::{Self, Option};
    use aptos_std::table_with_length::{Self, TableWithLength};

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
        assert!(table.head.is_none(), 0);
        assert!(table.tail.is_none(), 0);
        let IterableTable {inner, head: _, tail: _} = table;
        inner.destroy_empty();
    }

    /// Add a new entry to the table. Aborts if an entry for this
    /// key already exists.
    public fun add<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K, val: V) {
        let wrapped_value = IterableValue {
            val,
            prev: table.tail,
            next: option::none(),
        };
        table.inner.add(key, wrapped_value);
        if (table.tail.is_some()) {
            let k = table.tail.borrow();
            table.inner.borrow_mut(*k).next = option::some(key);
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
        &table.inner.borrow(key).val
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K): &mut V {
        &mut table.inner.borrow_mut(key).val
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
        table.inner.length()
    }

    /// Returns true if this table is empty.
    public fun empty<K: copy + store + drop, V: store>(table: &IterableTable<K, V>): bool {
        table.inner.empty()
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K: copy + store + drop, V: store>(table: &IterableTable<K, V>, key: K): bool {
        table.inner.contains(key)
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
        let v = table.inner.borrow(key);
        (&v.val, v.prev, v.next)
    }

    /// Acquire a mutable reference to the value and previous/next key which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_iter_mut<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K): (&mut V, Option<K>, Option<K>) {
        let v = table.inner.borrow_mut(key);
        (&mut v.val, v.prev, v.next)
    }

    /// Remove from `table` and return the value and previous/next key which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove_iter<K: copy + store + drop, V: store>(table: &mut IterableTable<K, V>, key: K): (V, Option<K>, Option<K>) {
        let val = table.inner.remove(copy key);
        if (table.tail.contains(&key)) {
            table.tail = val.prev;
        };
        if (table.head.contains(&key)) {
            table.head = val.next;
        };
        if (val.prev.is_some()) {
            let key = val.prev.borrow();
            table.inner.borrow_mut(*key).next = val.next;
        };
        if (val.next.is_some()) {
            let key = val.next.borrow();
            table.inner.borrow_mut(*key).prev = val.prev;
        };
        let IterableValue {val, prev, next} = val;
        (val, prev, next)
    }

    /// Remove all items from v2 and append to v1.
    public fun append<K: copy + store + drop, V: store>(v1: &mut IterableTable<K, V>, v2: &mut IterableTable<K, V>) {
        let key = head_key(v2);
        while (key.is_some()) {
            let (val, _, next) = remove_iter(v2, *key.borrow());
            add(v1, *key.borrow(), val);
            key = next;
        };
    }

    #[test]
    fun iterable_table_test() {
        let table = new();
        let i = 0;
        while (i < 100) {
            add(&mut table, i, i);
            i += 1;
        };
        assert!(length(&table) == 100, 0);
        i = 0;
        while (i < 100) {
            assert!(remove(&mut table, i) == i, 0);
            i += 2;
        };
        assert!(!empty(&table), 0);
        let key = head_key(&table);
        i = 1;
        while (key.is_some()) {
            let (val, _, next) = borrow_iter(&table, *key.borrow());
            assert!(*val == i, 0);
            key = next;
            i += 2;
        };
        assert!(i == 101, 0);
        let table2 = new();
        append(&mut table2, &mut table);
        destroy_empty(table);
        let key = tail_key(&table2);
        while (key.is_some()) {
            let (val, prev, _) = remove_iter(&mut table2, *key.borrow());
            assert!(val == *key.borrow(), 0);
            key = prev;
        };
        destroy_empty(table2);
    }
}
