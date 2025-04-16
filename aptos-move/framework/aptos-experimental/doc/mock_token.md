
<a id="0x7_mock_token"></a>

# Module `0x7::mock_token`



-  [Resource `TokenStore`](#0x7_mock_token_TokenStore)
-  [Function `init_module`](#0x7_mock_token_init_module)
-  [Function `mint_to`](#0x7_mock_token_mint_to)
-  [Function `get_token_metadata`](#0x7_mock_token_get_token_metadata)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x7_mock_token_TokenStore"></a>

## Resource `TokenStore`



<pre><code><b>struct</b> <a href="mock_token.md#0x7_mock_token_TokenStore">TokenStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_ref: <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_mock_token_init_module"></a>

## Function `init_module`



<pre><code><b>fun</b> <a href="mock_token.md#0x7_mock_token_init_module">init_module</a>(deployer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="mock_token.md#0x7_mock_token_init_module">init_module</a>(deployer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>let</b> ctor_ref = &<a href="../../aptos-framework/doc/object.md#0x1_object_create_sticky_object">object::create_sticky_object</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer));

    <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset">primary_fungible_store::create_primary_store_enabled_fungible_asset</a>(
        ctor_ref,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        utf8(b"MockToken"),
        utf8(b"MT"),
        0,
        utf8(b"https://"),
        utf8(b"https://"),
    );

    <b>let</b> mint_ref = <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_generate_mint_ref">fungible_asset::generate_mint_ref</a>(ctor_ref);

    <b>move_to</b>(deployer, <a href="mock_token.md#0x7_mock_token_TokenStore">TokenStore</a> { mint_ref });
}
</code></pre>



</details>

<a id="0x7_mock_token_mint_to"></a>

## Function `mint_to`



<pre><code><b>public</b> entry <b>fun</b> <a href="mock_token.md#0x7_mock_token_mint_to">mint_to</a>(<b>to</b>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="mock_token.md#0x7_mock_token_mint_to">mint_to</a>(<b>to</b>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64) <b>acquires</b> <a href="mock_token.md#0x7_mock_token_TokenStore">TokenStore</a> {
    <b>let</b> store = <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<b>to</b>), <a href="mock_token.md#0x7_mock_token_get_token_metadata">get_token_metadata</a>());
    <b>let</b> mint_ref = &<b>borrow_global</b>&lt;<a href="mock_token.md#0x7_mock_token_TokenStore">TokenStore</a>&gt;(@aptos_experimental).mint_ref;

    <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_mint_to">fungible_asset::mint_to</a>(mint_ref, store, amount);
}
</code></pre>



</details>

<a id="0x7_mock_token_get_token_metadata"></a>

## Function `get_token_metadata`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="mock_token.md#0x7_mock_token_get_token_metadata">get_token_metadata</a>(): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mock_token.md#0x7_mock_token_get_token_metadata">get_token_metadata</a>(): Object&lt;Metadata&gt; <b>acquires</b> <a href="mock_token.md#0x7_mock_token_TokenStore">TokenStore</a> {
    <b>let</b> token_store = <b>borrow_global</b>&lt;<a href="mock_token.md#0x7_mock_token_TokenStore">TokenStore</a>&gt;(@aptos_experimental);

    <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">fungible_asset::mint_ref_metadata</a>(&token_store.mint_ref)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
