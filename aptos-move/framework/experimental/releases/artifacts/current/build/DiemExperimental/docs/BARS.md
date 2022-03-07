
<a name="0x1_BARSToken"></a>

# Module `0x1::BARSToken`



-  [Struct `BARSToken`](#0x1_BARSToken_BARSToken)
-  [Constants](#@Constants_0)
-  [Function `register_bars_user`](#0x1_BARSToken_register_bars_user)
-  [Function `register_user_internal`](#0x1_BARSToken_register_user_internal)
-  [Function `mint_bars`](#0x1_BARSToken_mint_bars)
-  [Function `mint_internal`](#0x1_BARSToken_mint_internal)
-  [Function `create_bars_token`](#0x1_BARSToken_create_bars_token)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID">0x1::GUID</a>;
<b>use</b> <a href="NFT.md#0x1_NFT">0x1::NFT</a>;
<b>use</b> <a href="NFTGallery.md#0x1_NFTGallery">0x1::NFTGallery</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
</code></pre>



<a name="0x1_BARSToken_BARSToken"></a>

## Struct `BARSToken`



<pre><code><b>struct</b> <a href="BARS.md#0x1_BARSToken">BARSToken</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>artist_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_BARSToken_ENOT_BARS_OWNER"></a>

Function can only be called by the module owner


<pre><code><b>const</b> <a href="BARS.md#0x1_BARSToken_ENOT_BARS_OWNER">ENOT_BARS_OWNER</a>: u64 = 0;
</code></pre>



<a name="0x1_BARSToken_register_bars_user"></a>

## Function `register_bars_user`

Call this function to set up relevant resources in order to
mint and receive tokens.
Note that this also gives BARS account a capability to mint BARS NFTs on behalf of the user.
(the NFTs of other types cannot be created by BARS account).


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BARS.md#0x1_BARSToken_register_bars_user">register_bars_user</a>(user: signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BARS.md#0x1_BARSToken_register_bars_user">register_bars_user</a>(user: signer) {
    <a href="BARS.md#0x1_BARSToken_register_user_internal">register_user_internal</a>(&user);
}
</code></pre>



</details>

<a name="0x1_BARSToken_register_user_internal"></a>

## Function `register_user_internal`

Need this internal function for testing, since the script fun version
consumes a signer


<pre><code><b>fun</b> <a href="BARS.md#0x1_BARSToken_register_user_internal">register_user_internal</a>(user: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="BARS.md#0x1_BARSToken_register_user_internal">register_user_internal</a>(user: &signer) {
    // publish TokenBalance&lt;<a href="BARS.md#0x1_BARSToken">BARSToken</a>&gt; resource
    <a href="NFTGallery.md#0x1_NFTGallery_publish_gallery">NFTGallery::publish_gallery</a>&lt;<a href="BARS.md#0x1_BARSToken">BARSToken</a>&gt;(user);

    // publish TokenDataCollection&lt;<a href="BARS.md#0x1_BARSToken">BARSToken</a>&gt; resource
    <a href="NFT.md#0x1_NFT_publish_token_data_collection">NFT::publish_token_data_collection</a>&lt;<a href="BARS.md#0x1_BARSToken">BARSToken</a>&gt;(user);

    // The user gives BARS account capability <b>to</b> generate BARS NFTs on their behalf.
    <a href="NFT.md#0x1_NFT_allow_creation_delegation">NFT::allow_creation_delegation</a>&lt;<a href="BARS.md#0x1_BARSToken">BARSToken</a>&gt;(user);
}
</code></pre>



</details>

<a name="0x1_BARSToken_mint_bars"></a>

## Function `mint_bars`

BARS account mints <code>amount</code> copies of BARS tokens to the artist's account.


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BARS.md#0x1_BARSToken_mint_bars">mint_bars</a>(bars_account: signer, artist: <b>address</b>, artist_name: vector&lt;u8&gt;, content_uri: vector&lt;u8&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BARS.md#0x1_BARSToken_mint_bars">mint_bars</a>(
    bars_account: signer,
    artist: <b>address</b>,
    artist_name: vector&lt;u8&gt;,
    content_uri: vector&lt;u8&gt;,
    amount: u64
) {
    <a href="BARS.md#0x1_BARSToken_mint_internal">mint_internal</a>(&bars_account, artist, artist_name, content_uri, amount);
}
</code></pre>



</details>

<a name="0x1_BARSToken_mint_internal"></a>

## Function `mint_internal`

Need this internal function for testing, since the script fun version
consumes a signer


<pre><code><b>fun</b> <a href="BARS.md#0x1_BARSToken_mint_internal">mint_internal</a>(bars_account: &signer, artist: <b>address</b>, artist_name: vector&lt;u8&gt;, content_uri: vector&lt;u8&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="BARS.md#0x1_BARSToken_mint_internal">mint_internal</a>(
    bars_account: &signer,
    artist: <b>address</b>,
    artist_name: vector&lt;u8&gt;,
    content_uri: vector&lt;u8&gt;,
    amount: u64
) {
    <b>let</b> token = <a href="NFT.md#0x1_NFT_create_for">NFT::create_for</a>&lt;<a href="BARS.md#0x1_BARSToken">BARSToken</a>&gt;(
        artist,
        <a href="BARS.md#0x1_BARSToken_create_bars_token">create_bars_token</a>(bars_account, artist_name),
        content_uri,
        amount,
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
    );
    <a href="NFTGallery.md#0x1_NFTGallery_add_to_gallery">NFTGallery::add_to_gallery</a>(artist, token);
}
</code></pre>



</details>

<a name="0x1_BARSToken_create_bars_token"></a>

## Function `create_bars_token`



<pre><code><b>fun</b> <a href="BARS.md#0x1_BARSToken_create_bars_token">create_bars_token</a>(<b>address</b>: &signer, artist_name: vector&lt;u8&gt;): <a href="BARS.md#0x1_BARSToken_BARSToken">BARSToken::BARSToken</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="BARS.md#0x1_BARSToken_create_bars_token">create_bars_token</a>(<b>address</b>: &signer, artist_name: vector&lt;u8&gt;): <a href="BARS.md#0x1_BARSToken">BARSToken</a> {
    <b>assert</b>!(Std::Signer::address_of(<b>address</b>) == @BARS, <a href="BARS.md#0x1_BARSToken_ENOT_BARS_OWNER">ENOT_BARS_OWNER</a>);
    <a href="BARS.md#0x1_BARSToken">BARSToken</a> { artist_name }
}
</code></pre>



</details>
