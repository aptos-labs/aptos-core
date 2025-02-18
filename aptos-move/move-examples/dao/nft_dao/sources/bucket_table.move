/// This package is copied from move-examples/data_structures/sources/bucket_table.move for demonstrate the full package deployed on-chain
module dao_platform::bucket_table {
    use std::error;
    use std::vector;
    use aptos_std::aptos_hash::sip_hash_from_value;
    use aptos_std::table_with_length::{Self, TableWithLength};

    const TARGET_LOAD_PER_BUCKET: u64 = 10;
    const SPLIT_THRESHOLD: u64 = 75;

    /// Key not found in the bucket table
    const ENOT_FOUND: u64 = 1;
    /// Bucket table capacity must be larger than 0
    const EZERO_CAPACITY: u64 = 2;
    /// Cannot destroy non-empty hashmap
    const ENOT_EMPTY: u64 = 3;
    /// Key already exists
    const EALREADY_EXIST: u64 = 4;

    /// BucketTable entry contains both the key and value.
    struct Entry<K, V> has store {
        hash: u64,
        key: K,
        value: V,
    }

    struct BucketTable<K, V> has store {
        buckets: TableWithLength<u64, vector<Entry<K, V>>>,
        num_buckets: u64,
        // number of bits to represent num_buckets
        level: u8,
        // total number of items
        len: u64,
    }

    /// Create an empty BucketTable with `initial_buckets` buckets.
    public fun new<K: drop + store, V: store>(initial_buckets: u64): BucketTable<K, V> {
        assert!(initial_buckets > 0, error::invalid_argument(EZERO_CAPACITY));
        let buckets = table_with_length::new();
        buckets.add(0, vector::empty());
        let map = BucketTable {
            buckets,
            num_buckets: 1,
            level: 0,
            len: 0,
        };
        split(&mut map, initial_buckets - 1);
        map
    }

    /// Destroy empty map.
    /// Aborts if it's not empty.
    public fun destroy_empty<K, V>(map: BucketTable<K, V>) {
        assert!(map.len == 0, error::invalid_argument(ENOT_EMPTY));
        let i = 0;
        while (i < map.num_buckets) {
            map.buckets.remove(i).destroy_empty();
            i += 1;
        };
        let BucketTable {buckets, num_buckets: _, level: _, len: _} = map;
        buckets.destroy_empty();
    }

    /// Add (key, value) pair in the hash map, it may grow one bucket if current load factor exceeds the threshold.
    /// Note it may not split the actual overflowed bucket.
    /// Abort if `key` already exists.
    public fun add<K, V>(map: &mut BucketTable<K, V>, key: K, value: V) {
        let hash = sip_hash_from_value(&key);
        let index = bucket_index(map.level, map.num_buckets, hash);
        let bucket = map.buckets.borrow_mut(index);
        bucket.for_each_ref(|entry| {
            let entry: &Entry<K, V> = entry;
            assert!(&entry.key != &key, error::invalid_argument(EALREADY_EXIST));
        });
        bucket.push_back(Entry {hash, key, value});
        map.len += 1;

        if (load_factor(map) > SPLIT_THRESHOLD) {
            split_one_bucket(map);
        }
    }

    fun xor(a: u64, b: u64): u64 {
        a ^ b
    }
    spec xor { // TODO: temporary mockup until Prover supports the operator `^`.
        pragma opaque;
        pragma verify = false;
    }

    /// Split the next bucket into two and re-insert existing items.
    fun split_one_bucket<K, V>(map: &mut BucketTable<K, V>) {
        let new_bucket_index = map.num_buckets;
        // the next bucket to split is num_bucket without the most significant bit.
        let to_split = xor(new_bucket_index, (1 << map.level));
        let new_bucket = vector::empty();
        map.num_buckets = new_bucket_index + 1;
        // if the whole level is splitted once, bump the level.
        if (to_split + 1 == 1 << map.level) {
            map.level += 1;
        };
        let old_bucket = map.buckets.borrow_mut(to_split);
        // partition the bucket. after the loop, i == j and [0..i) stays in old bucket, [j..len) goes to new bucket
        let i = 0;
        let j = old_bucket.length();
        let len = j;
        while (i < j) {
            let entry = old_bucket.borrow(i);
            let index = bucket_index(map.level, map.num_buckets, entry.hash);
            if (index == new_bucket_index) {
                j -= 1;
                old_bucket.swap(i, j);
            } else {
                i += 1;
            };
        };
        while (j < len) {
            let entry = old_bucket.pop_back();
            new_bucket.push_back(entry);
            len -= 1;
        };
        map.buckets.add(new_bucket_index, new_bucket);
    }

