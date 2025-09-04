
<a id="0x3_token_transfers"></a>

# Module `0x3::token_transfers`

This module provides the foundation for transferring of Tokens


-  [Resource `PendingClaims`](#0x3_token_transfers_PendingClaims)
-  [Struct `TokenOfferId`](#0x3_token_transfers_TokenOfferId)
-  [Struct `TokenOffer`](#0x3_token_transfers_TokenOffer)
-  [Struct `Offer`](#0x3_token_transfers_Offer)
-  [Struct `TokenCancelOfferEvent`](#0x3_token_transfers_TokenCancelOfferEvent)
-  [Struct `CancelOffer`](#0x3_token_transfers_CancelOffer)
-  [Struct `TokenClaimEvent`](#0x3_token_transfers_TokenClaimEvent)
-  [Struct `Claim`](#0x3_token_transfers_Claim)
-  [Struct `TokenOfferEvent`](#0x3_token_transfers_TokenOfferEvent)
-  [Struct `TokenCancelOffer`](#0x3_token_transfers_TokenCancelOffer)
-  [Struct `TokenClaim`](#0x3_token_transfers_TokenClaim)
-  [Constants](#@Constants_0)
-  [Function `initialize_token_transfers`](#0x3_token_transfers_initialize_token_transfers)
-  [Function `create_token_offer_id`](#0x3_token_transfers_create_token_offer_id)
-  [Function `offer_script`](#0x3_token_transfers_offer_script)
-  [Function `offer`](#0x3_token_transfers_offer)
-  [Function `claim_script`](#0x3_token_transfers_claim_script)
-  [Function `claim`](#0x3_token_transfers_claim)
-  [Function `cancel_offer_script`](#0x3_token_transfers_cancel_offer_script)
-  [Function `cancel_offer`](#0x3_token_transfers_cancel_offer)
-  [Specification](#@Specification_1)
    -  [Function `initialize_token_transfers`](#@Specification_1_initialize_token_transfers)
    -  [Function `create_token_offer_id`](#@Specification_1_create_token_offer_id)
    -  [Function `offer_script`](#@Specification_1_offer_script)
    -  [Function `offer`](#@Specification_1_offer)
    -  [Function `claim_script`](#@Specification_1_claim_script)
    -  [Function `claim`](#@Specification_1_claim)
    -  [Function `cancel_offer_script`](#@Specification_1_cancel_offer_script)
    -  [Function `cancel_offer`](#@Specification_1_cancel_offer)


<pre><code><b>use</b> <a href="../../velor-framework/doc/account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="token.md#0x3_token">0x3::token</a>;
</code></pre>



<a id="0x3_token_transfers_PendingClaims"></a>

## Resource `PendingClaims`



<pre><code><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pending_claims: <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferId">token_transfers::TokenOfferId</a>, <a href="token.md#0x3_token_Token">token::Token</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>offer_events: <a href="../../velor-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">token_transfers::TokenOfferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancel_offer_events: <a href="../../velor-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">token_transfers::TokenCancelOfferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_events: <a href="../../velor-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">token_transfers::TokenClaimEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_transfers_TokenOfferId"></a>

## Struct `TokenOfferId`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x3_token_transfers_TokenOffer"></a>

## Struct `TokenOffer`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOffer">TokenOffer</a> <b>has</b> drop, store
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

<a id="0x3_token_transfers_Offer"></a>

## Struct `Offer`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_Offer">Offer</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../velor-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x3_token_transfers_TokenCancelOfferEvent"></a>

## Struct `TokenCancelOfferEvent`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a> <b>has</b> drop, store
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

<a id="0x3_token_transfers_CancelOffer"></a>

## Struct `CancelOffer`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_CancelOffer">CancelOffer</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../velor-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x3_token_transfers_TokenClaimEvent"></a>

## Struct `TokenClaimEvent`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a> <b>has</b> drop, store
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

<a id="0x3_token_transfers_Claim"></a>

## Struct `Claim`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_Claim">Claim</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../velor-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
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

<a id="0x3_token_transfers_TokenOfferEvent"></a>

## Struct `TokenOfferEvent`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a> <b>has</b> drop, store
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

<a id="0x3_token_transfers_TokenCancelOffer"></a>

## Struct `TokenCancelOffer`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenCancelOffer">TokenCancelOffer</a> <b>has</b> drop, store
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

<a id="0x3_token_transfers_TokenClaim"></a>

## Struct `TokenClaim`



<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenClaim">TokenClaim</a> <b>has</b> drop, store
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

<a id="@Constants_0"></a>

## Constants


<a id="0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST"></a>

Token offer doesn't exist


<pre><code><b>const</b> <a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>: u64 = 1;
</code></pre>



<a id="0x3_token_transfers_initialize_token_transfers"></a>

## Function `initialize_token_transfers`



<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(
        <a href="../../velor-framework/doc/account.md#0x1_account">account</a>,
        <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
            pending_claims: <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a>, Token&gt;(),
            offer_events: <a href="../../velor-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a>&gt;(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>),
            cancel_offer_events: <a href="../../velor-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a>&gt;(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>),
            claim_events: <a href="../../velor-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a>&gt;(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>),
        }
    )
}
</code></pre>



</details>

<a id="0x3_token_transfers_create_token_offer_id"></a>

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

<a id="0x3_token_transfers_offer_script"></a>

## Function `offer_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(sender: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(
    sender: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
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

<a id="0x3_token_transfers_offer"></a>

## Function `offer`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    receiver: <b>address</b>,
    token_id: TokenId,
    amount: u64,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> sender_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>if</b> (!<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr)) {
        <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(sender)
    };

    <b>let</b> pending_claims =
        &<b>mut</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>[sender_addr].pending_claims;
    <b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);
    <b>let</b> <a href="token.md#0x3_token">token</a> = <a href="token.md#0x3_token_withdraw_token">token::withdraw_token</a>(sender, token_id, amount);
    <b>if</b> (!pending_claims.contains(token_offer_id)) {
        pending_claims.add(token_offer_id, <a href="token.md#0x3_token">token</a>);
    } <b>else</b> {
        <b>let</b> dst_token = pending_claims.borrow_mut(token_offer_id);
        <a href="token.md#0x3_token_merge">token::merge</a>(dst_token, <a href="token.md#0x3_token">token</a>);
    };

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../velor-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="token_transfers.md#0x3_token_transfers_Offer">Offer</a> {
                <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: sender_addr,
                to_address: receiver,
                token_id,
                amount,
            }
        )
    } <b>else</b> {
        <a href="../../velor-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a>&gt;(
            &<b>mut</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>[sender_addr].offer_events,
            <a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a> {
                to_address: receiver,
                token_id,
                amount,
            },
        );
    }
}
</code></pre>



</details>

<a id="0x3_token_transfers_claim_script"></a>

## Function `claim_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(receiver: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, creator: <b>address</b>, collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(
    receiver: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
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

<a id="0x3_token_transfers_claim"></a>

## Function `claim`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(receiver: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(
    receiver: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    sender: <b>address</b>,
    token_id: TokenId,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender), <a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>);
    <b>let</b> pending_claims =
        &<b>mut</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>[sender].pending_claims;
    <b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver), token_id);
    <b>assert</b>!(pending_claims.contains(token_offer_id), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>));
    <b>let</b> tokens = pending_claims.remove(token_offer_id);
    <b>let</b> amount = <a href="token.md#0x3_token_get_token_amount">token::get_token_amount</a>(&tokens);
    <a href="token.md#0x3_token_deposit_token">token::deposit_token</a>(receiver, tokens);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../velor-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="token_transfers.md#0x3_token_transfers_Claim">Claim</a> {
                <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: sender,
                to_address: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver),
                token_id,
                amount,
            }
        )
    } <b>else</b> {
        <a href="../../velor-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a>&gt;(
            &<b>mut</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>[sender].claim_events,
            <a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a> {
                to_address: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver),
                token_id,
                amount,
            },
        );
    };
}
</code></pre>



</details>

<a id="0x3_token_transfers_cancel_offer_script"></a>

## Function `cancel_offer_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(sender: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(
    sender: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
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

<a id="0x3_token_transfers_cancel_offer"></a>

## Function `cancel_offer`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    receiver: <b>address</b>,
    token_id: TokenId,
) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> {
    <b>let</b> sender_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);
    <b>assert</b>!(<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr), <a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>);
    <b>let</b> pending_claims =
        &<b>mut</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>[sender_addr].pending_claims;
    <b>let</b> <a href="token.md#0x3_token">token</a> = pending_claims.remove(token_offer_id);
    <b>let</b> amount = <a href="token.md#0x3_token_get_token_amount">token::get_token_amount</a>(&<a href="token.md#0x3_token">token</a>);
    <a href="token.md#0x3_token_deposit_token">token::deposit_token</a>(sender, <a href="token.md#0x3_token">token</a>);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../velor-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="token_transfers.md#0x3_token_transfers_CancelOffer">CancelOffer</a> {
                <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: sender_addr,
                to_address: receiver,
                token_id,
                amount,
            },
        )
    } <b>else</b> {
        <a href="../../velor-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a>&gt;(
            &<b>mut</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>[sender_addr].cancel_offer_events,
            <a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a> {
                to_address: receiver,
                token_id,
                amount,
            },
        );
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>


Get the amount from sender token


<a id="0x3_token_transfers_spce_get"></a>


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_spce_get">spce_get</a>(
   account_addr: <b>address</b>,
   id: TokenId,
   amount: u64
): u64 {
   <b>use</b> velor_token::token::{TokenStore};
   <b>use</b> velor_std::table::{<b>Self</b>};
   <b>let</b> tokens = <b>global</b>&lt;TokenStore&gt;(account_addr).tokens;
   <b>let</b> balance = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(tokens, id).amount;
   <b>if</b> (balance &gt; amount) {
       amount
   } <b>else</b> {
       <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(tokens, id).amount
   }
}
</code></pre>



<a id="@Specification_1_initialize_token_transfers"></a>

### Function `initialize_token_transfers`


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>include</b> <a href="token_transfers.md#0x3_token_transfers_InitializeTokenTransfersAbortsIf">InitializeTokenTransfersAbortsIf</a>;
</code></pre>


Abort according to the code


<a id="0x3_token_transfers_InitializeTokenTransfersAbortsIf"></a>


<pre><code><b>schema</b> <a href="token_transfers.md#0x3_token_transfers_InitializeTokenTransfersAbortsIf">InitializeTokenTransfersAbortsIf</a> {
    <a href="../../velor-framework/doc/account.md#0x1_account">account</a>: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>let</b> addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../velor-framework/doc/account.md#0x1_account">account</a>);
    <b>aborts_if</b> <b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(addr);
    <b>let</b> <a href="../../velor-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;Account&gt;(addr);
    <b>aborts_if</b> !<b>exists</b>&lt;Account&gt;(addr);
    <b>aborts_if</b> <a href="../../velor-framework/doc/account.md#0x1_account">account</a>.guid_creation_num + 3 &gt;= <a href="../../velor-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
    <b>aborts_if</b> <a href="../../velor-framework/doc/account.md#0x1_account">account</a>.guid_creation_num + 3 &gt; MAX_U64;
}
</code></pre>



<a id="@Specification_1_create_token_offer_id"></a>

### Function `create_token_offer_id`


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(to_addr: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">token_transfers::TokenOfferId</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_offer_script"></a>

### Function `offer_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(sender: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);
</code></pre>



<a id="@Specification_1_offer"></a>

### Function `offer`


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> sender_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
<b>include</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr) ==&gt; <a href="token_transfers.md#0x3_token_transfers_InitializeTokenTransfersAbortsIf">InitializeTokenTransfersAbortsIf</a>{<a href="../../velor-framework/doc/account.md#0x1_account">account</a> : sender};
<b>let</b> pending_claims = <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;
<b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);
<b>let</b> tokens = <b>global</b>&lt;TokenStore&gt;(sender_addr).tokens;
<b>aborts_if</b> amount &lt;= 0;
<b>aborts_if</b> <a href="token.md#0x3_token_spec_balance_of">token::spec_balance_of</a>(sender_addr, token_id) &lt; amount;
<b>aborts_if</b> !<b>exists</b>&lt;TokenStore&gt;(sender_addr);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(tokens, token_id);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);
<b>let</b> a = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);
<b>let</b> dst_token = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);
<b>aborts_if</b> dst_token.amount + <a href="token_transfers.md#0x3_token_transfers_spce_get">spce_get</a>(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), token_id, amount) &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_claim_script"></a>

