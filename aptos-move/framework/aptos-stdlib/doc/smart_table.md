
<a id="0x1_smart_table"></a>

# Module `0x1::smart_table`

A smart table implementation based on linear hashing. (https://en.wikipedia.org/wiki/Linear_hashing)<br/> Compare to Table, it uses less storage slots but has higher chance of collision, a trade&#45;off between space and time.<br/> Compare to other dynamic hashing implementation, linear hashing splits one bucket a time instead of doubling buckets<br/> when expanding to avoid unexpected gas cost.<br/> SmartTable uses faster hash function SipHash instead of cryptographically secure hash functions like sha3&#45;256 since<br/> it tolerates collisions.


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


<pre><code>use 0x1::aptos_hash;<br/>use 0x1::error;<br/>use 0x1::math64;<br/>use 0x1::option;<br/>use 0x1::simple_map;<br/>use 0x1::table_with_length;<br/>use 0x1::type_info;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_smart_table_Entry"></a>

## Struct `Entry`

SmartTable entry contains both the key and value.


<pre><code>struct Entry&lt;K, V&gt; has copy, drop, store<br/></code></pre>



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



<pre><code>struct SmartTable&lt;K, V&gt; has store<br/></code></pre>



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

Cannot destroy non&#45;empty hashmap


<pre><code>const ENOT_EMPTY: u64 &#61; 3;<br/></code></pre>



<a id="0x1_smart_table_ENOT_FOUND"></a>

Key not found in the smart table


<pre><code>const ENOT_FOUND: u64 &#61; 1;<br/></code></pre>



<a id="0x1_smart_table_EALREADY_EXIST"></a>

Key already exists


<pre><code>const EALREADY_EXIST: u64 &#61; 4;<br/></code></pre>



<a id="0x1_smart_table_EEXCEED_MAX_BUCKET_SIZE"></a>

Invalid target bucket size.


<pre><code>const EEXCEED_MAX_BUCKET_SIZE: u64 &#61; 7;<br/></code></pre>



<a id="0x1_smart_table_EINVALID_BUCKET_INDEX"></a>

Invalid bucket index.


<pre><code>const EINVALID_BUCKET_INDEX: u64 &#61; 8;<br/></code></pre>



<a id="0x1_smart_table_EINVALID_LOAD_THRESHOLD_PERCENT"></a>

Invalid load threshold percent to trigger split.


<pre><code>const EINVALID_LOAD_THRESHOLD_PERCENT: u64 &#61; 5;<br/></code></pre>



<a id="0x1_smart_table_EINVALID_TARGET_BUCKET_SIZE"></a>

Invalid target bucket size.


<pre><code>const EINVALID_TARGET_BUCKET_SIZE: u64 &#61; 6;<br/></code></pre>



<a id="0x1_smart_table_EINVALID_VECTOR_INDEX"></a>

Invalid vector index within a bucket.


<pre><code>const EINVALID_VECTOR_INDEX: u64 &#61; 9;<br/></code></pre>



<a id="0x1_smart_table_EZERO_CAPACITY"></a>

Smart table capacity must be larger than 0


<pre><code>const EZERO_CAPACITY: u64 &#61; 2;<br/></code></pre>



<a id="0x1_smart_table_new"></a>

## Function `new`

Create an empty SmartTable with default configurations.


<pre><code>public fun new&lt;K: copy, drop, store, V: store&gt;(): smart_table::SmartTable&lt;K, V&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new&lt;K: copy &#43; drop &#43; store, V: store&gt;(): SmartTable&lt;K, V&gt; &#123;<br/>    new_with_config&lt;K, V&gt;(0, 0, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_new_with_config"></a>

## Function `new_with_config`

Create an empty SmartTable with customized configurations.<br/> <code>num_initial_buckets</code>: The number of buckets on initialization. 0 means using default value.<br/> <code>split_load_threshold</code>: The percent number which once reached, split will be triggered. 0 means using default<br/> value.<br/> <code>target_bucket_size</code>: The target number of entries per bucket, though not guaranteed. 0 means not set and will<br/> dynamically assgined by the contract code.


<pre><code>public fun new_with_config&lt;K: copy, drop, store, V: store&gt;(num_initial_buckets: u64, split_load_threshold: u8, target_bucket_size: u64): smart_table::SmartTable&lt;K, V&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_with_config&lt;K: copy &#43; drop &#43; store, V: store&gt;(<br/>    num_initial_buckets: u64,<br/>    split_load_threshold: u8,<br/>    target_bucket_size: u64<br/>): SmartTable&lt;K, V&gt; &#123;<br/>    assert!(split_load_threshold &lt;&#61; 100, error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT));<br/>    let buckets &#61; table_with_length::new();<br/>    table_with_length::add(&amp;mut buckets, 0, vector::empty());<br/>    let table &#61; SmartTable &#123;<br/>        buckets,<br/>        num_buckets: 1,<br/>        level: 0,<br/>        size: 0,<br/>        // The default split load threshold is 75%.<br/>        split_load_threshold: if (split_load_threshold &#61;&#61; 0) &#123; 75 &#125; else &#123; split_load_threshold &#125;,<br/>        target_bucket_size,<br/>    &#125;;<br/>    // The default number of initial buckets is 2.<br/>    if (num_initial_buckets &#61;&#61; 0) &#123;<br/>        num_initial_buckets &#61; 2;<br/>    &#125;;<br/>    while (num_initial_buckets &gt; 1) &#123;<br/>        num_initial_buckets &#61; num_initial_buckets &#45; 1;<br/>        split_one_bucket(&amp;mut table);<br/>    &#125;;<br/>    table<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_destroy_empty"></a>

## Function `destroy_empty`

Destroy empty table.<br/> Aborts if it&apos;s not empty.


<pre><code>public fun destroy_empty&lt;K, V&gt;(table: smart_table::SmartTable&lt;K, V&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty&lt;K, V&gt;(table: SmartTable&lt;K, V&gt;) &#123;<br/>    assert!(table.size &#61;&#61; 0, error::invalid_argument(ENOT_EMPTY));<br/>    let i &#61; 0;<br/>    while (i &lt; table.num_buckets) &#123;<br/>        vector::destroy_empty(table_with_length::remove(&amp;mut table.buckets, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    let SmartTable &#123; buckets, num_buckets: _, level: _, size: _, split_load_threshold: _, target_bucket_size: _ &#125; &#61; table;<br/>    table_with_length::destroy_empty(buckets);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_destroy"></a>

## Function `destroy`

Destroy a table completely when V has <code>drop</code>.


<pre><code>public fun destroy&lt;K: drop, V: drop&gt;(table: smart_table::SmartTable&lt;K, V&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy&lt;K: drop, V: drop&gt;(table: SmartTable&lt;K, V&gt;) &#123;<br/>    clear(&amp;mut table);<br/>    destroy_empty(table);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_clear"></a>

## Function `clear`

Clear a table completely when T has <code>drop</code>.


<pre><code>public fun clear&lt;K: drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun clear&lt;K: drop, V: drop&gt;(table: &amp;mut SmartTable&lt;K, V&gt;) &#123;<br/>    &#42;table_with_length::borrow_mut(&amp;mut table.buckets, 0) &#61; vector::empty();<br/>    let i &#61; 1;<br/>    while (i &lt; table.num_buckets) &#123;<br/>        table_with_length::remove(&amp;mut table.buckets, i);<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    table.num_buckets &#61; 1;<br/>    table.level &#61; 0;<br/>    table.size &#61; 0;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_add"></a>

## Function `add`

Add (key, value) pair in the hash map, it may grow one bucket if current load factor exceeds the threshold.<br/> Note it may not split the actual overflowed bucket. Instead, it was determined by <code>num_buckets</code> and <code>level</code>.<br/> For standard linear hash algorithm, it is stored as a variable but <code>num_buckets</code> here could be leveraged.<br/> Abort if <code>key</code> already exists.<br/> Note: This method may occasionally cost much more gas when triggering bucket split.


<pre><code>public fun add&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K, value: V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K, value: V) &#123;<br/>    let hash &#61; sip_hash_from_value(&amp;key);<br/>    let index &#61; bucket_index(table.level, table.num_buckets, hash);<br/>    let bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, index);<br/>    // We set a per&#45;bucket limit here with a upper bound (10000) that nobody should normally reach.<br/>    assert!(vector::length(bucket) &lt;&#61; 10000, error::permission_denied(EEXCEED_MAX_BUCKET_SIZE));<br/>    assert!(vector::all(bucket, &#124; entry &#124; &#123;<br/>        let e: &amp;Entry&lt;K, V&gt; &#61; entry;<br/>        &amp;e.key !&#61; &amp;key<br/>    &#125;), error::invalid_argument(EALREADY_EXIST));<br/>    let e &#61; Entry &#123; hash, key, value &#125;;<br/>    if (table.target_bucket_size &#61;&#61; 0) &#123;<br/>        let estimated_entry_size &#61; max(size_of_val(&amp;e), 1);<br/>        table.target_bucket_size &#61; max(1024 /&#42; free_write_quota &#42;/ / estimated_entry_size, 1);<br/>    &#125;;<br/>    vector::push_back(bucket, e);<br/>    table.size &#61; table.size &#43; 1;<br/><br/>    if (load_factor(table) &gt;&#61; (table.split_load_threshold as u64)) &#123;<br/>        split_one_bucket(table);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the smart table. The keys must not already exist.


<pre><code>public fun add_all&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, keys: vector&lt;K&gt;, values: vector&lt;V&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_all&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, keys: vector&lt;K&gt;, values: vector&lt;V&gt;) &#123;<br/>    vector::zip(keys, values, &#124;key, value&#124; &#123; add(table, key, value); &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_unzip_entries"></a>

## Function `unzip_entries`



<pre><code>fun unzip_entries&lt;K: copy, V: copy&gt;(entries: &amp;vector&lt;smart_table::Entry&lt;K, V&gt;&gt;): (vector&lt;K&gt;, vector&lt;V&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun unzip_entries&lt;K: copy, V: copy&gt;(entries: &amp;vector&lt;Entry&lt;K, V&gt;&gt;): (vector&lt;K&gt;, vector&lt;V&gt;) &#123;<br/>    let keys &#61; vector[];<br/>    let values &#61; vector[];<br/>    vector::for_each_ref(entries, &#124;e&#124;&#123;<br/>        let entry: &amp;Entry&lt;K, V&gt; &#61; e;<br/>        vector::push_back(&amp;mut keys, entry.key);<br/>        vector::push_back(&amp;mut values, entry.value);<br/>    &#125;);<br/>    (keys, values)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_to_simple_map"></a>

## Function `to_simple_map`

Convert a smart table to a simple_map, which is supposed to be called mostly by view functions to get an atomic<br/> view of the whole table.<br/> Disclaimer: This function may be costly as the smart table may be huge in size. Use it at your own discretion.


<pre><code>public fun to_simple_map&lt;K: copy, drop, store, V: copy, store&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): simple_map::SimpleMap&lt;K, V&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_simple_map&lt;K: store &#43; copy &#43; drop, V: store &#43; copy&gt;(<br/>    table: &amp;SmartTable&lt;K, V&gt;,<br/>): SimpleMap&lt;K, V&gt; &#123;<br/>    let i &#61; 0;<br/>    let res &#61; simple_map::new&lt;K, V&gt;();<br/>    while (i &lt; table.num_buckets) &#123;<br/>        let (keys, values) &#61; unzip_entries(table_with_length::borrow(&amp;table.buckets, i));<br/>        simple_map::add_all(&amp;mut res, keys, values);<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    res<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_keys"></a>

## Function `keys`

Get all keys in a smart table.<br/><br/> For a large enough smart table this function will fail due to execution gas limits, and<br/> <code>keys_paginated</code> should be used instead.


<pre><code>public fun keys&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;): vector&lt;K&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keys&lt;K: store &#43; copy &#43; drop, V: store &#43; copy&gt;(<br/>    table_ref: &amp;SmartTable&lt;K, V&gt;<br/>): vector&lt;K&gt; &#123;<br/>    let (keys, _, _) &#61; keys_paginated(table_ref, 0, 0, length(table_ref));<br/>    keys<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_keys_paginated"></a>

