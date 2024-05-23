
<a id="0x3_token_coin_swap"></a>

# Module `0x3::token_coin_swap`

Deprecated module


-  [Struct `TokenCoinSwap`](#0x3_token_coin_swap_TokenCoinSwap)
-  [Resource `TokenListings`](#0x3_token_coin_swap_TokenListings)
-  [Struct `TokenEscrow`](#0x3_token_coin_swap_TokenEscrow)
-  [Resource `TokenStoreEscrow`](#0x3_token_coin_swap_TokenStoreEscrow)
-  [Struct `TokenListingEvent`](#0x3_token_coin_swap_TokenListingEvent)
-  [Struct `TokenSwapEvent`](#0x3_token_coin_swap_TokenSwapEvent)
-  [Constants](#@Constants_0)
-  [Function `does_listing_exist`](#0x3_token_coin_swap_does_listing_exist)
-  [Function `exchange_coin_for_token`](#0x3_token_coin_swap_exchange_coin_for_token)
-  [Function `list_token_for_swap`](#0x3_token_coin_swap_list_token_for_swap)
-  [Function `initialize_token_listing`](#0x3_token_coin_swap_initialize_token_listing)
-  [Function `initialize_token_store_escrow`](#0x3_token_coin_swap_initialize_token_store_escrow)
-  [Function `deposit_token_to_escrow`](#0x3_token_coin_swap_deposit_token_to_escrow)
-  [Function `withdraw_token_from_escrow_internal`](#0x3_token_coin_swap_withdraw_token_from_escrow_internal)
-  [Function `withdraw_token_from_escrow`](#0x3_token_coin_swap_withdraw_token_from_escrow)
-  [Function `cancel_token_listing`](#0x3_token_coin_swap_cancel_token_listing)
-  [Specification](#@Specification_1)


<pre><code>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::string;<br/>use 0x1::table;<br/>use 0x1::type_info;<br/>use 0x3::token;<br/></code></pre>



<a id="0x3_token_coin_swap_TokenCoinSwap"></a>

## Struct `TokenCoinSwap`

TokenCoinSwap records a swap ask for swapping token_amount with CoinType with a minimal price per token


<pre><code>struct TokenCoinSwap&lt;CoinType&gt; has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>min_price_per_token: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenListings"></a>

## Resource `TokenListings`

The listing of all tokens for swapping stored at token owner&apos;s account


<pre><code>struct TokenListings&lt;CoinType&gt; has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>listings: table::Table&lt;token::TokenId, token_coin_swap::TokenCoinSwap&lt;CoinType&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>listing_events: event::EventHandle&lt;token_coin_swap::TokenListingEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>swap_events: event::EventHandle&lt;token_coin_swap::TokenSwapEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenEscrow"></a>

## Struct `TokenEscrow`

TokenEscrow holds the tokens that cannot be withdrawn or transferred


<pre><code>struct TokenEscrow has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token: token::Token</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_until_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenStoreEscrow"></a>

## Resource `TokenStoreEscrow`

TokenStoreEscrow holds a map of token id to their tokenEscrow


<pre><code>struct TokenStoreEscrow has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_escrows: table::Table&lt;token::TokenId, token_coin_swap::TokenEscrow&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenListingEvent"></a>

## Struct `TokenListingEvent`



<pre><code>struct TokenListingEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>min_price: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_until_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>coin_type_info: type_info::TypeInfo</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenSwapEvent"></a>

## Struct `TokenSwapEvent`



<pre><code>struct TokenSwapEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>token_buyer: address</code>
</dt>
<dd>

</dd>
<dt>
<code>token_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>coin_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>coin_type_info: type_info::TypeInfo</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_token_coin_swap_EDEPRECATED_MODULE"></a>

Deprecated module


<pre><code>const EDEPRECATED_MODULE: u64 &#61; 8;<br/></code></pre>



<a id="0x3_token_coin_swap_ENOT_ENOUGH_COIN"></a>

Not enough coin to buy token


<pre><code>const ENOT_ENOUGH_COIN: u64 &#61; 7;<br/></code></pre>



<a id="0x3_token_coin_swap_ETOKEN_ALREADY_LISTED"></a>

Token already listed


<pre><code>const ETOKEN_ALREADY_LISTED: u64 &#61; 1;<br/></code></pre>



<a id="0x3_token_coin_swap_ETOKEN_AMOUNT_NOT_MATCH"></a>

Token buy amount doesn&apos;t match listing amount


<pre><code>const ETOKEN_AMOUNT_NOT_MATCH: u64 &#61; 6;<br/></code></pre>



<a id="0x3_token_coin_swap_ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME"></a>

Token cannot be moved out of escrow before the lockup time


<pre><code>const ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME: u64 &#61; 4;<br/></code></pre>



<a id="0x3_token_coin_swap_ETOKEN_LISTING_NOT_EXIST"></a>

Token listing no longer exists


<pre><code>const ETOKEN_LISTING_NOT_EXIST: u64 &#61; 2;<br/></code></pre>



<a id="0x3_token_coin_swap_ETOKEN_MIN_PRICE_NOT_MATCH"></a>

Token buy price doesn&apos;t match listing price


<pre><code>const ETOKEN_MIN_PRICE_NOT_MATCH: u64 &#61; 5;<br/></code></pre>



<a id="0x3_token_coin_swap_ETOKEN_NOT_IN_ESCROW"></a>

Token is not in escrow


<pre><code>const ETOKEN_NOT_IN_ESCROW: u64 &#61; 3;<br/></code></pre>



<a id="0x3_token_coin_swap_does_listing_exist"></a>

## Function `does_listing_exist`



<pre><code>public fun does_listing_exist&lt;CoinType&gt;(_token_owner: address, _token_id: token::TokenId): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun does_listing_exist&lt;CoinType&gt;(<br/>    _token_owner: address,<br/>    _token_id: TokenId<br/>): bool &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_exchange_coin_for_token"></a>

## Function `exchange_coin_for_token`

Coin owner withdraw coin to swap with tokens listed for swapping at the token owner&apos;s address.


<pre><code>public fun exchange_coin_for_token&lt;CoinType&gt;(_coin_owner: &amp;signer, _coin_amount: u64, _token_owner: address, _creators_address: address, _collection: string::String, _name: string::String, _property_version: u64, _token_amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun exchange_coin_for_token&lt;CoinType&gt;(<br/>    _coin_owner: &amp;signer,<br/>    _coin_amount: u64,<br/>    _token_owner: address,<br/>    _creators_address: address,<br/>    _collection: String,<br/>    _name: String,<br/>    _property_version: u64,<br/>    _token_amount: u64,<br/>) &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_list_token_for_swap"></a>

## Function `list_token_for_swap`

Token owner lists their token for swapping


<pre><code>public entry fun list_token_for_swap&lt;CoinType&gt;(_token_owner: &amp;signer, _creators_address: address, _collection: string::String, _name: string::String, _property_version: u64, _token_amount: u64, _min_coin_per_token: u64, _locked_until_secs: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun list_token_for_swap&lt;CoinType&gt;(<br/>    _token_owner: &amp;signer,<br/>    _creators_address: address,<br/>    _collection: String,<br/>    _name: String,<br/>    _property_version: u64,<br/>    _token_amount: u64,<br/>    _min_coin_per_token: u64,<br/>    _locked_until_secs: u64<br/>) &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_initialize_token_listing"></a>

## Function `initialize_token_listing`

Initalize the token listing for a token owner


<pre><code>fun initialize_token_listing&lt;CoinType&gt;(_token_owner: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_token_listing&lt;CoinType&gt;(_token_owner: &amp;signer) &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_initialize_token_store_escrow"></a>

## Function `initialize_token_store_escrow`

Intialize the token escrow


<pre><code>fun initialize_token_store_escrow(_token_owner: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_token_store_escrow(_token_owner: &amp;signer) &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_deposit_token_to_escrow"></a>

## Function `deposit_token_to_escrow`

Put the token into escrow that cannot be transferred or withdrawed by the owner.


<pre><code>public fun deposit_token_to_escrow(_token_owner: &amp;signer, _token_id: token::TokenId, _tokens: token::Token, _locked_until_secs: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_token_to_escrow(<br/>    _token_owner: &amp;signer,<br/>    _token_id: TokenId,<br/>    _tokens: Token,<br/>    _locked_until_secs: u64<br/>) &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_withdraw_token_from_escrow_internal"></a>

## Function `withdraw_token_from_escrow_internal`

Private function for withdraw tokens from an escrow stored in token owner address


<pre><code>fun withdraw_token_from_escrow_internal(_token_owner_addr: address, _token_id: token::TokenId, _amount: u64): token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun withdraw_token_from_escrow_internal(<br/>    _token_owner_addr: address,<br/>    _token_id: TokenId,<br/>    _amount: u64<br/>): Token &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_withdraw_token_from_escrow"></a>

## Function `withdraw_token_from_escrow`

Withdraw tokens from the token escrow. It needs a signer to authorize


<pre><code>public fun withdraw_token_from_escrow(_token_owner: &amp;signer, _token_id: token::TokenId, _amount: u64): token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_token_from_escrow(<br/>    _token_owner: &amp;signer,<br/>    _token_id: TokenId,<br/>    _amount: u64<br/>): Token &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_coin_swap_cancel_token_listing"></a>

## Function `cancel_token_listing`

Cancel token listing for a fixed amount


<pre><code>public fun cancel_token_listing&lt;CoinType&gt;(_token_owner: &amp;signer, _token_id: token::TokenId, _token_amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun cancel_token_listing&lt;CoinType&gt;(<br/>    _token_owner: &amp;signer,<br/>    _token_id: TokenId,<br/>    _token_amount: u64,<br/>) &#123;<br/>    abort error::invalid_argument(EDEPRECATED_MODULE)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
