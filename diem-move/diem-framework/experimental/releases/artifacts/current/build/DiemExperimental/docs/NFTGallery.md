
<a name="0x1_NFTGallery"></a>

# Module `0x1::NFTGallery`



-  [Resource `NFTGallery`](#0x1_NFTGallery_NFTGallery)
-  [Constants](#@Constants_0)
-  [Function `add_to_gallery`](#0x1_NFTGallery_add_to_gallery)
-  [Function `has_token`](#0x1_NFTGallery_has_token)
-  [Function `get_token_balance`](#0x1_NFTGallery_get_token_balance)
-  [Function `get_token_supply`](#0x1_NFTGallery_get_token_supply)
-  [Function `get_token_content_uri`](#0x1_NFTGallery_get_token_content_uri)
-  [Function `get_token_metadata`](#0x1_NFTGallery_get_token_metadata)
-  [Function `get_token_parent_id`](#0x1_NFTGallery_get_token_parent_id)
-  [Function `transfer_token_between_galleries`](#0x1_NFTGallery_transfer_token_between_galleries)
-  [Function `transfer_token_between_galleries_impl`](#0x1_NFTGallery_transfer_token_between_galleries_impl)
-  [Function `publish_gallery`](#0x1_NFTGallery_publish_gallery)
-  [Function `index_of_token`](#0x1_NFTGallery_index_of_token)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID">0x1::GUID</a>;
<b>use</b> <a href="NFT.md#0x1_NFT">0x1::NFT</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_NFTGallery_NFTGallery"></a>

## Resource `NFTGallery`

Gallery holding tokens of <code>TokenType</code> as well as information of approved operators.


<pre><code><b>struct</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gallery: vector&lt;<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_NFTGallery_EGALLERY_ALREADY_PUBLISHED"></a>



<pre><code><b>const</b> <a href="NFTGallery.md#0x1_NFTGallery_EGALLERY_ALREADY_PUBLISHED">EGALLERY_ALREADY_PUBLISHED</a>: u64 = 2;
</code></pre>



<a name="0x1_NFTGallery_EGALLERY_NOT_PUBLISHED"></a>



<pre><code><b>const</b> <a href="NFTGallery.md#0x1_NFTGallery_EGALLERY_NOT_PUBLISHED">EGALLERY_NOT_PUBLISHED</a>: u64 = 1;
</code></pre>



<a name="0x1_NFTGallery_EID_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="NFTGallery.md#0x1_NFTGallery_EID_NOT_FOUND">EID_NOT_FOUND</a>: u64 = 0;
</code></pre>



<a name="0x1_NFTGallery_EINVALID_AMOUNT_OF_TRANSFER"></a>



<pre><code><b>const</b> <a href="NFTGallery.md#0x1_NFTGallery_EINVALID_AMOUNT_OF_TRANSFER">EINVALID_AMOUNT_OF_TRANSFER</a>: u64 = 3;
</code></pre>



<a name="0x1_NFTGallery_add_to_gallery"></a>

## Function `add_to_gallery`

Add a token to the owner's gallery.
The specifics of the addition depend on the token data inlining.
In case the token data is inlined, the addition is trivial (join / split operations are not allowed).
Otherwise, the addition might include joining of the two tokens.


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_add_to_gallery">add_to_gallery</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(owner: <b>address</b>, token: <a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_add_to_gallery">add_to_gallery</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(owner: <b>address</b>, token: Token&lt;TokenType&gt;)
<b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner), <a href="NFTGallery.md#0x1_NFTGallery_EGALLERY_NOT_PUBLISHED">EGALLERY_NOT_PUBLISHED</a>);
    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery;
    <b>if</b> (!<a href="NFT.md#0x1_NFT_is_data_inlined">NFT::is_data_inlined</a>&lt;TokenType&gt;(&token)) {
        <b>let</b> index_opt = <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType&gt;(gallery, &<a href="NFT.md#0x1_NFT_id">NFT::id</a>&lt;TokenType&gt;(&token));
        <b>if</b> (<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&index_opt)) {
            <b>let</b> prev_token_idx = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_extract">Option::extract</a>(&<b>mut</b> index_opt);
            // The gallery already <b>has</b> the given token: <b>update</b> its balance
            <a href="NFT.md#0x1_NFT_join">NFT::join</a>&lt;TokenType&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow_mut">Vector::borrow_mut</a>(gallery, prev_token_idx), token);
            <b>return</b>
        }
    };
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_push_back">Vector::push_back</a>(gallery, token)
}
</code></pre>



</details>

<a name="0x1_NFTGallery_has_token"></a>

## Function `has_token`

Returns whether the owner has a token with given id.


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_has_token">has_token</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_has_token">has_token</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): bool <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&<a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>(&<b>borrow_global</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery, token_id))
}
</code></pre>



</details>

<a name="0x1_NFTGallery_get_token_balance"></a>

## Function `get_token_balance`



<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_balance">get_token_balance</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_balance">get_token_balance</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
): u64 <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <b>let</b> gallery = &<b>borrow_global</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery;
    <b>let</b> index_opt = <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType&gt;(gallery, token_id);
    <b>if</b> (<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_none">Option::is_none</a>(&index_opt)) {
        0
    } <b>else</b> {
        <b>let</b> token = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_extract">Option::extract</a>(&<b>mut</b> index_opt));
        <a href="NFT.md#0x1_NFT_get_balance">NFT::get_balance</a>(token)
    }
}
</code></pre>



</details>

<a name="0x1_NFTGallery_get_token_supply"></a>

## Function `get_token_supply`

Returns the overall supply for the given token (across this and potentially other galleries),


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_supply">get_token_supply</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_supply">get_token_supply</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): u64 <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <b>let</b> gallery = &<b>borrow_global</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery;
    <b>let</b> index_opt = <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType&gt;(gallery, token_id);
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&index_opt), <a href="NFTGallery.md#0x1_NFTGallery_EID_NOT_FOUND">EID_NOT_FOUND</a>);
    <b>let</b> token = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_extract">Option::extract</a>(&<b>mut</b> index_opt));
    <a href="NFT.md#0x1_NFT_get_supply">NFT::get_supply</a>(token)
}
</code></pre>



