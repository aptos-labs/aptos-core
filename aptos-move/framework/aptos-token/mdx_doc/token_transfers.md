
<a id="0x3_token_transfers"></a>

# Module `0x3::token_transfers`

This module provides the foundation for transferring of Tokens


-  [Resource `PendingClaims`](#0x3_token_transfers_PendingClaims)
-  [Struct `TokenOfferId`](#0x3_token_transfers_TokenOfferId)
-  [Struct `TokenOffer`](#0x3_token_transfers_TokenOffer)
-  [Struct `TokenOfferEvent`](#0x3_token_transfers_TokenOfferEvent)
-  [Struct `TokenCancelOfferEvent`](#0x3_token_transfers_TokenCancelOfferEvent)
-  [Struct `TokenCancelOffer`](#0x3_token_transfers_TokenCancelOffer)
-  [Struct `TokenClaimEvent`](#0x3_token_transfers_TokenClaimEvent)
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


<pre><code><b>use</b> <a href="../../aptos-framework/doc/account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="token.md#0x3_token">0x3::token</a>;<br /></code></pre>



<a id="0x3_token_transfers_PendingClaims"></a>

## Resource `PendingClaims`



<pre><code><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pending_claims: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferId">token_transfers::TokenOfferId</a>, <a href="token.md#0x3_token_Token">token::Token</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>offer_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">token_transfers::TokenOfferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancel_offer_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">token_transfers::TokenCancelOfferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">token_transfers::TokenClaimEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_transfers_TokenOfferId"></a>

## Struct `TokenOfferId`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOffer">TokenOffer</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x3_token_transfers_TokenOfferEvent"></a>

## Struct `TokenOfferEvent`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x3_token_transfers_TokenCancelOfferEvent"></a>

## Struct `TokenCancelOfferEvent`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenCancelOffer">TokenCancelOffer</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x3_token_transfers_TokenClaimEvent"></a>

## Struct `TokenClaimEvent`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token_transfers.md#0x3_token_transfers_TokenClaim">TokenClaim</a> <b>has</b> drop, store<br /></code></pre>



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

Token offer doesn&apos;t exist


<pre><code><b>const</b> <a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x3_token_transfers_initialize_token_transfers"></a>

## Function `initialize_token_transfers`



<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <b>move_to</b>(<br />        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,<br />        <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> &#123;<br />            pending_claims: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a>, Token&gt;(),<br />            offer_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />            cancel_offer_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />            claim_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />        &#125;<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfers_create_token_offer_id"></a>

## Function `create_token_offer_id`



<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(to_addr: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">token_transfers::TokenOfferId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(to_addr: <b>address</b>, token_id: TokenId): <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a> &#123;<br />    <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">TokenOfferId</a> &#123;<br />        to_addr,<br />        token_id<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfers_offer_script"></a>

## Function `offer_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(<br />    sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    receiver: <b>address</b>,<br />    creator: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    property_version: u64,<br />    amount: u64,<br />) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> &#123;<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);<br />    <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(&amp;sender, receiver, token_id, amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfers_offer"></a>

## Function `offer`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(<br />    sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    receiver: <b>address</b>,<br />    token_id: TokenId,<br />    amount: u64,<br />) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> &#123;<br />    <b>let</b> sender_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr)) &#123;<br />        <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(sender)<br />    &#125;;<br /><br />    <b>let</b> pending_claims &#61;<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;<br />    <b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);<br />    <b>let</b> <a href="token.md#0x3_token">token</a> &#61; <a href="token.md#0x3_token_withdraw_token">token::withdraw_token</a>(sender, token_id, amount);<br />    <b>if</b> (!<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(pending_claims, token_offer_id)) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(pending_claims, token_offer_id, <a href="token.md#0x3_token">token</a>);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> dst_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(pending_claims, token_offer_id);<br />        <a href="token.md#0x3_token_merge">token::merge</a>(dst_token, <a href="token.md#0x3_token">token</a>);<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_transfers.md#0x3_token_transfers_TokenOffer">TokenOffer</a> &#123;<br />                to_address: receiver,<br />                token_id,<br />                amount,<br />            &#125;<br />        )<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a>&gt;(<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).offer_events,<br />        <a href="token_transfers.md#0x3_token_transfers_TokenOfferEvent">TokenOfferEvent</a> &#123;<br />            to_address: receiver,<br />            token_id,<br />            amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfers_claim_script"></a>

## Function `claim_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(receiver: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(<br />    receiver: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    sender: <b>address</b>,<br />    creator: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    property_version: u64,<br />) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> &#123;<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);<br />    <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(&amp;receiver, sender, token_id);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfers_claim"></a>

## Function `claim`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(<br />    receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    sender: <b>address</b>,<br />    token_id: TokenId,<br />) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender), <a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>);<br />    <b>let</b> pending_claims &#61;<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).pending_claims;<br />    <b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver), token_id);<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(pending_claims, token_offer_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>));<br />    <b>let</b> tokens &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(pending_claims, token_offer_id);<br />    <b>let</b> amount &#61; <a href="token.md#0x3_token_get_token_amount">token::get_token_amount</a>(&amp;tokens);<br />    <a href="token.md#0x3_token_deposit_token">token::deposit_token</a>(receiver, tokens);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_transfers.md#0x3_token_transfers_TokenClaim">TokenClaim</a> &#123;<br />                to_address: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver),<br />                token_id,<br />                amount,<br />            &#125;<br />        )<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a>&gt;(<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).claim_events,<br />        <a href="token_transfers.md#0x3_token_transfers_TokenClaimEvent">TokenClaimEvent</a> &#123;<br />            to_address: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver),<br />            token_id,<br />            amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfers_cancel_offer_script"></a>

## Function `cancel_offer_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(<br />    sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    receiver: <b>address</b>,<br />    creator: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    property_version: u64,<br />) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> &#123;<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);<br />    <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(&amp;sender, receiver, token_id);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfers_cancel_offer"></a>

## Function `cancel_offer`



<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(<br />    sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    receiver: <b>address</b>,<br />    token_id: TokenId,<br />) <b>acquires</b> <a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a> &#123;<br />    <b>let</b> sender_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);<br />    <b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr), <a href="token_transfers.md#0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST">ETOKEN_OFFER_NOT_EXIST</a>);<br />    <b>let</b> pending_claims &#61;<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;<br />    <b>let</b> <a href="token.md#0x3_token">token</a> &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(pending_claims, token_offer_id);<br />    <b>let</b> amount &#61; <a href="token.md#0x3_token_get_token_amount">token::get_token_amount</a>(&amp;<a href="token.md#0x3_token">token</a>);<br />    <a href="token.md#0x3_token_deposit_token">token::deposit_token</a>(sender, <a href="token.md#0x3_token">token</a>);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token_transfers.md#0x3_token_transfers_TokenCancelOffer">TokenCancelOffer</a> &#123;<br />                to_address: receiver,<br />                token_id,<br />                amount,<br />            &#125;,<br />        )<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a>&gt;(<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).cancel_offer_events,<br />        <a href="token_transfers.md#0x3_token_transfers_TokenCancelOfferEvent">TokenCancelOfferEvent</a> &#123;<br />            to_address: receiver,<br />            token_id,<br />            amount,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_initialize_token_transfers"></a>

### Function `initialize_token_transfers`


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_initialize_token_transfers">initialize_token_transfers</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="token_transfers.md#0x3_token_transfers_InitializeTokenTransfersAbortsIf">InitializeTokenTransfersAbortsIf</a>;<br /></code></pre>


Abort according to the code


<a id="0x3_token_transfers_InitializeTokenTransfersAbortsIf"></a>


<pre><code><b>schema</b> <a href="token_transfers.md#0x3_token_transfers_InitializeTokenTransfersAbortsIf">InitializeTokenTransfersAbortsIf</a> &#123;<br /><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(addr);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 3 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 3 &gt; MAX_U64;<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_token_offer_id"></a>

### Function `create_token_offer_id`


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(to_addr: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token_transfers.md#0x3_token_transfers_TokenOfferId">token_transfers::TokenOfferId</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_offer_script"></a>

### Function `offer_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer_script">offer_script</a>(sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);<br /></code></pre>



<a id="@Specification_1_offer"></a>

### Function `offer`


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_offer">offer</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> sender_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);<br /><b>include</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr) &#61;&#61;&gt; <a href="token_transfers.md#0x3_token_transfers_InitializeTokenTransfersAbortsIf">InitializeTokenTransfersAbortsIf</a>&#123;<a href="../../aptos-framework/doc/account.md#0x1_account">account</a> : sender&#125;;<br /><b>let</b> pending_claims &#61; <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;<br /><b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);<br /><b>let</b> tokens &#61; <b>global</b>&lt;TokenStore&gt;(sender_addr).tokens;<br /><b>aborts_if</b> amount &lt;&#61; 0;<br /><b>aborts_if</b> <a href="token.md#0x3_token_spec_balance_of">token::spec_balance_of</a>(sender_addr, token_id) &lt; amount;<br /><b>aborts_if</b> !<b>exists</b>&lt;TokenStore&gt;(sender_addr);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(tokens, token_id);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);<br /><b>let</b> a &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);<br /><b>let</b> dst_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);<br /><b>aborts_if</b> dst_token.amount &#43; <a href="token_transfers.md#0x3_token_transfers_spce_get">spce_get</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), token_id, amount) &gt; MAX_U64;<br /></code></pre>


Get the amount from sender token


<a id="0x3_token_transfers_spce_get"></a>


<pre><code><b>fun</b> <a href="token_transfers.md#0x3_token_transfers_spce_get">spce_get</a>(<br />   account_addr: <b>address</b>,<br />   id: TokenId,<br />   amount: u64<br />): u64 &#123;<br />   <b>use</b> aptos_token::token::&#123;TokenStore&#125;;<br />   <b>use</b> aptos_std::table::&#123;<b>Self</b>&#125;;<br />   <b>let</b> tokens &#61; <b>global</b>&lt;TokenStore&gt;(account_addr).tokens;<br />   <b>let</b> balance &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(tokens, id).amount;<br />   <b>if</b> (balance &gt; amount) &#123;<br />       amount<br />   &#125; <b>else</b> &#123;<br />       <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(tokens, id).amount<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_claim_script"></a>

### Function `claim_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim_script">claim_script</a>(receiver: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender);<br /><b>let</b> pending_claims &#61; <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).pending_claims;<br /><b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver), token_id);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);<br /><b>let</b> tokens &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);<br /><b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>&#123;<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: receiver &#125;;<br /><b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver);<br /><b>let</b> <a href="token.md#0x3_token">token</a> &#61; tokens;<br /><b>let</b> token_store &#61; <b>global</b>&lt;TokenStore&gt;(account_addr);<br /><b>let</b> recipient_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>let</b> b &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;&#61; 0;<br /></code></pre>



<a id="@Specification_1_claim"></a>

### Function `claim`


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_claim">claim</a>(receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, sender: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender);<br /><b>let</b> pending_claims &#61; <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender).pending_claims;<br /><b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver), token_id);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);<br /><b>let</b> tokens &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);<br /><b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>&#123;<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: receiver &#125;;<br /><b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(receiver);<br /><b>let</b> <a href="token.md#0x3_token">token</a> &#61; tokens;<br /><b>let</b> token_store &#61; <b>global</b>&lt;TokenStore&gt;(account_addr);<br /><b>let</b> recipient_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>let</b> b &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;&#61; 0;<br /></code></pre>



<a id="@Specification_1_cancel_offer_script"></a>

### Function `cancel_offer_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer_script">cancel_offer_script</a>(sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">token::create_token_id_raw</a>(creator, collection, name, property_version);<br /><b>let</b> sender_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr);<br /><b>let</b> pending_claims &#61; <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;<br /><b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);<br /><b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>&#123;<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: sender &#125;;<br /><b>let</b> dst_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);<br /><b>let</b> account_addr &#61; sender_addr;<br /><b>let</b> <a href="token.md#0x3_token">token</a> &#61; dst_token;<br /><b>let</b> token_store &#61; <b>global</b>&lt;TokenStore&gt;(account_addr);<br /><b>let</b> recipient_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>let</b> b &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;&#61; 0;<br /></code></pre>



<a id="@Specification_1_cancel_offer"></a>

### Function `cancel_offer`


<pre><code><b>public</b> <b>fun</b> <a href="token_transfers.md#0x3_token_transfers_cancel_offer">cancel_offer</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> sender_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr);<br /><b>let</b> pending_claims &#61; <b>global</b>&lt;<a href="token_transfers.md#0x3_token_transfers_PendingClaims">PendingClaims</a>&gt;(sender_addr).pending_claims;<br /><b>let</b> token_offer_id &#61; <a href="token_transfers.md#0x3_token_transfers_create_token_offer_id">create_token_offer_id</a>(receiver, token_id);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(pending_claims, token_offer_id);<br /><b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">token::InitializeTokenStore</a>&#123;<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: sender &#125;;<br /><b>let</b> dst_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(pending_claims, token_offer_id);<br /><b>let</b> account_addr &#61; sender_addr;<br /><b>let</b> <a href="token.md#0x3_token">token</a> &#61; dst_token;<br /><b>let</b> token_store &#61; <b>global</b>&lt;TokenStore&gt;(account_addr);<br /><b>let</b> recipient_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>let</b> b &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br /><b>aborts_if</b> <a href="token.md#0x3_token">token</a>.amount &lt;&#61; 0;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