### Function `claim_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(receiver: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, creator: <b>address</b>, collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender);
<b>let</b> pending_claims = <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).pending_claims;
<b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver), token_id);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);
<b>let</b> tokens = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);
<b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>{<a href="../../velor-framework/doc/account.md#0x1_account">account</a>: receiver };
<b>let</b> account_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver);
<b>let</b> <a href="token.md#0x3_token">token</a> = tokens;
<b>let</b> token_store = <b>global</b>&lt;TokenStore&gt;(account_addr);
<b>let</b> recipient_token = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>let</b> b = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;= 0;
</code></pre>



<a id="@Specification_1_claim"></a>

### Function `claim`


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(receiver: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender);
<b>let</b> pending_claims = <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).pending_claims;
<b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver), token_id);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);
<b>let</b> tokens = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);
<b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>{<a href="../../velor-framework/doc/account.md#0x1_account">account</a>: receiver };
<b>let</b> account_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver);
<b>let</b> <a href="token.md#0x3_token">token</a> = tokens;
<b>let</b> token_store = <b>global</b>&lt;TokenStore&gt;(account_addr);
<b>let</b> recipient_token = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>let</b> b = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;= 0;
</code></pre>



<a id="@Specification_1_cancel_offer_script"></a>

### Function `cancel_offer_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(sender: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);
<b>let</b> sender_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr);
<b>let</b> pending_claims = <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;
<b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);
<b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>{<a href="../../velor-framework/doc/account.md#0x1_account">account</a>: sender };
<b>let</b> dst_token = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);
<b>let</b> account_addr = sender_addr;
<b>let</b> <a href="token.md#0x3_token">token</a> = dst_token;
<b>let</b> token_store = <b>global</b>&lt;TokenStore&gt;(account_addr);
<b>let</b> recipient_token = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>let</b> b = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;= 0;
</code></pre>



<a id="@Specification_1_cancel_offer"></a>

### Function `cancel_offer`


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> sender_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr);
<b>let</b> pending_claims = <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;
<b>let</b> token_offer_id = <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);
<b>aborts_if</b> !<a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);
<b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>{<a href="../../velor-framework/doc/account.md#0x1_account">account</a>: sender };
<b>let</b> dst_token = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);
<b>let</b> account_addr = sender_addr;
<b>let</b> <a href="token.md#0x3_token">token</a> = dst_token;
<b>let</b> token_store = <b>global</b>&lt;TokenStore&gt;(account_addr);
<b>let</b> recipient_token = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>let</b> b = <a href="../../velor-framework/../velor-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);
<b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;= 0;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
