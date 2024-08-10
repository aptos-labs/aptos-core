
<a id="0x1_unique_key"></a>

# Module `0x1::unique_key`



-  [Resource `Counters`](#0x1_unique_key_Counters)
-  [Constants](#@Constants_0)
-  [Function `initialize_at_address`](#0x1_unique_key_initialize_at_address)
-  [Function `generate_unique_key`](#0x1_unique_key_generate_unique_key)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
</code></pre>



<a id="0x1_unique_key_Counters"></a>

## Resource `Counters`



<pre><code><b>struct</b> <a href="unique_key.md#0x1_unique_key_Counters">Counters</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>values: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;u16, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_unique_key_EINDEX_OUT_OF_BOUNDS"></a>

The index is out of bounds


<pre><code><b>const</b> <a href="unique_key.md#0x1_unique_key_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>: u64 = 1;
</code></pre>



<a id="0x1_unique_key_EUNAUTHORIZED"></a>

Unauthorized


<pre><code><b>const</b> <a href="unique_key.md#0x1_unique_key_EUNAUTHORIZED">EUNAUTHORIZED</a>: u64 = 1;
</code></pre>



<a id="0x1_unique_key_initialize_at_address"></a>

## Function `initialize_at_address`



<pre><code>entry <b>fun</b> <a href="unique_key.md#0x1_unique_key_initialize_at_address">initialize_at_address</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="unique_key.md#0x1_unique_key_initialize_at_address">initialize_at_address</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner) == @aptos_framework, <a href="unique_key.md#0x1_unique_key_EUNAUTHORIZED">EUNAUTHORIZED</a>);

    <b>let</b> counter = <a href="unique_key.md#0x1_unique_key_Counters">Counters</a> {
        values: <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_new">table_with_length::new</a>(),
    };

    <b>let</b> i = 0;
    <b>while</b> (i &lt; 256) {
        <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_add">table_with_length::add</a>(&<b>mut</b> counter.values, i, 0);
        i = i + 1;
    };

    <b>move_to</b>(owner, counter);
}
</code></pre>



</details>

<a id="0x1_unique_key_generate_unique_key"></a>

## Function `generate_unique_key`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="unique_key.md#0x1_unique_key_generate_unique_key">generate_unique_key</a>(counter_addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="unique_key.md#0x1_unique_key_generate_unique_key">generate_unique_key</a>(counter_addr: <b>address</b>): u64 <b>acquires</b> <a href="unique_key.md#0x1_unique_key_Counters">Counters</a> {
    <b>let</b> index = (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">transaction_context::get_transaction_hash</a>(), 0) <b>as</b> u16);
    <b>assert</b>!(index &lt; 256, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="unique_key.md#0x1_unique_key_EINDEX_OUT_OF_BOUNDS">EINDEX_OUT_OF_BOUNDS</a>));

    <b>let</b> counters = <b>borrow_global_mut</b>&lt;<a href="unique_key.md#0x1_unique_key_Counters">Counters</a>&gt;(counter_addr);
    <b>let</b> value = <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length_borrow_mut">table_with_length::borrow_mut</a>(&<b>mut</b> counters.values, index);
    *value = *value + 1;

    *value * 256 + (index <b>as</b> u64)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
