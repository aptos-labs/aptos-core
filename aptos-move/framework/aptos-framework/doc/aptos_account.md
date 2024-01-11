
<a id="0x1_aptos_account"></a>

# Module `0x1::aptos_account`



-  [Resource `DirectTransferConfig`](#0x1_aptos_account_DirectTransferConfig)
-  [Struct `DirectCoinTransferConfigUpdatedEvent`](#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent)
-  [Constants](#@Constants_0)
-  [Function `create_account`](#0x1_aptos_account_create_account)
-  [Function `batch_transfer`](#0x1_aptos_account_batch_transfer)
-  [Function `transfer`](#0x1_aptos_account_transfer)
-  [Function `batch_transfer_coins`](#0x1_aptos_account_batch_transfer_coins)
-  [Function `transfer_coins`](#0x1_aptos_account_transfer_coins)
-  [Function `deposit_coins`](#0x1_aptos_account_deposit_coins)
-  [Function `assert_account_exists`](#0x1_aptos_account_assert_account_exists)
-  [Function `assert_account_is_registered_for_apt`](#0x1_aptos_account_assert_account_is_registered_for_apt)
-  [Function `set_allow_direct_coin_transfers`](#0x1_aptos_account_set_allow_direct_coin_transfers)
-  [Function `can_receive_direct_coin_transfers`](#0x1_aptos_account_can_receive_direct_coin_transfers)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create_account`](#@Specification_1_create_account)
    -  [Function `batch_transfer`](#@Specification_1_batch_transfer)
    -  [Function `transfer`](#@Specification_1_transfer)
    -  [Function `batch_transfer_coins`](#@Specification_1_batch_transfer_coins)
    -  [Function `transfer_coins`](#@Specification_1_transfer_coins)
    -  [Function `deposit_coins`](#@Specification_1_deposit_coins)
    -  [Function `assert_account_exists`](#@Specification_1_assert_account_exists)
    -  [Function `assert_account_is_registered_for_apt`](#@Specification_1_assert_account_is_registered_for_apt)
    -  [Function `set_allow_direct_coin_transfers`](#@Specification_1_set_allow_direct_coin_transfers)
    -  [Function `can_receive_direct_coin_transfers`](#@Specification_1_can_receive_direct_coin_transfers)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_aptos_account_DirectTransferConfig"></a>

## Resource `DirectTransferConfig`

Configuration for whether an account can receive direct transfers of coins that they have not registered.

By default, this is enabled. Users can opt-out by disabling at any time.


<pre><code><b>struct</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allow_arbitrary_coin_transfers: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>update_coin_transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">aptos_account::DirectCoinTransferConfigUpdatedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent"></a>

## Struct `DirectCoinTransferConfigUpdatedEvent`

Event emitted when an account's direct coins transfer config is updated.


<pre><code><b>struct</b> <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>new_allow_direct_transfers: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS"></a>

Account opted out of receiving coins that they did not register to receive.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS">EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS</a>: u64 = 3;
</code></pre>



<a id="0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS"></a>

Account opted out of directly receiving NFT tokens.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS">EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS</a>: u64 = 4;
</code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_FOUND"></a>

Account does not exist.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_FOUND">EACCOUNT_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT"></a>

Account is not registered to receive APT.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT">EACCOUNT_NOT_REGISTERED_FOR_APT</a>: u64 = 2;
</code></pre>



<a id="0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH"></a>

The lengths of the recipients and amounts lists don't match.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH">EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH</a>: u64 = 5;
</code></pre>



<a id="0x1_aptos_account_create_account"></a>

## Function `create_account`

Basic account creation methods.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>) {
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="account.md#0x1_account_create_account">account::create_account</a>(auth_key);
    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>);
}
</code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer"></a>

## Function `batch_transfer`

Batch version of APT transfer.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer">batch_transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer">batch_transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) {
    <b>let</b> recipients_len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&recipients);
    <b>assert</b>!(
        recipients_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amounts),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_account.md#0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH">EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH</a>),
    );

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(&recipients, |i, <b>to</b>| {
        <b>let</b> amount = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amounts, i);
        <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source, *<b>to</b>, amount);
    });
}
</code></pre>



</details>

<a id="0x1_aptos_account_transfer"></a>

## Function `transfer`

Convenient function to transfer APT to a recipient account that might not exist.
This would create the recipient account first, which also registers it to receive APT, before transferring.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64) {
    <b>if</b> (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>)) {
        <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(<b>to</b>)
    };
    // Resource accounts can be created without registering them <b>to</b> receive APT.
    // This conveniently does the registration <b>if</b> necessary.
    <b>if</b> (!<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(<b>to</b>)) {
        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&<a href="create_signer.md#0x1_create_signer">create_signer</a>(<b>to</b>));
    };
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(source, <b>to</b>, amount)
}
</code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer_coins"></a>

## Function `batch_transfer_coins`

Batch version of transfer_coins.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_coins">batch_transfer_coins</a>&lt;CoinType&gt;(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_coins">batch_transfer_coins</a>&lt;CoinType&gt;(
    from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> {
    <b>let</b> recipients_len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&recipients);
    <b>assert</b>!(
        recipients_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amounts),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_account.md#0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH">EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH</a>),
    );

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(&recipients, |i, <b>to</b>| {
        <b>let</b> amount = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amounts, i);
        <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from, *<b>to</b>, amount);
    });
}
</code></pre>



</details>

<a id="0x1_aptos_account_transfer_coins"></a>

## Function `transfer_coins`

Convenient function to transfer a custom CoinType to a recipient account that might not exist.
This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> {
    <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>(<b>to</b>, <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;CoinType&gt;(from, amount));
}
</code></pre>



</details>

<a id="0x1_aptos_account_deposit_coins"></a>

## Function `deposit_coins`

Convenient function to deposit a custom CoinType into a recipient account that might not exist.
This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>&lt;CoinType&gt;(<b>to</b>: <b>address</b>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>&lt;CoinType&gt;(<b>to</b>: <b>address</b>, coins: Coin&lt;CoinType&gt;) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> {
    <b>if</b> (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>)) {
        <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(<b>to</b>);
        <b>spec</b> {
            <b>assert</b> <a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(<b>to</b>);
            <b>assume</b> aptos_std::type_info::type_of&lt;CoinType&gt;() == aptos_std::type_info::type_of&lt;AptosCoin&gt;() ==&gt;
                <a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(<b>to</b>);
        };
    };
    <b>if</b> (!<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(<b>to</b>)) {
        <b>assert</b>!(
            <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<b>to</b>),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS">EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS</a>),
        );
        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;CoinType&gt;(&<a href="create_signer.md#0x1_create_signer">create_signer</a>(<b>to</b>));
    };
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>&lt;CoinType&gt;(<b>to</b>, coins)
}
</code></pre>



</details>

<a id="0x1_aptos_account_assert_account_exists"></a>

## Function `assert_account_exists`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>) {
    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_FOUND">EACCOUNT_NOT_FOUND</a>));
}
</code></pre>



</details>

<a id="0x1_aptos_account_assert_account_is_registered_for_apt"></a>

## Function `assert_account_is_registered_for_apt`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>) {
    <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr);
    <b>assert</b>!(<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT">EACCOUNT_NOT_REGISTERED_FOR_APT</a>));
}
</code></pre>



</details>

<a id="0x1_aptos_account_set_allow_direct_coin_transfers"></a>

## Function `set_allow_direct_coin_transfers`

Set whether <code><a href="account.md#0x1_account">account</a></code> can receive direct transfers of coins that they have not explicitly registered to receive.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_set_allow_direct_coin_transfers">set_allow_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_set_allow_direct_coin_transfers">set_allow_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allow: bool) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>if</b> (<b>exists</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(addr)) {
        <b>let</b> direct_transfer_config = <b>borrow_global_mut</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(addr);
        // Short-circuit <b>to</b> avoid emitting an <a href="event.md#0x1_event">event</a> <b>if</b> direct transfer config is not changing.
        <b>if</b> (direct_transfer_config.allow_arbitrary_coin_transfers == allow) {
            <b>return</b>
        };

        direct_transfer_config.allow_arbitrary_coin_transfers = allow;
        emit_event(
            &<b>mut</b> direct_transfer_config.update_coin_transfer_events,
            <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> { new_allow_direct_transfers: allow });
    } <b>else</b> {
        <b>let</b> direct_transfer_config = <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> {
            allow_arbitrary_coin_transfers: allow,
            update_coin_transfer_events: new_event_handle&lt;<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),
        };
        emit_event(
            &<b>mut</b> direct_transfer_config.update_coin_transfer_events,
            <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> { new_allow_direct_transfers: allow });
        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, direct_transfer_config);
    };
}
</code></pre>



</details>

<a id="0x1_aptos_account_can_receive_direct_coin_transfers"></a>

## Function `can_receive_direct_coin_transfers`

Return true if <code><a href="account.md#0x1_account">account</a></code> can receive direct transfers of coins that they have not explicitly registered to
receive.

By default, this returns true if an account has not explicitly set whether the can receive direct transfers.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> {
    !<b>exists</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>) ||
        <b>borrow_global</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>).allow_arbitrary_coin_transfers
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>During the creation of an Aptos account the following rules should hold: (1) the authentication key should be 32 bytes in length, (2) an Aptos account should not already exist for that authentication key, and (3) the address of the authentication key should not be equal to a reserved address (0x0, 0x1, or 0x3).</td>
<td>Critical</td>
<td>The authentication key which is passed in as an argument to create_account should satisfy all necessary conditions.</td>
<td>Formally verified via <a href="#high-level-req-1">CreateAccountAbortsIf</a>.</td>
</tr>

<tr>
<td>2</td>
<td>After creating an Aptos account, the account should become registered to receive AptosCoin.</td>
<td>Critical</td>
<td>The create_account function creates a new account for the particular address and registers AptosCoin.</td>
<td>Formally verified via <a href="#high-level-req-2">create_account</a>.</td>
</tr>

<tr>
<td>3</td>
<td>An account may receive a direct transfer of coins they have not registered for if and only if the transfer of arbitrary coins is enabled. By default the option should always set to be enabled for an account.</td>
<td>Low</td>
<td>Transfers of a coin to an account that has not yet registered for that coin should abort if and only if the allow_arbitrary_coin_transfers flag is explicitly set to false.</td>
<td>Formally verified via <a href="#high-level-req-3">can_receive_direct_coin_transfers</a>.</td>
</tr>

<tr>
<td>4</td>
<td>Setting direct coin transfers may only occur if and only if a direct transfer config is associated with the provided account address.</td>
<td>Low</td>
<td>The set_allow_direct_coin_transfers function ensures the DirectTransferConfig structure exists for the signer.</td>
<td>Formally verified via <a href="#high-level-req-4">set_allow_direct_coin_transfers</a>.</td>
</tr>

<tr>
<td>5</td>
<td>The transfer function should ensure an account is created for the provided destination if one does not exist; then, register AptosCoin for that account if a particular is unregistered before transferring the amount.</td>
<td>Critical</td>
<td>The transfer function checks if the recipient account exists. If the account does not exist, the function creates one and registers the account to AptosCoin if not registered.</td>
<td>Formally verified via <a href="#high-level-req-5">transfer</a>.</td>
</tr>

<tr>
<td>6</td>
<td>Creating an account for the provided destination and registering it for that particular CoinType should be the only way to enable depositing coins, provided the account does not already exist.</td>
<td>Critical</td>
<td>The deposit_coins function verifies if the recipient account exists. If the account does not exist, the function creates one and ensures that the account becomes registered for the specified CointType.</td>
<td>Formally verified via <a href="#high-level-req-6">deposit_coins</a>.</td>
</tr>

<tr>
<td>7</td>
<td>When performing a batch transfer of Aptos Coin and/or a batch transfer of a custom coin type, it should ensure that the vector containing destination addresses and the vector containing the corresponding amounts are equal in length.</td>
<td>Low</td>
<td>The batch_transfer and batch_transfer_coins functions verify that the length of the recipient addresses vector matches the length of the amount vector through an assertion.</td>
<td>Formally verified via <a href="#high-level-req-7">batch_transfer_coins</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>)
</code></pre>


