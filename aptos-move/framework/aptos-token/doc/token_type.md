
<a name="0x1_token_type"></a>

# Module `0x1::token_type`



-  [Constants](#@Constants_0)
-  [Function `get_token_type`](#0x1_token_type_get_token_type)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="token.md#0x3_token">0x3::token</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1_token_type_FUNGIBLE"></a>



<pre><code><b>const</b> <a href="token_type.md#0x1_token_type_FUNGIBLE">FUNGIBLE</a>: u64 = 2;
</code></pre>



<a name="0x1_token_type_NFT"></a>



<pre><code><b>const</b> <a href="token_type.md#0x1_token_type_NFT">NFT</a>: u64 = 0;
</code></pre>



<a name="0x1_token_type_NFT_PRINT"></a>



<pre><code><b>const</b> <a href="token_type.md#0x1_token_type_NFT_PRINT">NFT_PRINT</a>: u64 = 1;
</code></pre>



<a name="0x1_token_type_get_token_type"></a>

## Function `get_token_type`



<pre><code><b>public</b> <b>fun</b> <a href="token_type.md#0x1_token_type_get_token_type">get_token_type</a>(token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_type.md#0x1_token_type_get_token_type">get_token_type</a>(token_id: TokenId): u64 {
    <b>let</b> token_data_id = <a href="token.md#0x3_token_get_tokendata_id">token::get_tokendata_id</a>(token_id);
    <b>let</b> (_, _, _, property_version) = <a href="token.md#0x3_token_get_token_id_fields">token::get_token_id_fields</a>(&token_id);
    <b>let</b> mutability_config = <a href="token.md#0x3_token_get_token_mutability_config">token::get_token_mutability_config</a>(token_data_id);
    <b>let</b> maximum = <a href="token.md#0x3_token_get_tokendata_maximum">token::get_tokendata_maximum</a>(token_data_id);
    <b>if</b> (maximum == 1 && !<a href="token.md#0x3_token_get_token_mutability_maximum">token::get_token_mutability_maximum</a>(mutability_config)){
        <a href="token_type.md#0x1_token_type_NFT">NFT</a>
    } <b>else</b> <b>if</b> (property_version &gt; 0) {
        <a href="token_type.md#0x1_token_type_NFT_PRINT">NFT_PRINT</a>
    } <b>else</b> {
        <a href="token_type.md#0x1_token_type_FUNGIBLE">FUNGIBLE</a>
    }
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
