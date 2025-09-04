/// A smart table implementation based on linear hashing. (https://en.wikipedia.org/wiki/Linear_hashing)
/// Compare to Table, it uses less storage slots but has higher chance of collision, a trade-off between space and time.
/// Compare to other dynamic hashing implementation, linear hashing splits one bucket a time instead of doubling buckets
/// when expanding to avoid unexpected gas cost.
/// SmartTable uses faster hash function SipHash instead of cryptographically secure hash functions like sha3-256 since
/// it tolerates collisions.
///
/// DEPRECATED: since it's implementation is inneficient, it
/// has been deprecated in favor of `big_ordered_map.move`.
module velor_std::smart_table {
    use std::error;
    use std::vector;
    use velor_std::velor_hash::sip_hash_from_value;
    use velor_std::table_with_length::{Self, TableWithLength};
    use velor_std::type_info::size_of_val;
    use velor_std::math64::max;
    use velor_std::simple_map::SimpleMap;
    use velor_std::simple_map;
    use std::option::{Self, Option};

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
    /// Invalid target bucket size.
    const EEXCEED_MAX_BUCKET_SIZE: u64 = 7;
    /// Invalid bucket index.
    const EINVALID_BUCKET_INDEX: u64 = 8;
    /// Invalid vector index within a bucket.
    const EINVALID_VECTOR_INDEX: u64 = 9;

    /// SmartTable entry contains both the key and value.
    struct Entry<K, V> has copy, drop, store {
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
        // Split will be triggered when target load threshold in percentage is reached when adding a new entry.
        split_load_threshold: u8,
        // The target size of each bucket, which is NOT enforced so oversized buckets can exist.
        target_bucket_size: u64,
    }

    /// Create an empty SmartTable with default configurations.
    public fun new<K: copy + drop + store, V: store>(): SmartTable<K, V> {
        new_with_config<K, V>(0, 0, 0)
    }

