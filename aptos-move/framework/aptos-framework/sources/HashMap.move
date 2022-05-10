module AptosFramework::HashMap {
    use Std::Errors;
    use Std::Vector;
    use Std::Hash::sip_hash;
    use AptosFramework::Table::{Self, Table};

    /// Not found in the table;
    const ENOT_FOUND: u64 = 0;

    /// HashMap entry contains both the key and value.
    struct Entry<K, V> has store {
        key: K,
        value: V,
    }

    /// A simple hashmap implementation based on separate chaining. Compare to Table, it uses less storage slots
    /// but has higher chance of collision, it's a trade-off between space and time.
    struct HashMap<K, V> has store {
        buckets: Table<u64, vector<Entry<K, V>>>,
        num_buckets: u64,
        len: u64,
    }

    /// Create an empty HashMap with `initial_buckets`, it's important to pick the proper number of buckets
    /// since resize is a every expensive operation.
    public fun new<K: drop + store, V: store>(initial_buckets: u64): HashMap<K, V> {
        // assert it's power of 2
        assert!(initial_buckets > 0 && (initial_buckets & (initial_buckets - 1) == 0), 0);
        let buckets = Table::new();
        let i = 0;
        while (i < initial_buckets) {
            Table::add(&mut buckets, &i, Vector::empty());
            i = i + 1;
        };
        HashMap {
            buckets,
            num_buckets: initial_buckets,
            len: 0,
        }
    }

    /// Destroy empty map.
    /// Aborts if it's not empty.
    public fun destroy_empty<K, V>(map: HashMap<K, V>) {
        assert!(map.len == 0, 0);
        let i = 0;
        while (i < map.num_buckets) {
            Vector::destroy_empty(Table::remove(&mut map.buckets, &i));
            i = i + 1;
        };
        let HashMap {buckets, num_buckets: _, len: _} = map;
        Table::destroy_empty(buckets);
    }

    /// Insert (key, value) pair in the hash map.
    /// Abort if `key` already exists.
    public fun insert<K, V>(map: &mut HashMap<K, V>, key: K, value: V) {
        let hash = sip_hash(&key) % map.num_buckets;
        let bucket = Table::borrow_mut(&mut map.buckets, &hash);
        let i = 0;
        let len = Vector::length(bucket);
        while (i < len) {
            let entry = Vector::borrow(bucket, i);
            assert!(&entry.key != &key, 0);
            i = i + 1;
        };
        Vector::push_back(bucket, Entry {key, value});
        map.len = map.len + 1;
    }

    /// Acquire an immutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    /// The requirement of &mut HashMap is to bypass the borrow checker issue described in https://github.com/move-language/move/issues/95
    public fun borrow<K, V>(map: &mut HashMap<K, V>, key: &K): &V {
        let hash = sip_hash(key) % map.num_buckets;
        let bucket = Table::borrow_mut(&mut map.buckets, &hash);
        let i = 0;
        let len = Vector::length(bucket);
        while (i < len) {
            let entry = Vector::borrow(bucket, i);
            if (&entry.key == key) {
                return &entry.value
            };
            i = i + 1;
        };
        abort Errors::invalid_argument(ENOT_FOUND)
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K, V>(map: &mut HashMap<K, V>, key: &K): &mut V {
        let hash = sip_hash(key) % map.num_buckets;
        let bucket = Table::borrow_mut(&mut map.buckets, &hash);
        let i = 0;
        let len = Vector::length(bucket);
        while (i < len) {
            let entry = Vector::borrow_mut(bucket, i);
            if (&entry.key == key) {
                return &mut entry.value
            };
            i = i + 1;
        };
        abort Errors::invalid_argument(ENOT_FOUND)
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K, V>(map: &HashMap<K, V>, key: &K): bool {
        let hash = sip_hash(key) % map.num_buckets;
        let bucket = Table::borrow(&map.buckets, &hash);
        let i = 0;
        let len = Vector::length(bucket);
        while (i < len) {
            let entry = Vector::borrow(bucket, i);
            if (&entry.key == key) {
                return true
            };
            i = i + 1;
        };
        false
    }

    /// Remove from `table` and return the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: drop, V>(map: &mut HashMap<K,V>, key: &K): V {
        let hash = sip_hash(key) % map.num_buckets;
        let bucket = Table::borrow_mut(&mut map.buckets, &hash);
        let i = 0;
        let len = Vector::length(bucket);
        while (i < len) {
            let entry = Vector::borrow(bucket, i);
            if (&entry.key == key) {
                let Entry {key:_, value} = Vector::swap_remove(bucket, i);
                map.len = map.len - 1;
                return value
            };
            i = i + 1;
        };
        abort Errors::invalid_argument(ENOT_FOUND)
    }

    /// Returns the length of the table, i.e. the number of entries.
    public fun length<K, V>(map: &HashMap<K, V>): u64 {
        map.len
    }

    /// Expand the number of buckets by 2, rehash all existing keys into new buckets, this is an expensive operation.
    public fun resize<K, V>(map: &mut HashMap<K, V>) {
        let num_buckets = map.num_buckets;
        map.num_buckets = map.num_buckets << 1;
        let next_num_buckets = map.num_buckets;
        let i = 0;
        let current_buckets = Vector::empty();
        while (i < next_num_buckets) {
            if (i < num_buckets) {
                let bucket = Table::remove(&mut map.buckets, &i);
                Vector::push_back(&mut current_buckets, bucket);
            };
            Table::add(&mut map.buckets, &i, Vector::empty());
            i = i + 1;
        };
        map.len = 0;
        while (!Vector::is_empty(&current_buckets)) {
            let bucket = Vector::pop_back(&mut current_buckets);
            while (!Vector::is_empty(&bucket)) {
                let Entry {key, value} = Vector::pop_back(&mut bucket);
                insert(map, key, value);
            };
            Vector::destroy_empty(bucket);
        };
        Vector::destroy_empty(current_buckets);
    }

    #[test]
    fun hash_map_test() {
        let map = new(8);
        let i = 0;
        while (i < 100) {
            insert(&mut map, i, i);
            i = i + 1;
        };
        assert!(length(&map) == 100, 0);
        i = 0;
        while (i < 100) {
            *borrow_mut(&mut map, &i) = i * 2;
            assert!(*borrow(&mut map, &i) == i * 2, 0);
            i = i + 1;
        };
        resize(&mut map);
        assert!(map.num_buckets == 16, 0);
        i = 0;
        while (i < 100) {
            assert!(contains(&map, &i), 0);
            assert!(remove(&mut map, &i) == i * 2, 0);
            i = i + 1;
        };
        destroy_empty(map);
    }
}