## Function `keys_paginated`

Get keys from a smart table, paginated.<br/><br/> This function can be used to paginate all keys in a large smart table outside of runtime,<br/> e.g. through chained view function calls. The maximum <code>num_keys_to_get</code> before hitting gas<br/> limits depends on the data types in the smart table.<br/><br/> When starting pagination, pass <code>starting_bucket_index</code> &#61; <code>starting_vector_index</code> &#61; 0.<br/><br/> The function will then return a vector of keys, an optional bucket index, and an optional<br/> vector index. The unpacked return indices can then be used as inputs to another pagination<br/> call, which will return a vector of more keys. This process can be repeated until the<br/> returned bucket index and vector index value options are both none, which means that<br/> pagination is complete. For an example, see <code>test_keys()</code>.


<pre><code>public fun keys_paginated&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;, starting_bucket_index: u64, starting_vector_index: u64, num_keys_to_get: u64): (vector&lt;K&gt;, option::Option&lt;u64&gt;, option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keys_paginated&lt;K: store &#43; copy &#43; drop, V: store &#43; copy&gt;(<br/>    table_ref: &amp;SmartTable&lt;K, V&gt;,<br/>    starting_bucket_index: u64,<br/>    starting_vector_index: u64,<br/>    num_keys_to_get: u64,<br/>): (<br/>    vector&lt;K&gt;,<br/>    Option&lt;u64&gt;,<br/>    Option&lt;u64&gt;,<br/>) &#123;<br/>    let num_buckets &#61; table_ref.num_buckets;<br/>    let buckets_ref &#61; &amp;table_ref.buckets;<br/>    assert!(starting_bucket_index &lt; num_buckets, EINVALID_BUCKET_INDEX);<br/>    let bucket_ref &#61; table_with_length::borrow(buckets_ref, starting_bucket_index);<br/>    let bucket_length &#61; vector::length(bucket_ref);<br/>    assert!(<br/>        // In the general case, starting vector index should never be equal to bucket length<br/>        // because then iteration will attempt to borrow a vector element that is out of bounds.<br/>        // However starting vector index can be equal to bucket length in the special case of<br/>        // starting iteration at the beginning of an empty bucket since buckets are never<br/>        // destroyed, only emptied.<br/>        starting_vector_index &lt; bucket_length &#124;&#124; starting_vector_index &#61;&#61; 0,<br/>        EINVALID_VECTOR_INDEX<br/>    );<br/>    let keys &#61; vector[];<br/>    if (num_keys_to_get &#61;&#61; 0) return
        (keys, option::some(starting_bucket_index), option::some(starting_vector_index));<br/>    for (bucket_index in starting_bucket_index..num_buckets) &#123;<br/>        bucket_ref &#61; table_with_length::borrow(buckets_ref, bucket_index);<br/>        bucket_length &#61; vector::length(bucket_ref);<br/>        for (vector_index in starting_vector_index..bucket_length) &#123;<br/>            vector::push_back(&amp;mut keys, vector::borrow(bucket_ref, vector_index).key);<br/>            num_keys_to_get &#61; num_keys_to_get &#45; 1;<br/>            if (num_keys_to_get &#61;&#61; 0) &#123;<br/>                vector_index &#61; vector_index &#43; 1;<br/>                return if (vector_index &#61;&#61; bucket_length) &#123;<br/>                    bucket_index &#61; bucket_index &#43; 1;<br/>                    if (bucket_index &lt; num_buckets) &#123;<br/>                        (keys, option::some(bucket_index), option::some(0))<br/>                    &#125; else &#123;<br/>                        (keys, option::none(), option::none())<br/>                    &#125;<br/>                &#125; else &#123;<br/>                    (keys, option::some(bucket_index), option::some(vector_index))<br/>                &#125;<br/>            &#125;;<br/>        &#125;;<br/>        starting_vector_index &#61; 0; // Start parsing the next bucket at vector index 0.<br/>    &#125;;<br/>    (keys, option::none(), option::none())<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_split_one_bucket"></a>

## Function `split_one_bucket`

Decide which is the next bucket to split and split it into two with the elements inside the bucket.


<pre><code>fun split_one_bucket&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun split_one_bucket&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;) &#123;<br/>    let new_bucket_index &#61; table.num_buckets;<br/>    // the next bucket to split is num_bucket without the most significant bit.<br/>    let to_split &#61; new_bucket_index ^ (1 &lt;&lt; table.level);<br/>    table.num_buckets &#61; new_bucket_index &#43; 1;<br/>    // if the whole level is splitted once, bump the level.<br/>    if (to_split &#43; 1 &#61;&#61; 1 &lt;&lt; table.level) &#123;<br/>        table.level &#61; table.level &#43; 1;<br/>    &#125;;<br/>    let old_bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, to_split);<br/>    // partition the bucket, [0..p) stays in old bucket, [p..len) goes to new bucket<br/>    let p &#61; vector::partition(old_bucket, &#124;e&#124; &#123;<br/>        let entry: &amp;Entry&lt;K, V&gt; &#61; e; // Explicit type to satisfy compiler<br/>        bucket_index(table.level, table.num_buckets, entry.hash) !&#61; new_bucket_index<br/>    &#125;);<br/>    let new_bucket &#61; vector::trim_reverse(old_bucket, p);<br/>    table_with_length::add(&amp;mut table.buckets, new_bucket_index, new_bucket);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_bucket_index"></a>

