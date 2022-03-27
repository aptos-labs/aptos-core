
<a name="0x1_SimpleToken"></a>

# Module `0x1::SimpleToken`

This exists to provide convenient access to Tokens within Aptos for folks that do not want
additional features within the metadata.


-  [Struct `NoMetadata`](#0x1_SimpleToken_NoMetadata)
-  [Function `create_simple_token`](#0x1_SimpleToken_create_simple_token)
-  [Function `create_finite_simple_collection`](#0x1_SimpleToken_create_finite_simple_collection)
-  [Function `create_unlimited_simple_collection`](#0x1_SimpleToken_create_unlimited_simple_collection)
-  [Function `transfer_simple_token_to`](#0x1_SimpleToken_transfer_simple_token_to)
-  [Function `receive_simple_token_from`](#0x1_SimpleToken_receive_simple_token_from)
-  [Function `stop_simple_token_transfer_to`](#0x1_SimpleToken_stop_simple_token_transfer_to)


<pre><code><b>use</b> <a href="Token.md#0x1_Token">0x1::Token</a>;
<b>use</b> <a href="TokenTransfers.md#0x1_TokenTransfers">0x1::TokenTransfers</a>;
</code></pre>



<a name="0x1_SimpleToken_NoMetadata"></a>

## Struct `NoMetadata`



<pre><code><b>struct</b> <a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
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
  <a href="Token.md#0x1_Token_create_token_script">Token::create_token_script</a>&lt;<a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a>&gt;(
      account,
      collection_name,
      description,
      name,
      supply,
      uri,
      <a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a> { },
  );
}
</code></pre>



</details>

<a name="0x1_SimpleToken_create_finite_simple_collection"></a>

## Function `create_finite_simple_collection`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_create_finite_simple_collection">create_finite_simple_collection</a>(account: signer, description: vector&lt;u8&gt;, name: vector&lt;u8&gt;, uri: vector&lt;u8&gt;, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_create_finite_simple_collection">create_finite_simple_collection</a>(
    account: signer,
    description: vector&lt;u8&gt;,
    name: vector&lt;u8&gt;,
    uri: vector&lt;u8&gt;,
    maximum: u64,
) {
    <a href="Token.md#0x1_Token_create_finite_collection_script">Token::create_finite_collection_script</a>&lt;<a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a>&gt;(
        account,
        description,
        name,
        uri,
        maximum,
    );
}
</code></pre>



</details>

<a name="0x1_SimpleToken_create_unlimited_simple_collection"></a>

## Function `create_unlimited_simple_collection`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_create_unlimited_simple_collection">create_unlimited_simple_collection</a>(account: signer, description: vector&lt;u8&gt;, name: vector&lt;u8&gt;, uri: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_create_unlimited_simple_collection">create_unlimited_simple_collection</a>(
    account: signer,
    description: vector&lt;u8&gt;,
    name: vector&lt;u8&gt;,
    uri: vector&lt;u8&gt;,
) {
    <a href="Token.md#0x1_Token_create_unlimited_collection_script">Token::create_unlimited_collection_script</a>&lt;<a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a>&gt;(account, description, name, uri);
}
</code></pre>



</details>

<a name="0x1_SimpleToken_transfer_simple_token_to"></a>

## Function `transfer_simple_token_to`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_transfer_simple_token_to">transfer_simple_token_to</a>(sender: signer, receiver: <b>address</b>, creator: <b>address</b>, token_creation_num: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_transfer_simple_token_to">transfer_simple_token_to</a>(
    sender: signer,
    receiver: <b>address</b>,
    creator: <b>address</b>,
    token_creation_num: u64,
    amount: u64,
) {
    <a href="TokenTransfers.md#0x1_TokenTransfers_transfer_to_script">TokenTransfers::transfer_to_script</a>&lt;<a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a>&gt;(
        sender,
        receiver,
        creator,
        token_creation_num,
        amount,
    );
}
</code></pre>



</details>

<a name="0x1_SimpleToken_receive_simple_token_from"></a>

## Function `receive_simple_token_from`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_receive_simple_token_from">receive_simple_token_from</a>(receiver: signer, sender: <b>address</b>, creator: <b>address</b>, token_creation_num: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_receive_simple_token_from">receive_simple_token_from</a>(
     receiver: signer,
     sender: <b>address</b>,
     creator: <b>address</b>,
     token_creation_num: u64,
 ) {
     <a href="TokenTransfers.md#0x1_TokenTransfers_receive_from_script">TokenTransfers::receive_from_script</a>&lt;<a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a>&gt;(
         receiver,
         sender,
         creator,
         token_creation_num,
     );
 }
</code></pre>



</details>

<a name="0x1_SimpleToken_stop_simple_token_transfer_to"></a>

## Function `stop_simple_token_transfer_to`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_stop_simple_token_transfer_to">stop_simple_token_transfer_to</a>(sender: signer, receiver: <b>address</b>, creator: <b>address</b>, token_creation_num: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SimpleToken.md#0x1_SimpleToken_stop_simple_token_transfer_to">stop_simple_token_transfer_to</a>(
    sender: signer,
    receiver: <b>address</b>,
    creator: <b>address</b>,
    token_creation_num: u64,
) {
    <a href="TokenTransfers.md#0x1_TokenTransfers_stop_transfer_to_script">TokenTransfers::stop_transfer_to_script</a>&lt;<a href="SimpleToken.md#0x1_SimpleToken_NoMetadata">NoMetadata</a>&gt;(
        sender,
        receiver,
        creator,
        token_creation_num,
    );
}
</code></pre>



</details>
