
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


<pre><code><b>use</b> <a href="hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
<b>use</b> <a href="type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_smart_table_Entry"></a>

## Struct `Entry`

SmartTable entry contains both the key and value.


<pre><code><b>struct</b> <a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u64</code>
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



<pre><code><b>struct</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>buckets: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;&gt;&gt;</code>
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


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_ENOT_EMPTY">ENOT_EMPTY</a>: u64 = 3;
</code></pre>



<a id="0x1_smart_table_ENOT_FOUND"></a>

Key not found in the smart table


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_ENOT_FOUND">ENOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_smart_table_EALREADY_EXIST"></a>

Key already exists


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_EALREADY_EXIST">EALREADY_EXIST</a>: u64 = 4;
</code></pre>



<a id="0x1_smart_table_EEXCEED_MAX_BUCKET_SIZE"></a>

Invalid target bucket size.


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_EEXCEED_MAX_BUCKET_SIZE">EEXCEED_MAX_BUCKET_SIZE</a>: u64 = 7;
</code></pre>



<a id="0x1_smart_table_EINVALID_BUCKET_INDEX"></a>

Invalid bucket index.


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_EINVALID_BUCKET_INDEX">EINVALID_BUCKET_INDEX</a>: u64 = 8;
</code></pre>



<a id="0x1_smart_table_EINVALID_LOAD_THRESHOLD_PERCENT"></a>

Invalid load threshold percent to trigger split.


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_EINVALID_LOAD_THRESHOLD_PERCENT">EINVALID_LOAD_THRESHOLD_PERCENT</a>: u64 = 5;
</code></pre>



<a id="0x1_smart_table_EINVALID_TARGET_BUCKET_SIZE"></a>

Invalid target bucket size.


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_EINVALID_TARGET_BUCKET_SIZE">EINVALID_TARGET_BUCKET_SIZE</a>: u64 = 6;
</code></pre>



<a id="0x1_smart_table_EINVALID_VECTOR_INDEX"></a>

Invalid vector index within a bucket.


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_EINVALID_VECTOR_INDEX">EINVALID_VECTOR_INDEX</a>: u64 = 9;
</code></pre>



<a id="0x1_smart_table_EZERO_CAPACITY"></a>

Smart table capacity must be larger than 0


<pre><code><b>const</b> <a href="smart_table.md#0x1_smart_table_EZERO_CAPACITY">EZERO_CAPACITY</a>: u64 = 2;
</code></pre>



<a id="0x1_smart_table_new"></a>

## Function `new`

Create an empty SmartTable with default configurations.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_new">new</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(): <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_new">new</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(): <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt; {
    <a href="smart_table.md#0x1_smart_table_new_with_config">new_with_config</a>&lt;K, V&gt;(0, 0, 0)
}
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


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_new_with_config">new_with_config</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(num_initial_buckets: u64, split_load_threshold: u8, target_bucket_size: u64): <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_new_with_config">new_with_config</a>&lt;K: <b>copy</b> + drop + store, V: store&gt;(
    num_initial_buckets: u64,
    split_load_threshold: u8,
    target_bucket_size: u64
): <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt; {
    <b>assert</b>!(split_load_threshold &lt;= 100, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_EINVALID_LOAD_THRESHOLD_PERCENT">EINVALID_LOAD_THRESHOLD_PERCENT</a>));
    <b>let</b> buckets = <a href="table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>();
    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> buckets, 0, <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());
    <b>let</b> <a href="table.md#0x1_table">table</a> = <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a> {
        buckets,
        num_buckets: 1,
        level: 0,
        size: 0,
        // The default split load threshold is 75%.
        split_load_threshold: <b>if</b> (split_load_threshold == 0) { 75 } <b>else</b> { split_load_threshold },
        target_bucket_size,
    };
    // The default number of initial buckets is 2.
    <b>if</b> (num_initial_buckets == 0) {
        num_initial_buckets = 2;
    };
    <b>while</b> (num_initial_buckets &gt; 1) {
        num_initial_buckets = num_initial_buckets - 1;
        <a href="smart_table.md#0x1_smart_table_split_one_bucket">split_one_bucket</a>(&<b>mut</b> <a href="table.md#0x1_table">table</a>);
    };
    <a href="table.md#0x1_table">table</a>
}
</code></pre>