## Function `bucket_index`

Return the expected bucket index to find the hash.<br/> Basically, it use different base <code>1 &lt;&lt; level</code> vs <code>1 &lt;&lt; (level &#43; 1)</code> in modulo operation based on the target<br/> bucket index compared to the index of the next bucket to split.


<pre><code>fun bucket_index(level: u8, num_buckets: u64, hash: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun bucket_index(level: u8, num_buckets: u64, hash: u64): u64 &#123;<br/>    let index &#61; hash % (1 &lt;&lt; (level &#43; 1));<br/>    if (index &lt; num_buckets) &#123;<br/>        // in existing bucket<br/>        index<br/>    &#125; else &#123;<br/>        // in unsplitted bucket<br/>        index % (1 &lt;&lt; level)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.<br/> Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow&lt;K: drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K): &amp;V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;K: drop, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, key: K): &amp;V &#123;<br/>    let index &#61; bucket_index(table.level, table.num_buckets, sip_hash_from_value(&amp;key));<br/>    let bucket &#61; table_with_length::borrow(&amp;table.buckets, index);<br/>    let i &#61; 0;<br/>    let len &#61; vector::length(bucket);<br/>    while (i &lt; len) &#123;<br/>        let entry &#61; vector::borrow(bucket, i);<br/>        if (&amp;entry.key &#61;&#61; &amp;key) &#123;<br/>            return &amp;entry.value<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    abort error::invalid_argument(ENOT_FOUND)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow_with_default"></a>

## Function `borrow_with_default`

Acquire an immutable reference to the value which <code>key</code> maps to.<br/> Returns specified default value if there is no entry for <code>key</code>.


<pre><code>public fun borrow_with_default&lt;K: copy, drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K, default: &amp;V): &amp;V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_with_default&lt;K: copy &#43; drop, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, key: K, default: &amp;V): &amp;V &#123;<br/>    if (!contains(table, copy key)) &#123;<br/>        default<br/>    &#125; else &#123;<br/>        borrow(table, copy key)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.<br/> Aborts if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut&lt;K: drop, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K): &amp;mut V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;K: drop, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K): &amp;mut V &#123;<br/>    let index &#61; bucket_index(table.level, table.num_buckets, sip_hash_from_value(&amp;key));<br/>    let bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, index);<br/>    let i &#61; 0;<br/>    let len &#61; vector::length(bucket);<br/>    while (i &lt; len) &#123;<br/>        let entry &#61; vector::borrow_mut(bucket, i);<br/>        if (&amp;entry.key &#61;&#61; &amp;key) &#123;<br/>            return &amp;mut entry.value<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    abort error::invalid_argument(ENOT_FOUND)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.<br/> Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code>public fun borrow_mut_with_default&lt;K: copy, drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K, default: V): &amp;mut V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut_with_default&lt;K: copy &#43; drop, V: drop&gt;(<br/>    table: &amp;mut SmartTable&lt;K, V&gt;,<br/>    key: K,<br/>    default: V<br/>): &amp;mut V &#123;<br/>    if (!contains(table, copy key)) &#123;<br/>        add(table, copy key, default)<br/>    &#125;;<br/>    borrow_mut(table, key)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_contains"></a>

## Function `contains`

Returns true iff <code>table</code> contains an entry for <code>key</code>.


<pre><code>public fun contains&lt;K: drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;K: drop, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, key: K): bool &#123;<br/>    let hash &#61; sip_hash_from_value(&amp;key);<br/>    let index &#61; bucket_index(table.level, table.num_buckets, hash);<br/>    let bucket &#61; table_with_length::borrow(&amp;table.buckets, index);<br/>    vector::any(bucket, &#124; entry &#124; &#123;<br/>        let e: &amp;Entry&lt;K, V&gt; &#61; entry;<br/>        e.hash &#61;&#61; hash &amp;&amp; &amp;e.key &#61;&#61; &amp;key<br/>    &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_remove"></a>