    /// Create an empty SmartTable with customized configurations.
    /// `num_initial_buckets`: The number of buckets on initialization. 0 means using default value.
    /// `split_load_threshold`: The percent number which once reached, split will be triggered. 0 means using default
    /// value.
    /// `target_bucket_size`: The target number of entries per bucket, though not guaranteed. 0 means not set and will
    /// dynamically assgined by the contract code.
    public fun new_with_config<K: copy + drop + store, V: store>(
        num_initial_buckets: u64,
        split_load_threshold: u8,
        target_bucket_size: u64
    ): SmartTable<K, V> {
        assert!(split_load_threshold <= 100, error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT));
        let buckets = table_with_length::new();
        buckets.add(0, vector::empty());
        let table = SmartTable {
            buckets,
            num_buckets: 1,
            level: 0,
            size: 0,
            // The default split load threshold is 75%.
            split_load_threshold: if (split_load_threshold == 0) { 75 } else { split_load_threshold },
            target_bucket_size,
        };
        // The default number of initial buckets is 2.
        if (num_initial_buckets == 0) {
            num_initial_buckets = 2;
        };
        while (num_initial_buckets > 1) {
            num_initial_buckets -= 1;
            table.split_one_bucket();
        };
        table
    }

    /// Destroy empty table.
    /// Aborts if it's not empty.
    public fun destroy_empty<K, V>(self: SmartTable<K, V>) {
        assert!(self.size == 0, error::invalid_argument(ENOT_EMPTY));
        for (i in 0..self.num_buckets) {
            self.buckets.remove(i).destroy_empty();
        };
        let SmartTable { buckets, num_buckets: _, level: _, size: _, split_load_threshold: _, target_bucket_size: _ } = self;
        buckets.destroy_empty();
    }

    /// Destroy a table completely when V has `drop`.
    public fun destroy<K: drop, V: drop>(self: SmartTable<K, V>) {
        self.clear();
        self.destroy_empty();
    }

    /// Clear a table completely when T has `drop`.
    public fun clear<K: drop, V: drop>(self: &mut SmartTable<K, V>) {
        *self.buckets.borrow_mut(0) = vector::empty();
        for (i in 1..self.num_buckets) {
            self.buckets.remove(i);
        };
        self.num_buckets = 1;
        self.level = 0;
        self.size = 0;
    }

    /// Add (key, value) pair in the hash map, it may grow one bucket if current load factor exceeds the threshold.
    /// Note it may not split the actual overflowed bucket. Instead, it was determined by `num_buckets` and `level`.
    /// For standard linear hash algorithm, it is stored as a variable but `num_buckets` here could be leveraged.
    /// Abort if `key` already exists.
    /// Note: This method may occasionally cost much more gas when triggering bucket split.
    public fun add<K, V>(self: &mut SmartTable<K, V>, key: K, value: V) {
        let hash = sip_hash_from_value(&key);
        let index = bucket_index(self.level, self.num_buckets, hash);
        let bucket = self.buckets.borrow_mut(index);
        // We set a per-bucket limit here with a upper bound (10000) that nobody should normally reach.
        assert!(bucket.length() <= 10000, error::permission_denied(EEXCEED_MAX_BUCKET_SIZE));
        assert!(bucket.all(| entry | {
            let e: &Entry<K, V> = entry;
            &e.key != &key
        }), error::invalid_argument(EALREADY_EXIST));
        let e = Entry { hash, key, value };
        if (self.target_bucket_size == 0) {
            let estimated_entry_size = max(size_of_val(&e), 1);
            self.target_bucket_size = max(1024 /* free_write_quota */ / estimated_entry_size, 1);
        };
        bucket.push_back(e);
        self.size += 1;

        if (self.load_factor() >= (self.split_load_threshold as u64)) {
            self.split_one_bucket();
        }
    }

    /// Add multiple key/value pairs to the smart table. The keys must not already exist.
    public fun add_all<K, V>(self: &mut SmartTable<K, V>, keys: vector<K>, values: vector<V>) {
        keys.zip(values, |key, value| { self.add(key, value); });
    }

    inline fun unzip_entries<K: copy, V: copy>(entries: &vector<Entry<K, V>>): (vector<K>, vector<V>) {
        let keys = vector[];
        let values = vector[];
        entries.for_each_ref(|e|{
            let entry: &Entry<K, V> = e;
            keys.push_back(entry.key);
            values.push_back(entry.value);
        });
        (keys, values)
    }

    /// Convert a smart table to a simple_map, which is supposed to be called mostly by view functions to get an atomic
    /// view of the whole table.
    /// Disclaimer: This function may be costly as the smart table may be huge in size. Use it at your own discretion.
    public fun to_simple_map<K: store + copy + drop, V: store + copy>(
        self: &SmartTable<K, V>,
    ): SimpleMap<K, V> {
        let res = simple_map::new<K, V>();
        for (i in 0..self.num_buckets) {
            let (keys, values) = unzip_entries(self.buckets.borrow(i));
            res.add_all(keys, values);
        };
        res
    }

    /// Get all keys in a smart table.
    ///
    /// For a large enough smart table this function will fail due to execution gas limits, and
    /// `keys_paginated` should be used instead.
    public fun keys<K: store + copy + drop, V: store + copy>(
        self: &SmartTable<K, V>
    ): vector<K> {
        let (keys, _, _) = self.keys_paginated(0, 0, self.length());
        keys
    }

    /// Get keys from a smart table, paginated.
    ///
    /// This function can be used to paginate all keys in a large smart table outside of runtime,
    /// e.g. through chained view function calls. The maximum `num_keys_to_get` before hitting gas
    /// limits depends on the data types in the smart table.
    ///
    /// When starting pagination, pass `starting_bucket_index` = `starting_vector_index` = 0.
    ///
    /// The function will then return a vector of keys, an optional bucket index, and an optional
    /// vector index. The unpacked return indices can then be used as inputs to another pagination
    /// call, which will return a vector of more keys. This process can be repeated until the
    /// returned bucket index and vector index value options are both none, which means that
    /// pagination is complete. For an example, see `test_keys()`.
    public fun keys_paginated<K: store + copy + drop, V: store + copy>(
        self: &SmartTable<K, V>,
        starting_bucket_index: u64,
        starting_vector_index: u64,
        num_keys_to_get: u64,
    ): (
        vector<K>,
        Option<u64>,
        Option<u64>,
    ) {
        let num_buckets = self.num_buckets;
        let buckets_ref = &self.buckets;
        assert!(starting_bucket_index < num_buckets, EINVALID_BUCKET_INDEX);
        let bucket_ref = buckets_ref.borrow(starting_bucket_index);
        let bucket_length = bucket_ref.length();
        assert!(
            // In the general case, starting vector index should never be equal to bucket length
            // because then iteration will attempt to borrow a vector element that is out of bounds.
            // However starting vector index can be equal to bucket length in the special case of
            // starting iteration at the beginning of an empty bucket since buckets are never
            // destroyed, only emptied.
            starting_vector_index < bucket_length || starting_vector_index == 0,
            EINVALID_VECTOR_INDEX
        );
        let keys = vector[];
        if (num_keys_to_get == 0) return
            (keys, option::some(starting_bucket_index), option::some(starting_vector_index));
        for (bucket_index in starting_bucket_index..num_buckets) {
            bucket_ref = buckets_ref.borrow(bucket_index);
            bucket_length = bucket_ref.length();
            for (vector_index in starting_vector_index..bucket_length) {
                keys.push_back(bucket_ref.borrow(vector_index).key);
                num_keys_to_get -= 1;
                if (num_keys_to_get == 0) {
                    vector_index += 1;
                    return if (vector_index == bucket_length) {
                        bucket_index += 1;
                        if (bucket_index < num_buckets) {
                            (keys, option::some(bucket_index), option::some(0))
                        } else {
                            (keys, option::none(), option::none())
                        }
                    } else {
                        (keys, option::some(bucket_index), option::some(vector_index))
                    }
                };
            };
            starting_vector_index = 0; // Start parsing the next bucket at vector index 0.
        };
        (keys, option::none(), option::none())
    }

    /// Decide which is the next bucket to split and split it into two with the elements inside the bucket.
    fun split_one_bucket<K, V>(self: &mut SmartTable<K, V>) {
        let new_bucket_index = self.num_buckets;
        // the next bucket to split is num_bucket without the most significant bit.
        let to_split = new_bucket_index ^ (1 << self.level);
        self.num_buckets = new_bucket_index + 1;
        // if the whole level is splitted once, bump the level.
        if (to_split + 1 == 1 << self.level) {
            self.level += 1;
        };
        let old_bucket = self.buckets.borrow_mut(to_split);
        // partition the bucket, [0..p) stays in old bucket, [p..len) goes to new bucket
        let p = old_bucket.partition(|e| {
            let entry: &Entry<K, V> = e; // Explicit type to satisfy compiler
            bucket_index(self.level, self.num_buckets, entry.hash) != new_bucket_index
        });
        let new_bucket = old_bucket.trim_reverse(p);
        self.buckets.add(new_bucket_index, new_bucket);
    }

    /// Return the expected bucket index to find the hash.
    /// Basically, it use different base `1 << level` vs `1 << (level + 1)` in modulo operation based on the target
    /// bucket index compared to the index of the next bucket to split.
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
    public fun borrow<K: drop, V>(self: &SmartTable<K, V>, key: K): &V {
        let index = bucket_index(self.level, self.num_buckets, sip_hash_from_value(&key));
        let bucket = self.buckets.borrow(index);
        let len = bucket.length();
        for (i in 0..len) {
            let entry = bucket.borrow(i);
            if (&entry.key == &key) {
                return &entry.value
            };
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Acquire an immutable reference to the value which `key` maps to.
    /// Returns specified default value if there is no entry for `key`.
    public fun borrow_with_default<K: copy + drop, V>(self: &SmartTable<K, V>, key: K, default: &V): &V {
        if (!self.contains(copy key)) {
            default
        } else {
            self.borrow(copy key)
        }
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun borrow_mut<K: drop, V>(self: &mut SmartTable<K, V>, key: K): &mut V {
        let index = bucket_index(self.level, self.num_buckets, sip_hash_from_value(&key));
        let bucket = self.buckets.borrow_mut(index);
        let len = bucket.length();
        for (i in 0..len) {
            let entry = bucket.borrow_mut(i);
            if (&entry.key == &key) {
                return &mut entry.value
            };
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Acquire a mutable reference to the value which `key` maps to.
    /// Insert the pair (`key`, `default`) first if there is no entry for `key`.
    public fun borrow_mut_with_default<K: copy + drop, V: drop>(
        self: &mut SmartTable<K, V>,
        key: K,
        default: V
    ): &mut V {
        if (!self.contains(copy key)) {
            self.add(copy key, default)
        };
        self.borrow_mut(key)
    }

    /// Returns true iff `table` contains an entry for `key`.
    public fun contains<K: drop, V>(self: &SmartTable<K, V>, key: K): bool {
        let hash = sip_hash_from_value(&key);
        let index = bucket_index(self.level, self.num_buckets, hash);
        let bucket = self.buckets.borrow(index);
        bucket.any(| e | {
            e.hash == hash && &e.key == &key
        })
    }

    /// Remove from `table` and return the value which `key` maps to.
    /// Aborts if there is no entry for `key`.
    public fun remove<K: copy + drop, V>(self: &mut SmartTable<K, V>, key: K): V {
        let index = bucket_index(self.level, self.num_buckets, sip_hash_from_value(&key));
        let bucket = self.buckets.borrow_mut(index);
        let len = bucket.length();
        for (i in 0..len) {
            let entry = bucket.borrow(i);
            if (&entry.key == &key) {
                let Entry { hash: _, key: _, value } = bucket.swap_remove(i);
                self.size -= 1;
                return value
            };
        };
        abort error::invalid_argument(ENOT_FOUND)
    }

    /// Insert the pair (`key`, `value`) if there is no entry for `key`.
    /// update the value of the entry for `key` to `value` otherwise
    public fun upsert<K: copy + drop, V: drop>(self: &mut SmartTable<K, V>, key: K, value: V) {
        if (!self.contains(copy key)) {
            self.add(copy key, value)
        } else {
            let ref = self.borrow_mut(key);
            *ref = value;
        };
    }

    /// Returns the length of the table, i.e. the number of entries.
    public fun length<K, V>(self: &SmartTable<K, V>): u64 {
        self.size
    }

    /// Return the load factor of the hashtable.
    public fun load_factor<K, V>(self: &SmartTable<K, V>): u64 {
        self.size * 100 / self.num_buckets / self.target_bucket_size
    }

    /// Update `split_load_threshold`.
    public fun update_split_load_threshold<K, V>(self: &mut SmartTable<K, V>, split_load_threshold: u8) {
        assert!(
            split_load_threshold <= 100 && split_load_threshold > 0,
            error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT)
        );
        self.split_load_threshold = split_load_threshold;
    }

    /// Update `target_bucket_size`.
    public fun update_target_bucket_size<K, V>(self: &mut SmartTable<K, V>, target_bucket_size: u64) {
        assert!(target_bucket_size > 0, error::invalid_argument(EINVALID_TARGET_BUCKET_SIZE));
        self.target_bucket_size = target_bucket_size;
    }

    /// Apply the function to a reference of each key-value pair in the table.
    public inline fun for_each_ref<K, V>(self: &SmartTable<K, V>, f: |&K, &V|) {
        for (i in 0..self.num_buckets()) {
            self.borrow_buckets().borrow(i).for_each_ref(|elem| {
                let (key, value) = elem.borrow_kv();
                f(key, value)
            });
        }
    }

    /// Apply the function to a mutable reference of each key-value pair in the table.
    public inline fun for_each_mut<K, V>(self: &mut SmartTable<K, V>, f: |&K, &mut V|) {
        for (i in 0..self.num_buckets()) {
            self.borrow_buckets_mut().borrow_mut(i).for_each_mut(|elem| {
                let (key, value) = elem.borrow_kv_mut();
                f(key, value)
            });
        };
    }

    /// Map the function over the references of key-value pairs in the table without modifying it.
    public inline fun map_ref<K: copy + drop + store, V1, V2: store>(
        self: &SmartTable<K, V1>,
        f: |&V1|V2
    ): SmartTable<K, V2> {
        let new_table = new<K, V2>();
        self.for_each_ref(|key, value| new_table.add(*key, f(value)));
        new_table
    }

    /// Return true if any key-value pair in the table satisfies the predicate.
    public inline fun any<K, V>(
        self: &SmartTable<K, V>,
        p: |&K, &V|bool
    ): bool {
        let found = false;
        for (i in 0..self.num_buckets()) {
            found = self.borrow_buckets().borrow(i).any(|elem| {
                let (key, value) = elem.borrow_kv();
                p(key, value)
            });
            if (found) break;
        };
        found
    }

    // Helper functions to circumvent the scope issue of inline functions.
    public fun borrow_kv<K, V>(self: &Entry<K, V>): (&K, &V) {
        (&self.key, &self.value)
    }

    public fun borrow_kv_mut<K, V>(self: &mut Entry<K, V>): (&mut K, &mut V) {
        (&mut self.key, &mut self.value)
    }

    public fun num_buckets<K, V>(self: &SmartTable<K, V>): u64 {
        self.num_buckets
    }

    public fun borrow_buckets<K, V>(self: &SmartTable<K, V>): &TableWithLength<u64, vector<Entry<K, V>>> {
        &self.buckets
    }

    public fun borrow_buckets_mut<K, V>(self: &mut SmartTable<K, V>): &mut TableWithLength<u64, vector<Entry<K, V>>> {
        &mut self.buckets
    }


    #[test]
    fun smart_table_test() {
        let table = new();
        let i = 0;
        while (i < 200) {
            table.add(i, i);
            i += 1;
        };
        assert!(table.length() == 200, 0);
        i = 0;
        while (i < 200) {
            *table.borrow_mut(i) = i * 2;
            assert!(*table.borrow(i) == i * 2, 0);
            i += 1;
        };
        i = 0;
        assert!(table.num_buckets > 5, table.num_buckets);
        while (i < 200) {
            assert!(table.contains(i), 0);
            assert!(table.remove(i) == i * 2, 0);
            i += 1;
        };
        table.destroy_empty();
    }

    #[test]
    fun smart_table_split_test() {
        let table: SmartTable<u64, u64> = new_with_config(1, 100, 1);
        let i = 1;
        let level = 0;
        while (i <= 256) {
            assert!(table.num_buckets == i, 0);
            assert!(table.level == level, i);
            table.add(i, i);
            i += 1;
            if (i == 1 << (level + 1)) {
                level += 1;
            };
        };
        let i = 1;
        while (i <= 256) {
            assert!(*table.borrow(i) == i, 0);
            i += 1;
        };
        assert!(table.num_buckets == 257, table.num_buckets);
        assert!(table.load_factor() == 99, 0);
        assert!(table.length() == 256, 0);
        table.destroy();
    }

    #[test]
    fun smart_table_update_configs() {
        let table = new();
        let i = 0;
        while (i < 200) {
            table.add(i, i);
            i += 1;
        };
        assert!(table.length() == 200, 0);
        table.update_target_bucket_size(10);
        table.update_split_load_threshold(50);
        while (i < 400) {
            table.add(i, i);
            i += 1;
        };
        assert!(table.length() == 400, 0);
        i = 0;
        while (i < 400) {
            assert!(table.contains(i), 0);
            assert!(table.remove(i) == i, 0);
            i += 1;
        };
        table.destroy_empty();
    }

    #[test]
    public fun smart_table_add_all_test() {
        let table: SmartTable<u64, u64> = new_with_config(1, 100, 2);
        assert!(table.length() == 0, 0);
        table.add_all(vector[1, 2, 3, 4, 5, 6, 7], vector[1, 2, 3, 4, 5, 6, 7]);
        assert!(table.length() == 7, 1);
        let i = 1;
        while (i < 8) {
            assert!(*table.borrow(i) == i, 0);
            i += 1;
        };
        i -= 1;
        while (i > 0) {
            table.remove(i);
            i -= 1;
        };
        table.destroy_empty();
    }

    #[test]
    public fun smart_table_to_simple_map_test() {
        let table = new();
        let i = 0;
        while (i < 200) {
            table.add(i, i);
            i += 1;
        };
        let map = table.to_simple_map();
        assert!(map.length() == 200, 0);
        table.destroy();
    }

    #[test]
    public fun smart_table_clear_test() {
        let table = new();
        let i = 0u64;
        while (i < 200) {
            table.add(i, i);
            i += 1;
        };
        table.clear();
        let i = 0;
        while (i < 200) {
            table.add(i, i);
            i += 1;
        };
        assert!(table.size == 200, 0);
        table.destroy();
    }

    #[test]
    fun test_keys() {
        let i = 0;
        let table = new();
        let expected_keys = vector[];
        let keys = table.keys();
        assert!(keys.is_empty(), 0);
        let starting_bucket_index = 0;
        let starting_vector_index = 0;
        let (keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(
            starting_bucket_index,
            starting_vector_index,
            0
        );
        assert!(starting_bucket_index_r == option::some(starting_bucket_index), 0);
        assert!(starting_vector_index_r == option::some(starting_vector_index), 0);
        assert!(keys.is_empty(), 0);
        while (i < 100) {
            table.add(i, 0);
            expected_keys.push_back(i);
            i += 1;
        };
        let keys = table.keys();
        assert!(keys.length() == expected_keys.length(), 0);
        keys.for_each_ref(|e_ref| {
            assert!(expected_keys.contains(e_ref), 0);
        });
        let keys = vector[];
        let starting_bucket_index = 0;
        let starting_vector_index = 0;
        let returned_keys = vector[];
        returned_keys.length(); // To eliminate erroneous compiler "unused" warning
        loop {
            (returned_keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(
                starting_bucket_index,
                starting_vector_index,
                15
            );
            keys.append(returned_keys);
            if (
                starting_bucket_index_r == option::none() ||
                starting_vector_index_r == option::none()
            ) break;
            starting_bucket_index = starting_bucket_index_r.destroy_some();
            starting_vector_index = starting_vector_index_r.destroy_some();
        };
        assert!(keys.length() == expected_keys.length(), 0);
        keys.for_each_ref(|e_ref| {
            assert!(expected_keys.contains(e_ref), 0);
        });
        table.destroy();
        table = new();
        table.add(1, 0);
        table.add(2, 0);
        (keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(0, 0, 1);
        (returned_keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(
            starting_bucket_index_r.destroy_some(),
            starting_vector_index_r.destroy_some(),
            1
        );
        keys.append(returned_keys);
        assert!(keys == vector[1, 2] || keys == vector[2, 1], 0);
        assert!(starting_bucket_index_r == option::none(), 0);
        assert!(starting_vector_index_r == option::none(), 0);
        (keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(0, 0, 0);
        assert!(keys == vector[], 0);
        assert!(starting_bucket_index_r == option::some(0), 0);
        assert!(starting_vector_index_r == option::some(0), 0);
        table.destroy();
    }

    #[test]
    fun test_keys_corner_cases() {
        let table = new();
        let expected_keys = vector[];
        for (i in 0..100) {
            table.add(i, 0);
            expected_keys.push_back(i);
        };
        let (keys, starting_bucket_index_r, starting_vector_index_r) =
            table.keys_paginated(0, 0, 5); // Both indices 0.
        assert!(keys.length() == 5, 0);
        keys.for_each_ref(|e_ref| {
            assert!(expected_keys.contains(e_ref), 0);
        });
        let starting_bucket_index = starting_bucket_index_r.destroy_some();
        let starting_vector_index = starting_vector_index_r.destroy_some();
        (keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(
            starting_bucket_index,
            starting_vector_index,
            0
        );
        assert!(keys == vector[], 0);
        assert!(starting_bucket_index_r == option::some(starting_bucket_index), 0);
        assert!(starting_vector_index_r == option::some(starting_vector_index), 0);
        (keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(starting_bucket_index, 0, 50);
        assert!(keys.length() == 50, 0);
        keys.for_each_ref(|e_ref| {
            assert!(expected_keys.contains(e_ref), 0);
        });
        let starting_bucket_index = starting_bucket_index_r.destroy_some();
        assert!(starting_bucket_index > 0, 0);
        assert!(starting_vector_index_r.is_some(), 0);
        (keys, starting_bucket_index_r, starting_vector_index_r) = table.keys_paginated(0, 1, 50);
        assert!(keys.length() == 50, 0);
        keys.for_each_ref(|e_ref| {
            assert!(expected_keys.contains(e_ref), 0);
        });
        assert!(starting_bucket_index_r.is_some(), 0);
        assert!(starting_vector_index_r.is_some(), 0);
        table.destroy();
    }

    #[test, expected_failure(abort_code = EINVALID_BUCKET_INDEX)]
    fun test_keys_invalid_bucket_index() {
        let table = new<u64, u64>();
        table.add(1, 0);
        let num_buckets = table.num_buckets;
        table.keys_paginated(num_buckets + 1, 0, 1);
        table.destroy();
    }

    #[test, expected_failure(abort_code = EINVALID_VECTOR_INDEX)]
    fun test_keys_invalid_vector_index() {
        let table = new();
        table.add(1, 0);
        table.keys_paginated(0, 1, 1);
        table.destroy();
    }
}
