
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /></code></pre>



<a id="0x1_aptos_account_DirectTransferConfig"></a>

## Resource `DirectTransferConfig`

Configuration for whether an account can receive direct transfers of coins that they have not registered.

By default, this is enabled. Users can opt&#45;out by disabling at any time.


<pre><code><b>struct</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> <b>has</b> key<br /></code></pre>



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

Event emitted when an account&apos;s direct coins transfer config is updated.


<pre><code><b>struct</b> <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdated">DirectCoinTransferConfigUpdated</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS">EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS"></a>

Account opted out of directly receiving NFT tokens.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS">EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_FOUND"></a>

Account does not exist.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_FOUND">EACCOUNT_NOT_FOUND</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT"></a>

Account is not registered to receive APT.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT">EACCOUNT_NOT_REGISTERED_FOR_APT</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH"></a>

The lengths of the recipients and amounts lists don&apos;t match.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH">EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_aptos_account_create_account"></a>

## Function `create_account`

Basic account creation methods.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>) &#123;<br />    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#61; <a href="account.md#0x1_account_create_account">account::create_account</a>(auth_key);<br />    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer"></a>

## Function `batch_transfer`

Batch version of APT transfer.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer">batch_transfer</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer">batch_transfer</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) &#123;<br />    <b>let</b> recipients_len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;recipients);<br />    <b>assert</b>!(<br />        recipients_len &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;amounts),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_account.md#0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH">EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH</a>),<br />    );<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(&amp;recipients, &#124;i, <b>to</b>&#124; &#123;<br />        <b>let</b> amount &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;amounts, i);<br />        <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source, &#42;<b>to</b>, amount);<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_transfer"></a>

## Function `transfer`

Convenient function to transfer APT to a recipient account that might not exist.
This would create the recipient account first, which also registers it to receive APT, before transferring.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64) &#123;<br />    <b>if</b> (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>)) &#123;<br />        <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(<b>to</b>)<br />    &#125;;<br />    // Resource accounts can be created without registering them <b>to</b> receive APT.<br />    // This conveniently does the registration <b>if</b> necessary.<br />    <b>if</b> (!<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(<b>to</b>)) &#123;<br />        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&amp;<a href="create_signer.md#0x1_create_signer">create_signer</a>(<b>to</b>));<br />    &#125;;<br />    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(source, <b>to</b>, amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer_coins"></a>

## Function `batch_transfer_coins`

