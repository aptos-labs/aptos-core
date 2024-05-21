
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


<pre><code>use 0x1::account;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::signer;
use 0x1::string;
use 0x1::table;
use 0x3::token;
</code></pre>



<a id="0x3_token_transfers_PendingClaims"></a>

## Resource `PendingClaims`



<pre><code>struct PendingClaims has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pending_claims: table::Table&lt;token_transfers::TokenOfferId, token::Token&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>offer_events: event::EventHandle&lt;token_transfers::TokenOfferEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cancel_offer_events: event::EventHandle&lt;token_transfers::TokenCancelOfferEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>claim_events: event::EventHandle&lt;token_transfers::TokenClaimEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_transfers_TokenOfferId"></a>

## Struct `TokenOfferId`



<pre><code>&#35;[event]
struct TokenOfferId has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>token_id: token::TokenId</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_transfers_TokenOffer"></a>

## Struct `TokenOffer`



<pre><code>&#35;[event]
struct TokenOffer has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: address</code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a id="0x3_token_transfers_TokenOfferEvent"></a>

## Struct `TokenOfferEvent`



<pre><code>&#35;[event]
struct TokenOfferEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: address</code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a id="0x3_token_transfers_TokenCancelOfferEvent"></a>

## Struct `TokenCancelOfferEvent`



<pre><code>&#35;[event]
struct TokenCancelOfferEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: address</code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a id="0x3_token_transfers_TokenCancelOffer"></a>

## Struct `TokenCancelOffer`



<pre><code>&#35;[event]
struct TokenCancelOffer has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: address</code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a id="0x3_token_transfers_TokenClaimEvent"></a>

## Struct `TokenClaimEvent`



<pre><code>&#35;[event]
struct TokenClaimEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: address</code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a id="0x3_token_transfers_TokenClaim"></a>

## Struct `TokenClaim`



<pre><code>&#35;[event]
struct TokenClaim has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to_address: address</code>
</dt>
<dd>

</dd>
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
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_token_transfers_ETOKEN_OFFER_NOT_EXIST"></a>

Token offer doesn't exist


<pre><code>const ETOKEN_OFFER_NOT_EXIST: u64 &#61; 1;
</code></pre>



<a id="0x3_token_transfers_initialize_token_transfers"></a>

## Function `initialize_token_transfers`



<pre><code>fun initialize_token_transfers(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_token_transfers(account: &amp;signer) &#123;
    move_to(
        account,
        PendingClaims &#123;
            pending_claims: table::new&lt;TokenOfferId, Token&gt;(),
            offer_events: account::new_event_handle&lt;TokenOfferEvent&gt;(account),
            cancel_offer_events: account::new_event_handle&lt;TokenCancelOfferEvent&gt;(account),
            claim_events: account::new_event_handle&lt;TokenClaimEvent&gt;(account),
        &#125;
    )
&#125;
</code></pre>



</details>

<a id="0x3_token_transfers_create_token_offer_id"></a>

## Function `create_token_offer_id`



<pre><code>fun create_token_offer_id(to_addr: address, token_id: token::TokenId): token_transfers::TokenOfferId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_token_offer_id(to_addr: address, token_id: TokenId): TokenOfferId &#123;
    TokenOfferId &#123;
        to_addr,
        token_id
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_transfers_offer_script"></a>

## Function `offer_script`



<pre><code>public entry fun offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun offer_script(
    sender: signer,
    receiver: address,
    creator: address,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64,
) acquires PendingClaims &#123;
    let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);
    offer(&amp;sender, receiver, token_id, amount);
&#125;
</code></pre>



</details>

<a id="0x3_token_transfers_offer"></a>

## Function `offer`



<pre><code>public fun offer(sender: &amp;signer, receiver: address, token_id: token::TokenId, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun offer(
    sender: &amp;signer,
    receiver: address,
    token_id: TokenId,
    amount: u64,
) acquires PendingClaims &#123;
    let sender_addr &#61; signer::address_of(sender);
    if (!exists&lt;PendingClaims&gt;(sender_addr)) &#123;
        initialize_token_transfers(sender)
    &#125;;

    let pending_claims &#61;
        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).pending_claims;
    let token_offer_id &#61; create_token_offer_id(receiver, token_id);
    let token &#61; token::withdraw_token(sender, token_id, amount);
    if (!table::contains(pending_claims, token_offer_id)) &#123;
        table::add(pending_claims, token_offer_id, token);
    &#125; else &#123;
        let dst_token &#61; table::borrow_mut(pending_claims, token_offer_id);
        token::merge(dst_token, token);
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            TokenOffer &#123;
                to_address: receiver,
                token_id,
                amount,
            &#125;
        )
    &#125;;
    event::emit_event&lt;TokenOfferEvent&gt;(
        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).offer_events,
        TokenOfferEvent &#123;
            to_address: receiver,
            token_id,
            amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_transfers_claim_script"></a>

## Function `claim_script`



<pre><code>public entry fun claim_script(receiver: signer, sender: address, creator: address, collection: string::String, name: string::String, property_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun claim_script(
    receiver: signer,
    sender: address,
    creator: address,
    collection: String,
    name: String,
    property_version: u64,
) acquires PendingClaims &#123;
    let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);
    claim(&amp;receiver, sender, token_id);
