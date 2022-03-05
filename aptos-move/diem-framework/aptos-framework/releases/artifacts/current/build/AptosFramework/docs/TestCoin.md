
<a name="0x1_TestCoin"></a>

# Module `0x1::TestCoin`

This module defines a minimal and generic Coin and Balance.
modified from https://github.com/diem/move/tree/main/language/documentation/tutorial


-  [Struct `Coin`](#0x1_TestCoin_Coin)
-  [Resource `Balance`](#0x1_TestCoin_Balance)
-  [Resource `MintCapability`](#0x1_TestCoin_MintCapability)
-  [Resource `BurnCapability`](#0x1_TestCoin_BurnCapability)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_TestCoin_initialize)
-  [Function `register`](#0x1_TestCoin_register)
-  [Function `mint`](#0x1_TestCoin_mint)
-  [Function `balance_of`](#0x1_TestCoin_balance_of)
-  [Function `transfer`](#0x1_TestCoin_transfer)
-  [Function `withdraw`](#0x1_TestCoin_withdraw)
-  [Function `deposit`](#0x1_TestCoin_deposit)
-  [Function `burn`](#0x1_TestCoin_burn)
-  [Function `burn_with_capability`](#0x1_TestCoin_burn_with_capability)
-  [Function `burn_gas`](#0x1_TestCoin_burn_gas)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
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

<a name="0x1_TestCoin_MintCapability"></a>

## Resource `MintCapability`



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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_TestCoin_EALREADY_HAS_BALANCE"></a>



<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EALREADY_HAS_BALANCE">EALREADY_HAS_BALANCE</a>: u64 = 1;
</code></pre>



<a name="0x1_TestCoin_EBALANCE_NOT_PUBLISHED"></a>



<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EBALANCE_NOT_PUBLISHED">EBALANCE_NOT_PUBLISHED</a>: u64 = 2;
</code></pre>



<a name="0x1_TestCoin_EINSUFFICIENT_BALANCE"></a>

Error codes


<pre><code><b>const</b> <a href="TestCoin.md#0x1_TestCoin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 0;
</code></pre>



<a name="0x1_TestCoin_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_initialize">initialize</a>(core_resource: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_initialize">initialize</a>(core_resource: &signer) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(core_resource);
    <b>move_to</b>(core_resource, <a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a> {});
    <b>move_to</b>(core_resource, <a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a> {});
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
}
</code></pre>



</details>

<a name="0x1_TestCoin_mint"></a>

## Function `mint`

Mint coins with capability.


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_mint">mint</a>(account: &signer, mint_addr: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_mint">mint</a>(account: &signer, mint_addr: <b>address</b>, amount: u64) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a>, <a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a> {
    <b>let</b> _cap = <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_MintCapability">MintCapability</a>&gt;(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account));
    // Deposit `amount` of tokens <b>to</b> `mint_addr`'s balance
    <a href="TestCoin.md#0x1_TestCoin_deposit">deposit</a>(mint_addr, <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> { value: amount });
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


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_transfer">transfer</a>(from: &signer, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_transfer">transfer</a>(from: &signer, <b>to</b>: <b>address</b>, amount: u64) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_Balance">Balance</a> {
    <b>let</b> check = <a href="TestCoin.md#0x1_TestCoin_withdraw">withdraw</a>(from, amount);
    <a href="TestCoin.md#0x1_TestCoin_deposit">deposit</a>(<b>to</b>, check);
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


<pre><code><b>public</b> <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn">burn</a>(account: &signer, coins: <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a>) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a> {
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


<pre><code><b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn_with_capability">burn_with_capability</a>(coins: <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a>, _cap: &<a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a>) {
    <b>let</b> <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a> { value: _value } = coins;
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


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="TestCoin.md#0x1_TestCoin_burn_gas">burn_gas</a>(fee: <a href="TestCoin.md#0x1_TestCoin_Coin">Coin</a>) <b>acquires</b> <a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a> {
    <b>let</b> cap = <b>borrow_global</b>&lt;<a href="TestCoin.md#0x1_TestCoin_BurnCapability">BurnCapability</a>&gt;(@CoreResources);
    <a href="TestCoin.md#0x1_TestCoin_burn_with_capability">burn_with_capability</a>(fee, cap);
}
</code></pre>



</details>