</details>

<a id="0x1_smart_table_destroy_empty"></a>

## Function `destroy_empty`

Destroy empty table.
Aborts if it's not empty.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_destroy_empty">destroy_empty</a>&lt;K, V&gt;(self: <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_destroy_empty">destroy_empty</a>&lt;K, V&gt;(self: <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;) {
    <b>assert</b>!(self.size == 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_ENOT_EMPTY">ENOT_EMPTY</a>));
    <b>let</b> i = 0;
    <b>while</b> (i &lt; self.num_buckets) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(<a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> self.buckets, i));
        i = i + 1;
    };
    <b>let</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a> { buckets, num_buckets: _, level: _, size: _, split_load_threshold: _, target_bucket_size: _ } = self;
    <a href="table_with_length.md#0x1_table_with_length_destroy_empty">table_with_length::destroy_empty</a>(buckets);
}
</code></pre>



</details>

<a id="0x1_smart_table_destroy"></a>

## Function `destroy`

Destroy a table completely when V has <code>drop</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_destroy">destroy</a>&lt;K: drop, V: drop&gt;(self: <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_destroy">destroy</a>&lt;K: drop, V: drop&gt;(self: <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;) {
    <a href="smart_table.md#0x1_smart_table_clear">clear</a>(&<b>mut</b> self);
    <a href="smart_table.md#0x1_smart_table_destroy_empty">destroy_empty</a>(self);
}
</code></pre>



</details>

<a id="0x1_smart_table_clear"></a>

## Function `clear`

Clear a table completely when T has <code>drop</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_clear">clear</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_clear">clear</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;) {
    *<a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> self.buckets, 0) = <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> i = 1;
    <b>while</b> (i &lt; self.num_buckets) {
        <a href="table_with_length.md#0x1_table_with_length_remove">table_with_length::remove</a>(&<b>mut</b> self.buckets, i);
        i = i + 1;
    };
    self.num_buckets = 1;
    self.level = 0;
    self.size = 0;
}
</code></pre>



</details>

<a id="0x1_smart_table_add"></a>

## Function `add`

Add (key, value) pair in the hash map, it may grow one bucket if current load factor exceeds the threshold.
Note it may not split the actual overflowed bucket. Instead, it was determined by <code>num_buckets</code> and <code>level</code>.
For standard linear hash algorithm, it is stored as a variable but <code>num_buckets</code> here could be leveraged.
Abort if <code>key</code> already exists.
Note: This method may occasionally cost much more gas when triggering bucket split.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_add">add</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_add">add</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, key: K, value: V) {
    <b>let</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a> = sip_hash_from_value(&key);
    <b>let</b> index = <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(self.level, self.num_buckets, <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>);
    <b>let</b> bucket = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> self.buckets, index);
    // We set a per-bucket limit here <b>with</b> a upper bound (10000) that nobody should normally reach.
    <b>assert</b>!(<a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bucket) &lt;= 10000, <a href="../../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="smart_table.md#0x1_smart_table_EEXCEED_MAX_BUCKET_SIZE">EEXCEED_MAX_BUCKET_SIZE</a>));
    <b>assert</b>!(<a href="../../move-stdlib/doc/vector.md#0x1_vector_all">vector::all</a>(bucket, | entry | {
        <b>let</b> e: &<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt; = entry;
        &e.key != &key
    }), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_EALREADY_EXIST">EALREADY_EXIST</a>));
    <b>let</b> e = <a href="smart_table.md#0x1_smart_table_Entry">Entry</a> { <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>, key, value };
    <b>if</b> (self.target_bucket_size == 0) {
        <b>let</b> estimated_entry_size = max(size_of_val(&e), 1);
        self.target_bucket_size = max(1024 /* free_write_quota */ / estimated_entry_size, 1);
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(bucket, e);
    self.size = self.size + 1;

    <b>if</b> (<a href="smart_table.md#0x1_smart_table_load_factor">load_factor</a>(self) &gt;= (self.split_load_threshold <b>as</b> u64)) {
        <a href="smart_table.md#0x1_smart_table_split_one_bucket">split_one_bucket</a>(self);
    }
}
</code></pre>



</details>

<a id="0x1_smart_table_add_all"></a>

## Function `add_all`

Add multiple key/value pairs to the smart table. The keys must not already exist.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_zip">vector::zip</a>(keys, values, |key, value| { <a href="smart_table.md#0x1_smart_table_add">add</a>(self, key, value); });
}
</code></pre>



