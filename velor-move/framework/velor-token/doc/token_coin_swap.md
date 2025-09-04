
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


<pre><code><b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="token.md#0x3_token">0x3::token</a>;
</code></pre>



<a id="0x3_token_coin_swap_TokenCoinSwap"></a>

## Struct `TokenCoinSwap`

TokenCoinSwap records a swap ask for swapping token_amount with CoinType with a minimal price per token


<pre><code><b>struct</b> <a href="token_coin_swap.md#0x3_token_coin_swap_TokenCoinSwap">TokenCoinSwap</a>&lt;CoinType&gt; <b>has</b> drop, store
</code></pre>



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

The listing of all tokens for swapping stored at token owner's account


<pre><code><b>struct</b> <a href="token_coin_swap.md#0x3_token_coin_swap_TokenListings">TokenListings</a>&lt;CoinType&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>listings: <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token.md#0x3_token_TokenId">token::TokenId</a>, <a href="token_coin_swap.md#0x3_token_coin_swap_TokenCoinSwap">token_coin_swap::TokenCoinSwap</a>&lt;CoinType&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>listing_events: <a href="../../velor-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_coin_swap.md#0x3_token_coin_swap_TokenListingEvent">token_coin_swap::TokenListingEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>swap_events: <a href="../../velor-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_coin_swap.md#0x3_token_coin_swap_TokenSwapEvent">token_coin_swap::TokenSwapEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenEscrow"></a>

## Struct `TokenEscrow`

TokenEscrow holds the tokens that cannot be withdrawn or transferred


<pre><code><b>struct</b> <a href="token_coin_swap.md#0x3_token_coin_swap_TokenEscrow">TokenEscrow</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a></code>
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


<pre><code><b>struct</b> <a href="token_coin_swap.md#0x3_token_coin_swap_TokenStoreEscrow">TokenStoreEscrow</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_escrows: <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token.md#0x3_token_TokenId">token::TokenId</a>, <a href="token_coin_swap.md#0x3_token_coin_swap_TokenEscrow">token_coin_swap::TokenEscrow</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenListingEvent"></a>

## Struct `TokenListingEvent`



<pre><code><b>struct</b> <a href="token_coin_swap.md#0x3_token_coin_swap_TokenListingEvent">TokenListingEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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
<code>coin_type_info: <a href="../../velor-framework/../velor-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_coin_swap_TokenSwapEvent"></a>

## Struct `TokenSwapEvent`



<pre><code><b>struct</b> <a href="token_coin_swap.md#0x3_token_coin_swap_TokenSwapEvent">TokenSwapEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>token_buyer: <b>address</b></code>
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
<code>coin_type_info: <a href="../../velor-framework/../velor-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_token_coin_swap_EDEPRECATED_MODULE"></a>

Deprecated module


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>: u64 = 8;
</code></pre>



<a id="0x3_token_coin_swap_ENOT_ENOUGH_COIN"></a>

Not enough coin to buy token


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_ENOT_ENOUGH_COIN">ENOT_ENOUGH_COIN</a>: u64 = 7;
</code></pre>



<a id="0x3_token_coin_swap_ETOKEN_ALREADY_LISTED"></a>

Token already listed


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_ETOKEN_ALREADY_LISTED">ETOKEN_ALREADY_LISTED</a>: u64 = 1;
</code></pre>



<a id="0x3_token_coin_swap_ETOKEN_AMOUNT_NOT_MATCH"></a>

Token buy amount doesn't match listing amount


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_ETOKEN_AMOUNT_NOT_MATCH">ETOKEN_AMOUNT_NOT_MATCH</a>: u64 = 6;
</code></pre>



<a id="0x3_token_coin_swap_ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME"></a>

Token cannot be moved out of escrow before the lockup time


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME">ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME</a>: u64 = 4;
</code></pre>



<a id="0x3_token_coin_swap_ETOKEN_LISTING_NOT_EXIST"></a>

Token listing no longer exists


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_ETOKEN_LISTING_NOT_EXIST">ETOKEN_LISTING_NOT_EXIST</a>: u64 = 2;
</code></pre>



<a id="0x3_token_coin_swap_ETOKEN_MIN_PRICE_NOT_MATCH"></a>

Token buy price doesn't match listing price


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_ETOKEN_MIN_PRICE_NOT_MATCH">ETOKEN_MIN_PRICE_NOT_MATCH</a>: u64 = 5;
</code></pre>



<a id="0x3_token_coin_swap_ETOKEN_NOT_IN_ESCROW"></a>

Token is not in escrow