## Function `remove`

Remove from <code>table</code> and return the value which <code>key</code> maps to.<br/> Aborts if there is no entry for <code>key</code>.


<pre><code>public fun remove&lt;K: copy, drop, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K): V<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove&lt;K: copy &#43; drop, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K): V &#123;<br/>    let index &#61; bucket_index(table.level, table.num_buckets, sip_hash_from_value(&amp;key));<br/>    let bucket &#61; table_with_length::borrow_mut(&amp;mut table.buckets, index);<br/>    let i &#61; 0;<br/>    let len &#61; vector::length(bucket);<br/>    while (i &lt; len) &#123;<br/>        let entry &#61; vector::borrow(bucket, i);<br/>        if (&amp;entry.key &#61;&#61; &amp;key) &#123;<br/>            let Entry &#123; hash: _, key: _, value &#125; &#61; vector::swap_remove(bucket, i);<br/>            table.size &#61; table.size &#45; 1;<br/>            return value<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    abort error::invalid_argument(ENOT_FOUND)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_upsert"></a>

## Function `upsert`

Insert the pair (<code>key</code>, <code>value</code>) if there is no entry for <code>key</code>.<br/> update the value of the entry for <code>key</code> to <code>value</code> otherwise


<pre><code>public fun upsert&lt;K: copy, drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, key: K, value: V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upsert&lt;K: copy &#43; drop, V: drop&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, key: K, value: V) &#123;<br/>    if (!contains(table, copy key)) &#123;<br/>        add(table, copy key, value)<br/>    &#125; else &#123;<br/>        let ref &#61; borrow_mut(table, key);<br/>        &#42;ref &#61; value;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_length"></a>

