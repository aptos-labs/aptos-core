
<a id="0x1_smart_table"></a>

# Module `0x1::smart_table`

A smart table implementation based on linear hashing. (https://en.wikipedia.org/wiki/Linear_hashing)
Compare to Table, it uses less storage slots but has higher chance of collision, a trade-off between space and time.
Compare to other dynamic hashing implementation, linear hashing splits one bucket a time instead of doubling buckets
when expanding to avoid unexpected gas cost.
SmartTable uses faster hash function SipHash instead of cryptographically secure hash functions like sha3-256 since
it tolerates collisions.


-  [Struct `Entry`](#0x1_smart_table_Entry)
-  [Struct `SmartTable`](#0x1_smart_table_SmartTable)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_smart_table_new)
-  [Function `new_with_config`](#0x1_smart_table_new_with_config)
-  [Function `destroy_empty`](#0x1_smart_table_destroy_empty)
-  [Function `destroy`](#0x1_smart_table_destroy)
-  [Function `clear`](#0x1_smart_table_clear)
-  [Function `add`](#0x1_smart_table_add)
-  [Function `add_all`](#0x1_smart_table_add_all)
-  [Function `unzip_entries`](#0x1_smart_table_unzip_entries)
-  [Function `to_simple_map`](#0x1_smart_table_to_simple_map)
-  [Function `keys`](#0x1_smart_table_keys)
-  [Function `keys_paginated`](#0x1_smart_table_keys_paginated)
-  [Function `split_one_bucket`](#0x1_smart_table_split_one_bucket)
-  [Function `bucket_index`](#0x1_smart_table_bucket_index)
-  [Function `borrow`](#0x1_smart_table_borrow)
-  [Function `borrow_with_default`](#0x1_smart_table_borrow_with_default)
-  [Function `borrow_mut`](#0x1_smart_table_borrow_mut)
-  [Function `borrow_mut_with_default`](#0x1_smart_table_borrow_mut_with_default)
-  [Function `contains`](#0x1_smart_table_contains)
-  [Function `remove`](#0x1_smart_table_remove)
-  [Function `upsert`](#0x1_smart_table_upsert)
-  [Function `length`](#0x1_smart_table_length)
-  [Function `load_factor`](#0x1_smart_table_load_factor)
-  [Function `update_split_load_threshold`](#0x1_smart_table_update_split_load_threshold)
-  [Function `update_target_bucket_size`](#0x1_smart_table_update_target_bucket_size)
-  [Function `for_each_ref`](#0x1_smart_table_for_each_ref)
-  [Function `for_each_mut`](#0x1_smart_table_for_each_mut)
-  [Function `map_ref`](#0x1_smart_table_map_ref)
-  [Function `any`](#0x1_smart_table_any)
-  [Function `borrow_kv`](#0x1_smart_table_borrow_kv)
-  [Function `borrow_kv_mut`](#0x1_smart_table_borrow_kv_mut)
-  [Function `num_buckets`](#0x1_smart_table_num_buckets)
-  [Function `borrow_buckets`](#0x1_smart_table_borrow_buckets)
-  [Function `borrow_buckets_mut`](#0x1_smart_table_borrow_buckets_mut)
-  [Specification](#@Specification_1)
    -  [Struct `SmartTable`](#@Specification_1_SmartTable)
    -  [Function `new_with_config`](#@Specification_1_new_with_config)
    -  [Function `destroy`](#@Specification_1_destroy)
    -  [Function `clear`](#@Specification_1_clear)
    -  [Function `add_all`](#@Specification_1_add_all)
    -  [Function `to_simple_map`](#@Specification_1_to_simple_map)
    -  [Function `keys`](#@Specification_1_keys)
    -  [Function `keys_paginated`](#@Specification_1_keys_paginated)
    -  [Function `split_one_bucket`](#@Specification_1_split_one_bucket)
    -  [Function `bucket_index`](#@Specification_1_bucket_index)
    -  [Function `borrow_with_default`](#@Specification_1_borrow_with_default)
    -  [Function `load_factor`](#@Specification_1_load_factor)
    -  [Function `update_split_load_threshold`](#@Specification_1_update_split_load_threshold)
    -  [Function `update_target_bucket_size`](#@Specification_1_update_target_bucket_size)
    -  [Function `borrow_kv`](#@Specification_1_borrow_kv)
    -  [Function `borrow_kv_mut`](#@Specification_1_borrow_kv_mut)
    -  [Function `num_buckets`](#@Specification_1_num_buckets)
    -  [Function `borrow_buckets`](#@Specification_1_borrow_buckets)
    -  [Function `borrow_buckets_mut`](#@Specification_1_borrow_buckets_mut)


<pre><code>use 0x1::aptos_hash;
use 0x1::error;
use 0x1::math64;
use 0x1::option;
use 0x1::simple_map;
use 0x1::table_with_length;
use 0x1::type_info;
use 0x1::vector;
</code></pre>



<a id="0x1_smart_table_Entry"></a>

## Struct `Entry`

SmartTable entry contains both the key and value.


<pre><code>struct Entry&lt;K, V&gt; has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hash: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>key: K</code>
</dt>
<dd>

</dd>
<dt>
<code>value: V</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_smart_table_SmartTable"></a>

## Struct `SmartTable`



<pre><code>struct SmartTable&lt;K, V&gt; has store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>buckets: table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_buckets: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>level: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>split_load_threshold: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>target_bucket_size: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_smart_table_ENOT_EMPTY"></a>

Cannot destroy non-empty hashmap


<pre><code>const ENOT_EMPTY: u64 &#61; 3;
</code></pre>



<a id="0x1_smart_table_ENOT_FOUND"></a>

Key not found in the smart table


<pre><code>const ENOT_FOUND: u64 &#61; 1;
</code></pre>



<a id="0x1_smart_table_EALREADY_EXIST"></a>

Key already exists


<pre><code>const EALREADY_EXIST: u64 &#61; 4;
</code></pre>



<a id="0x1_smart_table_EEXCEED_MAX_BUCKET_SIZE"></a>

Invalid target bucket size.


<pre><code>const EEXCEED_MAX_BUCKET_SIZE: u64 &#61; 7;
</code></pre>



<a id="0x1_smart_table_EINVALID_BUCKET_INDEX"></a>

Invalid bucket index.


<pre><code>const EINVALID_BUCKET_INDEX: u64 &#61; 8;
</code></pre>



<a id="0x1_smart_table_EINVALID_LOAD_THRESHOLD_PERCENT"></a>

Invalid load threshold percent to trigger split.


<pre><code>const EINVALID_LOAD_THRESHOLD_PERCENT: u64 &#61; 5;
</code></pre>



<a id="0x1_smart_table_EINVALID_TARGET_BUCKET_SIZE"></a>

Invalid target bucket size.


<pre><code>const EINVALID_TARGET_BUCKET_SIZE: u64 &#61; 6;
</code></pre>



<a id="0x1_smart_table_EINVALID_VECTOR_INDEX"></a>

Invalid vector index within a bucket.


<pre><code>const EINVALID_VECTOR_INDEX: u64 &#61; 9;
</code></pre>



<a id="0x1_smart_table_EZERO_CAPACITY"></a>

Smart table capacity must be larger than 0


<pre><code>const EZERO_CAPACITY: u64 &#61; 2;
</code></pre>



<a id="0x1_smart_table_new"></a>

## Function `new`

Create an empty SmartTable with default configurations.


<pre><code>public fun new&lt;K: copy, drop, store, V: store&gt;(): smart_table::SmartTable&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new&lt;K: copy &#43; drop &#43; store, V: store&gt;(): SmartTable&lt;K, V&gt; &#123;
    new_with_config&lt;K, V&gt;(0, 0, 0)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_new_with_config"></a>

## Function `new_with_config`

Create an empty SmartTable with customized configurations.
<code>num_initial_buckets</code>: The number of buckets on initialization. 0 means using default value.
<code>split_load_threshold</code>: The percent number which once reached, split will be triggered. 0 means using default
value.
<code>target_bucket_size</code>: The target number of entries per bucket, though not guaranteed. 0 means not set and will
dynamically assgined by the contract code.


<pre><code>public fun new_with_config&lt;K: copy, drop, store, V: store&gt;(num_initial_buckets: u64, split_load_threshold: u8, target_bucket_size: u64): smart_table::SmartTable&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_with_config&lt;K: copy &#43; drop &#43; store, V: store&gt;(
    num_initial_buckets: u64,
    split_load_threshold: u8,
    target_bucket_size: u64
): SmartTable&lt;K, V&gt; &#123;
    assert!(split_load_threshold &lt;&#61; 100, error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT));
    let buckets &#61; table_with_length::new();
    table_with_length::add(&amp;mut buckets, 0, vector::empty());
    let table &#61; SmartTable &#123;
        buckets,
        num_buckets: 1,
        level: 0,
        size: 0,
        // The default split load threshold is 75%.
        split_load_threshold: if (split_load_threshold &#61;&#61; 0) &#123; 75 &#125; else &#123; split_load_threshold &#125;,
        target_bucket_size,
    &#125;;
    // The default number of initial buckets is 2.
    if (num_initial_buckets &#61;&#61; 0) &#123;
        num_initial_buckets &#61; 2;
    &#125;;
    while (num_initial_buckets &gt; 1) &#123;
        num_initial_buckets &#61; num_initial_buckets &#45; 1;
        split_one_bucket(&amp;mut table);
    &#125;;
    table
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_destroy_empty"></a>

## Function `destroy_empty`

Destroy empty table.
Aborts if it's not empty.


<pre><code>public fun destroy_empty&lt;K, V&gt;(table: smart_table::SmartTable&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;K, V&gt;(table: SmartTable&lt;K, V&gt;) &#123;
    assert!(table.size &#61;&#61; 0, error::invalid_argument(ENOT_EMPTY));
    let i &#61; 0;
    while (i &lt; table.num_buckets) &#123;
        vector::destroy_empty(table_with_length::remove(&amp;mut table.buckets, i));
        i &#61; i &#43; 1;
    &#125;;
    let SmartTable &#123; buckets, num_buckets: _, level: _, size: _, split_load_threshold: _, target_bucket_size: _ &#125; &#61; table;
    table_with_length::destroy_empty(buckets);
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_destroy"></a>

## Function `destroy`

Destroy a table completely when V has <code>drop</code>.


<pre><code>public fun destroy&lt;K: drop, V: drop&gt;(table: smart_table::SmartTable&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy&lt;K: drop, V: drop&gt;(table: SmartTable&lt;K, V&gt;) &#123;
    clear(&amp;mut table);
    destroy_empty(table);
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_clear"></a>

## Function `clear`

Clear a table completely when T has <code>drop</code>.


<pre><code>public fun clear&lt;K: drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun clear&lt;K: drop, V: drop&gt;(table: &amp;mut SmartTable&lt;K, V&gt;) &#123;
    &#42;table_with_length::borrow_mut(&amp;mut table.buckets, 0) &#61; vector::empty();
    let i &#61; 1;
    while (i &lt; table.num_buckets) &#123;
        table_with_length::remove(&amp;mut table.buckets, i);
        i &#61; i &#43; 1;
    &#125;;
    table.num_buckets &#61; 1;
    table.level &#61; 0;
    table.size &#61; 0;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_add"></a>

## Function `add`

Add (key, value) pair in the hash map, it may grow one bucket if current load factor exceeds the threshold.
Note it may not split the actual overflowed bucket. Instead, it was determined by <code>num_buckets</code> and <code>level</code>.
For standard linear hash algorithm, it is stored as a variable but <code>num_buckets</code> here could be leveraged.
Abort if <code>key</code> already exists.
Note: This method may occasionally cost much more gas when triggering bucket split.


<pre><code>public fun add&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K, value: V) &#123;
    let hash &#61; sip_hash_from_value(&amp;key);
    let index &#61; bucket_index(table.level, table.num_buckets, hash);
    let bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, index);
    // We set a per&#45;bucket limit here with a upper bound (10000) that nobody should normally reach.
    assert!(vector::length(bucket) &lt;&#61; 10000, error::permission_denied(EEXCEED_MAX_BUCKET_SIZE));
    assert!(vector::all(bucket, &#124; entry &#124; &#123;
        let e: &amp;Entry&lt;K, V&gt; &#61; entry;
        &amp;e.key !&#61; &amp;key
    &#125;), error::invalid_argument(EALREADY_EXIST));
    let e &#61; Entry &#123; hash, key, value &#125;;
    if (table.target_bucket_size &#61;&#61; 0) &#123;
        let estimated_entry_size &#61; max(size_of_val(&amp;e), 1);
        table.target_bucket_size &#61; max(1024 /&#42; free_write_quota &#42;/ / estimated_entry_size, 1);
    &#125;;
    vector::push_back(bucket, e);
    table.size &#61; table.size &#43; 1;

    if (load_factor(table) &gt;&#61; (table.split_load_threshold as u64)) &#123;
        split_one_bucket(table);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the smart table. The keys must not already exist.


<pre><code>public fun add_all&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, keys: vector&lt;K&gt;, values: vector&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_all&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, keys: vector&lt;K&gt;, values: vector&lt;V&gt;) &#123;
    vector::zip(keys, values, &#124;key, value&#124; &#123; add(table, key, value); &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_unzip_entries"></a>

## Function `unzip_entries`



<pre><code>fun unzip_entries&lt;K: copy, V: copy&gt;(entries: &amp;vector&lt;smart_table::Entry&lt;K, V&gt;&gt;): (vector&lt;K&gt;, vector&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun unzip_entries&lt;K: copy, V: copy&gt;(entries: &amp;vector&lt;Entry&lt;K, V&gt;&gt;): (vector&lt;K&gt;, vector&lt;V&gt;) &#123;
    let keys &#61; vector[];
    let values &#61; vector[];
    vector::for_each_ref(entries, &#124;e&#124;&#123;
        let entry: &amp;Entry&lt;K, V&gt; &#61; e;
        vector::push_back(&amp;mut keys, entry.key);
        vector::push_back(&amp;mut values, entry.value);
    &#125;);
    (keys, values)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_to_simple_map"></a>

## Function `to_simple_map`

Convert a smart table to a simple_map, which is supposed to be called mostly by view functions to get an atomic
view of the whole table.
Disclaimer: This function may be costly as the smart table may be huge in size. Use it at your own discretion.


<pre><code>public fun to_simple_map&lt;K: copy, drop, store, V: copy, store&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): simple_map::SimpleMap&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_simple_map&lt;K: store &#43; copy &#43; drop, V: store &#43; copy&gt;(
    table: &amp;SmartTable&lt;K, V&gt;,
): SimpleMap&lt;K, V&gt; &#123;
    let i &#61; 0;
    let res &#61; simple_map::new&lt;K, V&gt;();
    while (i &lt; table.num_buckets) &#123;
        let (keys, values) &#61; unzip_entries(table_with_length::borrow(&amp;table.buckets, i));
        simple_map::add_all(&amp;mut res, keys, values);
        i &#61; i &#43; 1;
    &#125;;
    res
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_keys"></a>

## Function `keys`

Get all keys in a smart table.

For a large enough smart table this function will fail due to execution gas limits, and
<code>keys_paginated</code> should be used instead.


<pre><code>public fun keys&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;): vector&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keys&lt;K: store &#43; copy &#43; drop, V: store &#43; copy&gt;(
    table_ref: &amp;SmartTable&lt;K, V&gt;
): vector&lt;K&gt; &#123;
    let (keys, _, _) &#61; keys_paginated(table_ref, 0, 0, length(table_ref));
    keys
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_keys_paginated"></a>

## Function `keys_paginated`

Get keys from a smart table, paginated.

This function can be used to paginate all keys in a large smart table outside of runtime,
e.g. through chained view function calls. The maximum <code>num_keys_to_get</code> before hitting gas
limits depends on the data types in the smart table.

When starting pagination, pass <code>starting_bucket_index</code> = <code>starting_vector_index</code> = 0.

The function will then return a vector of keys, an optional bucket index, and an optional
vector index. The unpacked return indices can then be used as inputs to another pagination
call, which will return a vector of more keys. This process can be repeated until the
returned bucket index and vector index value options are both none, which means that
pagination is complete. For an example, see <code>test_keys()</code>.


<pre><code>public fun keys_paginated&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;, starting_bucket_index: u64, starting_vector_index: u64, num_keys_to_get: u64): (vector&lt;K&gt;, option::Option&lt;u64&gt;, option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keys_paginated&lt;K: store &#43; copy &#43; drop, V: store &#43; copy&gt;(
    table_ref: &amp;SmartTable&lt;K, V&gt;,
    starting_bucket_index: u64,
    starting_vector_index: u64,
    num_keys_to_get: u64,
): (
    vector&lt;K&gt;,
    Option&lt;u64&gt;,
    Option&lt;u64&gt;,
) &#123;
    let num_buckets &#61; table_ref.num_buckets;
    let buckets_ref &#61; &amp;table_ref.buckets;
    assert!(starting_bucket_index &lt; num_buckets, EINVALID_BUCKET_INDEX);
    let bucket_ref &#61; table_with_length::borrow(buckets_ref, starting_bucket_index);
    let bucket_length &#61; vector::length(bucket_ref);
    assert!(
        // In the general case, starting vector index should never be equal to bucket length
        // because then iteration will attempt to borrow a vector element that is out of bounds.
        // However starting vector index can be equal to bucket length in the special case of
        // starting iteration at the beginning of an empty bucket since buckets are never
        // destroyed, only emptied.
        starting_vector_index &lt; bucket_length &#124;&#124; starting_vector_index &#61;&#61; 0,
        EINVALID_VECTOR_INDEX
    );
    let keys &#61; vector[];
    if (num_keys_to_get &#61;&#61; 0) return
        (keys, option::some(starting_bucket_index), option::some(starting_vector_index));
    for (bucket_index in starting_bucket_index..num_buckets) &#123;
        bucket_ref &#61; table_with_length::borrow(buckets_ref, bucket_index);
        bucket_length &#61; vector::length(bucket_ref);
        for (vector_index in starting_vector_index..bucket_length) &#123;
            vector::push_back(&amp;mut keys, vector::borrow(bucket_ref, vector_index).key);
            num_keys_to_get &#61; num_keys_to_get &#45; 1;
            if (num_keys_to_get &#61;&#61; 0) &#123;
                vector_index &#61; vector_index &#43; 1;
                return if (vector_index &#61;&#61; bucket_length) &#123;
                    bucket_index &#61; bucket_index &#43; 1;
                    if (bucket_index &lt; num_buckets) &#123;
                        (keys, option::some(bucket_index), option::some(0))
                    &#125; else &#123;
                        (keys, option::none(), option::none())
                    &#125;
                &#125; else &#123;
                    (keys, option::some(bucket_index), option::some(vector_index))
                &#125;
            &#125;;
        &#125;;
        starting_vector_index &#61; 0; // Start parsing the next bucket at vector index 0.
    &#125;;
    (keys, option::none(), option::none())
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_split_one_bucket"></a>

## Function `split_one_bucket`

Decide which is the next bucket to split and split it into two with the elements inside the bucket.


<pre><code>fun split_one_bucket&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun split_one_bucket&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;) &#123;
    let new_bucket_index &#61; table.num_buckets;
    // the next bucket to split is num_bucket without the most significant bit.
    let to_split &#61; new_bucket_index ^ (1 &lt;&lt; table.level);
    table.num_buckets &#61; new_bucket_index &#43; 1;
    // if the whole level is splitted once, bump the level.
    if (to_split &#43; 1 &#61;&#61; 1 &lt;&lt; table.level) &#123;
        table.level &#61; table.level &#43; 1;
    &#125;;
    let old_bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, to_split);
    // partition the bucket, [0..p) stays in old bucket, [p..len) goes to new bucket
    let p &#61; vector::partition(old_bucket, &#124;e&#124; &#123;
        let entry: &amp;Entry&lt;K, V&gt; &#61; e; // Explicit type to satisfy compiler
        bucket_index(table.level, table.num_buckets, entry.hash) !&#61; new_bucket_index
    &#125;);
    let new_bucket &#61; vector::trim_reverse(old_bucket, p);
    table_with_length::add(&amp;mut table.buckets, new_bucket_index, new_bucket);
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_bucket_index"></a>

## Function `bucket_index`

Return the expected bucket index to find the hash.
Basically, it use different base <code>1 &lt;&lt; level</code> vs <code>1 &lt;&lt; (level &#43; 1)</code> in modulo operation based on the target
bucket index compared to the index of the next bucket to split.


<pre><code>fun bucket_index(level: u8, num_buckets: u64, hash: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun bucket_index(level: u8, num_buckets: u64, hash: u64): u64 &#123;
    let index &#61; hash % (1 &lt;&lt; (level &#43; 1));
    if (index &lt; num_buckets) &#123;
        // in existing bucket
        index
    &#125; else &#123;
        // in unsplitted bucket
        index % (1 &lt;&lt; level)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow&lt;K: drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K): &amp;V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;K: drop, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, key: K): &amp;V &#123;
    let index &#61; bucket_index(table.level, table.num_buckets, sip_hash_from_value(&amp;key));
    let bucket &#61; table_with_length::borrow(&amp;table.buckets, index);
    let i &#61; 0;
    let len &#61; vector::length(bucket);
    while (i &lt; len) &#123;
        let entry &#61; vector::borrow(bucket, i);
        if (&amp;entry.key &#61;&#61; &amp;key) &#123;
            return &amp;entry.value
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    abort error::invalid_argument(ENOT_FOUND)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow_with_default"></a>

## Function `borrow_with_default`

Acquire an immutable reference to the value which <code>key</code> maps to.
Returns specified default value if there is no entry for <code>key</code>.


<pre><code>public fun borrow_with_default&lt;K: copy, drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K, default: &amp;V): &amp;V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_with_default&lt;K: copy &#43; drop, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, key: K, default: &amp;V): &amp;V &#123;
    if (!contains(table, copy key)) &#123;
        default
    &#125; else &#123;
        borrow(table, copy key)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut&lt;K: drop, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K): &amp;mut V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;K: drop, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K): &amp;mut V &#123;
    let index &#61; bucket_index(table.level, table.num_buckets, sip_hash_from_value(&amp;key));
    let bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, index);
    let i &#61; 0;
    let len &#61; vector::length(bucket);
    while (i &lt; len) &#123;
        let entry &#61; vector::borrow_mut(bucket, i);
        if (&amp;entry.key &#61;&#61; &amp;key) &#123;
            return &amp;mut entry.value
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    abort error::invalid_argument(ENOT_FOUND)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.
Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut_with_default&lt;K: copy, drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K, default: V): &amp;mut V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut_with_default&lt;K: copy &#43; drop, V: drop&gt;(
    table: &amp;mut SmartTable&lt;K, V&gt;,
    key: K,
    default: V
): &amp;mut V &#123;
    if (!contains(table, copy key)) &#123;
        add(table, copy key, default)
    &#125;;
    borrow_mut(table, key)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_contains"></a>

## Function `contains`

Returns true iff <code>table</code> contains an entry for <code>key</code>.


<pre><code>public fun contains&lt;K: drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;K: drop, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, key: K): bool &#123;
    let hash &#61; sip_hash_from_value(&amp;key);
    let index &#61; bucket_index(table.level, table.num_buckets, hash);
    let bucket &#61; table_with_length::borrow(&amp;table.buckets, index);
    vector::any(bucket, &#124; entry &#124; &#123;
        let e: &amp;Entry&lt;K, V&gt; &#61; entry;
        e.hash &#61;&#61; hash &amp;&amp; &amp;e.key &#61;&#61; &amp;key
    &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_remove"></a>

## Function `remove`

Remove from <code>table</code> and return the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code>public fun remove&lt;K: copy, drop, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;K: copy &#43; drop, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K): V &#123;
    let index &#61; bucket_index(table.level, table.num_buckets, sip_hash_from_value(&amp;key));
    let bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, index);
    let i &#61; 0;
    let len &#61; vector::length(bucket);
    while (i &lt; len) &#123;
        let entry &#61; vector::borrow(bucket, i);
        if (&amp;entry.key &#61;&#61; &amp;key) &#123;
            let Entry &#123; hash: _, key: _, value &#125; &#61; vector::swap_remove(bucket, i);
            table.size &#61; table.size &#45; 1;
            return value
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    abort error::invalid_argument(ENOT_FOUND)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_upsert"></a>

## Function `upsert`

Insert the pair (<code>key</code>, <code>value</code>) if there is no entry for <code>key</code>.
update the value of the entry for <code>key</code> to <code>value</code> otherwise


<pre><code>public fun upsert&lt;K: copy, drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert&lt;K: copy &#43; drop, V: drop&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K, value: V) &#123;
    if (!contains(table, copy key)) &#123;
        add(table, copy key, value)
    &#125; else &#123;
        let ref &#61; borrow_mut(table, key);
        &#42;ref &#61; value;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_length"></a>

## Function `length`

Returns the length of the table, i.e. the number of entries.


<pre><code>public fun length&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): u64 &#123;
    table.size
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_load_factor"></a>

## Function `load_factor`

Return the load factor of the hashtable.


<pre><code>public fun load_factor&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun load_factor&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): u64 &#123;
    table.size &#42; 100 / table.num_buckets / table.target_bucket_size
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_update_split_load_threshold"></a>

## Function `update_split_load_threshold`

Update <code>split_load_threshold</code>.


<pre><code>public fun update_split_load_threshold&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, split_load_threshold: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_split_load_threshold&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, split_load_threshold: u8) &#123;
    assert!(
        split_load_threshold &lt;&#61; 100 &amp;&amp; split_load_threshold &gt; 0,
        error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT)
    );
    table.split_load_threshold &#61; split_load_threshold;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_update_target_bucket_size"></a>

## Function `update_target_bucket_size`

Update <code>target_bucket_size</code>.


<pre><code>public fun update_target_bucket_size&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, target_bucket_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_target_bucket_size&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, target_bucket_size: u64) &#123;
    assert!(target_bucket_size &gt; 0, error::invalid_argument(EINVALID_TARGET_BUCKET_SIZE));
    table.target_bucket_size &#61; target_bucket_size;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each key-value pair in the table.


<pre><code>public fun for_each_ref&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, f: &#124;(&amp;K, &amp;V)&#124;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_ref&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, f: &#124;&amp;K, &amp;V&#124;) &#123;
    let i &#61; 0;
    while (i &lt; aptos_std::smart_table::num_buckets(table)) &#123;
        vector::for_each_ref(
            aptos_std::table_with_length::borrow(aptos_std::smart_table::borrow_buckets(table), i),
            &#124;elem&#124; &#123;
                let (key, value) &#61; aptos_std::smart_table::borrow_kv(elem);
                f(key, value)
            &#125;
        );
        i &#61; i &#43; 1;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference of each key-value pair in the table.


<pre><code>public fun for_each_mut&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, f: &#124;(&amp;K, &amp;mut V)&#124;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_mut&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, f: &#124;&amp;K, &amp;mut V&#124;) &#123;
    let i &#61; 0;
    while (i &lt; aptos_std::smart_table::num_buckets(table)) &#123;
        vector::for_each_mut(
            table_with_length::borrow_mut(aptos_std::smart_table::borrow_buckets_mut(table), i),
            &#124;elem&#124; &#123;
                let (key, value) &#61; aptos_std::smart_table::borrow_kv_mut(elem);
                f(key, value)
            &#125;
        );
        i &#61; i &#43; 1;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_map_ref"></a>

## Function `map_ref`

Map the function over the references of key-value pairs in the table without modifying it.


<pre><code>public fun map_ref&lt;K: copy, drop, store, V1, V2: store&gt;(table: &amp;smart_table::SmartTable&lt;K, V1&gt;, f: &#124;&amp;V1&#124;V2): smart_table::SmartTable&lt;K, V2&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map_ref&lt;K: copy &#43; drop &#43; store, V1, V2: store&gt;(
    table: &amp;SmartTable&lt;K, V1&gt;,
    f: &#124;&amp;V1&#124;V2
): SmartTable&lt;K, V2&gt; &#123;
    let new_table &#61; new&lt;K, V2&gt;();
    for_each_ref(table, &#124;key, value&#124; add(&amp;mut new_table, &#42;key, f(value)));
    new_table
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_any"></a>

## Function `any`

Return true if any key-value pair in the table satisfies the predicate.


<pre><code>public fun any&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, p: &#124;(&amp;K, &amp;V)&#124;bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun any&lt;K, V&gt;(
    table: &amp;SmartTable&lt;K, V&gt;,
    p: &#124;&amp;K, &amp;V&#124;bool
): bool &#123;
    let found &#61; false;
    let i &#61; 0;
    while (i &lt; aptos_std::smart_table::num_buckets(table)) &#123;
        found &#61; vector::any(table_with_length::borrow(aptos_std::smart_table::borrow_buckets(table), i), &#124;elem&#124; &#123;
            let (key, value) &#61; aptos_std::smart_table::borrow_kv(elem);
            p(key, value)
        &#125;);
        if (found) break;
        i &#61; i &#43; 1;
    &#125;;
    found
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow_kv"></a>

## Function `borrow_kv`



<pre><code>public fun borrow_kv&lt;K, V&gt;(e: &amp;smart_table::Entry&lt;K, V&gt;): (&amp;K, &amp;V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_kv&lt;K, V&gt;(e: &amp;Entry&lt;K, V&gt;): (&amp;K, &amp;V) &#123;
    (&amp;e.key, &amp;e.value)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow_kv_mut"></a>

## Function `borrow_kv_mut`



<pre><code>public fun borrow_kv_mut&lt;K, V&gt;(e: &amp;mut smart_table::Entry&lt;K, V&gt;): (&amp;mut K, &amp;mut V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_kv_mut&lt;K, V&gt;(e: &amp;mut Entry&lt;K, V&gt;): (&amp;mut K, &amp;mut V) &#123;
    (&amp;mut e.key, &amp;mut e.value)
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_num_buckets"></a>

## Function `num_buckets`



<pre><code>public fun num_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun num_buckets&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): u64 &#123;
    table.num_buckets
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow_buckets"></a>

## Function `borrow_buckets`



<pre><code>public fun borrow_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): &amp;table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_buckets&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): &amp;TableWithLength&lt;u64, vector&lt;Entry&lt;K, V&gt;&gt;&gt; &#123;
    &amp;table.buckets
&#125;
</code></pre>



</details>

<a id="0x1_smart_table_borrow_buckets_mut"></a>

## Function `borrow_buckets_mut`



<pre><code>public fun borrow_buckets_mut&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;): &amp;mut table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_buckets_mut&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;): &amp;mut TableWithLength&lt;u64, vector&lt;Entry&lt;K, V&gt;&gt;&gt; &#123;
    &amp;mut table.buckets
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SmartTable"></a>

### Struct `SmartTable`


<pre><code>struct SmartTable&lt;K, V&gt; has store
</code></pre>



<dl>
<dt>
<code>buckets: table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_buckets: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>level: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>split_load_threshold: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>target_bucket_size: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>pragma intrinsic &#61; map,
    map_new &#61; new,
    map_destroy_empty &#61; destroy_empty,
    map_len &#61; length,
    map_has_key &#61; contains,
    map_add_no_override &#61; add,
    map_add_override_if_exists &#61; upsert,
    map_del_must_exist &#61; remove,
    map_borrow &#61; borrow,
    map_borrow_mut &#61; borrow_mut,
    map_borrow_mut_with_default &#61; borrow_mut_with_default,
    map_spec_get &#61; spec_get,
    map_spec_set &#61; spec_set,
    map_spec_del &#61; spec_remove,
    map_spec_len &#61; spec_len,
map_spec_has_key &#61; spec_contains;
</code></pre>



<a id="@Specification_1_new_with_config"></a>

### Function `new_with_config`


<pre><code>public fun new_with_config&lt;K: copy, drop, store, V: store&gt;(num_initial_buckets: u64, split_load_threshold: u8, target_bucket_size: u64): smart_table::SmartTable&lt;K, V&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code>public fun destroy&lt;K: drop, V: drop&gt;(table: smart_table::SmartTable&lt;K, V&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_clear"></a>

### Function `clear`


<pre><code>public fun clear&lt;K: drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_add_all"></a>

### Function `add_all`


<pre><code>public fun add_all&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, keys: vector&lt;K&gt;, values: vector&lt;V&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_to_simple_map"></a>

### Function `to_simple_map`


<pre><code>public fun to_simple_map&lt;K: copy, drop, store, V: copy, store&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): simple_map::SimpleMap&lt;K, V&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code>public fun keys&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;): vector&lt;K&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_keys_paginated"></a>

### Function `keys_paginated`


<pre><code>public fun keys_paginated&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;, starting_bucket_index: u64, starting_vector_index: u64, num_keys_to_get: u64): (vector&lt;K&gt;, option::Option&lt;u64&gt;, option::Option&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_split_one_bucket"></a>

### Function `split_one_bucket`


<pre><code>fun split_one_bucket&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_bucket_index"></a>

### Function `bucket_index`


<pre><code>fun bucket_index(level: u8, num_buckets: u64, hash: u64): u64
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_borrow_with_default"></a>

### Function `borrow_with_default`


<pre><code>public fun borrow_with_default&lt;K: copy, drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K, default: &amp;V): &amp;V
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_load_factor"></a>

### Function `load_factor`


<pre><code>public fun load_factor&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_update_split_load_threshold"></a>

### Function `update_split_load_threshold`


<pre><code>public fun update_split_load_threshold&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, split_load_threshold: u8)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_update_target_bucket_size"></a>

### Function `update_target_bucket_size`


<pre><code>public fun update_target_bucket_size&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, target_bucket_size: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_borrow_kv"></a>

### Function `borrow_kv`


<pre><code>public fun borrow_kv&lt;K, V&gt;(e: &amp;smart_table::Entry&lt;K, V&gt;): (&amp;K, &amp;V)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_borrow_kv_mut"></a>

### Function `borrow_kv_mut`


<pre><code>public fun borrow_kv_mut&lt;K, V&gt;(e: &amp;mut smart_table::Entry&lt;K, V&gt;): (&amp;mut K, &amp;mut V)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_num_buckets"></a>

### Function `num_buckets`


<pre><code>public fun num_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_borrow_buckets"></a>

### Function `borrow_buckets`


<pre><code>public fun borrow_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): &amp;table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_borrow_buckets_mut"></a>

### Function `borrow_buckets_mut`


<pre><code>public fun borrow_buckets_mut&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;): &amp;mut table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>




<a id="0x1_smart_table_spec_len"></a>


<pre><code>native fun spec_len&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;): num;
</code></pre>




<a id="0x1_smart_table_spec_contains"></a>


<pre><code>native fun spec_contains&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K): bool;
</code></pre>




<a id="0x1_smart_table_spec_set"></a>


<pre><code>native fun spec_set&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K, v: V): SmartTable&lt;K, V&gt;;
</code></pre>




<a id="0x1_smart_table_spec_remove"></a>


<pre><code>native fun spec_remove&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K): SmartTable&lt;K, V&gt;;
</code></pre>




<a id="0x1_smart_table_spec_get"></a>


<pre><code>native fun spec_get&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K): V;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
