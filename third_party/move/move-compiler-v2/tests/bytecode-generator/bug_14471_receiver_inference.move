module 0x815::m {

    struct ValueWrap has store, drop {
        val: u64,
    }
    struct Table<T1, T2> has store {
        x: T1,
        y: T2
    }
    fun add<T1:drop, T2:drop>(self: &mut Table<T1, T2>, _key: T1, _val: T2) {
    }
    fun contains<T1:drop, T2:drop>(self: &Table<T1, T2>, _key: T1): bool {
        true
    }

    struct MyMap has key {
        table: Table<address, ValueWrap>,
    }

    public fun add_when_missing(key: address, val:u64) acquires MyMap {
        let my_map = borrow_global_mut<MyMap>(@0x815);
        if (!contains(&my_map.table, key))
        {
            let wrap = ValueWrap { val };
            my_map.table.add(key, wrap);
        }
    }
}