    /// Return the expected bucket index to find the hash.
    fun bucket_index(level: u8, num_buckets: u64, hash: u64): u64 {
        let index = hash % (1 << (level + 1));
        if (index < num_buckets) {
            // in existing bucket
            index
        } else {
            // in unsplitted bucket
            index % (1 << level)
        }
    }

    /// Acquire an immutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    /// The requirement of &mut BucketTable is to bypass the borrow checker issue described in https://github.com/move-language/move/issues/95
    /// Once Table supports borrow by K, we can remove the &mut
    public fun borrow<K: copy + drop, V>(map: &mut BucketTable<K, V>, key: K): &V {
        let index = bucket_index(map.level, map.num_buckets, sip_hash_from_value(&key));
        let bucket = map.buckets.borrow_mut(index);
        let i = 0;
        let len = bucket.length();
        while (i < len) {
            let entry = bucket.borrow(i);
            if (&entry.key == &key) {
                return &entry.value
            };
            i += 1;
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: copy + drop, V>(map: &mut BucketTable<K, V>, key: K): &mut V {
        let index = bucket_index(map.level, map.num_buckets, sip_hash_from_value(&key));
        let bucket = map.buckets.borrow_mut(index);
        let i = 0;
        let len = bucket.length();
        while (i < len) {
            let entry = bucket.borrow_mut(i);
            if (&entry.key == &key) {
                return &mut entry.value
            };
            i += 1;
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K, V>(map: &BucketTable<K, V>, key: &K): bool {
        let index = bucket_index(map.level, map.num_buckets, sip_hash_from_value(key));
        let bucket = map.buckets.borrow(index);
        bucket.any(|entry| {
            let entry: &Entry<K, V> = entry;
            &entry.key == key
        })
    }

    /// Remove from `table` and return the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: drop, V>(map: &mut BucketTable<K,V>, key: &K): V {
        let index = bucket_index(map.level, map.num_buckets, sip_hash_from_value(key));
        let bucket = map.buckets.borrow_mut(index);
        let i = 0;
        let len = bucket.length();
        while (i < len) {
            let entry = bucket.borrow(i);
            if (&entry.key == key) {
                let Entry {hash:_, key:_, value} = bucket.swap_remove(i);
                map.len -= 1;
                return value
            };
            i += 1;
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Returns the length of the table, i.e. the number of entries.
    public fun length<K, V>(map: &BucketTable<K, V>): u64 {
        map.len
    }

    /// Return the load factor of the hashmap.
    public fun load_factor<K, V>(map: &BucketTable<K, V>): u64 {
        map.len * 100 / (map.num_buckets * TARGET_LOAD_PER_BUCKET)
    }

    /// Reserve `additional_buckets` more buckets.
    public fun split<K, V>(map: &mut BucketTable<K, V>, additional_buckets: u64) {
        while (additional_buckets > 0) {
            additional_buckets -= 1;
            split_one_bucket(map);
        }
    }

    #[test]
    fun hash_map_test() {
        let map = new(1);
        let i = 0;
        while (i < 200) {
            add(&mut map, i, i);
            i += 1;
        };
        assert!(length(&map) == 200, 0);
        i = 0;
        while (i < 200) {
            *borrow_mut(&mut map, i) = i * 2;
            assert!(*borrow(&mut map, i) == i * 2, 0);
            i += 1;
        };
        i = 0;
        assert!(map.num_buckets > 20, map.num_buckets);
        while (i < 200) {
            assert!(contains(&map, &i), 0);
            assert!(remove(&mut map, &i) == i * 2, 0);
            i += 1;
        };
        destroy_empty(map);
    }

    #[test]
    fun hash_map_split_test() {
        let map: BucketTable<u64, u64> = new(1);
        let i = 1;
        let level = 0;
        while (i <= 256) {
            assert!(map.num_buckets == i, 0);
            assert!(map.level == level, i);
            split_one_bucket(&mut map);
            i += 1;
            if (i == 1 << (level + 1)) {
                level += 1;
            };
        };
        destroy_empty(map);
    }

    #[test]
    fun hash_map_bucket_index_test() {
        let map: BucketTable<u64, u64> = new(8);
        assert!(map.level == 3, 0);
        let i = 0;
        while (i < 4) {
            split_one_bucket(&mut map);
            i += 1;
        };
        assert!(map.level == 3, 0);
        assert!(map.num_buckets == 12, 0);
        i = 0;
        while (i < 256) {
            let j = i & 15; // i % 16
            if (j >= map.num_buckets) {
                j = xor(j, 8); // i % 8
            };
            let index = bucket_index(map.level, map.num_buckets, i);
            assert!(index == j, 0);
            i += 1;
        };
        destroy_empty(map);
    }
}