Check if the bytes of the auth_key is 32.
The Account does not exist under the auth_key before creating the account.
Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a>;
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(auth_key);
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(auth_key);
</code></pre>



<a id="@Specification_1_batch_transfer"></a>

### Function `batch_transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer">batch_transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);
<b>let</b> coin_store_source = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source);
<b>let</b> balance_source = coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;
<b>requires</b> <b>forall</b> i in 0..len(recipients):
    recipients[i] != account_addr_source;
<b>requires</b> <b>exists</b> i in 0..len(recipients):
    amounts[i] &gt; 0;
<b>aborts_if</b> len(recipients) != len(amounts);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i]);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @aptos_framework || recipients[i] == @aptos_token);
<b>ensures</b> <b>forall</b> i in 0..len(recipients):
        (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) ==&gt; !<a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i])) &&
            (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) ==&gt; (recipients[i] != @vm_reserved && recipients[i] != @aptos_framework && recipients[i] != @aptos_token));
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    coin_store_source.frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source).<a href="coin.md#0x1_coin">coin</a>.value &lt; amounts[i];
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]).frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);
<b>requires</b> account_addr_source != <b>to</b>;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;AptosCoin&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;AptosCoin&gt;{from: source};
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_TransferEnsures">TransferEnsures</a>&lt;AptosCoin&gt;;
<b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(<b>to</b>) && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(<b>to</b>).frozen;
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
<b>ensures</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);
<b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(<b>to</b>);
</code></pre>



