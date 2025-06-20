
<a id="0x1_nonce_validation"></a>

# Module `0x1::nonce_validation`



-  [Resource `NonceHistory`](#0x1_nonce_validation_NonceHistory)
-  [Struct `Bucket`](#0x1_nonce_validation_Bucket)
-  [Struct `NonceKeyWithExpTime`](#0x1_nonce_validation_NonceKeyWithExpTime)
-  [Struct `NonceKey`](#0x1_nonce_validation_NonceKey)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_nonce_validation_initialize)
-  [Function `initialize_nonce_table`](#0x1_nonce_validation_initialize_nonce_table)
-  [Function `empty_bucket`](#0x1_nonce_validation_empty_bucket)
-  [Function `add_nonce_buckets`](#0x1_nonce_validation_add_nonce_buckets)
-  [Function `check_and_insert_nonce`](#0x1_nonce_validation_check_and_insert_nonce)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_nonce_validation_NonceHistory"></a>

## Resource `NonceHistory`



<pre><code><b>struct</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>nonce_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="nonce_validation.md#0x1_nonce_validation_Bucket">nonce_validation::Bucket</a>&gt;</code>
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

<a id="0x1_nonce_validation_Bucket"></a>

## Struct `Bucket`



<pre><code><b>struct</b> <a href="nonce_validation.md#0x1_nonce_validation_Bucket">Bucket</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>nonces_ordered_by_exp_time: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceKeyWithExpTime">nonce_validation::NonceKeyWithExpTime</a>, bool&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>nonce_to_exp_time_map: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceKey">nonce_validation::NonceKey</a>, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_nonce_validation_NonceKeyWithExpTime"></a>

## Struct `NonceKeyWithExpTime`



<pre><code><b>struct</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceKeyWithExpTime">NonceKeyWithExpTime</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>txn_expiration_time: u64</code>
</dt>
<dd>

</dd>
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

<a id="0x1_nonce_validation_NonceKey"></a>

## Struct `NonceKey`



<pre><code><b>struct</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceKey">NonceKey</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_nonce_validation_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE"></a>



<pre><code><b>const</b> <a href="nonce_validation.md#0x1_nonce_validation_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE">ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE</a>: u64 = 1002;
</code></pre>



<a id="0x1_nonce_validation_E_NONCE_HISTORY_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="nonce_validation.md#0x1_nonce_validation_E_NONCE_HISTORY_DOES_NOT_EXIST">E_NONCE_HISTORY_DOES_NOT_EXIST</a>: u64 = 1001;
</code></pre>



<a id="0x1_nonce_validation_MAX_ENTRIES_GARBAGE_COLLECTED_PER_CALL"></a>



<pre><code><b>const</b> <a href="nonce_validation.md#0x1_nonce_validation_MAX_ENTRIES_GARBAGE_COLLECTED_PER_CALL">MAX_ENTRIES_GARBAGE_COLLECTED_PER_CALL</a>: u64 = 5;
</code></pre>



<a id="0x1_nonce_validation_NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS"></a>



<pre><code><b>const</b> <a href="nonce_validation.md#0x1_nonce_validation_NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS">NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS</a>: u64 = 65;
</code></pre>



<a id="0x1_nonce_validation_NUM_BUCKETS"></a>



<pre><code><b>const</b> <a href="nonce_validation.md#0x1_nonce_validation_NUM_BUCKETS">NUM_BUCKETS</a>: u64 = 50000;
</code></pre>



<a id="0x1_nonce_validation_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="nonce_validation.md#0x1_nonce_validation_initialize_nonce_table">initialize_nonce_table</a>(aptos_framework);
}
</code></pre>



</details>

<a id="0x1_nonce_validation_initialize_nonce_table"></a>

## Function `initialize_nonce_table`



<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_initialize_nonce_table">initialize_nonce_table</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_initialize_nonce_table">initialize_nonce_table</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>&gt;(@aptos_framework)) {
        <b>let</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
        <b>let</b> nonce_history = <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> {
            nonce_table: <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>,
            next_key: 0,
        };
        <b>move_to</b>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>&gt;(aptos_framework, nonce_history);
    };
}
</code></pre>



</details>

<a id="0x1_nonce_validation_empty_bucket"></a>

## Function `empty_bucket`



<pre><code><b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_empty_bucket">empty_bucket</a>(pre_allocate_slots: bool): <a href="nonce_validation.md#0x1_nonce_validation_Bucket">nonce_validation::Bucket</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_empty_bucket">empty_bucket</a>(pre_allocate_slots: bool): <a href="nonce_validation.md#0x1_nonce_validation_Bucket">Bucket</a> {
    <b>let</b> bucket = <a href="nonce_validation.md#0x1_nonce_validation_Bucket">Bucket</a> {
        nonces_ordered_by_exp_time: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_reusable">big_ordered_map::new_with_reusable</a>(),
        nonce_to_exp_time_map: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_reusable">big_ordered_map::new_with_reusable</a>(),
    };

    <b>if</b> (pre_allocate_slots) {
        // Initiating big ordered maps <b>with</b> 5 pre-allocated storage slots.
        // (expiration time, <b>address</b>, nonce) is together 48 bytes.
        // A 4 KB storage slot can store 80+ such tuples.
        // The 5 slots should be more than enough for the current <b>use</b> case.
        bucket.nonces_ordered_by_exp_time.allocate_spare_slots(5);
        bucket.nonce_to_exp_time_map.allocate_spare_slots(5);
    };
    bucket
}
</code></pre>



</details>

<a id="0x1_nonce_validation_add_nonce_buckets"></a>

## Function `add_nonce_buckets`



<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_add_nonce_buckets">add_nonce_buckets</a>(count: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_add_nonce_buckets">add_nonce_buckets</a>(count: u64) <b>acquires</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>&gt;(@aptos_framework), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="nonce_validation.md#0x1_nonce_validation_E_NONCE_HISTORY_DOES_NOT_EXIST">E_NONCE_HISTORY_DOES_NOT_EXIST</a>));
    <b>let</b> nonce_history = &<b>mut</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>[@aptos_framework];
    for (i in 0..count) {
        <b>if</b> (nonce_history.next_key &lt;= <a href="nonce_validation.md#0x1_nonce_validation_NUM_BUCKETS">NUM_BUCKETS</a>) {
            <b>if</b> (!nonce_history.nonce_table.contains(nonce_history.next_key)) {
                nonce_history.nonce_table.add(
                    nonce_history.next_key,
                    <a href="nonce_validation.md#0x1_nonce_validation_empty_bucket">empty_bucket</a>(<b>true</b>)
                );
            };
            nonce_history.next_key = nonce_history.next_key + 1;
        }
    }
}
</code></pre>



</details>

<a id="0x1_nonce_validation_check_and_insert_nonce"></a>

## Function `check_and_insert_nonce`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_check_and_insert_nonce">check_and_insert_nonce</a>(sender_address: <b>address</b>, nonce: u64, txn_expiration_time: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_check_and_insert_nonce">check_and_insert_nonce</a>(
    sender_address: <b>address</b>,
    nonce: u64,
    txn_expiration_time: u64,
): bool <b>acquires</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>&gt;(@aptos_framework), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="nonce_validation.md#0x1_nonce_validation_E_NONCE_HISTORY_DOES_NOT_EXIST">E_NONCE_HISTORY_DOES_NOT_EXIST</a>));
    // Check <b>if</b> the transaction expiration time is too far in the future.
    <b>assert</b>!(txn_expiration_time &lt;= <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() + <a href="nonce_validation.md#0x1_nonce_validation_NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS">NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="nonce_validation.md#0x1_nonce_validation_ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE">ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE</a>));
    <b>let</b> nonce_history = &<b>mut</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>[@aptos_framework];
    <b>let</b> nonce_key = <a href="nonce_validation.md#0x1_nonce_validation_NonceKey">NonceKey</a> {
        sender_address,
        nonce,
    };
    <b>let</b> bucket_index = sip_hash_from_value(&nonce_key) % <a href="nonce_validation.md#0x1_nonce_validation_NUM_BUCKETS">NUM_BUCKETS</a>;
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>if</b> (!nonce_history.nonce_table.contains(bucket_index)) {
        nonce_history.nonce_table.add(
            bucket_index,
            <a href="nonce_validation.md#0x1_nonce_validation_empty_bucket">empty_bucket</a>(<b>false</b>)
        );
    };
    <b>let</b> bucket = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> nonce_history.nonce_table, bucket_index);

    <b>let</b> existing_exp_time = bucket.nonce_to_exp_time_map.get(&nonce_key);
    <b>if</b> (existing_exp_time.is_some()) {
        <b>let</b> existing_exp_time = existing_exp_time.extract();

        // If the existing (<b>address</b>, nonce) pair <b>has</b> not expired, <b>return</b> <b>false</b>.
        <b>if</b> (existing_exp_time &gt;= current_time) {
            <b>return</b> <b>false</b>;
        };

        // We maintain an <b>invariant</b> that two transaction <b>with</b> the same (<b>address</b>, nonce) pair cannot be stored
        // in the nonce history <b>if</b> their transaction expiration times are less than `<a href="nonce_validation.md#0x1_nonce_validation_NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS">NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS</a>`
        // seconds apart.
        <b>if</b> (txn_expiration_time &lt;= existing_exp_time + <a href="nonce_validation.md#0x1_nonce_validation_NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS">NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS</a>) {
            <b>return</b> <b>false</b>;
        };

        // If the existing (<b>address</b>, nonce) pair <b>has</b> expired, garbage collect it.
        bucket.nonce_to_exp_time_map.remove(&nonce_key);
        bucket.nonces_ordered_by_exp_time.remove(&<a href="nonce_validation.md#0x1_nonce_validation_NonceKeyWithExpTime">NonceKeyWithExpTime</a> {
            txn_expiration_time: existing_exp_time,
            sender_address,
            nonce,
        });
    };

    // Garbage collect upto <a href="nonce_validation.md#0x1_nonce_validation_MAX_ENTRIES_GARBAGE_COLLECTED_PER_CALL">MAX_ENTRIES_GARBAGE_COLLECTED_PER_CALL</a> expired nonces in the bucket.
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="nonce_validation.md#0x1_nonce_validation_MAX_ENTRIES_GARBAGE_COLLECTED_PER_CALL">MAX_ENTRIES_GARBAGE_COLLECTED_PER_CALL</a> && !bucket.nonces_ordered_by_exp_time.is_empty()) {
        <b>let</b> (front_k, _) = bucket.nonces_ordered_by_exp_time.borrow_front();
        // We garbage collect a nonce after it <b>has</b> expired and the <a href="nonce_validation.md#0x1_nonce_validation_NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS">NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS</a>
        // seconds have passed.
        <b>if</b> (front_k.txn_expiration_time + <a href="nonce_validation.md#0x1_nonce_validation_NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS">NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS</a> &lt; current_time) {
            bucket.nonces_ordered_by_exp_time.pop_front();
            bucket.nonce_to_exp_time_map.remove(&<a href="nonce_validation.md#0x1_nonce_validation_NonceKey">NonceKey</a> {
                sender_address: front_k.sender_address,
                nonce: front_k.nonce,
            });
        } <b>else</b> {
            <b>break</b>;
        };
        i = i + 1;
    };

    // Insert the (<b>address</b>, nonce) pair in the bucket.
    <b>let</b> nonce_key_with_exp_time = <a href="nonce_validation.md#0x1_nonce_validation_NonceKeyWithExpTime">NonceKeyWithExpTime</a> {
        txn_expiration_time,
        sender_address,
        nonce,
    };
    bucket.nonces_ordered_by_exp_time.add(nonce_key_with_exp_time, <b>true</b>);
    bucket.nonce_to_exp_time_map.add(nonce_key, txn_expiration_time);
    <b>true</b>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