## Function `length`

Returns the length of the table, i.e. the number of entries.


<pre><code>public fun length&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun length&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): u64 &#123;<br/>    table.size<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_load_factor"></a>

## Function `load_factor`

Return the load factor of the hashtable.


<pre><code>public fun load_factor&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun load_factor&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): u64 &#123;<br/>    table.size &#42; 100 / table.num_buckets / table.target_bucket_size<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_update_split_load_threshold"></a>

## Function `update_split_load_threshold`

Update <code>split_load_threshold</code>.


<pre><code>public fun update_split_load_threshold&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, split_load_threshold: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_split_load_threshold&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, split_load_threshold: u8) &#123;<br/>    assert!(<br/>        split_load_threshold &lt;&#61; 100 &amp;&amp; split_load_threshold &gt; 0,<br/>        error::invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT)<br/>    );<br/>    table.split_load_threshold &#61; split_load_threshold;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_update_target_bucket_size"></a>

## Function `update_target_bucket_size`

Update <code>target_bucket_size</code>.


<pre><code>public fun update_target_bucket_size&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, target_bucket_size: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_target_bucket_size&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, target_bucket_size: u64) &#123;<br/>    assert!(target_bucket_size &gt; 0, error::invalid_argument(EINVALID_TARGET_BUCKET_SIZE));<br/>    table.target_bucket_size &#61; target_bucket_size;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each key&#45;value pair in the table.