<a id="@Specification_1_batch_transfer_coins"></a>

### Function `batch_transfer_coins`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_coins">batch_transfer_coins</a>&lt;CoinType&gt;(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);
<b>let</b> coin_store_source = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);
<b>let</b> balance_source = coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;
<b>requires</b> <b>forall</b> i in 0..len(recipients):
    recipients[i] != account_addr_source;
<b>requires</b> <b>exists</b> i in 0..len(recipients):
    amounts[i] &gt; 0;
// This enforces <a id="high-level-req-7" href="#high-level-req">high-level requirement 7</a>:
<b>aborts_if</b> len(recipients) != len(amounts);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i]);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @aptos_framework || recipients[i] == @aptos_token);
<b>ensures</b> <b>forall</b> i in 0..len(recipients):
        (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) ==&gt; !<a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i])) &&
            (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) ==&gt; (recipients[i] != @vm_reserved && recipients[i] != @aptos_framework && recipients[i] != @aptos_token));
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    coin_store_source.frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source).<a href="coin.md#0x1_coin">coin</a>.value &lt; amounts[i];
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]).frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt; MAX_U64;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    !<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(recipients[i]) && !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;CoinType&gt;();
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    !<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(recipients[i]) && !<a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(recipients[i]);
</code></pre>



