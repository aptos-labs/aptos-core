
<a name="0x1_Token"></a>

# Module `0x1::Token`

This module provides the foundation for (collectible) Tokens often called NFTs


-  [Resource `Collections`](#0x1_Token_Collections)
-  [Struct `Collection`](#0x1_Token_Collection)
-  [Resource `Gallery`](#0x1_Token_Gallery)
-  [Struct `Token`](#0x1_Token_Token)
-  [Struct `TokenMetadata`](#0x1_Token_TokenMetadata)
-  [Struct `TokenData`](#0x1_Token_TokenData)
-  [Constants](#@Constants_0)
-  [Function `initialize_collections`](#0x1_Token_initialize_collections)
-  [Function `create_collection`](#0x1_Token_create_collection)
-  [Function `initialize_gallery`](#0x1_Token_initialize_gallery)
-  [Function `token_id`](#0x1_Token_token_id)
-  [Function `create_token`](#0x1_Token_create_token)
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



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_Collections">Collections</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collections: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, <a href="Token.md#0x1_Token_Collection">Token::Collection</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_Collection"></a>

## Struct `Collection`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_Collection">Collection</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tokens: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, <a href="Token.md#0x1_Token_TokenMetadata">Token::TokenMetadata</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claimed_tokens: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
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



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_Gallery">Gallery</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gallery: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, <a href="Token.md#0x1_Token_Token">Token::Token</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_Token"></a>

## Struct `Token`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token">Token</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> drop, store
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
<code>collection: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>data: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="Token.md#0x1_Token_TokenData">Token::TokenData</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_TokenMetadata"></a>

## Struct `TokenMetadata`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_TokenMetadata">TokenMetadata</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> drop, store
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
<code>data: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="Token.md#0x1_Token_TokenData">Token::TokenData</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Token_TokenData"></a>

## Struct `TokenData`



<pre><code><b>struct</b> <a href="Token.md#0x1_Token_TokenData">TokenData</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: TokenType</code>
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



<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_collections">initialize_collections</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_collections">initialize_collections</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(account: &signer) {
    <b>move_to</b>(
        account,
        <a href="Token.md#0x1_Token_Collections">Collections</a> {
            collections: <a href="Table.md#0x1_Table_create">Table::create</a>&lt;ID, <a href="Token.md#0x1_Token_Collection">Collection</a>&lt;TokenType&gt;&gt;(),
        },
    )
}
</code></pre>



</details>

<a name="0x1_Token_create_collection"></a>

## Function `create_collection`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_collection">create_collection</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer, description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, maximum: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt;): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_collection">create_collection</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    account: &signer,
    description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    maximum: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;u64&gt;,
): ID <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&lt;TokenType&gt;&gt;(account_addr)) {
        <a href="Token.md#0x1_Token_initialize_collections">initialize_collections</a>&lt;TokenType&gt;(account)
    };
    <b>if</b> (!<b>exists</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&lt;TokenType&gt;&gt;(account_addr)) {
        <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>&lt;TokenType&gt;(account)
    };

    <b>let</b> collections = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&lt;TokenType&gt;&gt;(account_addr).collections;
    <b>let</b> collection = <a href="Token.md#0x1_Token_Collection">Collection</a>&lt;TokenType&gt; {
        tokens: <a href="Table.md#0x1_Table_create">Table::create</a>(),
        claimed_tokens: <a href="Table.md#0x1_Table_create">Table::create</a>(),
        id: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id">GUID::id</a>(&<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_create">GUID::create</a>(account)),
        description,
        name,
        uri,
        count: 0,
        maximum,
    };

    <b>let</b> id = *&collection.id;
    <a href="Table.md#0x1_Table_insert">Table::insert</a>(collections, *&id, collection);
    id
}
</code></pre>



</details>

<a name="0x1_Token_initialize_gallery"></a>

## Function `initialize_gallery`



<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(signer: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(signer: &signer) {
    <b>move_to</b>(
        signer,
        <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
            gallery: <a href="Table.md#0x1_Table_create">Table::create</a>&lt;ID, <a href="Token.md#0x1_Token">Token</a>&lt;TokenType&gt;&gt;(),
        },
    )
}
</code></pre>



</details>

<a name="0x1_Token_token_id"></a>

## Function `token_id`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_token_id">token_id</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="Token.md#0x1_Token_Token">Token::Token</a>&lt;TokenType&gt;): &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_token_id">token_id</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(token: &<a href="Token.md#0x1_Token">Token</a>&lt;TokenType&gt;): &ID {
    &token.id
}
</code></pre>



</details>

<a name="0x1_Token_create_token"></a>

## Function `create_token`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_token">create_token</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer, collection_id: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, supply: u64, uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>, metadata: TokenType): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_create_token">create_token</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    account: &signer,
    collection_id: ID,
    description: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    name: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    supply: u64,
    uri: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_String">ASCII::String</a>,
    metadata: TokenType,
): ID <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a>, <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>let</b> collections = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&lt;TokenType&gt;&gt;(account_addr).collections;
    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&lt;TokenType&gt;&gt;(account_addr).gallery;

    <b>let</b> some_data = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(<a href="Token.md#0x1_Token_TokenData">TokenData</a> {
        description,
        metadata,
        name,
        supply,
        uri,
    });

    <b>let</b> (collection_data, gallery_data) = <b>if</b> (supply == 1) {
        (<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(), some_data)
    } <b>else</b> {
        (some_data, <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>())
    };

    <b>let</b> collection_token = <a href="Token.md#0x1_Token_TokenMetadata">TokenMetadata</a> {
        id: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id">GUID::id</a>(&<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_create">GUID::create</a>(account)),
        data: collection_data,
    };

    <b>let</b> token_id  = *&collection_token.id;
    <b>let</b> collection = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(collections, &collection_id);
    <b>if</b> (supply == 1) {
        <a href="Table.md#0x1_Table_insert">Table::insert</a>(&<b>mut</b> collection.claimed_tokens, *&collection_token.id, account_addr)
    };
    <a href="Table.md#0x1_Table_insert">Table::insert</a>(&<b>mut</b> collection.tokens, *&collection_token.id, collection_token);

    <b>let</b> gallery_token = <a href="Token.md#0x1_Token">Token</a> {
        id: *&token_id,
        collection: collection_id,
        balance: supply,
        data: gallery_data,
    };

    <a href="Table.md#0x1_Table_insert">Table::insert</a>(gallery, *&gallery_token.id, gallery_token);
    token_id
}
</code></pre>



</details>

<a name="0x1_Token_withdraw_token"></a>

## Function `withdraw_token`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_withdraw_token">withdraw_token</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer, token_id: &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, amount: u64): <a href="Token.md#0x1_Token_Token">Token::Token</a>&lt;TokenType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_withdraw_token">withdraw_token</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    account: &signer,
    token_id: &ID,
    amount: u64,
): <a href="Token.md#0x1_Token">Token</a>&lt;TokenType&gt; <b>acquires</b> <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);

    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&lt;TokenType&gt;&gt;(account_addr).gallery;
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
            collection: *&token.collection,
            balance: amount,
            data: *&token.data,
        }
    }
}
</code></pre>