&#125;
</code></pre>



</details>

<a id="0x3_token_transfers_claim"></a>

## Function `claim`



<pre><code>public fun claim(receiver: &amp;signer, sender: address, token_id: token::TokenId)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun claim(
    receiver: &amp;signer,
    sender: address,
    token_id: TokenId,
) acquires PendingClaims &#123;
    assert!(exists&lt;PendingClaims&gt;(sender), ETOKEN_OFFER_NOT_EXIST);
    let pending_claims &#61;
        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender).pending_claims;
    let token_offer_id &#61; create_token_offer_id(signer::address_of(receiver), token_id);
    assert!(table::contains(pending_claims, token_offer_id), error::not_found(ETOKEN_OFFER_NOT_EXIST));
    let tokens &#61; table::remove(pending_claims, token_offer_id);
    let amount &#61; token::get_token_amount(&amp;tokens);
    token::deposit_token(receiver, tokens);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            TokenClaim &#123;
                to_address: signer::address_of(receiver),
                token_id,
                amount,
            &#125;
        )
    &#125;;
    event::emit_event&lt;TokenClaimEvent&gt;(
        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender).claim_events,
        TokenClaimEvent &#123;
            to_address: signer::address_of(receiver),
            token_id,
            amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_transfers_cancel_offer_script"></a>

## Function `cancel_offer_script`



<pre><code>public entry fun cancel_offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun cancel_offer_script(
    sender: signer,
    receiver: address,
    creator: address,
    collection: String,
    name: String,
    property_version: u64,
) acquires PendingClaims &#123;
    let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);
    cancel_offer(&amp;sender, receiver, token_id);
&#125;
</code></pre>



</details>

<a id="0x3_token_transfers_cancel_offer"></a>

## Function `cancel_offer`



<pre><code>public fun cancel_offer(sender: &amp;signer, receiver: address, token_id: token::TokenId)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun cancel_offer(
    sender: &amp;signer,
    receiver: address,
    token_id: TokenId,
) acquires PendingClaims &#123;
    let sender_addr &#61; signer::address_of(sender);
    let token_offer_id &#61; create_token_offer_id(receiver, token_id);
    assert!(exists&lt;PendingClaims&gt;(sender_addr), ETOKEN_OFFER_NOT_EXIST);
    let pending_claims &#61;
        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).pending_claims;
    let token &#61; table::remove(pending_claims, token_offer_id);
    let amount &#61; token::get_token_amount(&amp;token);
    token::deposit_token(sender, token);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            TokenCancelOffer &#123;
                to_address: receiver,
                token_id,
                amount,
            &#125;,
        )
    &#125;;
    event::emit_event&lt;TokenCancelOfferEvent&gt;(
        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).cancel_offer_events,
        TokenCancelOfferEvent &#123;
            to_address: receiver,
            token_id,
            amount,
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize_token_transfers"></a>

### Function `initialize_token_transfers`


<pre><code>fun initialize_token_transfers(account: &amp;signer)
</code></pre>




<pre><code>include InitializeTokenTransfersAbortsIf;
</code></pre>


Abort according to the code


<a id="0x3_token_transfers_InitializeTokenTransfersAbortsIf"></a>


<pre><code>schema InitializeTokenTransfersAbortsIf &#123;
    account: &amp;signer;
    let addr &#61; signer::address_of(account);
    aborts_if exists&lt;PendingClaims&gt;(addr);
    let account &#61; global&lt;Account&gt;(addr);
    aborts_if !exists&lt;Account&gt;(addr);
    aborts_if account.guid_creation_num &#43; 3 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    aborts_if account.guid_creation_num &#43; 3 &gt; MAX_U64;
&#125;
</code></pre>



<a id="@Specification_1_create_token_offer_id"></a>

### Function `create_token_offer_id`


<pre><code>fun create_token_offer_id(to_addr: address, token_id: token::TokenId): token_transfers::TokenOfferId
</code></pre>




<pre><code>aborts_if false;
</code></pre>



<a id="@Specification_1_offer_script"></a>

### Function `offer_script`


<pre><code>public entry fun offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);
</code></pre>



<a id="@Specification_1_offer"></a>

### Function `offer`