<pre><code>public fun for_each_ref&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, f: &#124;(&amp;K, &amp;V)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_ref&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;, f: &#124;&amp;K, &amp;V&#124;) &#123;<br/>    let i &#61; 0;<br/>    while (i &lt; aptos_std::smart_table::num_buckets(table)) &#123;<br/>        vector::for_each_ref(<br/>            aptos_std::table_with_length::borrow(aptos_std::smart_table::borrow_buckets(table), i),<br/>            &#124;elem&#124; &#123;<br/>                let (key, value) &#61; aptos_std::smart_table::borrow_kv(elem);<br/>                f(key, value)<br/>            &#125;<br/>        );<br/>        i &#61; i &#43; 1;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference of each key&#45;value pair in the table.


<pre><code>public fun for_each_mut&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, f: &#124;(&amp;K, &amp;mut V)&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_mut&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;, f: &#124;&amp;K, &amp;mut V&#124;) &#123;<br/>    let i &#61; 0;<br/>    while (i &lt; aptos_std::smart_table::num_buckets(table)) &#123;<br/>        vector::for_each_mut(<br/>            table_with_length::borrow_mut(aptos_std::smart_table::borrow_buckets_mut(table), i),<br/>            &#124;elem&#124; &#123;<br/>                let (key, value) &#61; aptos_std::smart_table::borrow_kv_mut(elem);<br/>                f(key, value)<br/>            &#125;<br/>        );<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_map_ref"></a>

## Function `map_ref`