<pre><code><b>const</b> <a href="token_coin_swap.md#0x3_token_coin_swap_ETOKEN_NOT_IN_ESCROW">ETOKEN_NOT_IN_ESCROW</a>: u64 = 3;
</code></pre>



<a id="0x3_token_coin_swap_does_listing_exist"></a>

## Function `does_listing_exist`



<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_does_listing_exist">does_listing_exist</a>&lt;CoinType&gt;(_token_owner: <b>address</b>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_does_listing_exist">does_listing_exist</a>&lt;CoinType&gt;(
    _token_owner: <b>address</b>,
    _token_id: TokenId
): bool {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_exchange_coin_for_token"></a>

## Function `exchange_coin_for_token`

Coin owner withdraw coin to swap with tokens listed for swapping at the token owner's address.


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_exchange_coin_for_token">exchange_coin_for_token</a>&lt;CoinType&gt;(_coin_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _coin_amount: u64, _token_owner: <b>address</b>, _creators_address: <b>address</b>, _collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, _name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, _property_version: u64, _token_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_exchange_coin_for_token">exchange_coin_for_token</a>&lt;CoinType&gt;(
    _coin_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _coin_amount: u64,
    _token_owner: <b>address</b>,
    _creators_address: <b>address</b>,
    _collection: String,
    _name: String,
    _property_version: u64,
    _token_amount: u64,
) {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_list_token_for_swap"></a>

## Function `list_token_for_swap`

Token owner lists their token for swapping


<pre><code><b>public</b> entry <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_list_token_for_swap">list_token_for_swap</a>&lt;CoinType&gt;(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _creators_address: <b>address</b>, _collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, _name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, _property_version: u64, _token_amount: u64, _min_coin_per_token: u64, _locked_until_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_list_token_for_swap">list_token_for_swap</a>&lt;CoinType&gt;(
    _token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _creators_address: <b>address</b>,
    _collection: String,
    _name: String,
    _property_version: u64,
    _token_amount: u64,
    _min_coin_per_token: u64,
    _locked_until_secs: u64
) {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_initialize_token_listing"></a>

## Function `initialize_token_listing`

Initalize the token listing for a token owner


<pre><code><b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_initialize_token_listing">initialize_token_listing</a>&lt;CoinType&gt;(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_initialize_token_listing">initialize_token_listing</a>&lt;CoinType&gt;(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_initialize_token_store_escrow"></a>

## Function `initialize_token_store_escrow`

Intialize the token escrow


<pre><code><b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_initialize_token_store_escrow">initialize_token_store_escrow</a>(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_initialize_token_store_escrow">initialize_token_store_escrow</a>(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_deposit_token_to_escrow"></a>

## Function `deposit_token_to_escrow`

Put the token into escrow that cannot be transferred or withdrawed by the owner.


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_deposit_token_to_escrow">deposit_token_to_escrow</a>(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, _tokens: <a href="token.md#0x3_token_Token">token::Token</a>, _locked_until_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_deposit_token_to_escrow">deposit_token_to_escrow</a>(
    _token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _token_id: TokenId,
    _tokens: Token,
    _locked_until_secs: u64
) {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_withdraw_token_from_escrow_internal"></a>

## Function `withdraw_token_from_escrow_internal`

Private function for withdraw tokens from an escrow stored in token owner address


<pre><code><b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_withdraw_token_from_escrow_internal">withdraw_token_from_escrow_internal</a>(_token_owner_addr: <b>address</b>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, _amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_withdraw_token_from_escrow_internal">withdraw_token_from_escrow_internal</a>(
    _token_owner_addr: <b>address</b>,
    _token_id: TokenId,
    _amount: u64
): Token {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_withdraw_token_from_escrow"></a>

## Function `withdraw_token_from_escrow`

Withdraw tokens from the token escrow. It needs a signer to authorize


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_withdraw_token_from_escrow">withdraw_token_from_escrow</a>(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, _amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_withdraw_token_from_escrow">withdraw_token_from_escrow</a>(
    _token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _token_id: TokenId,
    _amount: u64
): Token {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="0x3_token_coin_swap_cancel_token_listing"></a>

## Function `cancel_token_listing`

Cancel token listing for a fixed amount


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_cancel_token_listing">cancel_token_listing</a>&lt;CoinType&gt;(_token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, _token_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_coin_swap.md#0x3_token_coin_swap_cancel_token_listing">cancel_token_listing</a>&lt;CoinType&gt;(
    _token_owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _token_id: TokenId,
    _token_amount: u64,
) {
    <b>abort</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token_coin_swap.md#0x3_token_coin_swap_EDEPRECATED_MODULE">EDEPRECATED_MODULE</a>)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
