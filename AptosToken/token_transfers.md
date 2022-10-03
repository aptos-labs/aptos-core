
<a name="0x3_token_transfers"></a>

# Module `0x3::token_transfers`

This module provides the foundation for transferring of Tokens


-  [Resource `PendingClaims`](#0x3_token_transfers_PendingClaims)
-  [Struct `TokenOfferId`](#0x3_token_transfers_TokenOfferId)
-  [Struct `TokenOfferEvent`](#0x3_token_transfers_TokenOfferEvent)
-  [Struct `TokenCancelOfferEvent`](#0x3_token_transfers_TokenCancelOfferEvent)
-  [Struct `TokenClaimEvent`](#0x3_token_transfers_TokenClaimEvent)
-  [Constants](#@Constants_0)
-  [Function `initialize_token_transfers`](#0x3_token_transfers_initialize_token_transfers)
-  [Function `create_token_offer_id`](#0x3_token_transfers_create_token_offer_id)
-  [Function `offer_script`](#0x3_token_transfers_offer_script)
-  [Function `offer`](#0x3_token_transfers_offer)
-  [Function `claim_script`](#0x3_token_transfers_claim_script)
-  [Function `claim`](#0x3_token_transfers_claim)
-  [Function `cancel_offer_script`](#0x3_token_transfers_cancel_offer_script)
-  [Function `cancel_offer`](#0x3_token_transfers_cancel_offer)


<pre><code><b>use</b> <a href="">0x1::account</a>;
<b>use</b> <a href="">0x1::error</a>;
<b>use</b> <a href="">0x1::event</a>;
<b>use</b> <a href="">0x1::signer</a>;
<b>use</b> <a href="">0x1::string</a>;
<b>use</b> <a href="">0x1::table</a>;
<b>use</b> <a href="token.md#0x3_token">0x3::token</a>;
</code></pre>



<a name="0x3_token_transfers_PendingClaims"></a>

## Resource `PendingClaims`



<pre><code><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pending_claims: <a href="_Table">table::Table</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferId">token_transfers::TokenOfferId</a>, <a href="token.md#0x3_token_Token">token::Token</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>offer_events: <a href="_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">token_transfers::TokenOfferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancel_offer_events: <a href="_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">token_transfers::TokenCancelOfferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_events: <a href="_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">token_transfers::TokenClaimEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_token_transfers_TokenOfferId"></a>

## Struct `TokenOfferId`



<pre><code><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x3_token_transfers_TokenOfferEvent"></a>

## Struct `TokenOfferEvent`



<pre><code><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: <b>address</b></code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a name="0x3_token_transfers_TokenCancelOfferEvent"></a>

## Struct `TokenCancelOfferEvent`



<pre><code><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: <b>address</b></code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a name="0x3_token_transfers_TokenClaimEvent"></a>

## Struct `TokenClaimEvent`



<pre><code><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: <b>address</b></code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST"></a>

Token offer doesn't exist


<pre><code><b>const</b> <a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>: u64 = 1;
</code></pre>



<a name="0x3_token_transfers_initialize_token_transfers"></a>

## Function `initialize_token_transfers`



<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="">account</a>: &<a href="">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="">account</a>: &<a href="">signer</a>) {
    <b>move_to</b>(
        <a href="">account</a>,
        <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
            pending_claims: <a href="_new">table::new</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a>, Token&gt;(),
            offer_events: <a href="_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a>&gt;(<a href="">account</a>),
            cancel_offer_events: <a href="_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a>&gt;(<a href="">account</a>),
            claim_events: <a href="_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a>&gt;(<a href="">account</a>),
        }
    )
}
</code></pre>



</details>

<a name="0x3_token_transfers_create_token_offer_id"></a>

## Function `create_token_offer_id`



<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(to_addr: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">token_transfers::TokenOfferId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(to_addr: <b>address</b>, token_id: TokenId): <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a> {
    <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a> {
        to_addr,
        token_id
    }
}
</code></pre>



</details>

<a name="0x3_token_transfers_offer_script"></a>

## Function `offer_script`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(sender: <a href="">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="_String">string::String</a>, name: <a href="_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(
    sender: <a href="">signer</a>,
    receiver: <b>address</b>,
    creator: <b>address</b>,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);
    <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(&sender, receiver, token_id, amount);
}
</code></pre>



</details>

<a name="0x3_token_transfers_offer"></a>

## Function `offer`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(sender: &<a href="">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(
    sender: &<a href="">signer</a>,
    receiver: <b>address</b>,
    token_id: TokenId,
    amount: u64,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> sender_addr = <a href="_address_of">signer::address_of</a>(sender);
    <b>if</b> (!<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr)) {
        <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(sender)
    };

    <b>let</b> pending_claims =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;
    <b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);
    <b>let</b> <a href="token.md#0x3_token">token</a> = <a href="token.md#0x3_token_withdraw_token">token::withdraw_token</a>(sender, token_id, amount);
    <b>if</b> (!<a href="_contains">table::contains</a>(pending_claims, token_offer_id)) {
        <a href="_add">table::add</a>(pending_claims, token_offer_id, <a href="token.md#0x3_token">token</a>);
    } <b>else</b> {
        <b>let</b> dst_token = <a href="_borrow_mut">table::borrow_mut</a>(pending_claims, token_offer_id);
        <a href="token.md#0x3_token_merge">token::merge</a>(dst_token, <a href="token.md#0x3_token">token</a>);
    };

    <a href="_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a>&gt;(
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).offer_events,
        <a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a> {
            to_address: receiver,
            token_id,
            amount,
        },
    );
}
</code></pre>



</details>

<a name="0x3_token_transfers_claim_script"></a>

## Function `claim_script`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(receiver: <a href="">signer</a>, sender: <b>address</b>, creator: <b>address</b>, collection: <a href="_String">string::String</a>, name: <a href="_String">string::String</a>, property_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(
    receiver: <a href="">signer</a>,
    sender: <b>address</b>,
    creator: <b>address</b>,
    collection: String,
    name: String,
    property_version: u64,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);
    <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(&receiver, sender, token_id);
}
</code></pre>



</details>

<a name="0x3_token_transfers_claim"></a>

## Function `claim`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(receiver: &<a href="">signer</a>, sender: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(
    receiver: &<a href="">signer</a>,
    sender: <b>address</b>,
    token_id: TokenId,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> pending_claims =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).pending_claims;
    <b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(<a href="_address_of">signer::address_of</a>(receiver), token_id);
    <b>assert</b>!(<a href="_contains">table::contains</a>(pending_claims, token_offer_id), <a href="_not_found">error::not_found</a>(<a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>));
    <b>let</b> tokens = <a href="_remove">table::remove</a>(pending_claims, token_offer_id);
    <b>let</b> amount = <a href="token.md#0x3_token_get_token_amount">token::get_token_amount</a>(&tokens);
    <a href="token.md#0x3_token_deposit_token">token::deposit_token</a>(receiver, tokens);

    <a href="_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a>&gt;(
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).claim_events,
        <a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a> {
            to_address: <a href="_address_of">signer::address_of</a>(receiver),
            token_id,
            amount,
        },
    );
}
</code></pre>