Map the function over the references of key&#45;value pairs in the table without modifying it.


<pre><code>public fun map_ref&lt;K: copy, drop, store, V1, V2: store&gt;(table: &amp;smart_table::SmartTable&lt;K, V1&gt;, f: &#124;&amp;V1&#124;V2): smart_table::SmartTable&lt;K, V2&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map_ref&lt;K: copy &#43; drop &#43; store, V1, V2: store&gt;(<br/>    table: &amp;SmartTable&lt;K, V1&gt;,<br/>    f: &#124;&amp;V1&#124;V2<br/>): SmartTable&lt;K, V2&gt; &#123;<br/>    let new_table &#61; new&lt;K, V2&gt;();<br/>    for_each_ref(table, &#124;key, value&#124; add(&amp;mut new_table, &#42;key, f(value)));<br/>    new_table<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_any"></a>

## Function `any`

Return true if any key&#45;value pair in the table satisfies the predicate.


<pre><code>public fun any&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, p: &#124;(&amp;K, &amp;V)&#124;bool): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun any&lt;K, V&gt;(<br/>    table: &amp;SmartTable&lt;K, V&gt;,<br/>    p: &#124;&amp;K, &amp;V&#124;bool<br/>): bool &#123;<br/>    let found &#61; false;<br/>    let i &#61; 0;<br/>    while (i &lt; aptos_std::smart_table::num_buckets(table)) &#123;<br/>        found &#61; vector::any(table_with_length::borrow(aptos_std::smart_table::borrow_buckets(table), i), &#124;elem&#124; &#123;<br/>            let (key, value) &#61; aptos_std::smart_table::borrow_kv(elem);<br/>            p(key, value)<br/>        &#125;);<br/>        if (found) break;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    found<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow_kv"></a>

## Function `borrow_kv`



<pre><code>public fun borrow_kv&lt;K, V&gt;(e: &amp;smart_table::Entry&lt;K, V&gt;): (&amp;K, &amp;V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_kv&lt;K, V&gt;(e: &amp;Entry&lt;K, V&gt;): (&amp;K, &amp;V) &#123;<br/>    (&amp;e.key, &amp;e.value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow_kv_mut"></a>

## Function `borrow_kv_mut`



<pre><code>public fun borrow_kv_mut&lt;K, V&gt;(e: &amp;mut smart_table::Entry&lt;K, V&gt;): (&amp;mut K, &amp;mut V)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_kv_mut&lt;K, V&gt;(e: &amp;mut Entry&lt;K, V&gt;): (&amp;mut K, &amp;mut V) &#123;<br/>    (&amp;mut e.key, &amp;mut e.value)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_num_buckets"></a>

## Function `num_buckets`



<pre><code>public fun num_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun num_buckets&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): u64 &#123;<br/>    table.num_buckets<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow_buckets"></a>

## Function `borrow_buckets`



<pre><code>public fun borrow_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): &amp;table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_buckets&lt;K, V&gt;(table: &amp;SmartTable&lt;K, V&gt;): &amp;TableWithLength&lt;u64, vector&lt;Entry&lt;K, V&gt;&gt;&gt; &#123;<br/>    &amp;table.buckets<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_smart_table_borrow_buckets_mut"></a>

## Function `borrow_buckets_mut`



<pre><code>public fun borrow_buckets_mut&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;): &amp;mut table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_buckets_mut&lt;K, V&gt;(table: &amp;mut SmartTable&lt;K, V&gt;): &amp;mut TableWithLength&lt;u64, vector&lt;Entry&lt;K, V&gt;&gt;&gt; &#123;<br/>    &amp;mut table.buckets<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SmartTable"></a>

### Struct `SmartTable`


<pre><code>struct SmartTable&lt;K, V&gt; has store<br/></code></pre>



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



<pre><code>pragma intrinsic &#61; map,<br/>    map_new &#61; new,<br/>    map_destroy_empty &#61; destroy_empty,<br/>    map_len &#61; length,<br/>    map_has_key &#61; contains,<br/>    map_add_no_override &#61; add,<br/>    map_add_override_if_exists &#61; upsert,<br/>    map_del_must_exist &#61; remove,<br/>    map_borrow &#61; borrow,<br/>    map_borrow_mut &#61; borrow_mut,<br/>    map_borrow_mut_with_default &#61; borrow_mut_with_default,<br/>    map_spec_get &#61; spec_get,<br/>    map_spec_set &#61; spec_set,<br/>    map_spec_del &#61; spec_remove,<br/>    map_spec_len &#61; spec_len,<br/>map_spec_has_key &#61; spec_contains;<br/></code></pre>



