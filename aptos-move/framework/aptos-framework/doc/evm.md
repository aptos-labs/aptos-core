
<a name="0x1_evm"></a>

# Module `0x1::evm`



-  [Struct `StorageKey`](#0x1_evm_StorageKey)
-  [Resource `EvmData`](#0x1_evm_EvmData)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_evm_initialize)
-  [Function `create_account`](#0x1_evm_create_account)
-  [Function `create`](#0x1_evm_create)
-  [Function `call`](#0x1_evm_call)
-  [Function `view`](#0x1_evm_view)
-  [Function `view2`](#0x1_evm_view2)
-  [Function `call_impl`](#0x1_evm_call_impl)
-  [Function `create2`](#0x1_evm_create2)
-  [Function `call2`](#0x1_evm_call2)
-  [Function `view_impl`](#0x1_evm_view_impl)
-  [Function `create_impl`](#0x1_evm_create_impl)
-  [Function `get_balance`](#0x1_evm_get_balance)
-  [Function `get_nonce`](#0x1_evm_get_nonce)
-  [Function `get_code`](#0x1_evm_get_code)
-  [Function `get_pub_key`](#0x1_evm_get_pub_key)
-  [Function `create_impl2`](#0x1_evm_create_impl2)
-  [Function `call_impl2`](#0x1_evm_call_impl2)
-  [Function `view_impl2`](#0x1_evm_view_impl2)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
</code></pre>



<a name="0x1_evm_StorageKey"></a>

## Struct `StorageKey`



<pre><code><b>struct</b> <a href="evm.md#0x1_evm_StorageKey">StorageKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>contract_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>offset: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_evm_EvmData"></a>

## Resource `EvmData`



<pre><code><b>struct</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>nonce: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>balance: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>storage: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="evm.md#0x1_evm_StorageKey">evm::StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pub_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_evm_ENO_ETH_DATA"></a>

Aptos framework doesn't have ETH Data resource


<pre><code><b>const</b> <a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>: u64 = 1;
</code></pre>



<a name="0x1_evm_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, eth_faucet_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, eth_faucet_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (<b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework)) {
        <b>return</b>;
    };
    <b>let</b> balance = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_upsert">table::upsert</a>(&<b>mut</b> balance, eth_faucet_address, 1000000000000);
    <b>let</b> nonce = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_upsert">table::upsert</a>(&<b>mut</b> nonce, eth_faucet_address, 1);
    <b>let</b> <a href="code.md#0x1_code">code</a> = <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>();
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_upsert">table::upsert</a>(&<b>mut</b> <a href="code.md#0x1_code">code</a>, eth_faucet_address, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;());
    <b>move_to</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(aptos_framework, <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
        nonce: nonce,
        balance: balance,
        <a href="code.md#0x1_code">code</a>: <a href="code.md#0x1_code">code</a>,
        storage: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        pub_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
    });
}
</code></pre>



</details>

<a name="0x1_evm_create_account"></a>

## Function `create_account`



<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_create_account">create_account</a>(eth_addr: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, pub_key: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_create_account">create_account</a>(eth_addr: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, pub_key: <b>address</b>) <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    // Make sure <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> of pubkey is the same <b>as</b> eth_addr
    // Keccack256(pub_key) | (Truncate it by 160 bit) == eth_addr.value

    //TODO: How <b>to</b> borrow <b>mut</b>?
    <b>let</b> data_ref = <b>borrow_global_mut</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_upsert">table::upsert</a>(&<b>mut</b> data_ref.pub_keys, eth_addr, pub_key);
}
</code></pre>



</details>

<a name="0x1_evm_create"></a>

## Function `create`



<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_create">create</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_create">create</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    //TODO: How <b>to</b> borrow <b>mut</b>?
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    <a href="evm.md#0x1_evm_create_impl">create_impl</a>(&data_ref.nonce, &data_ref.balance, &data_ref.<a href="code.md#0x1_code">code</a>, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature);
}
</code></pre>



</details>

<a name="0x1_evm_call"></a>

## Function `call`



<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_call">call</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_call">call</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    print(&data_ref.balance);
    <a href="evm.md#0x1_evm_call_impl">call_impl</a>(&data_ref.nonce, &data_ref.balance, &data_ref.<a href="code.md#0x1_code">code</a>, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature);
}
</code></pre>



</details>

<a name="0x1_evm_view"></a>

## Function `view`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_view">view</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_view">view</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);

    <a href="evm.md#0x1_evm_view_impl">view_impl</a>(&data_ref.nonce, &data_ref.balance, &data_ref.<a href="code.md#0x1_code">code</a>, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature)
}
</code></pre>



</details>

<a name="0x1_evm_view2"></a>

## Function `view2`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_view2">view2</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_view2">view2</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);

    <a href="evm.md#0x1_evm_view_impl2">view_impl2</a>(&data_ref.nonce, &data_ref.balance, &data_ref.<a href="code.md#0x1_code">code</a>, &data_ref.storage, &data_ref.pub_keys, caller, <b>address</b>, value, data)
}
</code></pre>



</details>

<a name="0x1_evm_call_impl"></a>

## Function `call_impl`



<pre><code><b>fun</b> <a href="evm.md#0x1_evm_call_impl">call_impl</a>(nonce: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="evm.md#0x1_evm_StorageKey">evm::StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="evm.md#0x1_evm_call_impl">call_impl</a>(nonce: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &Table&lt;<a href="evm.md#0x1_evm_StorageKey">StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_evm_create2"></a>

## Function `create2`



<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_create2">create2</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, init_code: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_create2">create2</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, init_code: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64) <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {

    //TODO: How <b>to</b> borrow <b>mut</b>?
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    <a href="evm.md#0x1_evm_create_impl2">create_impl2</a>(&data_ref.nonce, &data_ref.balance, &data_ref.<a href="code.md#0x1_code">code</a>, &data_ref.storage, &data_ref.pub_keys, caller, value, init_code, gas_limit);
}
</code></pre>



</details>

<a name="0x1_evm_call2"></a>

## Function `call2`



<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_call2">call2</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="evm.md#0x1_evm_call2">call2</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64) <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    print(&data_ref.balance);
    <a href="evm.md#0x1_evm_call_impl2">call_impl2</a>(&data_ref.nonce, &data_ref.balance, &data_ref.<a href="code.md#0x1_code">code</a>, &data_ref.storage, &data_ref.pub_keys, caller, <b>address</b>, value, data, gas_limit);
}
</code></pre>



</details>

<a name="0x1_evm_view_impl"></a>

## Function `view_impl`



<pre><code><b>fun</b> <a href="evm.md#0x1_evm_view_impl">view_impl</a>(nonce: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="evm.md#0x1_evm_StorageKey">evm::StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="evm.md#0x1_evm_view_impl">view_impl</a>(nonce: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &Table&lt;<a href="evm.md#0x1_evm_StorageKey">StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_evm_create_impl"></a>

## Function `create_impl`



<pre><code><b>fun</b> <a href="evm.md#0x1_evm_create_impl">create_impl</a>(nonce: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="evm.md#0x1_evm_StorageKey">evm::StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="evm.md#0x1_evm_create_impl">create_impl</a>(nonce: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &Table&lt;<a href="evm.md#0x1_evm_StorageKey">StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;);
</code></pre>



</details>

<a name="0x1_evm_get_balance"></a>

## Function `get_balance`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_balance">get_balance</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_balance">get_balance</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256 <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&data_ref.balance, caller)
}
</code></pre>



</details>

<a name="0x1_evm_get_nonce"></a>

## Function `get_nonce`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_nonce">get_nonce</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_nonce">get_nonce</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256 <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&data_ref.nonce, caller)
}
</code></pre>



</details>

<a name="0x1_evm_get_code"></a>

## Function `get_code`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_code">get_code</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_code">get_code</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&data_ref.<a href="code.md#0x1_code">code</a>, caller)
}
</code></pre>



</details>

<a name="0x1_evm_get_pub_key"></a>

## Function `get_pub_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_pub_key">get_pub_key</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="evm.md#0x1_evm_get_pub_key">get_pub_key</a>(caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b> <b>acquires</b> <a href="evm.md#0x1_evm_EvmData">EvmData</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="evm.md#0x1_evm_ENO_ETH_DATA">ENO_ETH_DATA</a>),
    );
    <b>let</b> data_ref = <b>borrow_global</b>&lt;<a href="evm.md#0x1_evm_EvmData">EvmData</a>&gt;(@aptos_framework);
    *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&data_ref.pub_keys, caller)
}
</code></pre>



</details>

<a name="0x1_evm_create_impl2"></a>

## Function `create_impl2`



<pre><code><b>fun</b> <a href="evm.md#0x1_evm_create_impl2">create_impl2</a>(nonce: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="evm.md#0x1_evm_StorageKey">evm::StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, init_code: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="evm.md#0x1_evm_create_impl2">create_impl2</a>(nonce: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &Table&lt;<a href="evm.md#0x1_evm_StorageKey">StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, init_code: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_evm_call_impl2"></a>

## Function `call_impl2`



<pre><code><b>fun</b> <a href="evm.md#0x1_evm_call_impl2">call_impl2</a>(nonce: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="evm.md#0x1_evm_StorageKey">evm::StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="evm.md#0x1_evm_call_impl2">call_impl2</a>(nonce: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &Table&lt;<a href="evm.md#0x1_evm_StorageKey">StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gas_limit: u64) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_evm_view_impl2"></a>

## Function `view_impl2`



<pre><code><b>fun</b> <a href="evm.md#0x1_evm_view_impl2">view_impl2</a>(nonce: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="evm.md#0x1_evm_StorageKey">evm::StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &<a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="evm.md#0x1_evm_view_impl2">view_impl2</a>(nonce: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, balance: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, u256&gt;, <a href="code.md#0x1_code">code</a>: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, storage: &Table&lt;<a href="evm.md#0x1_evm_StorageKey">StorageKey</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, pub_keys: &Table&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>&gt;, caller: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <b>address</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, data: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
