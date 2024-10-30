
<a id="0x1_nonce_validation_bi_ordered_map"></a>

# Module `0x1::nonce_validation_bi_ordered_map`



-  [Struct `Bucket`](#0x1_nonce_validation_bi_ordered_map_Bucket)
-  [Struct `NonceKey`](#0x1_nonce_validation_bi_ordered_map_NonceKey)
-  [Resource `NonceHistory`](#0x1_nonce_validation_bi_ordered_map_NonceHistory)
-  [Function `initialize`](#0x1_nonce_validation_bi_ordered_map_initialize)
-  [Function `initialize_nonce_table`](#0x1_nonce_validation_bi_ordered_map_initialize_nonce_table)
-  [Function `add_nonce_bucket`](#0x1_nonce_validation_bi_ordered_map_add_nonce_bucket)
-  [Function `check_and_insert_nonce`](#0x1_nonce_validation_bi_ordered_map_check_and_insert_nonce)
-  [Function `check_nonce`](#0x1_nonce_validation_bi_ordered_map_check_nonce)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="ordered_map.md#0x1_ordered_map">0x1::ordered_map</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_nonce_validation_bi_ordered_map_Bucket"></a>

## Struct `Bucket`



<pre><code><b>struct</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_Bucket">Bucket</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>nonces: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;<a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceKey">nonce_validation_bi_ordered_map::NonceKey</a>, u64&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>last_stored_times: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_nonce_validation_bi_ordered_map_NonceKey"></a>

## Struct `NonceKey`



<pre><code><b>struct</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceKey">NonceKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sender_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>nonce: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_nonce_validation_bi_ordered_map_NonceHistory"></a>

## Resource `NonceHistory`



<pre><code><b>struct</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>nonce_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_Bucket">nonce_validation_bi_ordered_map::Bucket</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>next_key: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_nonce_validation_bi_ordered_map_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_initialize_nonce_table">initialize_nonce_table</a>(aptos_framework);
}
</code></pre>



</details>

<a id="0x1_nonce_validation_bi_ordered_map_initialize_nonce_table"></a>

## Function `initialize_nonce_table`



<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_initialize_nonce_table">initialize_nonce_table</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_initialize_nonce_table">initialize_nonce_table</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>if</b> (!<b>exists</b>&lt;<a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a>&gt;(@aptos_framework)) {
        <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
        <b>let</b> nonce_history = <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a> {
            nonce_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>,
            next_key: 0,
        };
        <b>move_to</b>&lt;<a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a>&gt;(aptos_framework, nonce_history);
    };
}
</code></pre>



</details>

<a id="0x1_nonce_validation_bi_ordered_map_add_nonce_bucket"></a>

## Function `add_nonce_bucket`



<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_add_nonce_bucket">add_nonce_bucket</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_add_nonce_bucket">add_nonce_bucket</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a>&gt;(@aptos_framework)) {
        <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
        <b>let</b> nonce_history = <b>borrow_global_mut</b>&lt;<a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a>&gt;(@aptos_framework);
        <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&nonce_history.nonce_table, nonce_history.next_key)) {
            <b>let</b> nonces = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
            <b>let</b> last_stored_times = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> nonces, <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>());
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> nonces, <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>());
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> last_stored_times, current_time);
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> last_stored_times, current_time);
            <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> nonce_history.nonce_table, nonce_history.next_key, <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_Bucket">Bucket</a> {
                nonces: nonces,
                last_stored_times: last_stored_times,
            });
        };
        nonce_history.next_key = nonce_history.next_key + 1;
    };
}
</code></pre>



</details>

<a id="0x1_nonce_validation_bi_ordered_map_check_and_insert_nonce"></a>

## Function `check_and_insert_nonce`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_check_and_insert_nonce">check_and_insert_nonce</a>(sender_address: <b>address</b>, nonce: u64, txn_expiration_time: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_check_and_insert_nonce">check_and_insert_nonce</a>(
    sender_address: <b>address</b>,
    nonce: u64,
    txn_expiration_time: u64,
): bool <b>acquires</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a> {
    <b>let</b> nonce_history = <b>borrow_global_mut</b>&lt;<a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a>&gt;(@aptos_framework);
    <b>let</b> nonce_key = <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceKey">NonceKey</a> {
        sender_address,
        nonce,
    };
    <b>let</b> index = sip_hash_from_value(&nonce_key) % 50000;
    <b>let</b> map_index = (txn_expiration_time/75) % 2;
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&nonce_history.nonce_table, index)) {
        <b>let</b> nonces = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
        <b>let</b> last_stored_times = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> nonces, <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>());
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> nonces, <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>());
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> last_stored_times, <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>());
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> last_stored_times, <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>());
        <a href="ordered_map.md#0x1_ordered_map_add">ordered_map::add</a>(&<b>mut</b> nonces[map_index], nonce_key, txn_expiration_time);
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> nonce_history.nonce_table, index, <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_Bucket">Bucket</a> {
            nonces: nonces,
            last_stored_times: last_stored_times,
        });
        <b>return</b> <b>true</b>;
    };
    <b>let</b> bucket = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> nonce_history.nonce_table, index);
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>if</b> (bucket.last_stored_times[1-map_index] &lt; (current_time - 60)) {
        <b>if</b> (<a href="ordered_map.md#0x1_ordered_map_length">ordered_map::length</a>(&bucket.nonces[1-map_index]) &gt; 0) {
            bucket.nonces[1-map_index] = <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>();
        }
    };
    <b>if</b> (<a href="ordered_map.md#0x1_ordered_map_contains">ordered_map::contains</a>(&bucket.nonces[1-map_index], &nonce_key)) {
        <b>return</b> <b>false</b>
    };
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="ordered_map.md#0x1_ordered_map_upsert">ordered_map::upsert</a>(&<b>mut</b> bucket.nonces[map_index], nonce_key, txn_expiration_time))) {
        <b>return</b> <b>false</b>
    };
    bucket.last_stored_times[map_index] = current_time;
    <b>true</b>
}
</code></pre>



</details>

<a id="0x1_nonce_validation_bi_ordered_map_check_nonce"></a>

## Function `check_nonce`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_check_nonce">check_nonce</a>(sender_address: <b>address</b>, nonce: u64, txn_expiration_time: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_check_nonce">check_nonce</a>(
    sender_address: <b>address</b>,
    nonce: u64,
    txn_expiration_time: u64,
): bool <b>acquires</b> <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a> {
    <b>let</b> nonce_key = <a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceKey">NonceKey</a> {
        sender_address,
        nonce,
    };
    <b>let</b> index = sip_hash_from_value(&nonce_key) % 50000;
    <b>let</b> nonce_history = <b>borrow_global</b>&lt;<a href="nonce_validation_bi_ordered_map.md#0x1_nonce_validation_bi_ordered_map_NonceHistory">NonceHistory</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&nonce_history.nonce_table, index)) {
        <b>let</b> bucket = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&nonce_history.nonce_table, index);
        <b>if</b> (<a href="ordered_map.md#0x1_ordered_map_contains">ordered_map::contains</a>(&bucket.nonces[0], &nonce_key)) {
            <b>return</b> <b>false</b>
        };
        <b>if</b> (<a href="ordered_map.md#0x1_ordered_map_contains">ordered_map::contains</a>(&bucket.nonces[1], &nonce_key)) {
            <b>return</b> <b>false</b>
        };
    };
    <b>true</b>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