</details>

<a id="0x1_smart_table_unzip_entries"></a>

## Function `unzip_entries`



<pre><code><b>fun</b> <a href="smart_table.md#0x1_smart_table_unzip_entries">unzip_entries</a>&lt;K: <b>copy</b>, V: <b>copy</b>&gt;(entries: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="smart_table.md#0x1_smart_table_unzip_entries">unzip_entries</a>&lt;K: <b>copy</b>, V: <b>copy</b>&gt;(entries: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt;&gt;): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;) {
    <b>let</b> keys = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> values = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(entries, |e|{
        <b>let</b> entry: &<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt; = e;
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> keys, entry.key);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> values, entry.value);
    });
    (keys, values)
}
</code></pre>



</details>

<a id="0x1_smart_table_to_simple_map"></a>

## Function `to_simple_map`

Convert a smart table to a simple_map, which is supposed to be called mostly by view functions to get an atomic
view of the whole table.
Disclaimer: This function may be costly as the smart table may be huge in size. Use it at your own discretion.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_to_simple_map">to_simple_map</a>&lt;K: <b>copy</b>, drop, store, V: <b>copy</b>, store&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;K, V&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_to_simple_map">to_simple_map</a>&lt;K: store + <b>copy</b> + drop, V: store + <b>copy</b>&gt;(
    self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;,
): SimpleMap&lt;K, V&gt; {
    <b>let</b> i = 0;
    <b>let</b> res = <a href="simple_map.md#0x1_simple_map_new">simple_map::new</a>&lt;K, V&gt;();
    <b>while</b> (i &lt; self.num_buckets) {
        <b>let</b> (keys, values) = <a href="smart_table.md#0x1_smart_table_unzip_entries">unzip_entries</a>(<a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&self.buckets, i));
        <a href="simple_map.md#0x1_simple_map_add_all">simple_map::add_all</a>(&<b>mut</b> res, keys, values);
        i = i + 1;
    };
    res
}
</code></pre>



</details>

<a id="0x1_smart_table_keys"></a>

## Function `keys`

Get all keys in a smart table.

For a large enough smart table this function will fail due to execution gas limits, and
<code>keys_paginated</code> should be used instead.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_keys">keys</a>&lt;K: <b>copy</b>, drop, store, V: <b>copy</b>, store&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_keys">keys</a>&lt;K: store + <b>copy</b> + drop, V: store + <b>copy</b>&gt;(
    self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;
): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt; {
    <b>let</b> (keys, _, _) = <a href="smart_table.md#0x1_smart_table_keys_paginated">keys_paginated</a>(self, 0, 0, <a href="smart_table.md#0x1_smart_table_length">length</a>(self));
    keys
}
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


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_keys_paginated">keys_paginated</a>&lt;K: <b>copy</b>, drop, store, V: <b>copy</b>, store&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, starting_bucket_index: u64, starting_vector_index: u64, num_keys_to_get: u64): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_keys_paginated">keys_paginated</a>&lt;K: store + <b>copy</b> + drop, V: store + <b>copy</b>&gt;(
    self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;,
    starting_bucket_index: u64,
    starting_vector_index: u64,
    num_keys_to_get: u64,
): (
    <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;,
    Option&lt;u64&gt;,
    Option&lt;u64&gt;,
) {
    <b>let</b> num_buckets = self.num_buckets;
    <b>let</b> buckets_ref = &self.buckets;
    <b>assert</b>!(starting_bucket_index &lt; num_buckets, <a href="smart_table.md#0x1_smart_table_EINVALID_BUCKET_INDEX">EINVALID_BUCKET_INDEX</a>);
    <b>let</b> bucket_ref = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(buckets_ref, starting_bucket_index);
    <b>let</b> bucket_length = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bucket_ref);
    <b>assert</b>!(
        // In the general case, starting <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> index should never be equal <b>to</b> bucket length
        // because then iteration will attempt <b>to</b> borrow a <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> element that is out of bounds.
        // However starting <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> index can be equal <b>to</b> bucket length in the special case of
        // starting iteration at the beginning of an empty bucket since buckets are never
        // destroyed, only emptied.
        starting_vector_index &lt; bucket_length || starting_vector_index == 0,
        <a href="smart_table.md#0x1_smart_table_EINVALID_VECTOR_INDEX">EINVALID_VECTOR_INDEX</a>
    );
    <b>let</b> keys = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>if</b> (num_keys_to_get == 0) <b>return</b>
        (keys, <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(starting_bucket_index), <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(starting_vector_index));
    for (bucket_index in starting_bucket_index..num_buckets) {
        bucket_ref = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(buckets_ref, bucket_index);
        bucket_length = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bucket_ref);
        for (vector_index in starting_vector_index..bucket_length) {
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> keys, <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(bucket_ref, vector_index).key);
            num_keys_to_get = num_keys_to_get - 1;
            <b>if</b> (num_keys_to_get == 0) {
                vector_index = vector_index + 1;
                <b>return</b> <b>if</b> (vector_index == bucket_length) {
                    bucket_index = bucket_index + 1;
                    <b>if</b> (<a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a> &lt; num_buckets) {
                        (keys, <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(bucket_index), <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(0))
                    } <b>else</b> {
                        (keys, <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
                    }
                } <b>else</b> {
                    (keys, <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(bucket_index), <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(vector_index))
                }
            };
        };
        starting_vector_index = 0; // Start parsing the next bucket at <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a> index 0.
    };
    (keys, <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
}
</code></pre>



