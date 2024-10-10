
<a id="0x1_nonce_validation"></a>

# Module `0x1::nonce_validation`



-  [Struct `NonceKey`](#0x1_nonce_validation_NonceKey)
-  [Resource `NonceHistory`](#0x1_nonce_validation_NonceHistory)
-  [Resource `NonceHistorySignerCap`](#0x1_nonce_validation_NonceHistorySignerCap)
-  [Function `initialize`](#0x1_nonce_validation_initialize)
-  [Function `insert_nonce`](#0x1_nonce_validation_insert_nonce)
-  [Function `nonce_exists`](#0x1_nonce_validation_nonce_exists)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ordered_map.md#0x1_ordered_map">0x1::ordered_map</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
</code></pre>



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

<a id="0x1_nonce_validation_NonceHistory"></a>

## Resource `NonceHistory`



<pre><code><b>struct</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>table_1: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/doc/ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceKey">nonce_validation::NonceKey</a>, u64&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_nonce_validation_NonceHistorySignerCap"></a>

## Resource `NonceHistorySignerCap`



<pre><code><b>struct</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistorySignerCap">NonceHistorySignerCap</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_nonce_validation_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>let</b> table_1 = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
    <b>let</b> nonce_history = <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> {
        table_1,
    };

    <b>move_to</b>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>&gt;(aptos_framework, nonce_history);
}
</code></pre>



</details>

<a id="0x1_nonce_validation_insert_nonce"></a>

## Function `insert_nonce`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_insert_nonce">insert_nonce</a>(sender_address: <b>address</b>, nonce: u64, txn_expiration_time: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_insert_nonce">insert_nonce</a>(
    sender_address: <b>address</b>,
    nonce: u64,
    txn_expiration_time: u64,
): bool <b>acquires</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> {
    <b>let</b> nonce_history = <b>borrow_global_mut</b>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>&gt;(@aptos_framework);
    <b>let</b> nonce_key = <a href="nonce_validation.md#0x1_nonce_validation_NonceKey">NonceKey</a> {
        sender_address,
        nonce,
    };
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = sip_hash_from_value(&nonce_key);
    <b>let</b> index = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> % 100000;
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&nonce_history.table_1, index)) {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> nonce_history.table_1, index, <a href="../../aptos-stdlib/doc/ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>());
    };
    <a href="../../aptos-stdlib/doc/ordered_map.md#0x1_ordered_map_add">ordered_map::add</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> nonce_history.table_1, index), nonce_key, txn_expiration_time)
}
</code></pre>



</details>

<a id="0x1_nonce_validation_nonce_exists"></a>

## Function `nonce_exists`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_nonce_exists">nonce_exists</a>(sender_address: <b>address</b>, nonce: u64, txn_expiration_time: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="nonce_validation.md#0x1_nonce_validation_nonce_exists">nonce_exists</a>(
    sender_address: <b>address</b>,
    nonce: u64,
    txn_expiration_time: u64,
): bool <b>acquires</b> <a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a> {
    <b>let</b> nonce_key = <a href="nonce_validation.md#0x1_nonce_validation_NonceKey">NonceKey</a> {
        sender_address,
        nonce,
    };
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = sip_hash_from_value(&nonce_key);
    <b>let</b> index = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> % 100000;
    <b>let</b> nonce_history = <b>borrow_global</b>&lt;<a href="nonce_validation.md#0x1_nonce_validation_NonceHistory">NonceHistory</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&nonce_history.table_1, index)) {
        <b>if</b> (<a href="../../aptos-stdlib/doc/ordered_map.md#0x1_ordered_map_contains">ordered_map::contains</a>(<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&nonce_history.table_1, index), &nonce_key)) {
            <b>return</b> <b>true</b>;
        }
    };
    <b>false</b>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