</details>

<a name="0x3_token_transfers_cancel_offer_script"></a>

## Function `cancel_offer_script`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(sender: <a href="">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="_String">string::String</a>, name: <a href="_String">string::String</a>, property_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(
    sender: <a href="">signer</a>,
    receiver: <b>address</b>,
    creator: <b>address</b>,
    collection: String,
    name: String,
    property_version: u64,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);
    <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(&sender, receiver, token_id);
}
</code></pre>



</details>

<a name="0x3_token_transfers_cancel_offer"></a>

## Function `cancel_offer`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(sender: &<a href="">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(
    sender: &<a href="">signer</a>,
    receiver: <b>address</b>,
    token_id: TokenId,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> sender_addr = <a href="_address_of">signer::address_of</a>(sender);
    <b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);
    <b>let</b> pending_claims =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;
    <b>let</b> <a href="token.md#0x3_token">token</a> = <a href="_remove">table::remove</a>(pending_claims, token_offer_id);
    <b>let</b> amount = <a href="token.md#0x3_token_get_token_amount">token::get_token_amount</a>(&<a href="token.md#0x3_token">token</a>);
    <a href="token.md#0x3_token_deposit_token">token::deposit_token</a>(sender, <a href="token.md#0x3_token">token</a>);

    <a href="_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a>&gt;(
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).cancel_offer_events,
        <a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a> {
            to_address: receiver,
            token_id,
            amount,
        },
    );
}
</code></pre>



</details>