</details>

<a id="0x1_smart_table_split_one_bucket"></a>

## Function `split_one_bucket`

Decide which is the next bucket to split and split it into two with the elements inside the bucket.


<pre><code><b>fun</b> <a href="smart_table.md#0x1_smart_table_split_one_bucket">split_one_bucket</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="smart_table.md#0x1_smart_table_split_one_bucket">split_one_bucket</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;) {
    <b>let</b> new_bucket_index = self.num_buckets;
    // the next bucket <b>to</b> split is num_bucket without the most significant bit.
    <b>let</b> to_split = new_bucket_index ^ (1 &lt;&lt; self.level);
    self.num_buckets = new_bucket_index + 1;
    // <b>if</b> the whole level is splitted once, bump the level.
    <b>if</b> (to_split + 1 == 1 &lt;&lt; self.level) {
        self.level = self.level + 1;
    };
    <b>let</b> old_bucket = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> self.buckets, to_split);
    // partition the bucket, [0..p) stays in <b>old</b> bucket, [p..len) goes <b>to</b> new bucket
    <b>let</b> p = <a href="../../move-stdlib/doc/vector.md#0x1_vector_partition">vector::partition</a>(old_bucket, |e| {
        <b>let</b> entry: &<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt; = e; // Explicit type <b>to</b> satisfy compiler
        <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(self.level, self.num_buckets, entry.<a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>) != new_bucket_index
    });
    <b>let</b> new_bucket = <a href="../../move-stdlib/doc/vector.md#0x1_vector_trim_reverse">vector::trim_reverse</a>(old_bucket, p);
    <a href="table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> self.buckets, new_bucket_index, new_bucket);
}
</code></pre>



</details>

<a id="0x1_smart_table_bucket_index"></a>

## Function `bucket_index`

Return the expected bucket index to find the hash.
Basically, it use different base <code>1 &lt;&lt; level</code> vs <code>1 &lt;&lt; (level + 1)</code> in modulo operation based on the target
bucket index compared to the index of the next bucket to split.


<pre><code><b>fun</b> <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(level: u8, num_buckets: u64, <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(level: u8, num_buckets: u64, <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u64): u64 {
    <b>let</b> index = <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a> % (1 &lt;&lt; (level + 1));
    <b>if</b> (index &lt; num_buckets) {
        // in existing bucket
        index
    } <b>else</b> {
        // in unsplitted bucket
        index % (1 &lt;&lt; level)
    }
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow"></a>

## Function `borrow`

Acquire an immutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow">borrow</a>&lt;K: drop, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow">borrow</a>&lt;K: drop, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, key: K): &V {
    <b>let</b> index = <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(self.level, self.num_buckets, sip_hash_from_value(&key));
    <b>let</b> bucket = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&self.buckets, index);
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bucket);
    <b>while</b> (i &lt; len) {
        <b>let</b> entry = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(bucket, i);
        <b>if</b> (&entry.key == &key) {
            <b>return</b> &entry.value
        };
        i = i + 1;
    };
    <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_ENOT_FOUND">ENOT_FOUND</a>)
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow_with_default"></a>

