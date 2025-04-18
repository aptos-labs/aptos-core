
<a id="0x1_aptos_account"></a>

# Module `0x1::aptos_account`



-  [Resource `DirectTransferConfig`](#0x1_aptos_account_DirectTransferConfig)
-  [Struct `DirectCoinTransferConfigUpdatedEvent`](#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent)
-  [Struct `DirectCoinTransferConfigUpdated`](#0x1_aptos_account_DirectCoinTransferConfigUpdated)
-  [Constants](#@Constants_0)
-  [Function `create_account`](#0x1_aptos_account_create_account)
-  [Function `batch_transfer`](#0x1_aptos_account_batch_transfer)
-  [Function `transfer`](#0x1_aptos_account_transfer)
-  [Function `batch_transfer_coins`](#0x1_aptos_account_batch_transfer_coins)
-  [Function `transfer_coins`](#0x1_aptos_account_transfer_coins)
-  [Function `deposit_coins`](#0x1_aptos_account_deposit_coins)
-  [Function `batch_transfer_fungible_assets`](#0x1_aptos_account_batch_transfer_fungible_assets)
-  [Function `transfer_fungible_assets`](#0x1_aptos_account_transfer_fungible_assets)
-  [Function `deposit_fungible_assets`](#0x1_aptos_account_deposit_fungible_assets)
-  [Function `assert_account_exists`](#0x1_aptos_account_assert_account_exists)
-  [Function `assert_account_is_registered_for_apt`](#0x1_aptos_account_assert_account_is_registered_for_apt)
-  [Function `set_allow_direct_coin_transfers`](#0x1_aptos_account_set_allow_direct_coin_transfers)
-  [Function `can_receive_direct_coin_transfers`](#0x1_aptos_account_can_receive_direct_coin_transfers)
-  [Function `register_apt`](#0x1_aptos_account_register_apt)
-  [Function `fungible_transfer_only`](#0x1_aptos_account_fungible_transfer_only)
-  [Function `is_fungible_balance_at_least`](#0x1_aptos_account_is_fungible_balance_at_least)
-  [Function `burn_from_fungible_store_for_gas`](#0x1_aptos_account_burn_from_fungible_store_for_gas)
-  [Function `ensure_primary_fungible_store_exists`](#0x1_aptos_account_ensure_primary_fungible_store_exists)
-  [Function `primary_fungible_store_address`](#0x1_aptos_account_primary_fungible_store_address)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create_account`](#@Specification_1_create_account)
    -  [Function `batch_transfer`](#@Specification_1_batch_transfer)
    -  [Function `transfer`](#@Specification_1_transfer)
    -  [Function `batch_transfer_coins`](#@Specification_1_batch_transfer_coins)
    -  [Function `transfer_coins`](#@Specification_1_transfer_coins)
    -  [Function `deposit_coins`](#@Specification_1_deposit_coins)
    -  [Function `batch_transfer_fungible_assets`](#@Specification_1_batch_transfer_fungible_assets)
    -  [Function `transfer_fungible_assets`](#@Specification_1_transfer_fungible_assets)
    -  [Function `deposit_fungible_assets`](#@Specification_1_deposit_fungible_assets)
    -  [Function `assert_account_exists`](#@Specification_1_assert_account_exists)
    -  [Function `assert_account_is_registered_for_apt`](#@Specification_1_assert_account_is_registered_for_apt)
    -  [Function `set_allow_direct_coin_transfers`](#@Specification_1_set_allow_direct_coin_transfers)
    -  [Function `can_receive_direct_coin_transfers`](#@Specification_1_can_receive_direct_coin_transfers)
    -  [Function `register_apt`](#@Specification_1_register_apt)
    -  [Function `fungible_transfer_only`](#@Specification_1_fungible_transfer_only)
    -  [Function `is_fungible_balance_at_least`](#@Specification_1_is_fungible_balance_at_least)
    -  [Function `burn_from_fungible_store_for_gas`](#@Specification_1_burn_from_fungible_store_for_gas)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
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

<a id="0x1_aptos_account_DirectCoinTransferConfigUpdated"></a>

## Struct `DirectCoinTransferConfigUpdated`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdated">DirectCoinTransferConfigUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
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
    <b>let</b> account_signer = <a href="account.md#0x1_account_create_account">account::create_account</a>(auth_key);
    <a href="aptos_account.md#0x1_aptos_account_register_apt">register_apt</a>(&account_signer);
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

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
        <a href="aptos_account.md#0x1_aptos_account_fungible_transfer_only">fungible_transfer_only</a>(source, <b>to</b>, amount)
    } <b>else</b> {
        // Resource accounts can be created without registering them <b>to</b> receive APT.
        // This conveniently does the registration <b>if</b> necessary.
        <b>if</b> (!<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(<b>to</b>)) {
            <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&<a href="create_signer.md#0x1_create_signer">create_signer</a>(<b>to</b>));
        };
        <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(source, <b>to</b>, amount)
    }
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
            // TODO(fa_migration)
            // <b>assert</b> <a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;AptosCoin&gt;(<b>to</b>);
            // <b>assume</b> aptos_std::type_info::type_of&lt;CoinType&gt;() == aptos_std::type_info::type_of&lt;AptosCoin&gt;() ==&gt;
            //     <a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;CoinType&gt;(<b>to</b>);
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

<a id="0x1_aptos_account_batch_transfer_fungible_assets"></a>

## Function `batch_transfer_fungible_assets`

Batch version of transfer_fungible_assets.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_fungible_assets">batch_transfer_fungible_assets</a>(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_fungible_assets">batch_transfer_fungible_assets</a>(
    from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: Object&lt;Metadata&gt;,
    recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) {
    <b>let</b> recipients_len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&recipients);
    <b>assert</b>!(
        recipients_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amounts),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_account.md#0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH">EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH</a>),
    );

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(&recipients, |i, <b>to</b>| {
        <b>let</b> amount = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amounts, i);
        <a href="aptos_account.md#0x1_aptos_account_transfer_fungible_assets">transfer_fungible_assets</a>(from, metadata, *<b>to</b>, amount);
    });
}
</code></pre>



</details>

<a id="0x1_aptos_account_transfer_fungible_assets"></a>

## Function `transfer_fungible_assets`

Convenient function to deposit fungible asset into a recipient account that might not exist.
This would create the recipient account first to receive the fungible assets.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_fungible_assets">transfer_fungible_assets</a>(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_fungible_assets">transfer_fungible_assets</a>(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: Object&lt;Metadata&gt;, <b>to</b>: <b>address</b>, amount: u64) {
    <a href="aptos_account.md#0x1_aptos_account_deposit_fungible_assets">deposit_fungible_assets</a>(<b>to</b>, <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw">primary_fungible_store::withdraw</a>(from, metadata, amount));
}
</code></pre>



</details>

<a id="0x1_aptos_account_deposit_fungible_assets"></a>

## Function `deposit_fungible_assets`

Convenient function to deposit fungible asset into a recipient account that might not exist.
This would create the recipient account first to receive the fungible assets.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_fungible_assets">deposit_fungible_assets</a>(<b>to</b>: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_fungible_assets">deposit_fungible_assets</a>(<b>to</b>: <b>address</b>, fa: FungibleAsset) {
    <b>if</b> (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>)) {
        <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(<b>to</b>);
    };
    <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">primary_fungible_store::deposit</a>(<b>to</b>, fa)
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

        <b>if</b> (std::features::module_event_migration_enabled()) {
            emit(<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdated">DirectCoinTransferConfigUpdated</a> { <a href="account.md#0x1_account">account</a>: addr, new_allow_direct_transfers: allow });
        } <b>else</b> {
            emit_event(
                &<b>mut</b> direct_transfer_config.update_coin_transfer_events,
                <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> { new_allow_direct_transfers: allow });
        };
    } <b>else</b> {
        <b>let</b> direct_transfer_config = <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> {
            allow_arbitrary_coin_transfers: allow,
            update_coin_transfer_events: new_event_handle&lt;<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),
        };
        <b>if</b> (std::features::module_event_migration_enabled()) {
            emit(<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdated">DirectCoinTransferConfigUpdated</a> { <a href="account.md#0x1_account">account</a>: addr, new_allow_direct_transfers: allow });
        } <b>else</b> {
            emit_event(
                &<b>mut</b> direct_transfer_config.update_coin_transfer_events,
                <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> { new_allow_direct_transfers: allow });
        };
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

<a id="0x1_aptos_account_register_apt"></a>

## Function `register_apt`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_register_apt">register_apt</a>(account_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_register_apt">register_apt</a>(account_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_new_accounts_default_to_fa_apt_store_enabled">features::new_accounts_default_to_fa_apt_store_enabled</a>()) {
        <a href="aptos_account.md#0x1_aptos_account_ensure_primary_fungible_store_exists">ensure_primary_fungible_store_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(account_signer));
    } <b>else</b> {
        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(account_signer);
    }
}
</code></pre>



</details>

<a id="0x1_aptos_account_fungible_transfer_only"></a>

## Function `fungible_transfer_only`

APT Primary Fungible Store specific specialized functions,
Utilized internally once migration of APT to FungibleAsset is complete.
Convenient function to transfer APT to a recipient account that might not exist.
This would create the recipient APT PFS first, which also registers it to receive APT, before transferring.
TODO: once migration is complete, rename to just "transfer_only" and make it an entry function (for cheapest way
to transfer APT) - if we want to allow APT PFS without account itself


<pre><code><b>public</b>(<b>friend</b>) entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_fungible_transfer_only">fungible_transfer_only</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_fungible_transfer_only">fungible_transfer_only</a>(
    source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64
) {
    <b>let</b> sender_store = <a href="aptos_account.md#0x1_aptos_account_ensure_primary_fungible_store_exists">ensure_primary_fungible_store_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source));
    <b>let</b> recipient_store = <a href="aptos_account.md#0x1_aptos_account_ensure_primary_fungible_store_exists">ensure_primary_fungible_store_exists</a>(<b>to</b>);

    // <b>use</b> <b>internal</b> APIs, <b>as</b> they skip:
    // - owner, frozen and dispatchable checks
    // <b>as</b> APT cannot be frozen or have dispatch, and PFS cannot be transferred
    // (PFS could potentially be burned. regular transfer would permanently unburn the store.
    // Ignoring the check here <b>has</b> the equivalent of unburning, transferring, and then burning again)
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_permission_check_by_address">fungible_asset::withdraw_permission_check_by_address</a>(source, sender_store, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_unchecked_deposit">fungible_asset::unchecked_deposit</a>(recipient_store, <a href="fungible_asset.md#0x1_fungible_asset_unchecked_withdraw">fungible_asset::unchecked_withdraw</a>(sender_store, amount));
}
</code></pre>



</details>

<a id="0x1_aptos_account_is_fungible_balance_at_least"></a>

## Function `is_fungible_balance_at_least`

Is balance from APT Primary FungibleStore at least the given amount


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">is_fungible_balance_at_least</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">is_fungible_balance_at_least</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): bool {
    <b>let</b> store_addr = <a href="aptos_account.md#0x1_aptos_account_primary_fungible_store_address">primary_fungible_store_address</a>(<a href="account.md#0x1_account">account</a>);
    <a href="fungible_asset.md#0x1_fungible_asset_is_address_balance_at_least">fungible_asset::is_address_balance_at_least</a>(store_addr, amount)
}
</code></pre>



</details>

<a id="0x1_aptos_account_burn_from_fungible_store_for_gas"></a>

## Function `burn_from_fungible_store_for_gas`

Burn from APT Primary FungibleStore for gas charge


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_burn_from_fungible_store_for_gas">burn_from_fungible_store_for_gas</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_burn_from_fungible_store_for_gas">burn_from_fungible_store_for_gas</a>(
    ref: &BurnRef,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    amount: u64,
) {
    // Skip burning <b>if</b> amount is zero. This shouldn't <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> out <b>as</b> it's called <b>as</b> part of transaction fee burning.
    <b>if</b> (amount != 0) {
        <b>let</b> store_addr = <a href="aptos_account.md#0x1_aptos_account_primary_fungible_store_address">primary_fungible_store_address</a>(<a href="account.md#0x1_account">account</a>);
        <a href="fungible_asset.md#0x1_fungible_asset_address_burn_from_for_gas">fungible_asset::address_burn_from_for_gas</a>(ref, store_addr, amount);
    };
}
</code></pre>



</details>

<a id="0x1_aptos_account_ensure_primary_fungible_store_exists"></a>

## Function `ensure_primary_fungible_store_exists`

Ensure that APT Primary FungibleStore exists (and create if it doesn't)


<pre><code><b>fun</b> <a href="aptos_account.md#0x1_aptos_account_ensure_primary_fungible_store_exists">ensure_primary_fungible_store_exists</a>(owner: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_ensure_primary_fungible_store_exists">ensure_primary_fungible_store_exists</a>(owner: <b>address</b>): <b>address</b> {
    <b>let</b> store_addr = <a href="aptos_account.md#0x1_aptos_account_primary_fungible_store_address">primary_fungible_store_address</a>(owner);
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(store_addr)) {
        store_addr
    } <b>else</b> {
        <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store">primary_fungible_store::create_primary_store</a>(owner, <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;Metadata&gt;(@aptos_fungible_asset)))
    }
}
</code></pre>



</details>

<a id="0x1_aptos_account_primary_fungible_store_address"></a>

## Function `primary_fungible_store_address`

Address of APT Primary Fungible Store


<pre><code><b>fun</b> <a href="aptos_account.md#0x1_aptos_account_primary_fungible_store_address">primary_fungible_store_address</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_primary_fungible_store_address">primary_fungible_store_address</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): <b>address</b> {
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(<a href="account.md#0x1_account">account</a>, @aptos_fungible_asset)
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
<b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a>;
</code></pre>




<a id="0x1_aptos_account_CreateAccountAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> {
    auth_key: <b>address</b>;
    <b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(auth_key);
    <b>aborts_if</b> auth_key == @vm_reserved || auth_key == @aptos_framework || auth_key == @aptos_token;
}
</code></pre>




<a id="0x1_aptos_account_length_judgment"></a>


<pre><code><b>fun</b> <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(auth_key: <b>address</b>): bool {
   <b>use</b> std::bcs;

   <b>let</b> authentication_key = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(auth_key);
   len(authentication_key) != 32
}
</code></pre>



<a id="@Specification_1_batch_transfer"></a>

### Function `batch_transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer">batch_transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);
<b>let</b> coin_store_source = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source);
<b>let</b> balance_source = coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;
<b>aborts_if</b> len(recipients) != len(amounts);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i]);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @aptos_framework || recipients[i] == @aptos_token);
<b>ensures</b> <b>forall</b> i in 0..len(recipients):
        (!<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) ==&gt; !<a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i])) &&
            (!<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) ==&gt; (recipients[i] != @vm_reserved && recipients[i] != @aptos_framework && recipients[i] != @aptos_token));
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    coin_store_source.frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source).<a href="coin.md#0x1_coin">coin</a>.value &lt; amounts[i];
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]).frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);
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
// This enforces <a id="high-level-req-7" href="#high-level-req">high-level requirement 7</a>:
<b>aborts_if</b> len(recipients) != len(amounts);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i]);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
        !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @aptos_framework || recipients[i] == @aptos_token);
<b>ensures</b> <b>forall</b> i in 0..len(recipients):
        (!<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) ==&gt; !<a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i])) &&
            (!<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) ==&gt; (recipients[i] != @vm_reserved && recipients[i] != @aptos_framework && recipients[i] != @aptos_token));
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    coin_store_source.frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source).<a href="coin.md#0x1_coin">coin</a>.value &lt; amounts[i];
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]).frozen;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>aborts_if</b> <b>exists</b> i in 0..len(recipients):
    <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(recipients[i]) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) && <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num + 2 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_transfer_coins"></a>

### Function `transfer_coins`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> account_addr_source = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;CoinType&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt;;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_TransferEnsures">TransferEnsures</a>&lt;CoinType&gt;;
<b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) && <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).frozen;
<b>ensures</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);
<b>ensures</b> <b>exists</b>&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(<b>to</b>);
</code></pre>



<a id="@Specification_1_deposit_coins"></a>

### Function `deposit_coins`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>&lt;CoinType&gt;(<b>to</b>: <b>address</b>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;
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



<a id="@Specification_1_batch_transfer_fungible_assets"></a>

### Function `batch_transfer_fungible_assets`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_fungible_assets">batch_transfer_fungible_assets</a>(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_transfer_fungible_assets"></a>

### Function `transfer_fungible_assets`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_fungible_assets">transfer_fungible_assets</a>(from: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_deposit_fungible_assets"></a>

### Function `deposit_fungible_assets`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_fungible_assets">deposit_fungible_assets</a>(<b>to</b>: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_assert_account_exists"></a>

### Function `assert_account_exists`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(addr);
</code></pre>



<a id="@Specification_1_assert_account_is_registered_for_apt"></a>

### Function `assert_account_is_registered_for_apt`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>)
</code></pre>


Check if the address existed.
Check if the AptosCoin under the address existed.


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(addr);
</code></pre>



<a id="@Specification_1_set_allow_direct_coin_transfers"></a>

### Function `set_allow_direct_coin_transfers`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_set_allow_direct_coin_transfers">set_allow_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allow: bool)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
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



<a id="@Specification_1_register_apt"></a>

### Function `register_apt`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_register_apt">register_apt</a>(account_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_fungible_transfer_only"></a>

### Function `fungible_transfer_only`


<pre><code><b>public</b>(<b>friend</b>) entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_fungible_transfer_only">fungible_transfer_only</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_is_fungible_balance_at_least"></a>

### Function `is_fungible_balance_at_least`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_is_fungible_balance_at_least">is_fungible_balance_at_least</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): bool
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_burn_from_fungible_store_for_gas"></a>

### Function `burn_from_fungible_store_for_gas`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_burn_from_fungible_store_for_gas">burn_from_fungible_store_for_gas</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_aptos_account_CreateAccountTransferAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a> {
    <b>to</b>: <b>address</b>;
    <b>aborts_if</b> !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(<b>to</b>) && <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(<b>to</b>);
    <b>aborts_if</b> !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(<b>to</b>) && (<b>to</b> == @vm_reserved || <b>to</b> == @aptos_framework || <b>to</b> == @aptos_token);
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
    <b>aborts_if</b> <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(<b>to</b>) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) && acc.guid_creation_num + 2 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
    <b>aborts_if</b> <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(<b>to</b>) && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) && acc.guid_creation_num + 2 &gt; MAX_U64;
}
</code></pre>




<a id="0x1_aptos_account_RegistCoinAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt; {
    <b>to</b>: <b>address</b>;
    <b>aborts_if</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);
    <b>aborts_if</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;() != <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;();
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


[move-book]: https://aptos.dev/move/book/SUMMARY
