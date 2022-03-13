
<a name="0x1_TestCoin"></a>

# Module `0x1::TestCoin`

This module defines a minimal and generic Coin and Balance.
modified from https://github.com/diem/move/tree/main/language/documentation/tutorial


-  [Struct `Coin`](#0x1_TestCoin_Coin)
-  [Resource `Balance`](#0x1_TestCoin_Balance)
-  [Resource `CoinInfo`](#0x1_TestCoin_CoinInfo)
-  [Resource `MintCapability`](#0x1_TestCoin_MintCapability)
-  [Resource `BurnCapability`](#0x1_TestCoin_BurnCapability)
-  [Struct `DelegatedMintCapability`](#0x1_TestCoin_DelegatedMintCapability)
-  [Resource `Delegations`](#0x1_TestCoin_Delegations)
-  [Resource `TransferEvents`](#0x1_TestCoin_TransferEvents)
-  [Struct `SentEvent`](#0x1_TestCoin_SentEvent)
-  [Struct `ReceivedEvent`](#0x1_TestCoin_ReceivedEvent)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_TestCoin_initialize)
-  [Function `register`](#0x1_TestCoin_register)
-  [Function `delegate_mint_capability`](#0x1_TestCoin_delegate_mint_capability)
-  [Function `claim_mint_capability`](#0x1_TestCoin_claim_mint_capability)
-  [Function `find_delegation`](#0x1_TestCoin_find_delegation)
-  [Function `mint`](#0x1_TestCoin_mint)
-  [Function `mint_internal`](#0x1_TestCoin_mint_internal)
-  [Function `exists_at`](#0x1_TestCoin_exists_at)
-  [Function `balance_of`](#0x1_TestCoin_balance_of)
-  [Function `transfer`](#0x1_TestCoin_transfer)
-  [Function `withdraw`](#0x1_TestCoin_withdraw)
-  [Function `deposit`](#0x1_TestCoin_deposit)
-  [Function `burn`](#0x1_TestCoin_burn)
-  [Function `burn_with_capability`](#0x1_TestCoin_burn_with_capability)
-  [Function `burn_gas`](#0x1_TestCoin_burn_gas)
-  [Function `total_supply`](#0x1_TestCoin_total_supply)
-  [Function `scaling_factor`](#0x1_TestCoin_scaling_factor)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event">0x1::Event</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_TestCoin_Coin"></a>

## Struct `Coin`



<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_Balance"></a>

## Resource `Balance`

Struct representing the balance of each address.


<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>coin: <a href="TestCoin.md#0x1_TestCoin_Coin">TestCoin::Coin</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_CoinInfo"></a>

## Resource `CoinInfo`

Represnets the metadata of the coin, store @CoreResources.


<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>total_value: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>scaling_factor: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_MintCapability"></a>

## Resource `MintCapability`

Capability required to mint coins.


<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_BurnCapability"></a>

## Resource `BurnCapability`

Capability required to burn coins.


<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_DelegatedMintCapability"></a>

## Struct `DelegatedMintCapability`

Delegation token created by delegator and can be claimed by the delegatee as MintCapability.


<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_DelegatedMintCapability">DelegatedMintCapability</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_Delegations"></a>

## Resource `Delegations`

The container stores the current pending delegations.


<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: vector&lt;<a href="TestCoin.md#0x1_TestCoin_DelegatedMintCapability">TestCoin::DelegatedMintCapability</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_TransferEvents"></a>

## Resource `TransferEvents`

Events handles.


<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_TransferEvents">TransferEvents</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sent_events: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="TestCoin.md#0x1_TestCoin_SentEvent">TestCoin::SentEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>received_events: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="TestCoin.md#0x1_TestCoin_ReceivedEvent">TestCoin::ReceivedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_SentEvent"></a>

## Struct `SentEvent`



<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_SentEvent">SentEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TestCoin_ReceivedEvent"></a>

## Struct `ReceivedEvent`



<pre><code><b>struct</b> <a href="TestCoin.md#0x1_TestCoin_ReceivedEvent">ReceivedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>from: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_TestCoin_EALREADY_DELEGATED"></a>



<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EALREADY_DELEGATED">EALREADY_DELEGATED</a>: u64 = 3;
</code></pre>



<a name="0x1_TestCoin_EALREADY_HAS_BALANCE"></a>



<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EALREADY_HAS_BALANCE">EALREADY_HAS_BALANCE</a>: u64 = 1;
</code></pre>



<a name="0x1_TestCoin_EBALANCE_NOT_PUBLISHED"></a>



<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EBALANCE_NOT_PUBLISHED">EBALANCE_NOT_PUBLISHED</a>: u64 = 2;
</code></pre>



<a name="0x1_TestCoin_EDELEGATION_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EDELEGATION_NOT_FOUND">EDELEGATION_NOT_FOUND</a>: u64 = 4;
</code></pre>



<a name="0x1_TestCoin_EINSUFFICIENT_BALANCE"></a>

Error codes


<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 0;
</code></pre>



<a name="0x1_TestCoin_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_initialize">initialize</a>(core_resource: &signer, scaling_factor: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_initialize">initialize</a>(core_resource: &signer, scaling_factor: u64) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(core_resource);
    <b>move_to</b>(core_resource, <a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a> {});
    <b>move_to</b>(core_resource, <a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a> {});
    <b>move_to</b>(core_resource, <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> { total_value: 0, scaling_factor });
    <b>move_to</b>(core_resource, <a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a> { inner: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>() });
    <a href="TestCoin.md#0x1_TestCoin_register">register</a>(core_resource);
}
</code></pre>



</details>

<a name="0x1_TestCoin_register"></a>

## Function `register`

Publish an empty balance resource under <code>account</code>'s address. This function must be called before
minting or transferring to the account.


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_register">register</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_register">register</a>(account: &signer) {
    <b>let</b> empty_coin = <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> { value: 0 };
    <b>assert</b>!(!<b>exists</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>&gt;(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account)), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="TestCoin.md#0x1_TestCoin_EALREADY_HAS_BALANCE">EALREADY_HAS_BALANCE</a>));
    <b>move_to</b>(account, <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a> { coin:  empty_coin });
    <b>move_to</b>(
        account,
        <a href="TestCoin.md#0x1_TestCoin_TransferEvents">TransferEvents</a> {
            sent_events: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="TestCoin.md#0x1_TestCoin_SentEvent">SentEvent</a>&gt;(account),
            received_events: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="TestCoin.md#0x1_TestCoin_ReceivedEvent">ReceivedEvent</a>&gt;(account),
        }
    );
}
</code></pre>



</details>

<a name="0x1_TestCoin_delegate_mint_capability"></a>

## Function `delegate_mint_capability`

Create delegated token for the address so the account could claim MintCapability later.


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_delegate_mint_capability">delegate_mint_capability</a>(account: signer, <b>to</b>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_delegate_mint_capability">delegate_mint_capability</a>(account: signer, <b>to</b>: <b>address</b>) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a> {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(&account);
    <b>let</b> delegations = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a>&gt;(@CoreResources).inner;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(delegations)) {
        <b>let</b> element = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(delegations, i);
        <b>assert</b>!(element.<b>to</b> != <b>to</b>, <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="TestCoin.md#0x1_TestCoin_EALREADY_DELEGATED">EALREADY_DELEGATED</a>));
    };
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_push_back">Vector::push_back</a>(delegations, <a href="TestCoin.md#0x1_TestCoin_DelegatedMintCapability">DelegatedMintCapability</a> { <b>to</b> });
}
</code></pre>



</details>

<a name="0x1_TestCoin_claim_mint_capability"></a>

## Function `claim_mint_capability`

Claim the delegated mint capability and destroy the delegated token.


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_claim_mint_capability">claim_mint_capability</a>(account: signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_claim_mint_capability">claim_mint_capability</a>(account: signer) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a> {
    <b>let</b> maybe_index = <a href="TestCoin.md#0x1_TestCoin_find_delegation">find_delegation</a>(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(&account));
    <b>assert</b>!(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_is_some">Option::is_some</a>(&maybe_index), <a href="TestCoin.md#0x1_TestCoin_EDELEGATION_NOT_FOUND">EDELEGATION_NOT_FOUND</a>);
    <b>let</b> idx = *<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_borrow">Option::borrow</a>(&maybe_index);
    <b>let</b> delegations = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a>&gt;(@CoreResources).inner;
    <b>let</b> <a href="TestCoin.md#0x1_TestCoin_DelegatedMintCapability">DelegatedMintCapability</a> { <b>to</b>: _} = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_swap_remove">Vector::swap_remove</a>(delegations, idx);

    <b>move_to</b>(&account, <a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a> {});
}
</code></pre>



</details>

<a name="0x1_TestCoin_find_delegation"></a>

## Function `find_delegation`



<pre><code><b>fun</b> <a href="TestCoin.md#0x1_TestCoin_find_delegation">find_delegation</a>(addr: <b>address</b>): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="TestCoin.md#0x1_TestCoin_find_delegation">find_delegation</a>(addr: <b>address</b>): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a>&lt;u64&gt; <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a> {
    <b>let</b> delegations = &<b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Delegations">Delegations</a>&gt;(@CoreResources).inner;
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(delegations);
    <b>let</b> index = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>();
    <b>while</b> (i &lt; len) {
        <b>let</b> element = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(delegations, i);
        <b>if</b> (element.<b>to</b> == addr) {
            index = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(i);
            <b>break</b>
        };
        i = i + 1;
    };
    index
}
</code></pre>



</details>

<a name="0x1_TestCoin_mint"></a>

## Function `mint`

Mint coins with capability.


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_mint">mint</a>(account: signer, mint_addr: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_mint">mint</a>(
    account: signer,
    mint_addr: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>, <a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a>, <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a>
{
    <a href="TestCoin.md#0x1_TestCoin_mint_internal">mint_internal</a>(&account, mint_addr, amount);
}
</code></pre>



</details>

<a name="0x1_TestCoin_mint_internal"></a>

## Function `mint_internal`



<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_mint_internal">mint_internal</a>(account: &signer, mint_addr: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_mint_internal">mint_internal</a>(account: &signer, mint_addr: <b>address</b>, amount: u64) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>, <a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a>, <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> {
    <b>let</b> sender_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>let</b> _cap = <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a>&gt;(sender_addr);
    // Deposit `amount` of tokens <b>to</b> `mint_addr`'s balance
    <a href="TestCoin.md#0x1_TestCoin_deposit">deposit</a>(mint_addr, <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> { value: amount });
    // Update the total supply
    <b>let</b> coin_info = <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a>&gt;(@CoreResources);
    coin_info.total_value = coin_info.total_value + (amount <b>as</b> u128);
}
</code></pre>



</details>

<a name="0x1_TestCoin_exists_at"></a>

## Function `exists_at`



<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_exists_at">exists_at</a>(addr: <b>address</b>): bool{
    <b>exists</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>&gt;(addr)
}
</code></pre>



</details>

<a name="0x1_TestCoin_balance_of"></a>

## Function `balance_of`

Returns the balance of <code>owner</code>.


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_balance_of">balance_of</a>(owner: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_balance_of">balance_of</a>(owner: <b>address</b>): u64 <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>&gt;(owner), <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="TestCoin.md#0x1_TestCoin_EBALANCE_NOT_PUBLISHED">EBALANCE_NOT_PUBLISHED</a>));
    <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>&gt;(owner).coin.value
}
</code></pre>



</details>

<a name="0x1_TestCoin_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of tokens from <code>from</code> to <code><b>to</b></code>.


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_transfer">transfer</a>(from: signer, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_transfer">transfer</a>(from: signer, <b>to</b>: <b>address</b>, amount: u64) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>, <a href="TestCoin.md#0x1_TestCoin_TransferEvents">TransferEvents</a> {
    <b>let</b> check = <a href="TestCoin.md#0x1_TestCoin_withdraw">withdraw</a>(&from, amount);
    <a href="TestCoin.md#0x1_TestCoin_deposit">deposit</a>(<b>to</b>, check);
    // emit events
    <b>let</b> sender_handle = <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_TransferEvents">TransferEvents</a>&gt;(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(&from));
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>&lt;<a href="TestCoin.md#0x1_TestCoin_SentEvent">SentEvent</a>&gt;(
        &<b>mut</b> sender_handle.sent_events,
        <a href="TestCoin.md#0x1_TestCoin_SentEvent">SentEvent</a> { amount, <b>to</b> },
    );
    <b>let</b> receiver_handle = <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_TransferEvents">TransferEvents</a>&gt;(<b>to</b>);
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>&lt;<a href="TestCoin.md#0x1_TestCoin_ReceivedEvent">ReceivedEvent</a>&gt;(
        &<b>mut</b> receiver_handle.received_events,
        <a href="TestCoin.md#0x1_TestCoin_ReceivedEvent">ReceivedEvent</a> { amount, from: <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(&from) },
    );
}
</code></pre>



</details>

<a name="0x1_TestCoin_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> number of tokens from the balance under <code>addr</code>.


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_withdraw">withdraw</a>(signer: &signer, amount: u64): <a href="TestCoin.md#0x1_TestCoin_Coin">TestCoin::Coin</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_withdraw">withdraw</a>(signer: &signer, amount: u64) : <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a> {
    <b>let</b> addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(signer);
    <b>let</b> balance = <a href="TestCoin.md#0x1_TestCoin_balance_of">balance_of</a>(addr);
    // balance must be greater than the withdraw amount
    <b>assert</b>!(balance &gt;= amount, <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_limit_exceeded">Errors::limit_exceeded</a>(<a href="TestCoin.md#0x1_TestCoin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
    <b>let</b> balance_ref = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>&gt;(addr).coin.value;
    *balance_ref = balance - amount;
    <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> { value: amount }
}
</code></pre>



</details>

<a name="0x1_TestCoin_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> number of tokens to the balance under <code>addr</code>.


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_deposit">deposit</a>(addr: <b>address</b>, check: <a href="TestCoin.md#0x1_TestCoin_Coin">TestCoin::Coin</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_deposit">deposit</a>(addr: <b>address</b>, check: <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a>) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a> {
    <b>let</b> balance = <a href="TestCoin.md#0x1_TestCoin_balance_of">balance_of</a>(addr);
    <b>let</b> balance_ref = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>&gt;(addr).coin.value;
    <b>let</b> <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> { value } = check;
    *balance_ref = balance + value;
}
</code></pre>



</details>

<a name="0x1_TestCoin_burn"></a>

## Function `burn`

Burn coins with capability.


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn">burn</a>(account: &signer, coins: <a href="TestCoin.md#0x1_TestCoin_Coin">TestCoin::Coin</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn">burn</a>(account: &signer, coins: <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a>) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a>, <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> {
    <b>let</b> cap = <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a>&gt;(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account));
    <a href="TestCoin.md#0x1_TestCoin_burn_with_capability">burn_with_capability</a>(coins, cap);
}
</code></pre>



</details>

<a name="0x1_TestCoin_burn_with_capability"></a>

## Function `burn_with_capability`



<pre><code><b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn_with_capability">burn_with_capability</a>(coins: <a href="TestCoin.md#0x1_TestCoin_Coin">TestCoin::Coin</a>, _cap: &<a href="TestCoin.md#0x1_TestCoin_BurnCapability">TestCoin::BurnCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn_with_capability">burn_with_capability</a>(coins: <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a>, _cap: &<a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a>) <b>acquires</b>  <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> {
    <b>let</b> <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> { value } = coins;
    // Update the total supply
    <b>let</b> coin_info = <b>borrow_global_mut</b>&lt;<a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a>&gt;(@CoreResources);
    coin_info.total_value = coin_info.total_value - (value <b>as</b> u128);
}
</code></pre>



</details>

<a name="0x1_TestCoin_burn_gas"></a>

## Function `burn_gas`

Burn transaction gas.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn_gas">burn_gas</a>(fee: <a href="TestCoin.md#0x1_TestCoin_Coin">TestCoin::Coin</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn_gas">burn_gas</a>(fee: <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a>) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a>, <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> {
    <b>let</b> cap = <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a>&gt;(@CoreResources);
    <a href="TestCoin.md#0x1_TestCoin_burn_with_capability">burn_with_capability</a>(fee, cap);
}
</code></pre>



</details>

<a name="0x1_TestCoin_total_supply"></a>

## Function `total_supply`

Get the current total supply of the coin.


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_total_supply">total_supply</a>(): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_total_supply">total_supply</a>(): u128 <b>acquires</b>  <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> {
    <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a>&gt;(@CoreResources).total_value
}
</code></pre>



</details>

<a name="0x1_TestCoin_scaling_factor"></a>

## Function `scaling_factor`



<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_scaling_factor">scaling_factor</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_scaling_factor">scaling_factor</a>(): u64 <b>acquires</b>  <a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a> {
    <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_CoinInfo">CoinInfo</a>&gt;(@CoreResources).scaling_factor
}
</code></pre>



</details>
