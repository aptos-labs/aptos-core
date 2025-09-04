
<a id="0x1_common_account_abstractions_utils"></a>

# Module `0x1::common_account_abstractions_utils`



-  [Function `network_name`](#0x1_common_account_abstractions_utils_network_name)
-  [Function `entry_function_name`](#0x1_common_account_abstractions_utils_entry_function_name)


<pre><code><b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../velor-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_common_account_abstractions_utils_network_name"></a>

## Function `network_name`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_network_name">network_name</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_network_name">network_name</a>(): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> <a href="chain_id.md#0x1_chain_id">chain_id</a> = <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>();
    <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 1) {
        b"mainnet"
    } <b>else</b> <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 2) {
        b"testnet"
    } <b>else</b> <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 4) {
        b"<b>local</b>"
    } <b>else</b> {
        <b>let</b> network_name = &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
        network_name.append(b"custom network: ");
        network_name.append(*<a href="../../velor-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(&<a href="chain_id.md#0x1_chain_id">chain_id</a>).bytes());
        *network_name
    }
}
</code></pre>



</details>

<a id="0x1_common_account_abstractions_utils_entry_function_name"></a>

## Function `entry_function_name`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_entry_function_name">entry_function_name</a>(entry_function_payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_entry_function_name">entry_function_name</a>(entry_function_payload: &EntryFunctionPayload): <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> entry_function_name = &<b>mut</b> <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> addr_str = <a href="../../velor-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(
        &<a href="transaction_context.md#0x1_transaction_context_account_address">transaction_context::account_address</a>(entry_function_payload)
    ).bytes();
    // .slice(1) <b>to</b> remove the leading '@' char
    entry_function_name.append(addr_str.slice(1, addr_str.length()));
    entry_function_name.append(b"::");
    entry_function_name.append(
        *<a href="transaction_context.md#0x1_transaction_context_module_name">transaction_context::module_name</a>(entry_function_payload).bytes()
    );
    entry_function_name.append(b"::");
    entry_function_name.append(
        *<a href="transaction_context.md#0x1_transaction_context_function_name">transaction_context::function_name</a>(entry_function_payload).bytes()
    );
    *entry_function_name
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
