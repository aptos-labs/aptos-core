
<a name="0x1_NFT"></a>

# Module `0x1::NFT`



-  [Resource `Token`](#0x1_NFT_Token)
-  [Resource `TokenData`](#0x1_NFT_TokenData)
-  [Resource `TokenDataCollection`](#0x1_NFT_TokenDataCollection)
-  [Struct `MintEvent`](#0x1_NFT_MintEvent)
-  [Struct `TransferEvent`](#0x1_NFT_TransferEvent)
-  [Resource `Admin`](#0x1_NFT_Admin)
-  [Resource `CreationDelegation`](#0x1_NFT_CreationDelegation)
-  [Constants](#@Constants_0)
-  [Function `nft_initialize`](#0x1_NFT_nft_initialize)
-  [Function `id`](#0x1_NFT_id)
-  [Function `get_balance`](#0x1_NFT_get_balance)
-  [Function `get_supply`](#0x1_NFT_get_supply)
-  [Function `get_content_uri`](#0x1_NFT_get_content_uri)
-  [Function `get_metadata`](#0x1_NFT_get_metadata)
-  [Function `get_parent_id`](#0x1_NFT_get_parent_id)
-  [Function `is_data_inlined`](#0x1_NFT_is_data_inlined)
-  [Function `index_of_token`](#0x1_NFT_index_of_token)
-  [Function `join`](#0x1_NFT_join)
-  [Function `split_out`](#0x1_NFT_split_out)
-  [Function `create_for`](#0x1_NFT_create_for)
-  [Function `create`](#0x1_NFT_create)
-  [Function `create_impl`](#0x1_NFT_create_impl)
-  [Function `publish_token_data_collection`](#0x1_NFT_publish_token_data_collection)
-  [Function `allow_creation_delegation`](#0x1_NFT_allow_creation_delegation)
-  [Function `emit_transfer_event`](#0x1_NFT_emit_transfer_event)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event">0x1::Event</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID">0x1::GUID</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_NFT_Token"></a>

## Resource `Token`

Struct representing a semi-fungible or non-fungible token (depending on the supply).
There can be multiple tokens with the same id (unless supply is 1). Each token's
corresponding token metadata is stored inside a TokenData inside TokenDataCollection
under the creator's address.
The TokenData might be inlined together with the token in case the token is unique, i.e., its balance is 1
(we might choose to extend inlining for the non-unique NFTs in future).
The TokenData can also be separated out to a separate creator's collection in order to normalize the
data layout: we'd want to keep a single instance of the token data in case its balance is large.


<pre><code><b>struct</b> <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>token_data: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="NFT.md#0x1_NFT_TokenData">NFT::TokenData</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_NFT_TokenData"></a>

## Resource `TokenData`

Struct representing data of a specific token with token_id,
stored under the creator's address inside TokenDataCollection.
For each token_id, there is only one TokenData.


<pre><code><b>struct</b> <a href="NFT.md#0x1_NFT_TokenData">TokenData</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: TokenType</code>
</dt>
<dd>

</dd>
<dt>
<code>token_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_GUID">GUID::GUID</a></code>
</dt>
<dd>
 Identifier for the token.
</dd>
<dt>
<code>content_uri: vector&lt;u8&gt;</code>
</dt>
<dd>
 Pointer to where the content and metadata is stored.
</dd>
<dt>
<code>supply: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>parent_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;</code>
</dt>
<dd>
 Parent NFT id
</dd>
</dl>


</details>

<a name="0x1_NFT_TokenDataCollection"></a>

## Resource `TokenDataCollection`

The data of the NFT tokens is either kept inline (in case their balance is 1), or is detached and kept
in the token data collection by the original creator.


<pre><code><b>struct</b> <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tokens: vector&lt;<a href="NFT.md#0x1_NFT_TokenData">NFT::TokenData</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_NFT_MintEvent"></a>

## Struct `MintEvent`



<pre><code><b>struct</b> <a href="NFT.md#0x1_NFT_MintEvent">MintEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>content_uri: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_NFT_TransferEvent"></a>

## Struct `TransferEvent`



<pre><code><b>struct</b> <a href="NFT.md#0x1_NFT_TransferEvent">TransferEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>from: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_NFT_Admin"></a>

## Resource `Admin`



<pre><code><b>struct</b> <a href="NFT.md#0x1_NFT_Admin">Admin</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_events: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="NFT.md#0x1_NFT_MintEvent">NFT::MintEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transfer_events: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="NFT.md#0x1_NFT_TransferEvent">NFT::TransferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_NFT_CreationDelegation"></a>

## Resource `CreationDelegation`

Indicates that a user allows creation delegation for a given TokenType


<pre><code><b>struct</b> <a href="NFT.md#0x1_NFT_CreationDelegation">CreationDelegation</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>guid_capability: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_CreateCapability">GUID::CreateCapability</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_NFT_MAX_U64"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a name="0x1_NFT_ADMIN"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_ADMIN">ADMIN</a>: <b>address</b> = a550c18;
</code></pre>



<a name="0x1_NFT_EAMOUNT_EXCEEDS_TOKEN_BALANCE"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_EAMOUNT_EXCEEDS_TOKEN_BALANCE">EAMOUNT_EXCEEDS_TOKEN_BALANCE</a>: u64 = 3;
</code></pre>



<a name="0x1_NFT_ECREATION_DELEGATION_NOT_ALLOWED"></a>

Creation delegation for a given token type is not allowed.


<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_ECREATION_DELEGATION_NOT_ALLOWED">ECREATION_DELEGATION_NOT_ALLOWED</a>: u64 = 9;
</code></pre>



<a name="0x1_NFT_EINDEX_EXCEEDS_LENGTH"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_EINDEX_EXCEEDS_LENGTH">EINDEX_EXCEEDS_LENGTH</a>: u64 = 5;
</code></pre>



<a name="0x1_NFT_EINLINE_DATA_OP"></a>

Trying to merge or split tokens with inlined data.


<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_EINLINE_DATA_OP">EINLINE_DATA_OP</a>: u64 = 10;
</code></pre>



<a name="0x1_NFT_ENOT_ADMIN"></a>

Function can only be called by the admin address


<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_ENOT_ADMIN">ENOT_ADMIN</a>: u64 = 0;
</code></pre>



<a name="0x1_NFT_EPARENT_NOT_SAME_ACCOUNT"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_EPARENT_NOT_SAME_ACCOUNT">EPARENT_NOT_SAME_ACCOUNT</a>: u64 = 7;
</code></pre>



<a name="0x1_NFT_ETOKEN_BALANCE_OVERFLOWS"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_ETOKEN_BALANCE_OVERFLOWS">ETOKEN_BALANCE_OVERFLOWS</a>: u64 = 2;
</code></pre>



<a name="0x1_NFT_ETOKEN_DATA_COLLECTION_ALREADY_PUBLISHED"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_ETOKEN_DATA_COLLECTION_ALREADY_PUBLISHED">ETOKEN_DATA_COLLECTION_ALREADY_PUBLISHED</a>: u64 = 8;
</code></pre>



<a name="0x1_NFT_ETOKEN_EXTRACTED"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_ETOKEN_EXTRACTED">ETOKEN_EXTRACTED</a>: u64 = 4;
</code></pre>



<a name="0x1_NFT_ETOKEN_PRESENT"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_ETOKEN_PRESENT">ETOKEN_PRESENT</a>: u64 = 6;
</code></pre>



<a name="0x1_NFT_EWRONG_TOKEN_ID"></a>



<pre><code><b>const</b> <a href="NFT.md#0x1_NFT_EWRONG_TOKEN_ID">EWRONG_TOKEN_ID</a>: u64 = 1;
</code></pre>



<a name="0x1_NFT_nft_initialize"></a>

## Function `nft_initialize`

Initialize this module


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="NFT.md#0x1_NFT_nft_initialize">nft_initialize</a>(account: signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="NFT.md#0x1_NFT_nft_initialize">nft_initialize</a>(account: signer) {
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(&account) == <a href="NFT.md#0x1_NFT_ADMIN">ADMIN</a>, <a href="NFT.md#0x1_NFT_ENOT_ADMIN">ENOT_ADMIN</a>);
    <b>move_to</b>(&account, <a href="NFT.md#0x1_NFT_Admin">Admin</a> {
        mint_events: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="NFT.md#0x1_NFT_MintEvent">MintEvent</a>&gt;(&account),
        transfer_events: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="NFT.md#0x1_NFT_TransferEvent">TransferEvent</a>&gt;(&account),
    })
}
</code></pre>



</details>

<a name="0x1_NFT_id"></a>

## Function `id`

Returns the id of given token


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_id">id</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_id">id</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a> {
    *&token.id
}
</code></pre>



</details>

<a name="0x1_NFT_get_balance"></a>

## Function `get_balance`

Returns the balance of given token


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_balance">get_balance</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_balance">get_balance</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;): u64 {
    token.balance
}
</code></pre>



</details>

<a name="0x1_NFT_get_supply"></a>

## Function `get_supply`

Returns the overall supply for the given token


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_supply">get_supply</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_supply">get_supply</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;): u64 <b>acquires</b> <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>{
    <b>if</b> (<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&token.token_data)) {
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&token.token_data).supply
    } <b>else</b> {
        <b>let</b> creator_addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id_creator_address">GUID::id_creator_address</a>(&token.id);
        <b>let</b> creator_tokens_data = &<b>borrow_global</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(creator_addr).tokens;
        <b>let</b> token_data_idx = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&<a href="NFT.md#0x1_NFT_index_of_token">index_of_token</a>&lt;TokenType&gt;(creator_tokens_data, &token.id));
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(creator_tokens_data, token_data_idx).supply
    }
}
</code></pre>



</details>

<a name="0x1_NFT_get_content_uri"></a>

## Function `get_content_uri`

Returns a copy of the token content uri


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_content_uri">get_content_uri</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_content_uri">get_content_uri</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;): vector&lt;u8&gt; <b>acquires</b> <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> {
    <b>if</b> (<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&token.token_data)) {
        *&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&token.token_data).content_uri
    } <b>else</b> {
        <b>let</b> creator_addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id_creator_address">GUID::id_creator_address</a>(&token.id);
        <b>let</b> creator_tokens_data = &<b>borrow_global</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(creator_addr).tokens;
        <b>let</b> token_data_idx = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&<a href="NFT.md#0x1_NFT_index_of_token">index_of_token</a>&lt;TokenType&gt;(creator_tokens_data, &token.id));
        *&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(creator_tokens_data, token_data_idx).content_uri
    }
}
</code></pre>



</details>

<a name="0x1_NFT_get_metadata"></a>

## Function `get_metadata`

Returns a copy of the token metadata


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_metadata">get_metadata</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;): TokenType
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_metadata">get_metadata</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;): TokenType <b>acquires</b> <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> {
    <b>if</b> (<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&token.token_data)) {
        *&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&token.token_data).metadata
    } <b>else</b> {
        <b>let</b> creator_addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id_creator_address">GUID::id_creator_address</a>(&token.id);
        <b>let</b> creator_tokens_data = &<b>borrow_global</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(creator_addr).tokens;
        <b>let</b> token_data_idx = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&<a href="NFT.md#0x1_NFT_index_of_token">index_of_token</a>&lt;TokenType&gt;(creator_tokens_data, &token.id));
        *&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(creator_tokens_data, token_data_idx).metadata
    }
}
</code></pre>



</details>

<a name="0x1_NFT_get_parent_id"></a>

## Function `get_parent_id`

Returns a copy of the token metadata


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_parent_id">get_parent_id</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_get_parent_id">get_parent_id</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt; <b>acquires</b> <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> {
    <b>if</b> (<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&token.token_data)) {
        *&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&token.token_data).parent_id
    } <b>else</b> {
        <b>let</b> creator_addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id_creator_address">GUID::id_creator_address</a>(&token.id);
        <b>let</b> creator_tokens_data = &<b>borrow_global</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(creator_addr).tokens;
        <b>let</b> token_data_idx = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&<a href="NFT.md#0x1_NFT_index_of_token">index_of_token</a>&lt;TokenType&gt;(creator_tokens_data, &token.id));
        *&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(creator_tokens_data, token_data_idx).parent_id
    }
}
</code></pre>



</details>

<a name="0x1_NFT_is_data_inlined"></a>

## Function `is_data_inlined`

Returns true if the token is keeping the token data inlined.


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_is_data_inlined">is_data_inlined</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_is_data_inlined">is_data_inlined</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;): bool {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&token.token_data)
}
</code></pre>



</details>

<a name="0x1_NFT_index_of_token"></a>

## Function `index_of_token`

Finds the index of token with the given id in the gallery.


<pre><code><b>fun</b> <a href="NFT.md#0x1_NFT_index_of_token">index_of_token</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(gallery: &vector&lt;<a href="NFT.md#0x1_NFT_TokenData">NFT::TokenData</a>&lt;TokenType&gt;&gt;, id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="NFT.md#0x1_NFT_index_of_token">index_of_token</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(gallery: &vector&lt;<a href="NFT.md#0x1_NFT_TokenData">TokenData</a>&lt;TokenType&gt;&gt;, id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;u64&gt; {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(gallery);
    <b>while</b> (i &lt; len) {
        <b>if</b> (<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_eq_id">GUID::eq_id</a>(&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, i).token_id, id)) {
            <b>return</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(i)
        };
        i = i + 1;
    };
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>()
}
</code></pre>



</details>

<a name="0x1_NFT_join"></a>

## Function `join`

Adds the balance of <code>TokenID</code> to the balance of the given <code><a href="NFT.md#0x1_NFT_Token">Token</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_join">join</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<b>mut</b> <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;, other: <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_join">join</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<b>mut</b> <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;, other: <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;) {
    <b>let</b> <a href="NFT.md#0x1_NFT_Token">Token</a> { id, balance, token_data } = other;
    // Inlining is allowed for single-token NFTs only.
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_destroy_none">Option::destroy_none</a>(token_data); // aborts in case token data is not None
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_none">Option::is_none</a>(&token.token_data), <a href="NFT.md#0x1_NFT_EINLINE_DATA_OP">EINLINE_DATA_OP</a>);
    <b>assert</b>!(*&token.id == id, <a href="NFT.md#0x1_NFT_EWRONG_TOKEN_ID">EWRONG_TOKEN_ID</a>);
    <b>assert</b>!(<a href="NFT.md#0x1_NFT_MAX_U64">MAX_U64</a> - token.balance &gt;= balance, <a href="NFT.md#0x1_NFT_ETOKEN_BALANCE_OVERFLOWS">ETOKEN_BALANCE_OVERFLOWS</a>);
    token.balance = token.balance + balance;
}
</code></pre>



</details>

<a name="0x1_NFT_split_out"></a>

## Function `split_out`

Split out a new token with the given amount from the original token.
Aborts in case amount is greater or equal than the given token balance.


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_split_out">split_out</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(token: &<b>mut</b> <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;, amount: u64): <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_split_out">split_out</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(token: &<b>mut</b> <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt;, amount: u64): <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt; {
    <b>assert</b>!(token.balance &gt;= amount, <a href="NFT.md#0x1_NFT_EAMOUNT_EXCEEDS_TOKEN_BALANCE">EAMOUNT_EXCEEDS_TOKEN_BALANCE</a>);
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_none">Option::is_none</a>(&token.token_data), <a href="NFT.md#0x1_NFT_EINLINE_DATA_OP">EINLINE_DATA_OP</a>);

    token.balance = token.balance - amount;
    <a href="NFT.md#0x1_NFT_Token">Token</a> {
        id: *&token.id,
        balance: amount,
        token_data: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
    }
}
</code></pre>



</details>

<a name="0x1_NFT_create_for"></a>

## Function `create_for`

Create an NFT on behalf of the given user, in case a user explicitly approved this delegation for the given
NFT type.
Only the entity, which can create an object of <code>TokenType</code>, will be able to call this function.


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_create_for">create_for</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(creator: <b>address</b>, metadata: TokenType, content_uri: vector&lt;u8&gt;, amount: u64, parent_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;): <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_create_for">create_for</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(
    creator: <b>address</b>, metadata: TokenType, content_uri: vector&lt;u8&gt;, amount: u64, parent_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;
): <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt; <b>acquires</b> <a href="NFT.md#0x1_NFT_CreationDelegation">CreationDelegation</a>, <a href="NFT.md#0x1_NFT_Admin">Admin</a>, <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> {
    <b>assert</b>! (<b>exists</b>&lt;<a href="NFT.md#0x1_NFT_CreationDelegation">CreationDelegation</a>&lt;TokenType&gt;&gt;(creator), <a href="NFT.md#0x1_NFT_ECREATION_DELEGATION_NOT_ALLOWED">ECREATION_DELEGATION_NOT_ALLOWED</a>);
    <b>let</b> guid_creation_cap = &<b>borrow_global</b>&lt;<a href="NFT.md#0x1_NFT_CreationDelegation">CreationDelegation</a>&lt;TokenType&gt;&gt;(creator).guid_capability;
    <b>let</b> guid = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_create_with_capability">GUID::create_with_capability</a>(creator, guid_creation_cap);
    <a href="NFT.md#0x1_NFT_create_impl">create_impl</a>&lt;TokenType&gt;(
        creator,
        guid,
        metadata,
        content_uri,
        amount,
        parent_id
    )
}
</code></pre>



</details>

<a name="0x1_NFT_create"></a>

## Function `create`

Create a<code> <a href="NFT.md#0x1_NFT_TokenData">TokenData</a>&lt;TokenType&gt;</code> that wraps <code>metadata</code> and with balance of <code>amount</code>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_create">create</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer, metadata: TokenType, content_uri: vector&lt;u8&gt;, amount: u64, parent_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;): <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_create">create</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(
    account: &signer, metadata: TokenType, content_uri: vector&lt;u8&gt;, amount: u64, parent_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;
): <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt; <b>acquires</b> <a href="NFT.md#0x1_NFT_Admin">Admin</a>, <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> {
    <b>let</b> guid = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_create">GUID::create</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account))) {
        <b>move_to</b>(account, <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> { tokens: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>&lt;<a href="NFT.md#0x1_NFT_TokenData">TokenData</a>&lt;TokenType&gt;&gt;() });
    };
    <a href="NFT.md#0x1_NFT_create_impl">create_impl</a>&lt;TokenType&gt;(
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account),
        guid,
        metadata,
        content_uri,
        amount,
        parent_id
    )
}
</code></pre>



</details>

<a name="0x1_NFT_create_impl"></a>

## Function `create_impl`



<pre><code><b>fun</b> <a href="NFT.md#0x1_NFT_create_impl">create_impl</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(addr: <b>address</b>, guid: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_GUID">GUID::GUID</a>, metadata: TokenType, content_uri: vector&lt;u8&gt;, amount: u64, parent_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;): <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="NFT.md#0x1_NFT_create_impl">create_impl</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(
    addr: <b>address</b>,
    guid: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_GUID">GUID::GUID</a>,
    metadata: TokenType,
    content_uri: vector&lt;u8&gt;,
    amount: u64,
    parent_id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;
): <a href="NFT.md#0x1_NFT_Token">Token</a>&lt;TokenType&gt; <b>acquires</b> <a href="NFT.md#0x1_NFT_Admin">Admin</a>, <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>(
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="NFT.md#0x1_NFT_Admin">Admin</a>&gt;(<a href="NFT.md#0x1_NFT_ADMIN">ADMIN</a>).mint_events,
        <a href="NFT.md#0x1_NFT_MintEvent">MintEvent</a> {
            id: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id">GUID::id</a>(&guid),
            creator: addr,
            content_uri: <b>copy</b> content_uri,
            amount,
        }
    );
    <b>let</b> id = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id">GUID::id</a>(&guid);
    <b>let</b> token_data = <a href="NFT.md#0x1_NFT_TokenData">TokenData</a> { metadata, token_id: guid, content_uri, supply: amount, parent_id };
    <b>if</b> (amount == 1) {
        // inline token data
        <a href="NFT.md#0x1_NFT_Token">Token</a> { id, balance: amount, token_data: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(token_data) }
    } <b>else</b> {
        // keep token data in the collection of the creator
        <b>let</b> token_data_collection = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(addr).tokens;
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_push_back">Vector::push_back</a>(token_data_collection, token_data);
        <a href="NFT.md#0x1_NFT_Token">Token</a> { id, balance: amount, token_data: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>() }
    }
}
</code></pre>



</details>

<a name="0x1_NFT_publish_token_data_collection"></a>

## Function `publish_token_data_collection`



<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_publish_token_data_collection">publish_token_data_collection</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_publish_token_data_collection">publish_token_data_collection</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(account: &signer) {
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account)),
        <a href="NFT.md#0x1_NFT_ETOKEN_DATA_COLLECTION_ALREADY_PUBLISHED">ETOKEN_DATA_COLLECTION_ALREADY_PUBLISHED</a>
    );
    <b>move_to</b>(account, <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt; { tokens: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>() });
}
</code></pre>



</details>

<a name="0x1_NFT_allow_creation_delegation"></a>

## Function `allow_creation_delegation`

Allow creation delegation for a given TokenType (the entity, which can generate a metadata of a given TokenType
is going to be allowed to create an NFT on behalf of the user).
This is useful in case a user is using a 3rd party app, which can create NFTs on their behalf.


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_allow_creation_delegation">allow_creation_delegation</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_allow_creation_delegation">allow_creation_delegation</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(account: &signer) {
    <b>if</b> (!<b>exists</b>&lt;<a href="NFT.md#0x1_NFT_CreationDelegation">CreationDelegation</a>&lt;TokenType&gt;&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account))) {
        <b>move_to</b>(account, <a href="NFT.md#0x1_NFT_CreationDelegation">CreationDelegation</a>&lt;TokenType&gt; { guid_capability: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_gen_create_capability">GUID::gen_create_capability</a>(account) });
        // In order <b>to</b> support creation delegation, prepare the token data collection ahead of time.
        <b>if</b> (!<b>exists</b>&lt;<a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a>&lt;TokenType&gt;&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account))) {
            <b>move_to</b>(account, <a href="NFT.md#0x1_NFT_TokenDataCollection">TokenDataCollection</a> { tokens: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>&lt;<a href="NFT.md#0x1_NFT_TokenData">TokenData</a>&lt;TokenType&gt;&gt;() });
        };
    };
}
</code></pre>



</details>

<a name="0x1_NFT_emit_transfer_event"></a>

## Function `emit_transfer_event`



<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_emit_transfer_event">emit_transfer_event</a>(guid: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, account: &signer, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFT.md#0x1_NFT_emit_transfer_event">emit_transfer_event</a>(
    guid: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>,
    account: &signer,
    <b>to</b>: <b>address</b>,
    amount: u64,
) <b>acquires</b> <a href="NFT.md#0x1_NFT_Admin">Admin</a> {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>(
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="NFT.md#0x1_NFT_Admin">Admin</a>&gt;(<a href="NFT.md#0x1_NFT_ADMIN">ADMIN</a>).transfer_events,
        <a href="NFT.md#0x1_NFT_TransferEvent">TransferEvent</a> {
            id: *guid,
            from: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account),
            <b>to</b>: <b>to</b>,
            amount: amount,
        }
    );
}
</code></pre>



</details>