</details>

<a name="0x1_NFTGallery_get_token_content_uri"></a>

## Function `get_token_content_uri`

Returns a copy of the token content uri


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_content_uri">get_token_content_uri</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_content_uri">get_token_content_uri</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): vector&lt;u8&gt; <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <b>let</b> gallery = &<b>borrow_global</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery;
    <b>let</b> index_opt = <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType&gt;(gallery, token_id);
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&index_opt), <a href="NFTGallery.md#0x1_NFTGallery_EID_NOT_FOUND">EID_NOT_FOUND</a>);
    <b>let</b> token = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_extract">Option::extract</a>(&<b>mut</b> index_opt));
    <a href="NFT.md#0x1_NFT_get_content_uri">NFT::get_content_uri</a>(token)
}
</code></pre>



</details>

<a name="0x1_NFTGallery_get_token_metadata"></a>

## Function `get_token_metadata`

Returns a copy of the token metadata


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_metadata">get_token_metadata</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): TokenType
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_metadata">get_token_metadata</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): TokenType <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <b>let</b> gallery = &<b>borrow_global</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery;
    <b>let</b> index_opt = <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType&gt;(gallery, token_id);
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&index_opt), <a href="NFTGallery.md#0x1_NFTGallery_EID_NOT_FOUND">EID_NOT_FOUND</a>);
    <b>let</b> token = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_extract">Option::extract</a>(&<b>mut</b> index_opt));
    <a href="NFT.md#0x1_NFT_get_metadata">NFT::get_metadata</a>(token)
}
</code></pre>



</details>

<a name="0x1_NFTGallery_get_token_parent_id"></a>

## Function `get_token_parent_id`

Returns a copy of the token parent id


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_parent_id">get_token_parent_id</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_get_token_parent_id">get_token_parent_id</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(owner: <b>address</b>, token_id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>&gt; <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <b>let</b> gallery = &<b>borrow_global</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery;
    <b>let</b> index_opt = <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType&gt;(gallery, token_id);
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&index_opt), <a href="NFTGallery.md#0x1_NFTGallery_EID_NOT_FOUND">EID_NOT_FOUND</a>);
    <b>let</b> token = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_extract">Option::extract</a>(&<b>mut</b> index_opt));
    <a href="NFT.md#0x1_NFT_get_parent_id">NFT::get_parent_id</a>(token)
}
</code></pre>



</details>

<a name="0x1_NFTGallery_transfer_token_between_galleries"></a>

## Function `transfer_token_between_galleries`

Transfer <code>amount</code> of token with id <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_id">GUID::id</a>(creator, creation_num)</code> from <code>owner</code>'s
balance to <code><b>to</b></code>'s balance. This operation has to be done by either the owner or an
approved operator of the owner.


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_transfer_token_between_galleries">transfer_token_between_galleries</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: signer, <b>to</b>: <b>address</b>, amount: u64, creator: <b>address</b>, creation_num: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_transfer_token_between_galleries">transfer_token_between_galleries</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(
    account: signer,
    <b>to</b>: <b>address</b>,
    amount: u64,
    creator: <b>address</b>,
    creation_num: u64
) <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <a href="NFTGallery.md#0x1_NFTGallery_transfer_token_between_galleries_impl">transfer_token_between_galleries_impl</a>&lt;TokenType&gt;(&account, <b>to</b>, amount, creator, creation_num)
}
</code></pre>



