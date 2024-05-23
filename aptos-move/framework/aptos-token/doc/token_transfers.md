
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


<pre><code>use 0x1::account;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x1::table;<br/>use 0x3::token;<br/></code></pre>



<a id="0x3_token_transfers_PendingClaims"></a>

## Resource `PendingClaims`



<pre><code>struct PendingClaims has key<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TokenOfferId has copy, drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TokenOffer has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TokenOfferEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TokenCancelOfferEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TokenCancelOffer has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TokenClaimEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TokenClaim has drop, store<br/></code></pre>



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

Token offer doesn&apos;t exist


<pre><code>const ETOKEN_OFFER_NOT_EXIST: u64 &#61; 1;<br/></code></pre>



<a id="0x3_token_transfers_initialize_token_transfers"></a>

## Function `initialize_token_transfers`



<pre><code>fun initialize_token_transfers(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_token_transfers(account: &amp;signer) &#123;<br/>    move_to(<br/>        account,<br/>        PendingClaims &#123;<br/>            pending_claims: table::new&lt;TokenOfferId, Token&gt;(),<br/>            offer_events: account::new_event_handle&lt;TokenOfferEvent&gt;(account),<br/>            cancel_offer_events: account::new_event_handle&lt;TokenCancelOfferEvent&gt;(account),<br/>            claim_events: account::new_event_handle&lt;TokenClaimEvent&gt;(account),<br/>        &#125;<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfers_create_token_offer_id"></a>

## Function `create_token_offer_id`



<pre><code>fun create_token_offer_id(to_addr: address, token_id: token::TokenId): token_transfers::TokenOfferId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_token_offer_id(to_addr: address, token_id: TokenId): TokenOfferId &#123;<br/>    TokenOfferId &#123;<br/>        to_addr,<br/>        token_id<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfers_offer_script"></a>

## Function `offer_script`



<pre><code>public entry fun offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun offer_script(<br/>    sender: signer,<br/>    receiver: address,<br/>    creator: address,<br/>    collection: String,<br/>    name: String,<br/>    property_version: u64,<br/>    amount: u64,<br/>) acquires PendingClaims &#123;<br/>    let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);<br/>    offer(&amp;sender, receiver, token_id, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfers_offer"></a>

## Function `offer`



<pre><code>public fun offer(sender: &amp;signer, receiver: address, token_id: token::TokenId, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun offer(<br/>    sender: &amp;signer,<br/>    receiver: address,<br/>    token_id: TokenId,<br/>    amount: u64,<br/>) acquires PendingClaims &#123;<br/>    let sender_addr &#61; signer::address_of(sender);<br/>    if (!exists&lt;PendingClaims&gt;(sender_addr)) &#123;<br/>        initialize_token_transfers(sender)<br/>    &#125;;<br/><br/>    let pending_claims &#61;<br/>        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).pending_claims;<br/>    let token_offer_id &#61; create_token_offer_id(receiver, token_id);<br/>    let token &#61; token::withdraw_token(sender, token_id, amount);<br/>    if (!table::contains(pending_claims, token_offer_id)) &#123;<br/>        table::add(pending_claims, token_offer_id, token);<br/>    &#125; else &#123;<br/>        let dst_token &#61; table::borrow_mut(pending_claims, token_offer_id);<br/>        token::merge(dst_token, token);<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            TokenOffer &#123;<br/>                to_address: receiver,<br/>                token_id,<br/>                amount,<br/>            &#125;<br/>        )<br/>    &#125;;<br/>    event::emit_event&lt;TokenOfferEvent&gt;(<br/>        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).offer_events,<br/>        TokenOfferEvent &#123;<br/>            to_address: receiver,<br/>            token_id,<br/>            amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfers_claim_script"></a>

## Function `claim_script`



<pre><code>public entry fun claim_script(receiver: signer, sender: address, creator: address, collection: string::String, name: string::String, property_version: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun claim_script(<br/>    receiver: signer,<br/>    sender: address,<br/>    creator: address,<br/>    collection: String,<br/>    name: String,<br/>    property_version: u64,<br/>) acquires PendingClaims &#123;<br/>    let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);<br/>    claim(&amp;receiver, sender, token_id);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfers_claim"></a>

## Function `claim`



<pre><code>public fun claim(receiver: &amp;signer, sender: address, token_id: token::TokenId)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun claim(<br/>    receiver: &amp;signer,<br/>    sender: address,<br/>    token_id: TokenId,<br/>) acquires PendingClaims &#123;<br/>    assert!(exists&lt;PendingClaims&gt;(sender), ETOKEN_OFFER_NOT_EXIST);<br/>    let pending_claims &#61;<br/>        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender).pending_claims;<br/>    let token_offer_id &#61; create_token_offer_id(signer::address_of(receiver), token_id);<br/>    assert!(table::contains(pending_claims, token_offer_id), error::not_found(ETOKEN_OFFER_NOT_EXIST));<br/>    let tokens &#61; table::remove(pending_claims, token_offer_id);<br/>    let amount &#61; token::get_token_amount(&amp;tokens);<br/>    token::deposit_token(receiver, tokens);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            TokenClaim &#123;<br/>                to_address: signer::address_of(receiver),<br/>                token_id,<br/>                amount,<br/>            &#125;<br/>        )<br/>    &#125;;<br/>    event::emit_event&lt;TokenClaimEvent&gt;(<br/>        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender).claim_events,<br/>        TokenClaimEvent &#123;<br/>            to_address: signer::address_of(receiver),<br/>            token_id,<br/>            amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfers_cancel_offer_script"></a>

## Function `cancel_offer_script`



<pre><code>public entry fun cancel_offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun cancel_offer_script(<br/>    sender: signer,<br/>    receiver: address,<br/>    creator: address,<br/>    collection: String,<br/>    name: String,<br/>    property_version: u64,<br/>) acquires PendingClaims &#123;<br/>    let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);<br/>    cancel_offer(&amp;sender, receiver, token_id);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfers_cancel_offer"></a>

## Function `cancel_offer`



<pre><code>public fun cancel_offer(sender: &amp;signer, receiver: address, token_id: token::TokenId)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun cancel_offer(<br/>    sender: &amp;signer,<br/>    receiver: address,<br/>    token_id: TokenId,<br/>) acquires PendingClaims &#123;<br/>    let sender_addr &#61; signer::address_of(sender);<br/>    let token_offer_id &#61; create_token_offer_id(receiver, token_id);<br/>    assert!(exists&lt;PendingClaims&gt;(sender_addr), ETOKEN_OFFER_NOT_EXIST);<br/>    let pending_claims &#61;<br/>        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).pending_claims;<br/>    let token &#61; table::remove(pending_claims, token_offer_id);<br/>    let amount &#61; token::get_token_amount(&amp;token);<br/>    token::deposit_token(sender, token);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            TokenCancelOffer &#123;<br/>                to_address: receiver,<br/>                token_id,<br/>                amount,<br/>            &#125;,<br/>        )<br/>    &#125;;<br/>    event::emit_event&lt;TokenCancelOfferEvent&gt;(<br/>        &amp;mut borrow_global_mut&lt;PendingClaims&gt;(sender_addr).cancel_offer_events,<br/>        TokenCancelOfferEvent &#123;<br/>            to_address: receiver,<br/>            token_id,<br/>            amount,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize_token_transfers"></a>

### Function `initialize_token_transfers`


<pre><code>fun initialize_token_transfers(account: &amp;signer)<br/></code></pre>




<pre><code>include InitializeTokenTransfersAbortsIf;<br/></code></pre>


Abort according to the code


<a id="0x3_token_transfers_InitializeTokenTransfersAbortsIf"></a>


<pre><code>schema InitializeTokenTransfersAbortsIf &#123;<br/>account: &amp;signer;<br/>let addr &#61; signer::address_of(account);<br/>aborts_if exists&lt;PendingClaims&gt;(addr);<br/>let account &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if account.guid_creation_num &#43; 3 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if account.guid_creation_num &#43; 3 &gt; MAX_U64;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_token_offer_id"></a>

### Function `create_token_offer_id`


<pre><code>fun create_token_offer_id(to_addr: address, token_id: token::TokenId): token_transfers::TokenOfferId<br/></code></pre>




<pre><code>aborts_if false;<br/></code></pre>



<a id="@Specification_1_offer_script"></a>

### Function `offer_script`


<pre><code>public entry fun offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);<br/></code></pre>



<a id="@Specification_1_offer"></a>

### Function `offer`


<pre><code>public fun offer(sender: &amp;signer, receiver: address, token_id: token::TokenId, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let sender_addr &#61; signer::address_of(sender);<br/>include !exists&lt;PendingClaims&gt;(sender_addr) &#61;&#61;&gt; InitializeTokenTransfersAbortsIf&#123;account : sender&#125;;<br/>let pending_claims &#61; global&lt;PendingClaims&gt;(sender_addr).pending_claims;<br/>let token_offer_id &#61; create_token_offer_id(receiver, token_id);<br/>let tokens &#61; global&lt;TokenStore&gt;(sender_addr).tokens;<br/>aborts_if amount &lt;&#61; 0;<br/>aborts_if token::spec_balance_of(sender_addr, token_id) &lt; amount;<br/>aborts_if !exists&lt;TokenStore&gt;(sender_addr);<br/>aborts_if !table::spec_contains(tokens, token_id);<br/>aborts_if !table::spec_contains(pending_claims, token_offer_id);<br/>let a &#61; table::spec_contains(pending_claims, token_offer_id);<br/>let dst_token &#61; table::spec_get(pending_claims, token_offer_id);<br/>aborts_if dst_token.amount &#43; spce_get(signer::address_of(sender), token_id, amount) &gt; MAX_U64;<br/></code></pre>


Get the amount from sender token


<a id="0x3_token_transfers_spce_get"></a>


<pre><code>fun spce_get(<br/>   account_addr: address,<br/>   id: TokenId,<br/>   amount: u64<br/>): u64 &#123;<br/>   use aptos_token::token::&#123;TokenStore&#125;;<br/>   use aptos_std::table::&#123;Self&#125;;<br/>   let tokens &#61; global&lt;TokenStore&gt;(account_addr).tokens;<br/>   let balance &#61; table::spec_get(tokens, id).amount;<br/>   if (balance &gt; amount) &#123;<br/>       amount<br/>   &#125; else &#123;<br/>       table::spec_get(tokens, id).amount<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_claim_script"></a>

### Function `claim_script`


<pre><code>public entry fun claim_script(receiver: signer, sender: address, creator: address, collection: string::String, name: string::String, property_version: u64)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);<br/>aborts_if !exists&lt;PendingClaims&gt;(sender);<br/>let pending_claims &#61; global&lt;PendingClaims&gt;(sender).pending_claims;<br/>let token_offer_id &#61; create_token_offer_id(signer::address_of(receiver), token_id);<br/>aborts_if !table::spec_contains(pending_claims, token_offer_id);<br/>let tokens &#61; table::spec_get(pending_claims, token_offer_id);<br/>include token::InitializeTokenStore&#123;account: receiver &#125;;<br/>let account_addr &#61; signer::address_of(receiver);<br/>let token &#61; tokens;<br/>let token_store &#61; global&lt;TokenStore&gt;(account_addr);<br/>let recipient_token &#61; table::spec_get(token_store.tokens, token.id);<br/>let b &#61; table::spec_contains(token_store.tokens, token.id);<br/>aborts_if token.amount &lt;&#61; 0;<br/></code></pre>



<a id="@Specification_1_claim"></a>

### Function `claim`


<pre><code>public fun claim(receiver: &amp;signer, sender: address, token_id: token::TokenId)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>aborts_if !exists&lt;PendingClaims&gt;(sender);<br/>let pending_claims &#61; global&lt;PendingClaims&gt;(sender).pending_claims;<br/>let token_offer_id &#61; create_token_offer_id(signer::address_of(receiver), token_id);<br/>aborts_if !table::spec_contains(pending_claims, token_offer_id);<br/>let tokens &#61; table::spec_get(pending_claims, token_offer_id);<br/>include token::InitializeTokenStore&#123;account: receiver &#125;;<br/>let account_addr &#61; signer::address_of(receiver);<br/>let token &#61; tokens;<br/>let token_store &#61; global&lt;TokenStore&gt;(account_addr);<br/>let recipient_token &#61; table::spec_get(token_store.tokens, token.id);<br/>let b &#61; table::spec_contains(token_store.tokens, token.id);<br/>aborts_if token.amount &lt;&#61; 0;<br/></code></pre>



<a id="@Specification_1_cancel_offer_script"></a>

### Function `cancel_offer_script`


<pre><code>public entry fun cancel_offer_script(sender: signer, receiver: address, creator: address, collection: string::String, name: string::String, property_version: u64)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let token_id &#61; token::create_token_id_raw(creator, collection, name, property_version);<br/>let sender_addr &#61; signer::address_of(sender);<br/>aborts_if !exists&lt;PendingClaims&gt;(sender_addr);<br/>let pending_claims &#61; global&lt;PendingClaims&gt;(sender_addr).pending_claims;<br/>let token_offer_id &#61; create_token_offer_id(receiver, token_id);<br/>aborts_if !table::spec_contains(pending_claims, token_offer_id);<br/>include token::InitializeTokenStore&#123;account: sender &#125;;<br/>let dst_token &#61; table::spec_get(pending_claims, token_offer_id);<br/>let account_addr &#61; sender_addr;<br/>let token &#61; dst_token;<br/>let token_store &#61; global&lt;TokenStore&gt;(account_addr);<br/>let recipient_token &#61; table::spec_get(token_store.tokens, token.id);<br/>let b &#61; table::spec_contains(token_store.tokens, token.id);<br/>aborts_if token.amount &lt;&#61; 0;<br/></code></pre>



<a id="@Specification_1_cancel_offer"></a>

### Function `cancel_offer`


<pre><code>public fun cancel_offer(sender: &amp;signer, receiver: address, token_id: token::TokenId)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let sender_addr &#61; signer::address_of(sender);<br/>aborts_if !exists&lt;PendingClaims&gt;(sender_addr);<br/>let pending_claims &#61; global&lt;PendingClaims&gt;(sender_addr).pending_claims;<br/>let token_offer_id &#61; create_token_offer_id(receiver, token_id);<br/>aborts_if !table::spec_contains(pending_claims, token_offer_id);<br/>include token::InitializeTokenStore&#123;account: sender &#125;;<br/>let dst_token &#61; table::spec_get(pending_claims, token_offer_id);<br/>let account_addr &#61; sender_addr;<br/>let token &#61; dst_token;<br/>let token_store &#61; global&lt;TokenStore&gt;(account_addr);<br/>let recipient_token &#61; table::spec_get(token_store.tokens, token.id);<br/>let b &#61; table::spec_contains(token_store.tokens, token.id);<br/>aborts_if token.amount &lt;&#61; 0;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
