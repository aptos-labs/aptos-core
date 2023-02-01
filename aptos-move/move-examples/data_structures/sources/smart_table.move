/// A smart table implementation based on linear hashing. (https://en.wikipedia.org/wiki/Linear_hashing)
/// Compare to Table, it uses less storage slots but has higher chance of collision, it's a trade-off between space and time.
/// Compare to other implementation, linear hashing splits one bucket a time instead of doubling buckets when expanding to avoid unexpected gas cost.
/// SmartTable uses faster hash function SipHash instead of cryptographically secure hash functions like sha3-256 since it tolerates collisions.
module aptos_std::smart_table {
    use std::error;
    use std::vector;
    use aptos_std::aptos_hash::sip_hash_from_value;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_std::type_info::size_of_val;
    use aptos_std::math64::max;

    /// Key not found in the smart table
    const ENOT_FOUND: u64 = 1;
    /// Smart table capacity must be larger than 0
    const EZERO_CAPACITY: u64 = 2;
    /// Cannot destroy non-empty hashmap
    const ENOT_EMPTY: u64 = 3;
    /// Key already exists
    const EALREADY_EXIST: u64 = 4;
    /// Invalid load threshold percent to trigger split.
    const EINVALID_LOAD_THRESHOLD_PERCENT: u64 = 5;
    /// Invalid target bucket size.
    const EINVALID_TARGET_BUCKET_SIZE: u64 = 6;
    /// Invariable broken.
    const EINVARIABLE_BROKEN: u64 = 7;

    /// SmartTable entry contains both the key and value.
    struct Entry<K, V> has store {
        hash: u64,
        key: K,
        value: V,
    }

    struct SmartTable<K, V> has store {
        buckets: TableWithLength<u64, vector<Entry<K, V>>>,
        num_buckets: u64,
        // number of bits to represent num_buckets
        level: u8,
        // total number of items
        size: u64,
        // Split will be triggered when target load threshold is reached when adding a new entry. In percent.
        split_load_threshold: u8,
        // The target size of each bucket, which is NOT enforced so oversized buckets can exist.
        bucket_size: u64,
    }

    /// Create an empty SmartTable with `initial_buckets` buckets.
    public fun new<K: copy + drop + store, V: store>(): SmartTable<K, V> {
        new_with_config<K, V>(0, 0, 0)
    }