<a id="@Specification_1_transfer_coins"></a>

### Function `transfer_coins`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);
<b>requires</b> account_addr_source != <b>to</b>;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;CoinType&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_TransferEnsures">TransferEnsures</a>&lt;CoinType&gt;;
<b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).frozen;
<b>ensures</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);
<b>ensures</b> <b>exists</b>&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(<b>to</b>);
</code></pre>




<a id="0x1_aptos_account_CreateAccountTransferAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a> {
    <b>to</b>: <b>address</b>;
    <b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) && <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(<b>to</b>);
    <b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) && (<b>to</b> == @vm_reserved || <b>to</b> == @aptos_framework || <b>to</b> == @aptos_token);
}
</code></pre>




<a id="0x1_aptos_account_WithdrawAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt; {
    from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    amount: u64;
    <b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);
    <b>let</b> coin_store_source = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);
    <b>let</b> balance_source = coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);
    <b>aborts_if</b> coin_store_source.frozen;
    <b>aborts_if</b> balance_source &lt; amount;
}
</code></pre>




<a id="0x1_aptos_account_GuidAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;CoinType&gt; {
    <b>to</b>: <b>address</b>;
    <b>let</b> acc = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(<b>to</b>);
    <b>aborts_if</b> <a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) && acc.guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
    <b>aborts_if</b> <a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) && acc.guid_creation_num + 2 &gt; MAX_U64;
}
</code></pre>




