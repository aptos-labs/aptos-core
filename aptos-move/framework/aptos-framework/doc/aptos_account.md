
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


<pre><code>use 0x1::account;<br/>use 0x1::aptos_coin;<br/>use 0x1::coin;<br/>use 0x1::create_signer;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::signer;<br/></code></pre>



<a id="0x1_aptos_account_DirectTransferConfig"></a>

## Resource `DirectTransferConfig`

Configuration for whether an account can receive direct transfers of coins that they have not registered.<br/><br/> By default, this is enabled. Users can opt&#45;out by disabling at any time.


<pre><code>struct DirectTransferConfig has key<br/></code></pre>



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

Event emitted when an account&apos;s direct coins transfer config is updated.


<pre><code>struct DirectCoinTransferConfigUpdatedEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct DirectCoinTransferConfigUpdated has drop, store<br/></code></pre>



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


<pre><code>const EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS: u64 &#61; 3;<br/></code></pre>



<a id="0x1_aptos_account_EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS"></a>

Account opted out of directly receiving NFT tokens.


<pre><code>const EACCOUNT_DOES_NOT_ACCEPT_DIRECT_TOKEN_TRANSFERS: u64 &#61; 4;<br/></code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_FOUND"></a>

Account does not exist.


<pre><code>const EACCOUNT_NOT_FOUND: u64 &#61; 1;<br/></code></pre>



<a id="0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT"></a>

Account is not registered to receive APT.


<pre><code>const EACCOUNT_NOT_REGISTERED_FOR_APT: u64 &#61; 2;<br/></code></pre>



<a id="0x1_aptos_account_EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH"></a>

The lengths of the recipients and amounts lists don&apos;t match.


<pre><code>const EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH: u64 &#61; 5;<br/></code></pre>



<a id="0x1_aptos_account_create_account"></a>

## Function `create_account`

Basic account creation methods.