Batch version of transfer_coins.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_coins">batch_transfer_coins</a>&lt;CoinType&gt;(from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_coins">batch_transfer_coins</a>&lt;CoinType&gt;(<br />    from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> &#123;<br />    <b>let</b> recipients_len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;recipients);<br />    <b>assert</b>!(<br />        recipients_len &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;amounts),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_account.md#0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH">EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH</a>),<br />    );<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(&amp;recipients, &#124;i, <b>to</b>&#124; &#123;<br />        <b>let</b> amount &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;amounts, i);<br />        <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from, &#42;<b>to</b>, amount);<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_transfer_coins"></a>

## Function `transfer_coins`

Convenient function to transfer a custom CoinType to a recipient account that might not exist.
This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> &#123;<br />    <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>(<b>to</b>, <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;CoinType&gt;(from, amount));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_deposit_coins"></a>

## Function `deposit_coins`

Convenient function to deposit a custom CoinType into a recipient account that might not exist.
This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>&lt;CoinType&gt;(<b>to</b>: <b>address</b>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>&lt;CoinType&gt;(<b>to</b>: <b>address</b>, coins: Coin&lt;CoinType&gt;) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> &#123;<br />    <b>if</b> (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>)) &#123;<br />        <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(<b>to</b>);<br />        <b>spec</b> &#123;<br />            <b>assert</b> <a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;AptosCoin&gt;(<b>to</b>);<br />            <b>assume</b> aptos_std::type_info::type_of&lt;CoinType&gt;() &#61;&#61; aptos_std::type_info::type_of&lt;AptosCoin&gt;() &#61;&#61;&gt;<br />                <a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;CoinType&gt;(<b>to</b>);<br />        &#125;;<br />    &#125;;<br />    <b>if</b> (!<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(<b>to</b>)) &#123;<br />        <b>assert</b>!(<br />            <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<b>to</b>),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS">EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS</a>),<br />        );<br />        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;CoinType&gt;(&amp;<a href="create_signer.md#0x1_create_signer">create_signer</a>(<b>to</b>));<br />    &#125;;<br />    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>&lt;CoinType&gt;(<b>to</b>, coins)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_assert_account_exists"></a>

## Function `assert_account_exists`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>) &#123;<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_FOUND">EACCOUNT_NOT_FOUND</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_assert_account_is_registered_for_apt"></a>

## Function `assert_account_is_registered_for_apt`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>) &#123;<br />    <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr);<br />    <b>assert</b>!(<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT">EACCOUNT_NOT_REGISTERED_FOR_APT</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_set_allow_direct_coin_transfers"></a>

## Function `set_allow_direct_coin_transfers`

Set whether <code><a href="account.md#0x1_account">account</a></code> can receive direct transfers of coins that they have not explicitly registered to receive.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_set_allow_direct_coin_transfers">set_allow_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allow: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_set_allow_direct_coin_transfers">set_allow_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allow: bool) <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>if</b> (<b>exists</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(addr)) &#123;<br />        <b>let</b> direct_transfer_config &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(addr);<br />        // Short&#45;circuit <b>to</b> avoid emitting an <a href="event.md#0x1_event">event</a> <b>if</b> direct transfer config is not changing.<br />        <b>if</b> (direct_transfer_config.allow_arbitrary_coin_transfers &#61;&#61; allow) &#123;<br />            <b>return</b><br />        &#125;;<br /><br />        direct_transfer_config.allow_arbitrary_coin_transfers &#61; allow;<br /><br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            emit(<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdated">DirectCoinTransferConfigUpdated</a> &#123; <a href="account.md#0x1_account">account</a>: addr, new_allow_direct_transfers: allow &#125;);<br />        &#125;;<br />        emit_event(<br />            &amp;<b>mut</b> direct_transfer_config.update_coin_transfer_events,<br />            <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> &#123; new_allow_direct_transfers: allow &#125;);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> direct_transfer_config &#61; <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> &#123;<br />            allow_arbitrary_coin_transfers: allow,<br />            update_coin_transfer_events: new_event_handle&lt;<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),<br />        &#125;;<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            emit(<a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdated">DirectCoinTransferConfigUpdated</a> &#123; <a href="account.md#0x1_account">account</a>: addr, new_allow_direct_transfers: allow &#125;);<br />        &#125;;<br />        emit_event(<br />            &amp;<b>mut</b> direct_transfer_config.update_coin_transfer_events,<br />            <a href="aptos_account.md#0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent">DirectCoinTransferConfigUpdatedEvent</a> &#123; new_allow_direct_transfers: allow &#125;);<br />        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, direct_transfer_config);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_account_can_receive_direct_coin_transfers"></a>

## Function `can_receive_direct_coin_transfers`

Return true if <code><a href="account.md#0x1_account">account</a></code> can receive direct transfers of coins that they have not explicitly registered to
receive.

By default, this returns true if an account has not explicitly set whether the can receive direct transfers.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool <b>acquires</b> <a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a> &#123;<br />    !<b>exists</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>) &#124;&#124;<br />        <b>borrow_global</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>).allow_arbitrary_coin_transfers<br />&#125;<br /></code></pre>



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


<pre><code><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>)<br /></code></pre>


Check if the bytes of the auth_key is 32.
The Account does not exist under the auth_key before creating the account.
Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a>;<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(auth_key);<br /></code></pre>




<a id="0x1_aptos_account_CreateAccountAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> &#123;<br />auth_key: <b>address</b>;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(auth_key);<br /><b>aborts_if</b> <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(auth_key);<br /><b>aborts_if</b> auth_key &#61;&#61; @vm_reserved &#124;&#124; auth_key &#61;&#61; @aptos_framework &#124;&#124; auth_key &#61;&#61; @aptos_token;<br />&#125;<br /></code></pre>




<a id="0x1_aptos_account_length_judgment"></a>


<pre><code><b>fun</b> <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(auth_key: <b>address</b>): bool &#123;<br />   <b>use</b> std::bcs;<br /><br />   <b>let</b> authentication_key &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(auth_key);<br />   len(authentication_key) !&#61; 32<br />&#125;<br /></code></pre>



<a id="@Specification_1_batch_transfer"></a>