<a id="0x1_aptos_account_RegistCoinAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt; {
    <b>to</b>: <b>address</b>;
    <b>aborts_if</b> !<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(<b>to</b>) && !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;CoinType&gt;();
    <b>aborts_if</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>)
        && !<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(<b>to</b>) && !<a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<b>to</b>);
    <b>aborts_if</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;() != <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;()
        && !<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(<b>to</b>) && !<a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<b>to</b>);
}
</code></pre>




<a id="0x1_aptos_account_TransferEnsures"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_TransferEnsures">TransferEnsures</a>&lt;CoinType&gt; {
    <b>to</b>: <b>address</b>;
    account_addr_source: <b>address</b>;
    amount: u64;
    <b>let</b> if_exist_account = <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(<b>to</b>);
    <b>let</b> if_exist_coin = <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);
    <b>let</b> coin_store_to = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);
    <b>let</b> coin_store_source = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);
    <b>let</b> <b>post</b> p_coin_store_to = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);
    <b>let</b> <b>post</b> p_coin_store_source = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);
    <b>ensures</b> coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value - amount == p_coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;
    <b>ensures</b> if_exist_account && if_exist_coin ==&gt; coin_store_to.<a href="coin.md#0x1_coin">coin</a>.value + amount == p_coin_store_to.<a href="coin.md#0x1_coin">coin</a>.value;
}
</code></pre>



<a id="@Specification_1_deposit_coins"></a>

### Function `deposit_coins`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>&lt;CoinType&gt;(<b>to</b>: <b>address</b>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)
</code></pre>




<pre><code><b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;CoinType&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt;;
<b>let</b> if_exist_coin = <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);
<b>aborts_if</b> if_exist_coin && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).frozen;
// This enforces <a id="high-level-spec-6" href="#high-level-req">high-level requirement 6</a>:
<b>ensures</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);
<b>ensures</b> <b>exists</b>&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(<b>to</b>);
<b>let</b> coin_store_to = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).<a href="coin.md#0x1_coin">coin</a>.value;
<b>let</b> <b>post</b> post_coin_store_to = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).<a href="coin.md#0x1_coin">coin</a>.value;
<b>ensures</b> if_exist_coin ==&gt; post_coin_store_to == coin_store_to + coins.value;
</code></pre>



<a id="@Specification_1_assert_account_exists"></a>

### Function `assert_account_exists`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr);
</code></pre>



<a id="@Specification_1_assert_account_is_registered_for_apt"></a>

### Function `assert_account_is_registered_for_apt`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>)
</code></pre>


Check if the address existed.
Check if the AptosCoin under the address existed.


<pre><code><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr);
<b>aborts_if</b> !<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(addr);
</code></pre>



<a id="@Specification_1_set_allow_direct_coin_transfers"></a>

### Function `set_allow_direct_coin_transfers`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_set_allow_direct_coin_transfers">set_allow_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allow: bool)
</code></pre>




<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>include</b> !<b>exists</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(addr) ==&gt; <a href="account.md#0x1_account_NewEventHandleAbortsIf">account::NewEventHandleAbortsIf</a>;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>ensures</b> <b>global</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(addr).allow_arbitrary_coin_transfers == allow;
</code></pre>



<a id="@Specification_1_can_receive_direct_coin_transfers"></a>

### Function `can_receive_direct_coin_transfers`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>ensures</b> result == (
    !<b>exists</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>) ||
        <b>global</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>).allow_arbitrary_coin_transfers
);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