## Function `borrow_with_default`

Acquire an immutable reference to the value which <code>key</code> maps to.
Returns specified default value if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_with_default">borrow_with_default</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K, default: &V): &V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_with_default">borrow_with_default</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, key: K, default: &V): &V {
    <b>if</b> (!<a href="smart_table.md#0x1_smart_table_contains">contains</a>(self, <b>copy</b> key)) {
        default
    } <b>else</b> {
        <a href="smart_table.md#0x1_smart_table_borrow">borrow</a>(self, <b>copy</b> key)
    }
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow_mut"></a>

## Function `borrow_mut`

Acquire a mutable reference to the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_mut">borrow_mut</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_mut">borrow_mut</a>&lt;K: drop, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, key: K): &<b>mut</b> V {
    <b>let</b> index = <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(self.level, self.num_buckets, sip_hash_from_value(&key));
    <b>let</b> bucket = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> self.buckets, index);
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bucket);
    <b>while</b> (i &lt; len) {
        <b>let</b> entry = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(bucket, i);
        <b>if</b> (&entry.key == &key) {
            <b>return</b> &<b>mut</b> entry.value
        };
        i = i + 1;
    };
    <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_ENOT_FOUND">ENOT_FOUND</a>)
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow_mut_with_default"></a>

## Function `borrow_mut_with_default`

Acquire a mutable reference to the value which <code>key</code> maps to.
Insert the pair (<code>key</code>, <code>default</code>) first if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K, default: V): &<b>mut</b> V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_mut_with_default">borrow_mut_with_default</a>&lt;K: <b>copy</b> + drop, V: drop&gt;(
    self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;,
    key: K,
    default: V
): &<b>mut</b> V {
    <b>if</b> (!<a href="smart_table.md#0x1_smart_table_contains">contains</a>(self, <b>copy</b> key)) {
        <a href="smart_table.md#0x1_smart_table_add">add</a>(self, <b>copy</b> key, default)
    };
    <a href="smart_table.md#0x1_smart_table_borrow_mut">borrow_mut</a>(self, key)
}
</code></pre>



</details>

<a id="0x1_smart_table_contains"></a>

## Function `contains`

Returns true iff <code><a href="table.md#0x1_table">table</a></code> contains an entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_contains">contains</a>&lt;K: drop, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_contains">contains</a>&lt;K: drop, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, key: K): bool {
    <b>let</b> <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a> = sip_hash_from_value(&key);
    <b>let</b> index = <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(self.level, self.num_buckets, <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>);
    <b>let</b> bucket = <a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(&self.buckets, index);
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_any">vector::any</a>(bucket, | entry | {
        <b>let</b> e: &<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt; = entry;
        e.<a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a> == <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a> && &e.key == &key
    })
}
</code></pre>



</details>

<a id="0x1_smart_table_remove"></a>

## Function `remove`

Remove from <code><a href="table.md#0x1_table">table</a></code> and return the value which <code>key</code> maps to.
Aborts if there is no entry for <code>key</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_remove">remove</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K): V
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_remove">remove</a>&lt;K: <b>copy</b> + drop, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, key: K): V {
    <b>let</b> index = <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(self.level, self.num_buckets, sip_hash_from_value(&key));
    <b>let</b> bucket = <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> self.buckets, index);
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bucket);
    <b>while</b> (i &lt; len) {
        <b>let</b> entry = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(bucket, i);
        <b>if</b> (&entry.key == &key) {
            <b>let</b> <a href="smart_table.md#0x1_smart_table_Entry">Entry</a> { <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>: _, key: _, value } = <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(bucket, i);
            self.size = self.size - 1;
            <b>return</b> value
        };
        i = i + 1;
    };
    <b>abort</b> <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_ENOT_FOUND">ENOT_FOUND</a>)
}
</code></pre>



</details>

<a id="0x1_smart_table_upsert"></a>