<a id="@Specification_1_new_with_config"></a>

### Function `new_with_config`


<pre><code>public fun new_with_config&lt;K: copy, drop, store, V: store&gt;(num_initial_buckets: u64, split_load_threshold: u8, target_bucket_size: u64): smart_table::SmartTable&lt;K, V&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code>public fun destroy&lt;K: drop, V: drop&gt;(table: smart_table::SmartTable&lt;K, V&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_clear"></a>

### Function `clear`


<pre><code>public fun clear&lt;K: drop, V: drop&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_add_all"></a>

### Function `add_all`


<pre><code>public fun add_all&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, keys: vector&lt;K&gt;, values: vector&lt;V&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_to_simple_map"></a>

### Function `to_simple_map`


<pre><code>public fun to_simple_map&lt;K: copy, drop, store, V: copy, store&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): simple_map::SimpleMap&lt;K, V&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code>public fun keys&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;): vector&lt;K&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_keys_paginated"></a>

### Function `keys_paginated`


<pre><code>public fun keys_paginated&lt;K: copy, drop, store, V: copy, store&gt;(table_ref: &amp;smart_table::SmartTable&lt;K, V&gt;, starting_bucket_index: u64, starting_vector_index: u64, num_keys_to_get: u64): (vector&lt;K&gt;, option::Option&lt;u64&gt;, option::Option&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_split_one_bucket"></a>

### Function `split_one_bucket`


<pre><code>fun split_one_bucket&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_bucket_index"></a>

### Function `bucket_index`


<pre><code>fun bucket_index(level: u8, num_buckets: u64, hash: u64): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_borrow_with_default"></a>

### Function `borrow_with_default`


<pre><code>public fun borrow_with_default&lt;K: copy, drop, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;, key: K, default: &amp;V): &amp;V<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_load_factor"></a>

### Function `load_factor`


<pre><code>public fun load_factor&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_update_split_load_threshold"></a>

### Function `update_split_load_threshold`


<pre><code>public fun update_split_load_threshold&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, split_load_threshold: u8)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_update_target_bucket_size"></a>

### Function `update_target_bucket_size`


<pre><code>public fun update_target_bucket_size&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;, target_bucket_size: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_borrow_kv"></a>

### Function `borrow_kv`


<pre><code>public fun borrow_kv&lt;K, V&gt;(e: &amp;smart_table::Entry&lt;K, V&gt;): (&amp;K, &amp;V)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_borrow_kv_mut"></a>

### Function `borrow_kv_mut`


<pre><code>public fun borrow_kv_mut&lt;K, V&gt;(e: &amp;mut smart_table::Entry&lt;K, V&gt;): (&amp;mut K, &amp;mut V)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_num_buckets"></a>

### Function `num_buckets`


<pre><code>public fun num_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): u64<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_borrow_buckets"></a>

### Function `borrow_buckets`


<pre><code>public fun borrow_buckets&lt;K, V&gt;(table: &amp;smart_table::SmartTable&lt;K, V&gt;): &amp;table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_borrow_buckets_mut"></a>

### Function `borrow_buckets_mut`


<pre><code>public fun borrow_buckets_mut&lt;K, V&gt;(table: &amp;mut smart_table::SmartTable&lt;K, V&gt;): &amp;mut table_with_length::TableWithLength&lt;u64, vector&lt;smart_table::Entry&lt;K, V&gt;&gt;&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>




<a id="0x1_smart_table_spec_len"></a>


<pre><code>native fun spec_len&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;): num;<br/></code></pre>




<a id="0x1_smart_table_spec_contains"></a>


<pre><code>native fun spec_contains&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K): bool;<br/></code></pre>




<a id="0x1_smart_table_spec_set"></a>


<pre><code>native fun spec_set&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K, v: V): SmartTable&lt;K, V&gt;;<br/></code></pre>




<a id="0x1_smart_table_spec_remove"></a>


<pre><code>native fun spec_remove&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K): SmartTable&lt;K, V&gt;;<br/></code></pre>




<a id="0x1_smart_table_spec_get"></a>


<pre><code>native fun spec_get&lt;K, V&gt;(t: SmartTable&lt;K, V&gt;, k: K): V;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
