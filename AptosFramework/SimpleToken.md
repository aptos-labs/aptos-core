
<a name="0x1_SimpleToken"></a>

# Module `0x1::SimpleToken`

This exists to demonstrate how one could define their own TokenMetadata


-  [Struct `SimpleToken`](#0x1_SimpleToken_SimpleToken)
-  [Function `create_simple_token`](#0x1_SimpleToken_create_simple_token)


<pre><code><b>use</b> <a href="Token.md#0x1_Token">0x1::Token</a>;
</code></pre>



<a name="0x1_SimpleToken_SimpleToken"></a>

## Struct `SimpleToken`



<pre><code><b>struct</b> <a href="SimpleToken.md#0x1_SimpleToken">SimpleToken</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>magic_number: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_SimpleToken_create_simple_token"></a>

## Function `create_simple_token`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_create_simple_token">create_simple_token</a>(account: signer, collection_name: vector&lt;u8&gt;, description: vector&lt;u8&gt;, name: vector&lt;u8&gt;, supply: u64, uri: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_create_simple_token">create_simple_token</a>(
    account: signer,
    collection_name: vector&lt;u8&gt;,
    description: vector&lt;u8&gt;,
    name: vector&lt;u8&gt;,
    supply: u64,
    uri: vector&lt;u8&gt;,
) {
  <a href="Token.md#0x1_Token_create_token_with_metadata_script">Token::create_token_with_metadata_script</a>&lt;<a href="SimpleToken.md#0x1_SimpleToken">SimpleToken</a>&gt;(
      account,
      collection_name,
      description,
      name,
      supply,
      uri,
      <a href="SimpleToken.md#0x1_SimpleToken">SimpleToken</a> { magic_number: 42 },
  );
}
</code></pre>



</details>