### Function `batch_transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer">batch_transfer</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr_source &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);<br /><b>let</b> coin_store_source &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source);<br /><b>let</b> balance_source &#61; coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>requires</b> <b>forall</b> i in 0..len(recipients):<br />    recipients[i] !&#61; account_addr_source;<br /><b>requires</b> <b>exists</b> i in 0..len(recipients):<br />    amounts[i] &gt; 0;<br /><b>aborts_if</b> len(recipients) !&#61; len(amounts);<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i]);<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; (recipients[i] &#61;&#61; @vm_reserved &#124;&#124; recipients[i] &#61;&#61; @aptos_framework &#124;&#124; recipients[i] &#61;&#61; @aptos_token);<br /><b>ensures</b> <b>forall</b> i in 0..len(recipients):<br />        (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &#61;&#61;&gt; !<a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i])) &amp;&amp;<br />            (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &#61;&#61;&gt; (recipients[i] !&#61; @vm_reserved &amp;&amp; recipients[i] !&#61; @aptos_framework &amp;&amp; recipients[i] !&#61; @aptos_token));<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source);<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    coin_store_source.frozen;<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(account_addr_source).<a href="coin.md#0x1_coin">coin</a>.value &lt; amounts[i];<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]).frozen;<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num &#43; 2 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num &#43; 2 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr_source &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);<br /><b>requires</b> account_addr_source !&#61; <b>to</b>;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;AptosCoin&gt;;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;AptosCoin&gt;&#123;from: source&#125;;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_TransferEnsures">TransferEnsures</a>&lt;AptosCoin&gt;;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(<b>to</b>) &amp;&amp; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(<b>to</b>).frozen;<br />// This enforces <a id="high-level-req-5" href="#high-level-req">high&#45;level requirement 5</a>:
<b>ensures</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);<br /><b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(<b>to</b>);<br /></code></pre>



<a id="@Specification_1_batch_transfer_coins"></a>

### Function `batch_transfer_coins`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_batch_transfer_coins">batch_transfer_coins</a>&lt;CoinType&gt;(from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipients: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr_source &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);<br /><b>let</b> coin_store_source &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);<br /><b>let</b> balance_source &#61; coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>requires</b> <b>forall</b> i in 0..len(recipients):<br />    recipients[i] !&#61; account_addr_source;<br /><b>requires</b> <b>exists</b> i in 0..len(recipients):<br />    amounts[i] &gt; 0;<br />// This enforces <a id="high-level-req-7" href="#high-level-req">high&#45;level requirement 7</a>:
<b>aborts_if</b> len(recipients) !&#61; len(amounts);<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i]);<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />        !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; (recipients[i] &#61;&#61; @vm_reserved &#124;&#124; recipients[i] &#61;&#61; @aptos_framework &#124;&#124; recipients[i] &#61;&#61; @aptos_token);<br /><b>ensures</b> <b>forall</b> i in 0..len(recipients):<br />        (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &#61;&#61;&gt; !<a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(recipients[i])) &amp;&amp;<br />            (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &#61;&#61;&gt; (recipients[i] !&#61; @vm_reserved &amp;&amp; recipients[i] !&#61; @aptos_framework &amp;&amp; recipients[i] !&#61; @aptos_token));<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    coin_store_source.frozen;<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source).<a href="coin.md#0x1_coin">coin</a>.value &lt; amounts[i];<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]).frozen;<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num &#43; 2 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    <a href="account.md#0x1_account_exists_at">account::exists_at</a>(recipients[i]) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(recipients[i]).guid_creation_num &#43; 2 &gt; MAX_U64;<br /><b>aborts_if</b> <b>exists</b> i in 0..len(recipients):<br />    !<a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;CoinType&gt;(recipients[i]) &amp;&amp; !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;CoinType&gt;();<br /></code></pre>



<a id="@Specification_1_transfer_coins"></a>

### Function `transfer_coins`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer_coins">transfer_coins</a>&lt;CoinType&gt;(from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr_source &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);<br /><b>requires</b> account_addr_source !&#61; <b>to</b>;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt;;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;CoinType&gt;;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt;;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_TransferEnsures">TransferEnsures</a>&lt;CoinType&gt;;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) &amp;&amp; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).frozen;<br /><b>ensures</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);<br /><b>ensures</b> <b>exists</b>&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(<b>to</b>);<br /></code></pre>




<a id="0x1_aptos_account_CreateAccountTransferAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a> &#123;<br /><b>to</b>: <b>address</b>;<br /><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) &amp;&amp; <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(<b>to</b>);<br /><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) &amp;&amp; (<b>to</b> &#61;&#61; @vm_reserved &#124;&#124; <b>to</b> &#61;&#61; @aptos_framework &#124;&#124; <b>to</b> &#61;&#61; @aptos_token);<br />&#125;<br /></code></pre>