## Function `upsert`

Insert the pair (<code>key</code>, <code>value</code>) if there is no entry for <code>key</code>.
update the value of the entry for <code>key</code> to <code>value</code> otherwise


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_upsert">upsert</a>&lt;K: <b>copy</b>, drop, V: drop&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K, value: V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_upsert">upsert</a>&lt;K: <b>copy</b> + drop, V: drop&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, key: K, value: V) {
    <b>if</b> (!<a href="smart_table.md#0x1_smart_table_contains">contains</a>(self, <b>copy</b> key)) {
        <a href="smart_table.md#0x1_smart_table_add">add</a>(self, <b>copy</b> key, value)
    } <b>else</b> {
        <b>let</b> ref = <a href="smart_table.md#0x1_smart_table_borrow_mut">borrow_mut</a>(self, key);
        *ref = value;
    };
}
</code></pre>



</details>

<a id="0x1_smart_table_length"></a>

## Function `length`

Returns the length of the table, i.e. the number of entries.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_length">length</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_length">length</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;): u64 {
    self.size
}
</code></pre>



</details>

<a id="0x1_smart_table_load_factor"></a>

## Function `load_factor`

Return the load factor of the hashtable.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_load_factor">load_factor</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_load_factor">load_factor</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;): u64 {
    self.size * 100 / self.num_buckets / self.target_bucket_size
}
</code></pre>



</details>

<a id="0x1_smart_table_update_split_load_threshold"></a>

## Function `update_split_load_threshold`

Update <code>split_load_threshold</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_update_split_load_threshold">update_split_load_threshold</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, split_load_threshold: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_update_split_load_threshold">update_split_load_threshold</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, split_load_threshold: u8) {
    <b>assert</b>!(
        split_load_threshold &lt;= 100 && split_load_threshold &gt; 0,
        <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_EINVALID_LOAD_THRESHOLD_PERCENT">EINVALID_LOAD_THRESHOLD_PERCENT</a>)
    );
    self.split_load_threshold = split_load_threshold;
}
</code></pre>



</details>

<a id="0x1_smart_table_update_target_bucket_size"></a>

## Function `update_target_bucket_size`

Update <code>target_bucket_size</code>.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_update_target_bucket_size">update_target_bucket_size</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, target_bucket_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_update_target_bucket_size">update_target_bucket_size</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, target_bucket_size: u64) {
    <b>assert</b>!(target_bucket_size &gt; 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="smart_table.md#0x1_smart_table_EINVALID_TARGET_BUCKET_SIZE">EINVALID_TARGET_BUCKET_SIZE</a>));
    self.target_bucket_size = target_bucket_size;
}
</code></pre>



</details>

<a id="0x1_smart_table_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to a reference of each key-value pair in the table.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_for_each_ref">for_each_ref</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, f: |(&K, &V)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_table.md#0x1_smart_table_for_each_ref">for_each_ref</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, f: |&K, &V|) {
    <b>let</b> i = 0;
    <b>while</b> (i &lt; aptos_std::smart_table::num_buckets(self)) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(
            aptos_std::table_with_length::borrow(aptos_std::smart_table::borrow_buckets(self), i),
            |elem| {
                <b>let</b> (key, value) = aptos_std::smart_table::borrow_kv(elem);
                f(key, value)
            }
        );
        i = i + 1;
    }
}
</code></pre>



</details>

<a id="0x1_smart_table_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to a mutable reference of each key-value pair in the table.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_for_each_mut">for_each_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, f: |(&K, &<b>mut</b> V)|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_table.md#0x1_smart_table_for_each_mut">for_each_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, f: |&K, &<b>mut</b> V|) {
    <b>let</b> i = 0;
    <b>while</b> (i &lt; aptos_std::smart_table::num_buckets(self)) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_mut">vector::for_each_mut</a>(
            <a href="table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(aptos_std::smart_table::borrow_buckets_mut(self), i),
            |elem| {
                <b>let</b> (key, value) = aptos_std::smart_table::borrow_kv_mut(elem);
                f(key, value)
            }
        );
        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x1_smart_table_map_ref"></a>

## Function `map_ref`