<pre><code>public fun offer(sender: &amp;signer, receiver: address, token_id: token::TokenId, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let sender_addr &#61; signer::address_of(sender);
include !exists&lt;PendingClaims&gt;(sender_addr) &#61;&#61;&gt; InitializeTokenTransfersAbortsIf&#123;account : sender&#125;;
let pending_claims &#61; global&lt;PendingClaims&gt;(sender_addr).pending_claims;
let token_offer_id &#61; create_token_offer_id(receiver, token_id);
let tokens &#61; global&lt;TokenStore&gt;(sender_addr).tokens;
aborts_if amount &lt;&#61; 0;
aborts_if token::spec_balance_of(sender_addr, token_id) &lt; amount;
aborts_if !exists&lt;TokenStore&gt;(sender_addr);
aborts_if !table::spec_contains(tokens, token_id);
aborts_if !table::spec_contains(pending_claims, token_offer_id);
let a &#61; table::spec_contains(pending_claims, token_offer_id);
let dst_token &#61; table::spec_get(pending_claims, token_offer_id);
aborts_if dst_token.amount &#43; spce_get(signer::address_of(sender), token_id, amount) &gt; MAX_U64;
</code></pre>


Get the amount from sender token


<a id="0x3_token_transfers_spce_get"></a>


<pre><code>fun spce_get(
   account_addr: address,
   id: TokenId,
   amount: u64
): u64 &#123;
   use aptos_token::token::&#123;TokenStore&#125;;
   use aptos_std::table::&#123;Self&#125;;
   let tokens &#61; global&lt;TokenStore&gt;(account_addr).tokens;
   let balance &#61; table::spec_get(tokens, id).amount;
   if (balance &gt; amount) &#123;
       amount
   &#125; else &#123;
       table::spec_get(tokens, id).amount
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_claim_script"></a>

### Function `claim_script`


<pre><code>public entry fun claim_script(receiver: signer, sender: address, creator: address, collection: string::String, name: string::String, property_version: u64)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);
aborts_if !exists&lt;PendingClaims&gt;(sender);
let pending_claims &#61; global&lt;PendingClaims&gt;(sender).pending_claims;
let token_offer_id &#61; create_token_offer_id(signer::address_of(receiver), token_id);
aborts_if !table::spec_contains(pending_claims, token_offer_id);
let tokens &#61; table::spec_get(pending_claims, token_offer_id);
include token::InitializeTokenStore&#123;account: receiver &#125;;
let account_addr &#61; signer::address_of(receiver);
let token &#61; tokens;
let token_store &#61; global&lt;TokenStore&gt;(account_addr);
let recipient_token &#61; table::spec_get(token_store.tokens, token.id);
let b &#61; table::spec_contains(token_store.tokens, token.id);
aborts_if token.amount &lt;&#61; 0;
</code></pre>



<a id="@Specification_1_claim"></a>

### Function `claim`


<pre><code>public fun claim(receiver: &amp;signer, sender: address, token_id: token::TokenId)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
aborts_if !exists&lt;PendingClaims&gt;(sender);
let pending_claims &#61; global&lt;PendingClaims&gt;(sender).pending_claims;
let token_offer_id &#61; create_token_offer_id(signer::address_of(receiver), token_id);
aborts_if !table::spec_contains(pending_claims, token_offer_id);
let tokens &#61; table::spec_get(pending_claims, token_offer_id);
include token::InitializeTokenStore&#123;account: receiver &#125;;
let account_addr &#61; signer::address_of(receiver);
let token &#61; tokens;
let token_store &#61; global&lt;TokenStore&gt;(account_addr);
let recipient_token &#61; table::spec_get(token_store.tokens, token.id);
let b &#61; table::spec_contains(token_store.tokens, token.id);
aborts_if token.amount &lt;&#61; 0;
</code></pre>



<a id="@Specification_1_cancel_offer_script"></a>

### Function `cancel_offer_script`


<pre><code>public entry fun cancel_offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);
let sender_addr &#61; signer::address_of(sender);
aborts_if !exists&lt;PendingClaims&gt;(sender_addr);
let pending_claims &#61; global&lt;PendingClaims&gt;(sender_addr).pending_claims;
let token_offer_id &#61; create_token_offer_id(receiver, token_id);
aborts_if !table::spec_contains(pending_claims, token_offer_id);
include token::InitializeTokenStore&#123;account: sender &#125;;
let dst_token &#61; table::spec_get(pending_claims, token_offer_id);
let account_addr &#61; sender_addr;
let token &#61; dst_token;
let token_store &#61; global&lt;TokenStore&gt;(account_addr);
let recipient_token &#61; table::spec_get(token_store.tokens, token.id);
let b &#61; table::spec_contains(token_store.tokens, token.id);
aborts_if token.amount &lt;&#61; 0;
</code></pre>



<a id="@Specification_1_cancel_offer"></a>

### Function `cancel_offer`


<pre><code>public fun cancel_offer(sender: &amp;signer, receiver: address, token_id: token::TokenId)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let sender_addr &#61; signer::address_of(sender);
aborts_if !exists&lt;PendingClaims&gt;(sender_addr);
let pending_claims &#61; global&lt;PendingClaims&gt;(sender_addr).pending_claims;
let token_offer_id &#61; create_token_offer_id(receiver, token_id);
aborts_if !table::spec_contains(pending_claims, token_offer_id);
include token::InitializeTokenStore&#123;account: sender &#125;;
let dst_token &#61; table::spec_get(pending_claims, token_offer_id);
let account_addr &#61; sender_addr;
let token &#61; dst_token;
let token_store &#61; global&lt;TokenStore&gt;(account_addr);
let recipient_token &#61; table::spec_get(token_store.tokens, token.id);
let b &#61; table::spec_contains(token_store.tokens, token.id);
aborts_if token.amount &lt;&#61; 0;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