<a id="0x1_aptos_account_WithdrawAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">WithdrawAbortsIf</a>&lt;CoinType&gt; &#123;<br />from: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />amount: u64;<br /><b>let</b> account_addr_source &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);<br /><b>let</b> coin_store_source &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);<br /><b>let</b> balance_source &#61; coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);<br /><b>aborts_if</b> coin_store_source.frozen;<br /><b>aborts_if</b> balance_source &lt; amount;<br />&#125;<br /></code></pre>




<a id="0x1_aptos_account_GuidAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;CoinType&gt; &#123;<br /><b>to</b>: <b>address</b>;<br /><b>let</b> acc &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(<b>to</b>);<br /><b>aborts_if</b> <a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> <a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>) &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;<br />&#125;<br /></code></pre>




<a id="0x1_aptos_account_RegistCoinAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt; &#123;<br /><b>to</b>: <b>address</b>;<br /><b>aborts_if</b> !<a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;CoinType&gt;(<b>to</b>) &amp;&amp; !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;CoinType&gt;();<br /><b>aborts_if</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;() !&#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;AptosCoin&gt;();<br />&#125;<br /></code></pre>




<a id="0x1_aptos_account_TransferEnsures"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_TransferEnsures">TransferEnsures</a>&lt;CoinType&gt; &#123;<br /><b>to</b>: <b>address</b>;<br />account_addr_source: <b>address</b>;<br />amount: u64;<br /><b>let</b> if_exist_account &#61; <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(<b>to</b>);<br /><b>let</b> if_exist_coin &#61; <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);<br /><b>let</b> coin_store_to &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);<br /><b>let</b> coin_store_source &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);<br /><b>let</b> <b>post</b> p_coin_store_to &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);<br /><b>let</b> <b>post</b> p_coin_store_source &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr_source);<br /><b>ensures</b> coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value &#45; amount &#61;&#61; p_coin_store_source.<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>ensures</b> if_exist_account &amp;&amp; if_exist_coin &#61;&#61;&gt; coin_store_to.<a href="coin.md#0x1_coin">coin</a>.value &#43; amount &#61;&#61; p_coin_store_to.<a href="coin.md#0x1_coin">coin</a>.value;<br />&#125;<br /></code></pre>



<a id="@Specification_1_deposit_coins"></a>

### Function `deposit_coins`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_deposit_coins">deposit_coins</a>&lt;CoinType&gt;(<b>to</b>: <b>address</b>, coins: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccountTransferAbortsIf">CreateAccountTransferAbortsIf</a>;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">GuidAbortsIf</a>&lt;CoinType&gt;;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_RegistCoinAbortsIf">RegistCoinAbortsIf</a>&lt;CoinType&gt;;<br /><b>let</b> if_exist_coin &#61; <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>);<br /><b>aborts_if</b> if_exist_coin &amp;&amp; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).frozen;<br />// This enforces <a id="high-level-spec-6" href="#high-level-req">high&#45;level requirement 6</a>:
<b>ensures</b> <b>exists</b>&lt;aptos_framework::account::Account&gt;(<b>to</b>);<br /><b>ensures</b> <b>exists</b>&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(<b>to</b>);<br /><b>let</b> coin_store_to &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>let</b> <b>post</b> post_coin_store_to &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(<b>to</b>).<a href="coin.md#0x1_coin">coin</a>.value;<br /><b>ensures</b> if_exist_coin &#61;&#61;&gt; post_coin_store_to &#61;&#61; coin_store_to &#43; coins.value;<br /></code></pre>



<a id="@Specification_1_assert_account_exists"></a>

### Function `assert_account_exists`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr);<br /></code></pre>



<a id="@Specification_1_assert_account_is_registered_for_apt"></a>

### Function `assert_account_is_registered_for_apt`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>)<br /></code></pre>


Check if the address existed.
Check if the AptosCoin under the address existed.


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr);<br /><b>aborts_if</b> !<a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;AptosCoin&gt;(addr);<br /></code></pre>



<a id="@Specification_1_set_allow_direct_coin_transfers"></a>

### Function `set_allow_direct_coin_transfers`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_set_allow_direct_coin_transfers">set_allow_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allow: bool)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_can_receive_direct_coin_transfers"></a>

### Function `can_receive_direct_coin_transfers`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_can_receive_direct_coin_transfers">can_receive_direct_coin_transfers</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> result &#61;&#61; (<br />    !<b>exists</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>) &#124;&#124;<br />        <b>global</b>&lt;<a href="aptos_account.md#0x1_aptos_account_DirectTransferConfig">DirectTransferConfig</a>&gt;(<a href="account.md#0x1_account">account</a>).allow_arbitrary_coin_transfers<br />);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