<pre><code>public entry fun create_account(auth_key: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_account(auth_key: address) &#123;<br/>    let signer &#61; account::create_account(auth_key);<br/>    coin::register&lt;AptosCoin&gt;(&amp;signer);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer"></a>

## Function `batch_transfer`

Batch version of APT transfer.


<pre><code>public entry fun batch_transfer(source: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun batch_transfer(source: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;) &#123;<br/>    let recipients_len &#61; vector::length(&amp;recipients);<br/>    assert!(<br/>        recipients_len &#61;&#61; vector::length(&amp;amounts),<br/>        error::invalid_argument(EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH),<br/>    );<br/><br/>    vector::enumerate_ref(&amp;recipients, &#124;i, to&#124; &#123;<br/>        let amount &#61; &#42;vector::borrow(&amp;amounts, i);<br/>        transfer(source, &#42;to, amount);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_transfer"></a>

## Function `transfer`

Convenient function to transfer APT to a recipient account that might not exist.<br/> This would create the recipient account first, which also registers it to receive APT, before transferring.


<pre><code>public entry fun transfer(source: &amp;signer, to: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer(source: &amp;signer, to: address, amount: u64) &#123;<br/>    if (!account::exists_at(to)) &#123;<br/>        create_account(to)<br/>    &#125;;<br/>    // Resource accounts can be created without registering them to receive APT.<br/>    // This conveniently does the registration if necessary.<br/>    if (!coin::is_account_registered&lt;AptosCoin&gt;(to)) &#123;<br/>        coin::register&lt;AptosCoin&gt;(&amp;create_signer(to));<br/>    &#125;;<br/>    coin::transfer&lt;AptosCoin&gt;(source, to, amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_batch_transfer_coins"></a>

## Function `batch_transfer_coins`

Batch version of transfer_coins.


<pre><code>public entry fun batch_transfer_coins&lt;CoinType&gt;(from: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun batch_transfer_coins&lt;CoinType&gt;(<br/>    from: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;) acquires DirectTransferConfig &#123;<br/>    let recipients_len &#61; vector::length(&amp;recipients);<br/>    assert!(<br/>        recipients_len &#61;&#61; vector::length(&amp;amounts),<br/>        error::invalid_argument(EMISMATCHING_RECIPIENTS_AND_AMOUNTS_LENGTH),<br/>    );<br/><br/>    vector::enumerate_ref(&amp;recipients, &#124;i, to&#124; &#123;<br/>        let amount &#61; &#42;vector::borrow(&amp;amounts, i);<br/>        transfer_coins&lt;CoinType&gt;(from, &#42;to, amount);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_transfer_coins"></a>

## Function `transfer_coins`

Convenient function to transfer a custom CoinType to a recipient account that might not exist.<br/> This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code>public entry fun transfer_coins&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_coins&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64) acquires DirectTransferConfig &#123;<br/>    deposit_coins(to, coin::withdraw&lt;CoinType&gt;(from, amount));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_deposit_coins"></a>

## Function `deposit_coins`

Convenient function to deposit a custom CoinType into a recipient account that might not exist.<br/> This would create the recipient account first and register it to receive the CoinType, before transferring.


<pre><code>public fun deposit_coins&lt;CoinType&gt;(to: address, coins: coin::Coin&lt;CoinType&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_coins&lt;CoinType&gt;(to: address, coins: Coin&lt;CoinType&gt;) acquires DirectTransferConfig &#123;<br/>    if (!account::exists_at(to)) &#123;<br/>        create_account(to);<br/>        spec &#123;<br/>            assert coin::spec_is_account_registered&lt;AptosCoin&gt;(to);<br/>            assume aptos_std::type_info::type_of&lt;CoinType&gt;() &#61;&#61; aptos_std::type_info::type_of&lt;AptosCoin&gt;() &#61;&#61;&gt;<br/>                coin::spec_is_account_registered&lt;CoinType&gt;(to);<br/>        &#125;;<br/>    &#125;;<br/>    if (!coin::is_account_registered&lt;CoinType&gt;(to)) &#123;<br/>        assert!(<br/>            can_receive_direct_coin_transfers(to),<br/>            error::permission_denied(EACCOUNT_DOES_NOT_ACCEPT_DIRECT_COIN_TRANSFERS),<br/>        );<br/>        coin::register&lt;CoinType&gt;(&amp;create_signer(to));<br/>    &#125;;<br/>    coin::deposit&lt;CoinType&gt;(to, coins)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_assert_account_exists"></a>

## Function `assert_account_exists`



<pre><code>public fun assert_account_exists(addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_account_exists(addr: address) &#123;<br/>    assert!(account::exists_at(addr), error::not_found(EACCOUNT_NOT_FOUND));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_assert_account_is_registered_for_apt"></a>

## Function `assert_account_is_registered_for_apt`



<pre><code>public fun assert_account_is_registered_for_apt(addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_account_is_registered_for_apt(addr: address) &#123;<br/>    assert_account_exists(addr);<br/>    assert!(coin::is_account_registered&lt;AptosCoin&gt;(addr), error::not_found(EACCOUNT_NOT_REGISTERED_FOR_APT));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_set_allow_direct_coin_transfers"></a>

## Function `set_allow_direct_coin_transfers`

Set whether <code>account</code> can receive direct transfers of coins that they have not explicitly registered to receive.


<pre><code>public entry fun set_allow_direct_coin_transfers(account: &amp;signer, allow: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_allow_direct_coin_transfers(account: &amp;signer, allow: bool) acquires DirectTransferConfig &#123;<br/>    let addr &#61; signer::address_of(account);<br/>    if (exists&lt;DirectTransferConfig&gt;(addr)) &#123;<br/>        let direct_transfer_config &#61; borrow_global_mut&lt;DirectTransferConfig&gt;(addr);<br/>        // Short&#45;circuit to avoid emitting an event if direct transfer config is not changing.<br/>        if (direct_transfer_config.allow_arbitrary_coin_transfers &#61;&#61; allow) &#123;<br/>            return<br/>        &#125;;<br/><br/>        direct_transfer_config.allow_arbitrary_coin_transfers &#61; allow;<br/><br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            emit(DirectCoinTransferConfigUpdated &#123; account: addr, new_allow_direct_transfers: allow &#125;);<br/>        &#125;;<br/>        emit_event(<br/>            &amp;mut direct_transfer_config.update_coin_transfer_events,<br/>            DirectCoinTransferConfigUpdatedEvent &#123; new_allow_direct_transfers: allow &#125;);<br/>    &#125; else &#123;<br/>        let direct_transfer_config &#61; DirectTransferConfig &#123;<br/>            allow_arbitrary_coin_transfers: allow,<br/>            update_coin_transfer_events: new_event_handle&lt;DirectCoinTransferConfigUpdatedEvent&gt;(account),<br/>        &#125;;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            emit(DirectCoinTransferConfigUpdated &#123; account: addr, new_allow_direct_transfers: allow &#125;);<br/>        &#125;;<br/>        emit_event(<br/>            &amp;mut direct_transfer_config.update_coin_transfer_events,<br/>            DirectCoinTransferConfigUpdatedEvent &#123; new_allow_direct_transfers: allow &#125;);<br/>        move_to(account, direct_transfer_config);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_account_can_receive_direct_coin_transfers"></a>

## Function `can_receive_direct_coin_transfers`

Return true if <code>account</code> can receive direct transfers of coins that they have not explicitly registered to<br/> receive.<br/><br/> By default, this returns true if an account has not explicitly set whether the can receive direct transfers.


<pre><code>&#35;[view]<br/>public fun can_receive_direct_coin_transfers(account: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_receive_direct_coin_transfers(account: address): bool acquires DirectTransferConfig &#123;<br/>    !exists&lt;DirectTransferConfig&gt;(account) &#124;&#124;<br/>        borrow_global&lt;DirectTransferConfig&gt;(account).allow_arbitrary_coin_transfers<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;During the creation of an Aptos account the following rules should hold: (1) the authentication key should be 32 bytes in length, (2) an Aptos account should not already exist for that authentication key, and (3) the address of the authentication key should not be equal to a reserved address (0x0, 0x1, or 0x3).&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The authentication key which is passed in as an argument to create_account should satisfy all necessary conditions.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;CreateAccountAbortsIf&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;After creating an Aptos account, the account should become registered to receive AptosCoin.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The create_account function creates a new account for the particular address and registers AptosCoin.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;create_account&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;An account may receive a direct transfer of coins they have not registered for if and only if the transfer of arbitrary coins is enabled. By default the option should always set to be enabled for an account.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;Transfers of a coin to an account that has not yet registered for that coin should abort if and only if the allow_arbitrary_coin_transfers flag is explicitly set to false.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;can_receive_direct_coin_transfers&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;Setting direct coin transfers may only occur if and only if a direct transfer config is associated with the provided account address.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The set_allow_direct_coin_transfers function ensures the DirectTransferConfig structure exists for the signer.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;set_allow_direct_coin_transfers&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;The transfer function should ensure an account is created for the provided destination if one does not exist; then, register AptosCoin for that account if a particular is unregistered before transferring the amount.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The transfer function checks if the recipient account exists. If the account does not exist, the function creates one and registers the account to AptosCoin if not registered.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5&quot;&gt;transfer&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;Creating an account for the provided destination and registering it for that particular CoinType should be the only way to enable depositing coins, provided the account does not already exist.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The deposit_coins function verifies if the recipient account exists. If the account does not exist, the function creates one and ensures that the account becomes registered for the specified CointType.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;6&quot;&gt;deposit_coins&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;When performing a batch transfer of Aptos Coin and/or a batch transfer of a custom coin type, it should ensure that the vector containing destination addresses and the vector containing the corresponding amounts are equal in length.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The batch_transfer and batch_transfer_coins functions verify that the length of the recipient addresses vector matches the length of the amount vector through an assertion.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;7&quot;&gt;batch_transfer_coins&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code>public entry fun create_account(auth_key: address)<br/></code></pre>


Check if the bytes of the auth_key is 32.<br/> The Account does not exist under the auth_key before creating the account.<br/> Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
pragma aborts_if_is_partial;<br/>include CreateAccountAbortsIf;<br/>ensures exists&lt;account::Account&gt;(auth_key);<br/></code></pre>




<a id="0x1_aptos_account_CreateAccountAbortsIf"></a>


<pre><code>schema CreateAccountAbortsIf &#123;<br/>auth_key: address;<br/>aborts_if exists&lt;account::Account&gt;(auth_key);<br/>aborts_if length_judgment(auth_key);<br/>aborts_if auth_key &#61;&#61; @vm_reserved &#124;&#124; auth_key &#61;&#61; @aptos_framework &#124;&#124; auth_key &#61;&#61; @aptos_token;<br/>&#125;<br/></code></pre>




<a id="0x1_aptos_account_length_judgment"></a>


<pre><code>fun length_judgment(auth_key: address): bool &#123;<br/>   use std::bcs;<br/><br/>   let authentication_key &#61; bcs::to_bytes(auth_key);<br/>   len(authentication_key) !&#61; 32<br/>&#125;<br/></code></pre>



<a id="@Specification_1_batch_transfer"></a>

### Function `batch_transfer`


<pre><code>public entry fun batch_transfer(source: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let account_addr_source &#61; signer::address_of(source);<br/>let coin_store_source &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account_addr_source);<br/>let balance_source &#61; coin_store_source.coin.value;<br/>requires forall i in 0..len(recipients):<br/>    recipients[i] !&#61; account_addr_source;<br/>requires exists i in 0..len(recipients):<br/>    amounts[i] &gt; 0;<br/>aborts_if len(recipients) !&#61; len(amounts);<br/>aborts_if exists i in 0..len(recipients):<br/>        !account::exists_at(recipients[i]) &amp;&amp; length_judgment(recipients[i]);<br/>aborts_if exists i in 0..len(recipients):<br/>        !account::exists_at(recipients[i]) &amp;&amp; (recipients[i] &#61;&#61; @vm_reserved &#124;&#124; recipients[i] &#61;&#61; @aptos_framework &#124;&#124; recipients[i] &#61;&#61; @aptos_token);<br/>ensures forall i in 0..len(recipients):<br/>        (!account::exists_at(recipients[i]) &#61;&#61;&gt; !length_judgment(recipients[i])) &amp;&amp;<br/>            (!account::exists_at(recipients[i]) &#61;&#61;&gt; (recipients[i] !&#61; @vm_reserved &amp;&amp; recipients[i] !&#61; @aptos_framework &amp;&amp; recipients[i] !&#61; @aptos_token));<br/>aborts_if exists i in 0..len(recipients):<br/>    !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account_addr_source);<br/>aborts_if exists i in 0..len(recipients):<br/>    coin_store_source.frozen;<br/>aborts_if exists i in 0..len(recipients):<br/>    global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account_addr_source).coin.value &lt; amounts[i];<br/>aborts_if exists i in 0..len(recipients):<br/>    exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]).frozen;<br/>aborts_if exists i in 0..len(recipients):<br/>    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if exists i in 0..len(recipients):<br/>    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public entry fun transfer(source: &amp;signer, to: address, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let account_addr_source &#61; signer::address_of(source);<br/>requires account_addr_source !&#61; to;<br/>include CreateAccountTransferAbortsIf;<br/>include GuidAbortsIf&lt;AptosCoin&gt;;<br/>include WithdrawAbortsIf&lt;AptosCoin&gt;&#123;from: source&#125;;<br/>include TransferEnsures&lt;AptosCoin&gt;;<br/>aborts_if exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(to) &amp;&amp; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(to).frozen;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
ensures exists&lt;aptos_framework::account::Account&gt;(to);<br/>ensures exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(to);<br/></code></pre>



<a id="@Specification_1_batch_transfer_coins"></a>

### Function `batch_transfer_coins`


<pre><code>public entry fun batch_transfer_coins&lt;CoinType&gt;(from: &amp;signer, recipients: vector&lt;address&gt;, amounts: vector&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let account_addr_source &#61; signer::address_of(from);<br/>let coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);<br/>let balance_source &#61; coin_store_source.coin.value;<br/>requires forall i in 0..len(recipients):<br/>    recipients[i] !&#61; account_addr_source;<br/>requires exists i in 0..len(recipients):<br/>    amounts[i] &gt; 0;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;7&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 7&lt;/a&gt;:
aborts_if len(recipients) !&#61; len(amounts);<br/>aborts_if exists i in 0..len(recipients):<br/>        !account::exists_at(recipients[i]) &amp;&amp; length_judgment(recipients[i]);<br/>aborts_if exists i in 0..len(recipients):<br/>        !account::exists_at(recipients[i]) &amp;&amp; (recipients[i] &#61;&#61; @vm_reserved &#124;&#124; recipients[i] &#61;&#61; @aptos_framework &#124;&#124; recipients[i] &#61;&#61; @aptos_token);<br/>ensures forall i in 0..len(recipients):<br/>        (!account::exists_at(recipients[i]) &#61;&#61;&gt; !length_judgment(recipients[i])) &amp;&amp;<br/>            (!account::exists_at(recipients[i]) &#61;&#61;&gt; (recipients[i] !&#61; @vm_reserved &amp;&amp; recipients[i] !&#61; @aptos_framework &amp;&amp; recipients[i] !&#61; @aptos_token));<br/>aborts_if exists i in 0..len(recipients):<br/>    !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);<br/>aborts_if exists i in 0..len(recipients):<br/>    coin_store_source.frozen;<br/>aborts_if exists i in 0..len(recipients):<br/>    global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source).coin.value &lt; amounts[i];<br/>aborts_if exists i in 0..len(recipients):<br/>    exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]).frozen;<br/>aborts_if exists i in 0..len(recipients):<br/>    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if exists i in 0..len(recipients):<br/>    account::exists_at(recipients[i]) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(recipients[i]) &amp;&amp; global&lt;account::Account&gt;(recipients[i]).guid_creation_num &#43; 2 &gt; MAX_U64;<br/>aborts_if exists i in 0..len(recipients):<br/>    !coin::spec_is_account_registered&lt;CoinType&gt;(recipients[i]) &amp;&amp; !type_info::spec_is_struct&lt;CoinType&gt;();<br/></code></pre>



<a id="@Specification_1_transfer_coins"></a>

### Function `transfer_coins`


<pre><code>public entry fun transfer_coins&lt;CoinType&gt;(from: &amp;signer, to: address, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let account_addr_source &#61; signer::address_of(from);<br/>requires account_addr_source !&#61; to;<br/>include CreateAccountTransferAbortsIf;<br/>include WithdrawAbortsIf&lt;CoinType&gt;;<br/>include GuidAbortsIf&lt;CoinType&gt;;<br/>include RegistCoinAbortsIf&lt;CoinType&gt;;<br/>include TransferEnsures&lt;CoinType&gt;;<br/>aborts_if exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to) &amp;&amp; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).frozen;<br/>ensures exists&lt;aptos_framework::account::Account&gt;(to);<br/>ensures exists&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(to);<br/></code></pre>




<a id="0x1_aptos_account_CreateAccountTransferAbortsIf"></a>


<pre><code>schema CreateAccountTransferAbortsIf &#123;<br/>to: address;<br/>aborts_if !account::exists_at(to) &amp;&amp; length_judgment(to);<br/>aborts_if !account::exists_at(to) &amp;&amp; (to &#61;&#61; @vm_reserved &#124;&#124; to &#61;&#61; @aptos_framework &#124;&#124; to &#61;&#61; @aptos_token);<br/>&#125;<br/></code></pre>




<a id="0x1_aptos_account_WithdrawAbortsIf"></a>


<pre><code>schema WithdrawAbortsIf&lt;CoinType&gt; &#123;<br/>from: &amp;signer;<br/>amount: u64;<br/>let account_addr_source &#61; signer::address_of(from);<br/>let coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);<br/>let balance_source &#61; coin_store_source.coin.value;<br/>aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);<br/>aborts_if coin_store_source.frozen;<br/>aborts_if balance_source &lt; amount;<br/>&#125;<br/></code></pre>




<a id="0x1_aptos_account_GuidAbortsIf"></a>


<pre><code>schema GuidAbortsIf&lt;CoinType&gt; &#123;<br/>to: address;<br/>let acc &#61; global&lt;account::Account&gt;(to);<br/>aborts_if account::exists_at(to) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to) &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if account::exists_at(to) &amp;&amp; !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to) &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;<br/>&#125;<br/></code></pre>




<a id="0x1_aptos_account_RegistCoinAbortsIf"></a>


<pre><code>schema RegistCoinAbortsIf&lt;CoinType&gt; &#123;<br/>to: address;<br/>aborts_if !coin::spec_is_account_registered&lt;CoinType&gt;(to) &amp;&amp; !type_info::spec_is_struct&lt;CoinType&gt;();<br/>aborts_if exists&lt;aptos_framework::account::Account&gt;(to);<br/>aborts_if type_info::type_of&lt;CoinType&gt;() !&#61; type_info::type_of&lt;AptosCoin&gt;();<br/>&#125;<br/></code></pre>




<a id="0x1_aptos_account_TransferEnsures"></a>


<pre><code>schema TransferEnsures&lt;CoinType&gt; &#123;<br/>to: address;<br/>account_addr_source: address;<br/>amount: u64;<br/>let if_exist_account &#61; exists&lt;account::Account&gt;(to);<br/>let if_exist_coin &#61; exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);<br/>let coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);<br/>let coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);<br/>let post p_coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);<br/>let post p_coin_store_source &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr_source);<br/>ensures coin_store_source.coin.value &#45; amount &#61;&#61; p_coin_store_source.coin.value;<br/>ensures if_exist_account &amp;&amp; if_exist_coin &#61;&#61;&gt; coin_store_to.coin.value &#43; amount &#61;&#61; p_coin_store_to.coin.value;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_deposit_coins"></a>

### Function `deposit_coins`


<pre><code>public fun deposit_coins&lt;CoinType&gt;(to: address, coins: coin::Coin&lt;CoinType&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include CreateAccountTransferAbortsIf;<br/>include GuidAbortsIf&lt;CoinType&gt;;<br/>include RegistCoinAbortsIf&lt;CoinType&gt;;<br/>let if_exist_coin &#61; exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to);<br/>aborts_if if_exist_coin &amp;&amp; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).frozen;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;spec&#45;6&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 6&lt;/a&gt;:
ensures exists&lt;aptos_framework::account::Account&gt;(to);<br/>ensures exists&lt;aptos_framework::coin::CoinStore&lt;CoinType&gt;&gt;(to);<br/>let coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).coin.value;<br/>let post post_coin_store_to &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(to).coin.value;<br/>ensures if_exist_coin &#61;&#61;&gt; post_coin_store_to &#61;&#61; coin_store_to &#43; coins.value;<br/></code></pre>



<a id="@Specification_1_assert_account_exists"></a>

### Function `assert_account_exists`


<pre><code>public fun assert_account_exists(addr: address)<br/></code></pre>




<pre><code>aborts_if !account::exists_at(addr);<br/></code></pre>



<a id="@Specification_1_assert_account_is_registered_for_apt"></a>

### Function `assert_account_is_registered_for_apt`


<pre><code>public fun assert_account_is_registered_for_apt(addr: address)<br/></code></pre>


Check if the address existed.<br/> Check if the AptosCoin under the address existed.


<pre><code>pragma aborts_if_is_partial;<br/>aborts_if !account::exists_at(addr);<br/>aborts_if !coin::spec_is_account_registered&lt;AptosCoin&gt;(addr);<br/></code></pre>



<a id="@Specification_1_set_allow_direct_coin_transfers"></a>

### Function `set_allow_direct_coin_transfers`


<pre><code>public entry fun set_allow_direct_coin_transfers(account: &amp;signer, allow: bool)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_can_receive_direct_coin_transfers"></a>

### Function `can_receive_direct_coin_transfers`


<pre><code>&#35;[view]<br/>public fun can_receive_direct_coin_transfers(account: address): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
ensures result &#61;&#61; (<br/>    !exists&lt;DirectTransferConfig&gt;(account) &#124;&#124;<br/>        global&lt;DirectTransferConfig&gt;(account).allow_arbitrary_coin_transfers<br/>);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
