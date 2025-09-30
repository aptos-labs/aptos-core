
<a id="0x1_common_account_abstractions_utils"></a>

# Module `0x1::common_account_abstractions_utils`



-  [Constants](#@Constants_0)
-  [Function `network_name`](#0x1_common_account_abstractions_utils_network_name)
-  [Function `entry_function_name`](#0x1_common_account_abstractions_utils_entry_function_name)
-  [Function `construct_message`](#0x1_common_account_abstractions_utils_construct_message)
-  [Function `daa_authenticate`](#0x1_common_account_abstractions_utils_daa_authenticate)


<pre><code><b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_common_account_abstractions_utils_EMISSING_ENTRY_FUNCTION_PAYLOAD"></a>

Entry function payload is missing.


<pre><code><b>const</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>: u64 = 1;
</code></pre>



<a id="0x1_common_account_abstractions_utils_network_name"></a>

## Function `network_name`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_network_name">network_name</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_network_name">network_name</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> <a href="chain_id.md#0x1_chain_id">chain_id</a> = <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>();
    <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 1) {
        b"mainnet"
    } <b>else</b> <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 2) {
        b"testnet"
    } <b>else</b> <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 4) {
        b"<b>local</b>"
    } <b>else</b> {
        <b>let</b> network_name = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
        network_name.append(b"custom network: ");
        network_name.append(*<a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(&<a href="chain_id.md#0x1_chain_id">chain_id</a>).bytes());
        *network_name
    }
}
</code></pre>



</details>

<a id="0x1_common_account_abstractions_utils_entry_function_name"></a>

## Function `entry_function_name`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_entry_function_name">entry_function_name</a>(entry_function_payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_entry_function_name">entry_function_name</a>(entry_function_payload: &EntryFunctionPayload): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> entry_function_name = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> addr_str = <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(
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

<a id="0x1_common_account_abstractions_utils_construct_message"></a>

## Function `construct_message`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_construct_message">construct_message</a>(chain_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_construct_message">construct_message</a>(
    chain_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    account_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> message = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    message.append(*domain);
    message.append(b" wants you <b>to</b> sign in <b>with</b> your ");
    message.append(*chain_name);
    message.append(b" <a href="account.md#0x1_account">account</a>:\n");
    message.append(*account_address);
    message.append(b"\n\nPlease confirm you explicitly initiated this request from ");
    message.append(*domain);
    message.append(b".");
    message.append(b" You are approving <b>to</b> execute transaction ");
    message.append(*entry_function_name);
    message.append(b" on Aptos blockchain");
    <b>let</b> network_name = <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_network_name">network_name</a>();
    message.append(b" (");
    message.append(network_name);
    message.append(b")");
    message.append(b".");
    message.append(b"\n\nNonce: ");
    message.append(*digest_utf8);
    *message
}
</code></pre>



</details>

<a id="0x1_common_account_abstractions_utils_daa_authenticate"></a>

## Function `daa_authenticate`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_daa_authenticate">daa_authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, auth_fn: |<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;|): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) inline <b>fun</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_daa_authenticate">daa_authenticate</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    aa_auth_data: AbstractionAuthData,
    auth_fn: |AbstractionAuthData, &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;|,
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> maybe_entry_function_payload = <a href="transaction_context.md#0x1_transaction_context_entry_function_payload">transaction_context::entry_function_payload</a>();
    <b>if</b> (maybe_entry_function_payload.is_some()) {
        <b>let</b> entry_function_payload = maybe_entry_function_payload.destroy_some();
        <b>let</b> entry_function_name = <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_entry_function_name">entry_function_name</a>(&entry_function_payload);

        // call the passed-in function value
        auth_fn(aa_auth_data, &entry_function_name);
        <a href="account.md#0x1_account">account</a>
    } <b>else</b> {
        <b>abort</b>(<a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>)
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