    /// num_initial_buckets: The number of buckets on initialization. 0 means using default value.
    /// split_load_threshold: The percent number which once reached, split will be triggered. 0 means using default value.
    /// bucket_size: The target number of entries per bucket, though not enforced. 0 means not set and will dynamically
    ///              assgined by the contract code.
    public fun new_with_config<K: copy + drop + store, V: store>(num_initial_buckets: u64, split_load_threshold: u8, bucket_size: u64): SmartTable<K, V> {
        assert!(split_load_threshold <= 100, error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT));
        let buckets = table_with_length::new();
        table_with_length::add(&mut buckets, 0, vector::empty());
        let table = SmartTable {
            buckets,
            num_buckets: 1,
            level: 0,
            size: 0,
            // The default split load threshold is 75%.
            split_load_threshold: if (split_load_threshold == 0) {75} else {split_load_threshold},
            bucket_size,
        };
        // The default number of initial buckets is 2.
        if (num_initial_buckets == 0) {
            num_initial_buckets = 2;
        };
        split(&mut table, num_initial_buckets- 1);
        table
    }

    /// Destroy empty tab-le.
    /// Aborts if it's not empty.
    public fun destroy_empty<K, V>(table: SmartTable<K, V>) {
        assert!(table.size == 0, error::invalid_argument(ENOT_EMPTY));
        let i = 0;
        while (i < table.num_buckets) {
            vector::destroy_empty(table_with_length::remove(&mut table.buckets, i));
            i = i + 1;
        };
        let SmartTable {buckets, num_buckets: _, level: _, size: _, split_load_threshold:_, bucket_size: _} = table;
        table_with_length::destroy_empty(buckets);
    }

    /// Add (key, value) pair in the hash map, it may grow one bucket if current load factor exceeds the threshold.
    /// Note it may not split the actual overflowed bucket.
    /// Abort if `key` already exists.
    public fun add<K, V>(table: &mut SmartTable<K, V>, key: K, value: V) {
        let hash = sip_hash_from_value(&key);
        let index = bucket_index(table.level, table.num_buckets, hash);
        let bucket = table_with_length::borrow_mut(&mut table.buckets, index);
        let i = 0;
        let len = vector::length(bucket);
        while (i < len) {
            let entry = vector::borrow(bucket, i);
            assert!(&entry.key != &key, error::invalid_argument(EALREADY_EXIST));
            i = i + 1;
        };
        let e = Entry {hash, key, value};
        if (table.bucket_size == 0) {
            assert!(table.size == 0, error::internal(EINVARIABLE_BROKEN));
            let estimated_entry_size = max(size_of_val(&e), 1);
            table.bucket_size = max(1024 /* free_write_quota */ / estimated_entry_size, 1);
        };
        vector::push_back(bucket, e);
        table.size = table.size + 1;

        if (load_factor(table) >= (table.split_load_threshold as u64)) {
            split_one_bucket(table);
        }
    }

    /// Split the next bucket into two and re-insert existing items.
    fun split_one_bucket<K, V>(table: &mut SmartTable<K, V>) {
        let new_bucket_index = table.num_buckets;
        // the next bucket to split is num_bucket without the most significant bit.
        let to_split = new_bucket_index ^ (1 << table.level);
        let new_bucket = vector::empty();
        table.num_buckets = new_bucket_index + 1;
        // if the whole level is splitted once, bump the level.
        if (to_split + 1 == 1 << table.level) {
            table.level = table.level + 1;
        };
        let old_bucket = table_with_length::borrow_mut(&mut table.buckets, to_split);
        // partition the bucket. after the loop, i == j and [0..i) stays in old bucket, [j..len) goes to new bucket
        let i = 0;
        let j = vector::length(old_bucket);
        let len = j;
        while (i < j) {
            let entry = vector::borrow(old_bucket, i);
            let index = bucket_index(table.level, table.num_buckets, entry.hash);
            if (index == new_bucket_index) {
                j = j - 1;
                vector::swap(old_bucket, i, j);
            } else {
                i = i + 1;
            };
        };
        while (j < len) {
            let entry = vector::pop_back(old_bucket);
            vector::push_back(&mut new_bucket, entry);
            len = len - 1;
        };
        table_with_length::add(&mut table.buckets, new_bucket_index, new_bucket);
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
    public fun borrow<K: drop, V>(table: &SmartTable<K, V>, key: K): &V {
        let index = bucket_index(table.level, table.num_buckets, sip_hash_from_value(&key));
        let bucket = table_with_length::borrow(&table.buckets, index);
        let i = 0;
        let len = vector::length(bucket);
        while (i < len) {
            let entry = vector::borrow(bucket, i);
            if (&entry.key == &key) {
                return &entry.value
            };
            i = i + 1;
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: drop, V>(table: &mut SmartTable<K, V>, key: K): &mut V {
        let index = bucket_index(table.level, table.num_buckets, sip_hash_from_value(&key));
        let bucket = table_with_length::borrow_mut(&mut table.buckets, index);
        let i = 0;
        let len = vector::length(bucket);
        while (i < len) {
            let entry = vector::borrow_mut(bucket, i);
            if (&entry.key == &key) {
                return &mut entry.value
            };
            i = i + 1;
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K: drop, V>(table: &SmartTable<K, V>, key: K): bool {
        let index = bucket_index(table.level, table.num_buckets, sip_hash_from_value(&key));
        let bucket = table_with_length::borrow(&table.buckets, index);
        let i = 0;
        let len = vector::length(bucket);
        while (i < len) {
            let entry = vector::borrow(bucket, i);
            if (&entry.key == &key) {
                return true
            };
            i = i + 1;
        };
        false
    }

    /// Remove from `table` and return the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: copy + drop, V>(table: &mut SmartTable<K, V>, key: K): V {
        let index = bucket_index(table.level, table.num_buckets, sip_hash_from_value(&key));
        let bucket = table_with_length::borrow_mut(&mut table.buckets, index);
        let i = 0;
        let len = vector::length(bucket);
        while (i < len) {
            let entry = vector::borrow(bucket, i);
            if (&entry.key == &key) {
                let Entry {hash:_, key:_, value} = vector::swap_remove(bucket, i);
                table.size = table.size - 1;
                return value
            };
            i = i + 1;
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Returns the length of the table, i.e. the number of entries.
    public fun length<K, V>(table: &SmartTable<K, V>): u64 {
        table.size
    }

    /// Return the load factor of the hashtable.
    public fun load_factor<K, V>(table: &SmartTable<K, V>): u64 {
        table.size * 100 / (table.num_buckets * table.bucket_size)
    }

    /// Update `split_load_threshold`.
    public fun update_split_load_threshold<K, V>(table: &mut SmartTable<K, V>, split_load_threshold: u8) {
        assert!(split_load_threshold <= 100 && split_load_threshold > 0, error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT));
        table.split_load_threshold = split_load_threshold;
    }

    /// Update `bucket_size`.
    public fun update_bucket_size<K, V>(table: &mut SmartTable<K, V>, bucket_size: u64) {
        assert!(bucket_size > 0, error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT));
        table.bucket_size = bucket_size;
    }

    /// Reserve `additional_buckets` more buckets.
    fun split<K, V>(table: &mut SmartTable<K, V>, additional_buckets: u64) {
        while (additional_buckets > 0) {
            additional_buckets = additional_buckets - 1;
            split_one_bucket(table);
        }
    }

    #[test]
    fun smart_table_test() {
        let table = new();
        let i = 0;
        while (i < 200) {
            add(&mut table, i, i);
            i = i + 1;
        };
        assert!(length(&table) == 200, 0);
        i = 0;
        while (i < 200) {
            *borrow_mut(&mut table, i) = i * 2;
            assert!(*borrow(&mut table, i) == i * 2, 0);
            i = i + 1;
        };
        i = 0;
        assert!(table.num_buckets > 5, table.num_buckets);
        while (i < 200) {
            assert!(contains(&table, i), 0);
            assert!(remove(&mut table, i) == i * 2, 0);
            i = i + 1;
        };
        destroy_empty(table);
    }

    #[test]
    fun smart_table_split_test() {
        let table: SmartTable<u64, u64> = new();
        let i = 2;
        let level = 1;
        while (i <= 256) {
            assert!(table.num_buckets == i, 0);
            assert!(table.level == level, i);
            split_one_bucket(&mut table);
            i = i + 1;
            if (i == 1 << (level + 1)) {
                level = level + 1;
            };
        };
        destroy_empty(table);
    }

    #[test]
    fun smart_table_bucket_index_test() {
        let table: SmartTable<u64, u64> = new_with_config<u64, u64>(8, 75, 0);
        assert!(table.level == 3, 0);
        let i = 0;
        while (i < 4) {
            split_one_bucket(&mut table);
            i = i + 1;
        };
        assert!(table.level == 3, 0);
        assert!(table.num_buckets == 12, 0);
        i = 0;
        while (i < 256) {
            let j = i & 15; // i % 16
            if (j >= table.num_buckets) {
                j = j ^ 8; // i % 8
            };
            let index = bucket_index(table.level, table.num_buckets, i);
            assert!(index == j, 0);
            i = i + 1;
        };
        destroy_empty(table);
    }
}
