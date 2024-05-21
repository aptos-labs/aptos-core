
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


<pre><code>use 0x1::account;
use 0x1::aptos_coin;
use 0x1::coin;
use 0x1::create_signer;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::signer;
</code></pre>



<a id="0x1_aptos_account_DirectTransferConfig"></a>

## Resource `DirectTransferConfig`

Configuration for whether an account can receive direct transfers of coins that they have not registered.

By default, this is enabled. Users can opt-out by disabling at any time.


<pre><code>struct DirectTransferConfig has key
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
<code>update_coin_transfer_events: event::EventHandle&lt;aptos_account::DirectCoinTransferConfigUpdatedEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_account_DirectCoinTransferConfigUpdatedEvent"></a>

## Struct `DirectCoinTransferConfigUpdatedEvent`

Event emitted when an account's direct coins transfer config is updated.


<pre><code>struct DirectCoinTransferConfigUpdatedEvent has drop, store
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



<pre><code>&#35;[event]
struct DirectCoinTransferConfigUpdated has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
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


<pre><code>const EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS: u64 &#61; 3;
</code></pre>



<a id="0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS"></a>

Account opted out of directly receiving NFT tokens.


<pre><code>const EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS: u64 &#61; 4;
</code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_FOUND"></a>

Account does not exist.


<pre><code>const EACCOUNT_NOT_FOUND: u64 &#61; 1;
</code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT"></a>

Account is not registered to receive APT.


<pre><code>const EACCOUNT_NOT_REGISTERED_FOR_APT: u64 &#61; 2;
</code></pre>



<a id="0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH"></a>

The lengths of the recipients and amounts lists don't match.


<pre><code>const EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH: u64 &#61; 5;
</code></pre>



<a id="0x1_aptos_account_create_account"></a>

## Function `create_account`

Basic account creation methods.