</details>

<a name="0x1_NFTGallery_transfer_token_between_galleries_impl"></a>

## Function `transfer_token_between_galleries_impl`

The implementation, which doesn't consume signer, and thus can be used for testing.


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_transfer_token_between_galleries_impl">transfer_token_between_galleries_impl</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer, <b>to</b>: <b>address</b>, amount: u64, creator: <b>address</b>, creation_num: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_transfer_token_between_galleries_impl">transfer_token_between_galleries_impl</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(
    account: &signer,
    <b>to</b>: <b>address</b>,
    amount: u64,
    creator: <b>address</b>,
    creation_num: u64
) <b>acquires</b> <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a> {
    <b>let</b> owner = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>assert</b>!(amount &gt; 0, <a href="NFTGallery.md#0x1_NFTGallery_EINVALID_AMOUNT_OF_TRANSFER">EINVALID_AMOUNT_OF_TRANSFER</a>);
    <b>let</b> gallery = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(owner).gallery;
    <b>let</b> id = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_create_id">GUID::create_id</a>(creator, creation_num);

    <b>let</b> index_opt = <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType&gt;(gallery, &id);
    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&index_opt), <a href="NFTGallery.md#0x1_NFTGallery_EID_NOT_FOUND">EID_NOT_FOUND</a>);
    <b>let</b> from_token_idx = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_extract">Option::extract</a>(&<b>mut</b> index_opt);

    <b>if</b> (<a href="NFT.md#0x1_NFT_is_data_inlined">NFT::is_data_inlined</a>(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, from_token_idx)) ||
            <a href="NFT.md#0x1_NFT_get_balance">NFT::get_balance</a>(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, from_token_idx)) == amount) {
        // Move the token from one gallery <b>to</b> another
        <b>let</b> token = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_remove">Vector::remove</a>(gallery, from_token_idx);
        <a href="NFTGallery.md#0x1_NFTGallery_add_to_gallery">add_to_gallery</a>&lt;TokenType&gt;(<b>to</b>, token)
    } <b>else</b> {
        // Split the original token and add the splitted part <b>to</b> another gallery
        <b>let</b> split_out_token = <a href="NFT.md#0x1_NFT_split_out">NFT::split_out</a>(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow_mut">Vector::borrow_mut</a>(gallery, from_token_idx), amount);
        <a href="NFTGallery.md#0x1_NFTGallery_add_to_gallery">add_to_gallery</a>&lt;TokenType&gt;(<b>to</b>, split_out_token)
    };
    // Emit transfer event
    <a href="NFT.md#0x1_NFT_emit_transfer_event">NFT::emit_transfer_event</a>(
        &id,
        account,
        <b>to</b>,
        amount,
    )
}
</code></pre>



</details>

<a name="0x1_NFTGallery_publish_gallery"></a>

## Function `publish_gallery`



<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_publish_gallery">publish_gallery</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_publish_gallery">publish_gallery</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(account: &signer) {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt;&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account)), <a href="NFTGallery.md#0x1_NFTGallery_EGALLERY_ALREADY_PUBLISHED">EGALLERY_ALREADY_PUBLISHED</a>);
    <b>move_to</b>(account, <a href="NFTGallery.md#0x1_NFTGallery">NFTGallery</a>&lt;TokenType&gt; { gallery: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>() });
}
</code></pre>



</details>

<a name="0x1_NFTGallery_index_of_token"></a>

## Function `index_of_token`

Finds the index of token with the given id in the gallery.


<pre><code><b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(gallery: &vector&lt;<a href="NFT.md#0x1_NFT_Token">NFT::Token</a>&lt;TokenType&gt;&gt;, id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="NFTGallery.md#0x1_NFTGallery_index_of_token">index_of_token</a>&lt;TokenType: <b>copy</b> + store + drop&gt;(gallery: &vector&lt;Token&lt;TokenType&gt;&gt;, id: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>): <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;u64&gt; {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(gallery);
    <b>while</b> (i &lt; len) {
        <b>if</b> (<a href="NFT.md#0x1_NFT_id">NFT::id</a>&lt;TokenType&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(gallery, i)) == *id) {
            <b>return</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(i)
        };
        i = i + 1;
    };
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>()
}
</code></pre>



</details>