Map the function over the references of key-value pairs in the table without modifying it.


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_map_ref">map_ref</a>&lt;K: <b>copy</b>, drop, store, V1, V2: store&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V1&gt;, f: |&V1|V2): <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V2&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="smart_table.md#0x1_smart_table_map_ref">map_ref</a>&lt;K: <b>copy</b> + drop + store, V1, V2: store&gt;(
    self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V1&gt;,
    f: |&V1|V2
): <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V2&gt; {
    <b>let</b> new_table = <a href="smart_table.md#0x1_smart_table_new">new</a>&lt;K, V2&gt;();
    <a href="smart_table.md#0x1_smart_table_for_each_ref">for_each_ref</a>(self, |key, value| <a href="smart_table.md#0x1_smart_table_add">add</a>(&<b>mut</b> new_table, *key, f(value)));
    new_table
}
</code></pre>



</details>

<a id="0x1_smart_table_any"></a>

## Function `any`

Return true if any key-value pair in the table satisfies the predicate.


<pre><code><b>public</b> <b>fun</b> <a href="any.md#0x1_any">any</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, p: |(&K, &V)|bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="any.md#0x1_any">any</a>&lt;K, V&gt;(
    self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;,
    p: |&K, &V|bool
): bool {
    <b>let</b> found = <b>false</b>;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; aptos_std::smart_table::num_buckets(self)) {
        found = <a href="../../move-stdlib/doc/vector.md#0x1_vector_any">vector::any</a>(<a href="table_with_length.md#0x1_table_with_length_borrow">table_with_length::borrow</a>(aptos_std::smart_table::borrow_buckets(self), i), |elem| {
            <b>let</b> (key, value) = aptos_std::smart_table::borrow_kv(elem);
            p(key, value)
        });
        <b>if</b> (found) <b>break</b>;
        i = i + 1;
    };
    found
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow_kv"></a>

## Function `borrow_kv`



<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_kv">borrow_kv</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;): (&K, &V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_kv">borrow_kv</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt;): (&K, &V) {
    (&self.key, &self.value)
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow_kv_mut"></a>

## Function `borrow_kv_mut`



<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_kv_mut">borrow_kv_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;): (&<b>mut</b> K, &<b>mut</b> V)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_kv_mut">borrow_kv_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt;): (&<b>mut</b> K, &<b>mut</b> V) {
    (&<b>mut</b> self.key, &<b>mut</b> self.value)
}
</code></pre>



</details>

<a id="0x1_smart_table_num_buckets"></a>

## Function `num_buckets`



<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_num_buckets">num_buckets</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_num_buckets">num_buckets</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;): u64 {
    self.num_buckets
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow_buckets"></a>

## Function `borrow_buckets`



<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_buckets">borrow_buckets</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): &<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_buckets">borrow_buckets</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;): &TableWithLength&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt;&gt;&gt; {
    &self.buckets
}
</code></pre>



</details>

<a id="0x1_smart_table_borrow_buckets_mut"></a>

## Function `borrow_buckets_mut`



<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_buckets_mut">borrow_buckets_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): &<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_buckets_mut">borrow_buckets_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;): &<b>mut</b> TableWithLength&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">Entry</a>&lt;K, V&gt;&gt;&gt; {
    &<b>mut</b> self.buckets
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_SmartTable"></a>

### Struct `SmartTable`


<pre><code><b>struct</b> <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt; <b>has</b> store
</code></pre>



<dl>
<dt>
<code>buckets: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;&gt;&gt;</code>
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



<pre><code><b>pragma</b> intrinsic = map,
    map_new = new,
    map_destroy_empty = destroy_empty,
    map_len = length,
    map_has_key = contains,
    map_add_no_override = add,
    map_add_override_if_exists = upsert,
    map_del_must_exist = remove,
    map_borrow = borrow,
    map_borrow_mut = borrow_mut,
    map_borrow_mut_with_default = borrow_mut_with_default,
    map_spec_get = spec_get,
    map_spec_set = spec_set,
    map_spec_del = spec_remove,
    map_spec_len = spec_len,
map_spec_has_key = spec_contains;
</code></pre>



<a id="@Specification_1_new_with_config"></a>

### Function `new_with_config`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_new_with_config">new_with_config</a>&lt;K: <b>copy</b>, drop, store, V: store&gt;(num_initial_buckets: u64, split_load_threshold: u8, target_bucket_size: u64): <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_destroy"></a>