<pre><code>public entry fun create_account(auth_key: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_account(auth_key: address) &#123;
    let signer &#61; account::create_account(auth_key);
    coin::register&lt;AptosCoin&gt;(&amp;signer);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer"></a>

## Function `batch_transfer`

Batch version of APT transfer.


<pre><code>public entry fun batch_transfer(source: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun batch_transfer(source: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;) &#123;
    let recipients_len &#61; vector::length(&amp;recipients);
    assert!(
        recipients_len &#61;&#61; vector::length(&amp;amounts),
        error::invalid_argument(EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH),
    );

    vector::enumerate_ref(&amp;recipients, &#124;i, to&#124; &#123;
        let amount &#61; &#42;vector::borrow(&amp;amounts, i);
        transfer(source, &#42;to, amount);
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_transfer"></a>

## Function `transfer`

Convenient function to transfer APT to a recipient account that might not exist.
This would create the recipient account first, which also registers it to receive APT, before transferring.


<pre><code>public entry fun transfer(source: &amp;signer, to: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer(source: &amp;signer, to: address, amount: u64) &#123;
    if (!account::exists_at(to)) &#123;
        create_account(to)
    &#125;;
    // Resource accounts can be created without registering them to receive APT.
    // This conveniently does the registration if necessary.
    if (!coin::is_account_registered&lt;AptosCoin&gt;(to)) &#123;
        coin::register&lt;AptosCoin&gt;(&amp;create_signer(to));
    &#125;;
    coin::transfer&lt;AptosCoin&gt;(source, to, amount)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer_coins"></a>

## Function `batch_transfer_coins`

Batch version of transfer_coins.


<pre><code>public entry fun batch_transfer_coins&lt;CoinType&gt;(from: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun batch_transfer_coins&lt;CoinType&gt;(
    from: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;) acquires DirectTransferConfig &#123;
    let recipients_len &#61; vector::length(&amp;recipients);
    assert!(
        recipients_len &#61;&#61; vector::length(&amp;amounts),
        error::invalid_argument(EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH),
    );

    vector::enumerate_ref(&amp;recipients, &#124;i, to&#124; &#123;
        let amount &#61; &#42;vector::borrow(&amp;amounts, i);
        transfer_coins&lt;CoinType&gt;(from, &#42;to, amount);
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_transfer_coins"></a>

## Function `transfer_coins`

Convenient function to transfer a custom CoinType to a recipient account that might not exist.
This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code>public entry fun transfer_coins&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_coins&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64) acquires DirectTransferConfig &#123;
    deposit_coins(to, coin::withdraw&lt;CoinType&gt;(from, amount));
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_deposit_coins"></a>

## Function `deposit_coins`

Convenient function to deposit a custom CoinType into a recipient account that might not exist.
This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code>public fun deposit_coins&lt;CoinType&gt;(to: address, coins: coin::Coin&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_coins&lt;CoinType&gt;(to: address, coins: Coin&lt;CoinType&gt;) acquires DirectTransferConfig &#123;
    if (!account::exists_at(to)) &#123;
        create_account(to);
        spec &#123;
            assert coin::spec_is_account_registered&lt;AptosCoin&gt;(to);
            assume aptos_std::type_info::type_of&lt;CoinType&gt;() &#61;&#61; aptos_std::type_info::type_of&lt;AptosCoin&gt;() &#61;&#61;&gt;
                coin::spec_is_account_registered&lt;CoinType&gt;(to);
        &#125;;
    &#125;;
    if (!coin::is_account_registered&lt;CoinType&gt;(to)) &#123;
        assert!(
            can_receive_direct_coin_transfers(to),
            error::permission_denied(EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS),
        );
        coin::register&lt;CoinType&gt;(&amp;create_signer(to));
    &#125;;
    coin::deposit&lt;CoinType&gt;(to, coins)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_assert_account_exists"></a>

## Function `assert_account_exists`



<pre><code>public fun assert_account_exists(addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_account_exists(addr: address) &#123;
    assert!(account::exists_at(addr), error::not_found(EACCOUNT_NOT_FOUND));
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_assert_account_is_registered_for_apt"></a>

## Function `assert_account_is_registered_for_apt`



<pre><code>public fun assert_account_is_registered_for_apt(addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_account_is_registered_for_apt(addr: address) &#123;
    assert_account_exists(addr);
    assert!(coin::is_account_registered&lt;AptosCoin&gt;(addr), error::not_found(EACCOUNT_NOT_REGISTERED_FOR_APT));
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_set_allow_direct_coin_transfers"></a>

## Function `set_allow_direct_coin_transfers`

Set whether <code>account</code> can receive direct transfers of coins that they have not explicitly registered to receive.


<pre><code>public entry fun set_allow_direct_coin_transfers(account: &amp;signer, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_allow_direct_coin_transfers(account: &amp;signer, allow: bool) acquires DirectTransferConfig &#123;
    let addr &#61; signer::address_of(account);
    if (exists&lt;DirectTransferConfig&gt;(addr)) &#123;
        let direct_transfer_config &#61; borrow_global_mut&lt;DirectTransferConfig&gt;(addr);
        // Short&#45;circuit to avoid emitting an event if direct transfer config is not changing.
        if (direct_transfer_config.allow_arbitrary_coin_transfers &#61;&#61; allow) &#123;
            return
        &#125;;

        direct_transfer_config.allow_arbitrary_coin_transfers &#61; allow;

        if (std::features::module_event_migration_enabled()) &#123;
            emit(DirectCoinTransferConfigUpdated &#123; account: addr, new_allow_direct_transfers: allow &#125;);
        &#125;;
        emit_event(
            &amp;mut direct_transfer_config.update_coin_transfer_events,
            DirectCoinTransferConfigUpdatedEvent &#123; new_allow_direct_transfers: allow &#125;);
    &#125; else &#123;
        let direct_transfer_config &#61; DirectTransferConfig &#123;
            allow_arbitrary_coin_transfers: allow,
            update_coin_transfer_events: new_event_handle&lt;DirectCoinTransferConfigUpdatedEvent&gt;(account),
        &#125;;
        if (std::features::module_event_migration_enabled()) &#123;
            emit(DirectCoinTransferConfigUpdated &#123; account: addr, new_allow_direct_transfers: allow &#125;);
        &#125;;
        emit_event(
            &amp;mut direct_transfer_config.update_coin_transfer_events,
            DirectCoinTransferConfigUpdatedEvent &#123; new_allow_direct_transfers: allow &#125;);
        move_to(account, direct_transfer_config);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_aptos_account_can_receive_direct_coin_transfers"></a>

## Function `can_receive_direct_coin_transfers`

Return true if <code>account</code> can receive direct transfers of coins that they have not explicitly registered to
receive.

By default, this returns true if an account has not explicitly set whether the can receive direct transfers.


<pre><code>&#35;[view]
public fun can_receive_direct_coin_transfers(account: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_receive_direct_coin_transfers(account: address): bool acquires DirectTransferConfig &#123;
    !exists&lt;DirectTransferConfig&gt;(account) &#124;&#124;
        borrow_global&lt;DirectTransferConfig&gt;(account).allow_arbitrary_coin_transfers
&#125;
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


<pre><code>pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code>public entry fun create_account(auth_key: address)
</code></pre>


Check if the bytes of the auth_key is 32.
The Account does not exist under the auth_key before creating the account.
Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
pragma aborts_if_is_partial;
include CreateAccountAbortsIf;
ensures exists&lt;account::Account&gt;(auth_key);
</code></pre>




<a id="0x1_aptos_account_CreateAccountAbortsIf"></a>


<pre><code>schema CreateAccountAbortsIf &#123;
    auth_key: address;
    aborts_if exists&lt;account::Account&gt;(auth_key);
    aborts_if length_judgment(auth_key);
    aborts_if auth_key &#61;&#61; @vm_reserved &#124;&#124; auth_key &#61;&#61; @aptos_framework &#124;&#124; auth_key &#61;&#61; @aptos_token;
&#125;
</code></pre>




<a id="0x1_aptos_account_length_judgment"></a>


<pre><code>fun length_judgment(auth_key: address): bool &#123;
   use std::bcs;

   let authentication_key &#61; bcs::to_bytes(auth_key);
   len(authentication_key) !&#61; 32
&#125;
</code></pre>



<a id="@Specification_1_batch_transfer"></a>

### Function `batch_transfer`


<pre><code>public entry fun batch_transfer(source: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
let account_addr_source &#61; signer::address_of(source);
let coin_store_source &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account_addr_source);
let balance_source &#61; coin_store_source.coin.value;
requires forall i in 0..len(recipients):
    recipients[i] !&#61; account_addr_source;
requires exists i in 0..len(recipients):
    amounts[i] &gt; 0;
aborts_if len(recipients) !&#61; len(amounts);
aborts_if exists i in 0..len(recipients):
        !account::exists_at(recipients[i]) &amp;&amp; length_judgment(recipients[i]);
aborts_if exists i in 0..len(recipients):
        !account::exists_at(recipients[i]) &amp;&amp; (recipients[i] &#61;&#61; @vm_reserved &#124;&#124; recipients[i] &#61;&#61; @aptos_framework &#124;&#124; recipients[i] &#61;&#61; @aptos_token);
ensures forall i in 0..len(recipients):
        (!account::exists_at(recipients[i]) &#61;&#61;&gt; !length_judgment(recipients[i])) &amp;&amp;
            (!account::exists_at(recipients[i]) &#61;&#61;&gt; (recipients[i] !&#61; @vm_reserved &amp;&amp; recipients[i] !&#61; @aptos_framework &amp;&amp; recipients[i] !&#61; @aptos_token));
aborts_if exists i in 0..len(recipients):
    !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account_addr_source);
aborts_if exists i in 0..len(recipients):
    coin_store_source.frozen;
aborts_if exists i in 0..len(recipients):
    global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account_addr_source).coin.value &lt; amounts[i];
aborts_if exists i in 0..len(recipients):
    exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]).frozen;
aborts_if exists i in 0..len(recipients):
    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if exists i in 0..len(recipients):
    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public entry fun transfer(source: &amp;signer, to: address, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let account_addr_source &#61; signer::address_of(source);
requires account_addr_source !&#61; to;
include CreateAccountTransferAbortsIf;
include GuidAbortsIf&lt;AptosCoin&gt;;
include WithdrawAbortsIf&lt;AptosCoin&gt;&#123;from: source&#125;;
include TransferEnsures&lt;AptosCoin&gt;;
aborts_if exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(to) &amp;&amp; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(to).frozen;
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
ensures exists&lt;aptos_framework::account::Account&gt;(to);
ensures exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(to);
</code></pre>



<a id="@Specification_1_batch_transfer_coins"></a>

### Function `batch_transfer_coins`


<pre><code>public entry fun batch_transfer_coins&lt;CoinType&gt;(from: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
let account_addr_source &#61; signer::address_of(from);
let coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);
let balance_source &#61; coin_store_source.coin.value;
requires forall i in 0..len(recipients):
    recipients[i] !&#61; account_addr_source;
requires exists i in 0..len(recipients):
    amounts[i] &gt; 0;
// This enforces <a id="high-level-req-7" href="#high-level-req">high-level requirement 7</a>:
aborts_if len(recipients) !&#61; len(amounts);
aborts_if exists i in 0..len(recipients):
        !account::exists_at(recipients[i]) &amp;&amp; length_judgment(recipients[i]);
aborts_if exists i in 0..len(recipients):
        !account::exists_at(recipients[i]) &amp;&amp; (recipients[i] &#61;&#61; @vm_reserved &#124;&#124; recipients[i] &#61;&#61; @aptos_framework &#124;&#124; recipients[i] &#61;&#61; @aptos_token);
ensures forall i in 0..len(recipients):
        (!account::exists_at(recipients[i]) &#61;&#61;&gt; !length_judgment(recipients[i])) &amp;&amp;
            (!account::exists_at(recipients[i]) &#61;&#61;&gt; (recipients[i] !&#61; @vm_reserved &amp;&amp; recipients[i] !&#61; @aptos_framework &amp;&amp; recipients[i] !&#61; @aptos_token));
aborts_if exists i in 0..len(recipients):
    !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);
aborts_if exists i in 0..len(recipients):
    coin_store_source.frozen;
aborts_if exists i in 0..len(recipients):
    global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source).coin.value &lt; amounts[i];
aborts_if exists i in 0..len(recipients):
    exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]).frozen;
aborts_if exists i in 0..len(recipients):
    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if exists i in 0..len(recipients):
    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt; MAX_U64;
aborts_if exists i in 0..len(recipients):
    !coin::spec_is_account_registered&lt;CoinType&gt;(recipients[i]) &amp;&amp; !type_info::spec_is_struct&lt;CoinType&gt;();
</code></pre>



<a id="@Specification_1_transfer_coins"></a>

### Function `transfer_coins`


<pre><code>public entry fun transfer_coins&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let account_addr_source &#61; signer::address_of(from);
requires account_addr_source !&#61; to;
include CreateAccountTransferAbortsIf;
include WithdrawAbortsIf&lt;CoinType&gt;;
include GuidAbortsIf&lt;CoinType&gt;;
include RegistCoinAbortsIf&lt;CoinType&gt;;
include TransferEnsures&lt;CoinType&gt;;
aborts_if exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to) &amp;&amp; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).frozen;
ensures exists&lt;aptos_framework::account::Account&gt;(to);
ensures exists&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(to);
</code></pre>




<a id="0x1_aptos_account_CreateAccountTransferAbortsIf"></a>


<pre><code>schema CreateAccountTransferAbortsIf &#123;
    to: address;
    aborts_if !account::exists_at(to) &amp;&amp; length_judgment(to);
    aborts_if !account::exists_at(to) &amp;&amp; (to &#61;&#61; @vm_reserved &#124;&#124; to &#61;&#61; @aptos_framework &#124;&#124; to &#61;&#61; @aptos_token);
&#125;
</code></pre>




<a id="0x1_aptos_account_WithdrawAbortsIf"></a>


<pre><code>schema WithdrawAbortsIf&lt;CoinType&gt; &#123;
    from: &amp;signer;
    amount: u64;
    let account_addr_source &#61; signer::address_of(from);
    let coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);
    let balance_source &#61; coin_store_source.coin.value;
    aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);
    aborts_if coin_store_source.frozen;
    aborts_if balance_source &lt; amount;
&#125;
</code></pre>




<a id="0x1_aptos_account_GuidAbortsIf"></a>


<pre><code>schema GuidAbortsIf&lt;CoinType&gt; &#123;
    to: address;
    let acc &#61; global&lt;account::Account&gt;(to);
    aborts_if account::exists_at(to) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to) &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    aborts_if account::exists_at(to) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to) &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;
&#125;
</code></pre>




<a id="0x1_aptos_account_RegistCoinAbortsIf"></a>


<pre><code>schema RegistCoinAbortsIf&lt;CoinType&gt; &#123;
    to: address;
    aborts_if !coin::spec_is_account_registered&lt;CoinType&gt;(to) &amp;&amp; !type_info::spec_is_struct&lt;CoinType&gt;();
    aborts_if exists&lt;aptos_framework::account::Account&gt;(to);
    aborts_if type_info::type_of&lt;CoinType&gt;() !&#61; type_info::type_of&lt;AptosCoin&gt;();
&#125;
</code></pre>




<a id="0x1_aptos_account_TransferEnsures"></a>


<pre><code>schema TransferEnsures&lt;CoinType&gt; &#123;
    to: address;
    account_addr_source: address;
    amount: u64;
    let if_exist_account &#61; exists&lt;account::Account&gt;(to);
    let if_exist_coin &#61; exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);
    let coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);
    let coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);
    let post p_coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);
    let post p_coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);
    ensures coin_store_source.coin.value &#45; amount &#61;&#61; p_coin_store_source.coin.value;
    ensures if_exist_account &amp;&amp; if_exist_coin &#61;&#61;&gt; coin_store_to.coin.value &#43; amount &#61;&#61; p_coin_store_to.coin.value;
&#125;
</code></pre>



<a id="@Specification_1_deposit_coins"></a>

### Function `deposit_coins`


<pre><code>public fun deposit_coins&lt;CoinType&gt;(to: address, coins: coin::Coin&lt;CoinType&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
include CreateAccountTransferAbortsIf;
include GuidAbortsIf&lt;CoinType&gt;;
include RegistCoinAbortsIf&lt;CoinType&gt;;
let if_exist_coin &#61; exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);
aborts_if if_exist_coin &amp;&amp; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).frozen;
// This enforces <a id="high-level-spec-6" href="#high-level-req">high-level requirement 6</a>:
ensures exists&lt;aptos_framework::account::Account&gt;(to);
ensures exists&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(to);
let coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).coin.value;
let post post_coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).coin.value;
ensures if_exist_coin &#61;&#61;&gt; post_coin_store_to &#61;&#61; coin_store_to &#43; coins.value;
</code></pre>



<a id="@Specification_1_assert_account_exists"></a>

### Function `assert_account_exists`


<pre><code>public fun assert_account_exists(addr: address)
</code></pre>




<pre><code>aborts_if !account::exists_at(addr);
</code></pre>



<a id="@Specification_1_assert_account_is_registered_for_apt"></a>

### Function `assert_account_is_registered_for_apt`


<pre><code>public fun assert_account_is_registered_for_apt(addr: address)
</code></pre>


Check if the address existed.
Check if the AptosCoin under the address existed.


<pre><code>pragma aborts_if_is_partial;
aborts_if !account::exists_at(addr);
aborts_if !coin::spec_is_account_registered&lt;AptosCoin&gt;(addr);
</code></pre>



<a id="@Specification_1_set_allow_direct_coin_transfers"></a>

### Function `set_allow_direct_coin_transfers`


<pre><code>public entry fun set_allow_direct_coin_transfers(account: &amp;signer, allow: bool)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_can_receive_direct_coin_transfers"></a>

### Function `can_receive_direct_coin_transfers`


<pre><code>&#35;[view]
public fun can_receive_direct_coin_transfers(account: address): bool
</code></pre>




<pre><code>aborts_if false;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures result &#61;&#61; (
    !exists&lt;DirectTransferConfig&gt;(account) &#124;&#124;
        global&lt;DirectTransferConfig&gt;(account).allow_arbitrary_coin_transfers
);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