</details>

<a name="0x1_Token_deposit_token"></a>

## Function `deposit_token`



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_deposit_token">deposit_token</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer, token: <a href="Token.md#0x1_Token_Token">Token::Token</a>&lt;TokenType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_deposit_token">deposit_token</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    account: &signer,
    token: <a href="Token.md#0x1_Token">Token</a>&lt;TokenType&gt;,
) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a>, <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> account_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&lt;TokenType&gt;&gt;(account_addr)) {
        <a href="Token.md#0x1_Token_initialize_gallery">initialize_gallery</a>&lt;TokenType&gt;(account)
    };

    <b>let</b> creator_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id_creator_address">GUID::id_creator_address</a>(&token.collection);
    <b>let</b> collections = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Collections">Collections</a>&lt;TokenType&gt;&gt;(creator_addr).collections;
    <b>let</b> collection = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(collections, &token.collection);
    <b>if</b> (<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&token.data) && <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&token.data).supply == 1) {
      <a href="Table.md#0x1_Table_remove">Table::remove</a>(&<b>mut</b> collection.claimed_tokens, &token.id);
      <a href="Table.md#0x1_Table_insert">Table::insert</a>(&<b>mut</b> collection.claimed_tokens, *&token.id, account_addr)
    };

    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Token.md#0x1_Token_Gallery">Gallery</a>&lt;TokenType&gt;&gt;(account_addr).gallery;
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



<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_merge_token">merge_token</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(source_token: <a href="Token.md#0x1_Token_Token">Token::Token</a>&lt;TokenType&gt;, dst_token: &<b>mut</b> <a href="Token.md#0x1_Token_Token">Token::Token</a>&lt;TokenType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Token.md#0x1_Token_merge_token">merge_token</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    source_token: <a href="Token.md#0x1_Token">Token</a>&lt;TokenType&gt;,
    dst_token: &<b>mut</b> <a href="Token.md#0x1_Token">Token</a>&lt;TokenType&gt;,
) {
    <b>assert</b>!(dst_token.id == source_token.id, <a href="Token.md#0x1_Token_EINVALID_TOKEN_MERGE">EINVALID_TOKEN_MERGE</a>);
    dst_token.balance = dst_token.balance + source_token.balance;
}
</code></pre>



</details>

<a name="0x1_Token_create_collection_and_token"></a>

## Function `create_collection_and_token`



<pre><code><b>fun</b> <a href="Token.md#0x1_Token_create_collection_and_token">create_collection_and_token</a>(creator: &signer, amount: u64): (<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Token.md#0x1_Token_create_collection_and_token">create_collection_and_token</a>(
    creator: &signer,
    amount: u64,
): (ID, ID) <b>acquires</b> <a href="Token.md#0x1_Token_Collections">Collections</a>, <a href="Token.md#0x1_Token_Gallery">Gallery</a> {
    <b>let</b> collection_id = <a href="Token.md#0x1_Token_create_collection">create_collection</a>&lt;u64&gt;(
        creator,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"<a href="Token.md#0x1_Token_Collection">Collection</a>: Hello, World"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"Hello, World"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"https://aptos.dev"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
    );

    <b>let</b> token_id = <a href="Token.md#0x1_Token_create_token">create_token</a>&lt;u64&gt;(
        creator,
        *&collection_id,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"<a href="Token.md#0x1_Token">Token</a>: Hello, <a href="Token.md#0x1_Token">Token</a>"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"Hello, <a href="Token.md#0x1_Token">Token</a>"),
        amount,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"https://aptos.dev"),
        0,
    );

    (collection_id, token_id)
}
</code></pre>



</details>