### Function `destroy`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_destroy">destroy</a>&lt;K: drop, V: drop&gt;(self: <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
</code></pre>



<a id="@Specification_1_clear"></a>

### Function `clear`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_clear">clear</a>&lt;K: drop, V: drop&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_add_all"></a>

### Function `add_all`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_add_all">add_all</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, keys: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, values: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;V&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_to_simple_map"></a>

### Function `to_simple_map`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_to_simple_map">to_simple_map</a>&lt;K: <b>copy</b>, drop, store, V: <b>copy</b>, store&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;K, V&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_keys"></a>

### Function `keys`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_keys">keys</a>&lt;K: <b>copy</b>, drop, store, V: <b>copy</b>, store&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_keys_paginated"></a>

### Function `keys_paginated`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_keys_paginated">keys_paginated</a>&lt;K: <b>copy</b>, drop, store, V: <b>copy</b>, store&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, starting_bucket_index: u64, starting_vector_index: u64, num_keys_to_get: u64): (<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;K&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;, <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_split_one_bucket"></a>

### Function `split_one_bucket`


<pre><code><b>fun</b> <a href="smart_table.md#0x1_smart_table_split_one_bucket">split_one_bucket</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_bucket_index"></a>

### Function `bucket_index`


<pre><code><b>fun</b> <a href="smart_table.md#0x1_smart_table_bucket_index">bucket_index</a>(level: u8, num_buckets: u64, <a href="../../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u64): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow_with_default"></a>

### Function `borrow_with_default`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_with_default">borrow_with_default</a>&lt;K: <b>copy</b>, drop, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, key: K, default: &V): &V
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_load_factor"></a>

### Function `load_factor`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_load_factor">load_factor</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_update_split_load_threshold"></a>

### Function `update_split_load_threshold`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_update_split_load_threshold">update_split_load_threshold</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, split_load_threshold: u8)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_update_target_bucket_size"></a>

### Function `update_target_bucket_size`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_update_target_bucket_size">update_target_bucket_size</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;, target_bucket_size: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow_kv"></a>

### Function `borrow_kv`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_kv">borrow_kv</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;): (&K, &V)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow_kv_mut"></a>

### Function `borrow_kv_mut`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_kv_mut">borrow_kv_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;): (&<b>mut</b> K, &<b>mut</b> V)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_num_buckets"></a>

### Function `num_buckets`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_num_buckets">num_buckets</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): u64
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow_buckets"></a>

### Function `borrow_buckets`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_buckets">borrow_buckets</a>&lt;K, V&gt;(self: &<a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): &<a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;&gt;&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_borrow_buckets_mut"></a>

### Function `borrow_buckets_mut`


<pre><code><b>public</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_borrow_buckets_mut">borrow_buckets_mut</a>&lt;K, V&gt;(self: &<b>mut</b> <a href="smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;K, V&gt;): &<b>mut</b> <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u64, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="smart_table.md#0x1_smart_table_Entry">smart_table::Entry</a>&lt;K, V&gt;&gt;&gt;
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_smart_table_spec_len"></a>


<pre><code><b>native</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_spec_len">spec_len</a>&lt;K, V&gt;(t: <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;): num;
</code></pre>




<a id="0x1_smart_table_spec_contains"></a>


<pre><code><b>native</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_spec_contains">spec_contains</a>&lt;K, V&gt;(t: <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, k: K): bool;
</code></pre>




<a id="0x1_smart_table_spec_set"></a>


<pre><code><b>native</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_spec_set">spec_set</a>&lt;K, V&gt;(t: <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, k: K, v: V): <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;;
</code></pre>




<a id="0x1_smart_table_spec_remove"></a>


<pre><code><b>native</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_spec_remove">spec_remove</a>&lt;K, V&gt;(t: <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, k: K): <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;;
</code></pre>




<a id="0x1_smart_table_spec_get"></a>


<pre><code><b>native</b> <b>fun</b> <a href="smart_table.md#0x1_smart_table_spec_get">spec_get</a>&lt;K, V&gt;(t: <a href="smart_table.md#0x1_smart_table_SmartTable">SmartTable</a>&lt;K, V&gt;, k: K): V;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
