
<a name="0x1_Token"></a>

# Module `0x1::Token`

This module provides the foundation for (collectible) Tokens often called NFTs


-  [Resource `Collections`](#0x1_Token_Collections)
-  [Struct `Collection`](#0x1_Token_Collection)
-  [Resource `Gallery`](#0x1_Token_Gallery)
-  [Struct `Token`](#0x1_Token_Token)
-  [Struct `TokenData`](#0x1_Token_TokenData)
-  [Constants](#@Constants_0)
-  [Function `initialize_collections`](#0x1_Token_initialize_collections)
-  [Function `create_finite_collection_script`](#0x1_Token_create_finite_collection_script)
-  [Function `create_unlimited_collection_script`](#0x1_Token_create_unlimited_collection_script)
-  [Function `create_collection`](#0x1_Token_create_collection)
-  [Function `initialize_gallery`](#0x1_Token_initialize_gallery)
-  [Function `create_token_script`](#0x1_Token_create_token_script)
-  [Function `create_token`](#0x1_Token_create_token)
-  [Function `token_id`](#0x1_Token_token_id)
-  [Function `withdraw_token`](#0x1_Token_withdraw_token)
-  [Function `deposit_token`](#0x1_Token_deposit_token)
-  [Function `merge_token`](#0x1_Token_merge_token)
-  [Function `create_collection_and_token`](#0x1_Token_create_collection_and_token)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII">0x1::ASCII</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID">0x1::GUID</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="Table.md#0x1_Table">0x1::Table</a>;
</code></pre>



<a name="0x1_Token_Collections"></a>

## Resource `Collections`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_Collections">Collections</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collections: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, <a href="Token.md#0x1_Token_Collection">Token::Collection</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_Collection"></a>

## Struct `Collection`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_Collection">Collection</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tokens: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, <a href="Token.md#0x1_Token_TokenData">Token::TokenData</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claimed_tokens: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>count: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_Gallery"></a>

## Resource `Gallery`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_Gallery">Gallery</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gallery: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, <a href="Token.md#0x1_Token_Token">Token::Token</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_Token"></a>

## Struct `Token`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token">Token</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>collection: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_TokenData"></a>

## Struct `TokenData`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_TokenData">TokenData</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>supply: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>
 URL for additional information / media
</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_Token_EINSUFFICIENT_BALANCE"></a>



<pre><code><b>const</b> <a href="Token.md#0x1_Token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 0;
</code></pre>



<a name="0x1_Token_EINVALID_TOKEN_MERGE"></a>



<pre><code><b>const</b> <a href="Token.md#0x1_Token_EINVALID_TOKEN_MERGE">EINVALID_TOKEN_MERGE</a>: u64 = 2;
</code></pre>



<a name="0x1_Token_EMISSING_CLAIMED_TOKEN"></a>



<pre><code><b>const</b> <a href="Token.md#0x1_Token_EMISSING_CLAIMED_TOKEN">EMISSING_CLAIMED_TOKEN</a>: u64 = 1;
</code></pre>



<a name="0x1_Token_initialize_collections"></a>

## Function `initialize_collections`



<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_collections">initialize_collections</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_collections">initialize_collections</a>(account: &signer) {
    <b>move_to</b>(
        account,
        <a href="Token.md#0x1_Token_Collections">Collections</a> {
            collections: <a href="Table.md#0x1_Table_create">Table::create</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, <a href="Token.md#0x1_Token_Collection">Collection</a>&gt;(),
        },
    )
}
</code></pre>



</details>

<a name="0x1_Token_create_finite_collection_script"></a>

## Function `create_finite_collection_script`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Token.md#0x1_Token_create_finite_collection_script">create_finite_collection_script</a>(account: signer, description: vector&lt;u8&gt;, name: vector&lt;u8&gt;, uri: vector&lt;u8&gt;, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Token.md#0x1_Token_create_finite_collection_script">create_finite_collection_script</a>(
    account: signer,
    description: vector&lt;u8&gt;,
    name: vector&lt;u8&gt;,
    uri: vector&lt;u8&gt;,
    maximum: u64,
) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a> {
    <a href="Token.md#0x1_Token_create_collection">create_collection</a>(
        &account,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(description),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(name),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(uri),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(maximum),
    );
}
</code></pre>



</details>

<a name="0x1_Token_create_unlimited_collection_script"></a>

## Function `create_unlimited_collection_script`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Token.md#0x1_Token_create_unlimited_collection_script">create_unlimited_collection_script</a>(account: signer, description: vector&lt;u8&gt;, name: vector&lt;u8&gt;, uri: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Token.md#0x1_Token_create_unlimited_collection_script">create_unlimited_collection_script</a>(
    account: signer,
    description: vector&lt;u8&gt;,
    name: vector&lt;u8&gt;,
    uri: vector&lt;u8&gt;,
) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a> {
    <a href="Token.md#0x1_Token_create_collection">create_collection</a>(
        &account,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(description),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(name),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(uri),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
    );
}
</code></pre>



</details>

<a name="0x1_Token_create_collection"></a>

## Function `create_collection`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_collection">create_collection</a>(account: &signer, description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, maximum: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_collection">create_collection</a>(
    account: &signer,
    description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    maximum: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;u64&gt;,
) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&gt;(account_addr)) {
        <a href="Token.md#0x1_Token_initialize_collections">initialize_collections</a>(account)
    };
    <b>if</b> (!<b>exists</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&gt;(account_addr)) {
        <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>(account)
    };

    <b>let</b> collections = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&gt;(account_addr).collections;
    <b>let</b> collection = <a href="Token.md#0x1_Token_Collection">Collection</a> {
        tokens: <a href="Table.md#0x1_Table_create">Table::create</a>(),
        claimed_tokens: <a href="Table.md#0x1_Table_create">Table::create</a>(),
        description,
        name,
        uri,
        count: 0,
        maximum,
    };

    <a href="Table.md#0x1_Table_insert">Table::insert</a>(collections, *&name, collection);
}
</code></pre>



</details>

<a name="0x1_Token_initialize_gallery"></a>

## Function `initialize_gallery`



<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>(signer: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>(signer: &signer) {
    <b>move_to</b>(
        signer,
        <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
            gallery: <a href="Table.md#0x1_Table_create">Table::create</a>&lt;ID, <a href="Token.md#0x1_Token">Token</a>&gt;(),
        },
    )
}
</code></pre>



</details>

<a name="0x1_Token_create_token_script"></a>

## Function `create_token_script`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Token.md#0x1_Token_create_token_script">create_token_script</a>(account: signer, collection_name: vector&lt;u8&gt;, description: vector&lt;u8&gt;, name: vector&lt;u8&gt;, supply: u64, uri: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Token.md#0x1_Token_create_token_script">create_token_script</a>(
    account: signer,
    collection_name: vector&lt;u8&gt;,
    description: vector&lt;u8&gt;,
    name: vector&lt;u8&gt;,
    supply: u64,
    uri: vector&lt;u8&gt;,
) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a>, <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
  <a href="Token.md#0x1_Token_create_token">create_token</a>(
      &account,
      <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(collection_name),
      <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(description),
      <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(name),
      supply,
      <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(uri),
  );
}
</code></pre>



</details>

<a name="0x1_Token_create_token"></a>

## Function `create_token`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_token">create_token</a>(account: &signer, collection_name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, supply: u64, uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_token">create_token</a>(
    account: &signer,
    collection_name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    supply: u64,
    uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
): ID <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a>, <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>let</b> collections = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&gt;(account_addr).collections;
    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&gt;(account_addr).gallery;

    <b>let</b> token_id = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id">GUID::id</a>(&<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_create">GUID::create</a>(account));
    <b>let</b> token = <a href="Token.md#0x1_Token">Token</a> {
        id: *&token_id,
        name: *&name,
        collection: *&collection_name,
        balance: supply,
    };

    <b>let</b> token_data = <a href="Token.md#0x1_Token_TokenData">TokenData</a> {
        id: *&token_id,
        description,
        name: *&name,
        supply,
        uri,
    };

    <b>let</b> collection = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(collections, &collection_name);
    <b>if</b> (supply == 1) {
        <a href="Table.md#0x1_Table_insert">Table::insert</a>(&<b>mut</b> collection.claimed_tokens, *&name, account_addr)
    };
    <a href="Table.md#0x1_Table_insert">Table::insert</a>(&<b>mut</b> collection.tokens, name, token_data);

    <a href="Table.md#0x1_Table_insert">Table::insert</a>(gallery, *&token_id, token);
    token_id
}
</code></pre>



</details>

<a name="0x1_Token_token_id"></a>

## Function `token_id`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_token_id">token_id</a>(token: &<a href="Token.md#0x1_Token_Token">Token::Token</a>): &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_token_id">token_id</a>(token: &<a href="Token.md#0x1_Token">Token</a>): &ID {
    &token.id
}
</code></pre>



</details>

<a name="0x1_Token_withdraw_token"></a>

## Function `withdraw_token`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_withdraw_token">withdraw_token</a>(account: &signer, token_id: &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, amount: u64): <a href="Token.md#0x1_Token_Token">Token::Token</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_withdraw_token">withdraw_token</a>(
    account: &signer,
    token_id: &ID,
    amount: u64,
): <a href="Token.md#0x1_Token">Token</a> <b>acquires</b> <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);

    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&gt;(account_addr).gallery;
    <b>let</b> balance = <a href="Table.md#0x1_Table_borrow">Table::borrow</a>(gallery, token_id).balance;
    <b>assert</b>!(balance &gt;= amount, <a href="Token.md#0x1_Token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>);

    <b>if</b> (balance == amount) {
        <b>let</b> (_key, value) = <a href="Table.md#0x1_Table_remove">Table::remove</a>(gallery, token_id);
        value
    } <b>else</b> {
        <b>let</b> token = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(gallery, token_id);
        token.balance = balance - amount;
        <a href="Token.md#0x1_Token">Token</a> {
            id: *&token.id,
            name: *&token.name,
            collection: *&token.collection,
            balance: amount,
        }
    }
}
</code></pre>



</details>

<a name="0x1_Token_deposit_token"></a>

## Function `deposit_token`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_deposit_token">deposit_token</a>(account: &signer, token: <a href="Token.md#0x1_Token_Token">Token::Token</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_deposit_token">deposit_token</a>(
    account: &signer,
    token: <a href="Token.md#0x1_Token">Token</a>,
) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a>, <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&gt;(account_addr)) {
        <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>(account)
    };

    <b>let</b> creator_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id_creator_address">GUID::id_creator_address</a>(&token.id);
    <b>let</b> collections = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&gt;(creator_addr).collections;
    <b>let</b> collection = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(collections, &token.collection);
    <b>if</b> (<a href="Table.md#0x1_Table_borrow">Table::borrow</a>(&collection.tokens, &token.name).supply == 1) {
      <a href="Table.md#0x1_Table_remove">Table::remove</a>(&<b>mut</b> collection.claimed_tokens, &token.name);
      <a href="Table.md#0x1_Table_insert">Table::insert</a>(&<b>mut</b> collection.claimed_tokens, *&token.name, account_addr)
    };

    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&gt;(account_addr).gallery;
    <b>if</b> (<a href="Table.md#0x1_Table_contains_key">Table::contains_key</a>(gallery, &token.id)) {
        <b>let</b> current_token = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(gallery, &token.id);
        <a href="Token.md#0x1_Token_merge_token">merge_token</a>(token, current_token);
    } <b>else</b> {
        <a href="Table.md#0x1_Table_insert">Table::insert</a>(gallery, *&token.id, token)
    }
}
</code></pre>



</details>

<a name="0x1_Token_merge_token"></a>

## Function `merge_token`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_merge_token">merge_token</a>(source_token: <a href="Token.md#0x1_Token_Token">Token::Token</a>, dst_token: &<b>mut</b> <a href="Token.md#0x1_Token_Token">Token::Token</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_merge_token">merge_token</a>(
    source_token: <a href="Token.md#0x1_Token">Token</a>,
    dst_token: &<b>mut</b> <a href="Token.md#0x1_Token">Token</a>,
) {
    <b>assert</b>!(dst_token.id == source_token.id, <a href="Token.md#0x1_Token_EINVALID_TOKEN_MERGE">EINVALID_TOKEN_MERGE</a>);
    dst_token.balance = dst_token.balance + source_token.balance;
}
</code></pre>



</details>

<a name="0x1_Token_create_collection_and_token"></a>

## Function `create_collection_and_token`



<pre><code><b>fun</b> <a href="Token.md#0x1_Token_create_collection_and_token">create_collection_and_token</a>(creator: &signer, amount: u64): (<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Token.md#0x1_Token_create_collection_and_token">create_collection_and_token</a>(
    creator: &signer,
    amount: u64,
): (<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, ID) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a>, <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> collection_name = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"Hello, World");
    <a href="Token.md#0x1_Token_create_collection">create_collection</a>(
        creator,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"<a href="Token.md#0x1_Token_Collection">Collection</a>: Hello, World"),
        *&collection_name,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"https://aptos.dev"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
    );

    <b>let</b> token_id = <a href="Token.md#0x1_Token_create_token">create_token</a>(
        creator,
        *&collection_name,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"<a href="Token.md#0x1_Token">Token</a>: Hello, <a href="Token.md#0x1_Token">Token</a>"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"Hello, <a href="Token.md#0x1_Token">Token</a>"),
        amount,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"https://aptos.dev"),
    );

    (collection_name, token_id)
}
</code></pre>



</details>
