
<a id="0x1_aptos_coin"></a>

# Module `0x1::aptos_coin`

This module defines a minimal and generic Coin and Balance.
modified from https://github.com/move-language/move/tree/main/language/documentation/tutorial


-  [Resource `AptosCoin`](#0x1_aptos_coin_AptosCoin)
-  [Resource `MintCapStore`](#0x1_aptos_coin_MintCapStore)
-  [Struct `DelegatedMintCapability`](#0x1_aptos_coin_DelegatedMintCapability)
-  [Resource `Delegations`](#0x1_aptos_coin_Delegations)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_aptos_coin_initialize)
-  [Function `has_mint_capability`](#0x1_aptos_coin_has_mint_capability)
-  [Function `destroy_mint_cap`](#0x1_aptos_coin_destroy_mint_cap)
-  [Function `configure_accounts_for_test`](#0x1_aptos_coin_configure_accounts_for_test)
-  [Function `mint`](#0x1_aptos_coin_mint)
-  [Function `delegate_mint_capability`](#0x1_aptos_coin_delegate_mint_capability)
-  [Function `claim_mint_capability`](#0x1_aptos_coin_claim_mint_capability)
-  [Function `find_delegation`](#0x1_aptos_coin_find_delegation)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `destroy_mint_cap`](#@Specification_1_destroy_mint_cap)
    -  [Function `configure_accounts_for_test`](#@Specification_1_configure_accounts_for_test)
    -  [Function `mint`](#@Specification_1_mint)
    -  [Function `delegate_mint_capability`](#@Specification_1_delegate_mint_capability)
    -  [Function `claim_mint_capability`](#@Specification_1_claim_mint_capability)
    -  [Function `find_delegation`](#@Specification_1_find_delegation)


<pre><code>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_aptos_coin_AptosCoin"></a>

## Resource `AptosCoin`



<pre><code>struct AptosCoin has key<br/></code></pre>



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

<a id="0x1_aptos_coin_MintCapStore"></a>

## Resource `MintCapStore`



<pre><code>struct MintCapStore has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_coin_DelegatedMintCapability"></a>

## Struct `DelegatedMintCapability`

Delegation token created by delegator and can be claimed by the delegatee as MintCapability.


<pre><code>struct DelegatedMintCapability has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>to: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_coin_Delegations"></a>

## Resource `Delegations`

The container stores the current pending delegations.


<pre><code>struct Delegations has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: vector&lt;aptos_coin::DelegatedMintCapability&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aptos_coin_EALREADY_DELEGATED"></a>

Mint capability has already been delegated to this specified address


<pre><code>const EALREADY_DELEGATED: u64 &#61; 2;<br/></code></pre>



<a id="0x1_aptos_coin_EDELEGATION_NOT_FOUND"></a>

Cannot find delegation of mint capability to this account


<pre><code>const EDELEGATION_NOT_FOUND: u64 &#61; 3;<br/></code></pre>



<a id="0x1_aptos_coin_ENO_CAPABILITIES"></a>

Account does not have mint capability


<pre><code>const ENO_CAPABILITIES: u64 &#61; 1;<br/></code></pre>



<a id="0x1_aptos_coin_initialize"></a>

## Function `initialize`

Can only called during genesis to initialize the Aptos coin.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer): (coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;, coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer): (BurnCapability&lt;AptosCoin&gt;, MintCapability&lt;AptosCoin&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    let (burn_cap, freeze_cap, mint_cap) &#61; coin::initialize_with_parallelizable_supply&lt;AptosCoin&gt;(<br/>        aptos_framework,<br/>        string::utf8(b&quot;Aptos Coin&quot;),<br/>        string::utf8(b&quot;APT&quot;),<br/>        8, // decimals<br/>        true, // monitor_supply<br/>    );<br/><br/>    // Aptos framework needs mint cap to mint coins to initial validators. This will be revoked once the validators<br/>    // have been initialized.<br/>    move_to(aptos_framework, MintCapStore &#123; mint_cap &#125;);<br/><br/>    coin::destroy_freeze_cap(freeze_cap);<br/>    (burn_cap, mint_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_coin_has_mint_capability"></a>

## Function `has_mint_capability`



<pre><code>public fun has_mint_capability(account: &amp;signer): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun has_mint_capability(account: &amp;signer): bool &#123;<br/>    exists&lt;MintCapStore&gt;(signer::address_of(account))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_coin_destroy_mint_cap"></a>

## Function `destroy_mint_cap`

Only called during genesis to destroy the aptos framework account's mint capability once all initial validators
and accounts have been initialized during genesis.


<pre><code>public(friend) fun destroy_mint_cap(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun destroy_mint_cap(aptos_framework: &amp;signer) acquires MintCapStore &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    let MintCapStore &#123; mint_cap &#125; &#61; move_from&lt;MintCapStore&gt;(@aptos_framework);<br/>    coin::destroy_mint_cap(mint_cap);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_coin_configure_accounts_for_test"></a>

## Function `configure_accounts_for_test`

Can only be called during genesis for tests to grant mint capability to aptos framework and core resources
accounts.


<pre><code>public(friend) fun configure_accounts_for_test(aptos_framework: &amp;signer, core_resources: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun configure_accounts_for_test(<br/>    aptos_framework: &amp;signer,<br/>    core_resources: &amp;signer,<br/>    mint_cap: MintCapability&lt;AptosCoin&gt;,<br/>) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    // Mint the core resource account AptosCoin for gas so it can execute system transactions.<br/>    coin::register&lt;AptosCoin&gt;(core_resources);<br/>    let coins &#61; coin::mint&lt;AptosCoin&gt;(<br/>        18446744073709551615,<br/>        &amp;mint_cap,<br/>    );<br/>    coin::deposit&lt;AptosCoin&gt;(signer::address_of(core_resources), coins);<br/><br/>    move_to(core_resources, MintCapStore &#123; mint_cap &#125;);<br/>    move_to(core_resources, Delegations &#123; inner: vector::empty() &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_coin_mint"></a>

## Function `mint`

Only callable in tests and testnets where the core resources account exists.
Create new coins and deposit them into dst_addr's account.


<pre><code>public entry fun mint(account: &amp;signer, dst_addr: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint(<br/>    account: &amp;signer,<br/>    dst_addr: address,<br/>    amount: u64,<br/>) acquires MintCapStore &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/><br/>    assert!(<br/>        exists&lt;MintCapStore&gt;(account_addr),<br/>        error::not_found(ENO_CAPABILITIES),<br/>    );<br/><br/>    let mint_cap &#61; &amp;borrow_global&lt;MintCapStore&gt;(account_addr).mint_cap;<br/>    let coins_minted &#61; coin::mint&lt;AptosCoin&gt;(amount, mint_cap);<br/>    coin::deposit&lt;AptosCoin&gt;(dst_addr, coins_minted);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_coin_delegate_mint_capability"></a>

## Function `delegate_mint_capability`

Only callable in tests and testnets where the core resources account exists.
Create delegated token for the address so the account could claim MintCapability later.


<pre><code>public entry fun delegate_mint_capability(account: signer, to: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun delegate_mint_capability(account: signer, to: address) acquires Delegations &#123;<br/>    system_addresses::assert_core_resource(&amp;account);<br/>    let delegations &#61; &amp;mut borrow_global_mut&lt;Delegations&gt;(@core_resources).inner;<br/>    vector::for_each_ref(delegations, &#124;element&#124; &#123;<br/>        let element: &amp;DelegatedMintCapability &#61; element;<br/>        assert!(element.to !&#61; to, error::invalid_argument(EALREADY_DELEGATED));<br/>    &#125;);<br/>    vector::push_back(delegations, DelegatedMintCapability &#123; to &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_coin_claim_mint_capability"></a>

## Function `claim_mint_capability`

Only callable in tests and testnets where the core resources account exists.
Claim the delegated mint capability and destroy the delegated token.


<pre><code>public entry fun claim_mint_capability(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun claim_mint_capability(account: &amp;signer) acquires Delegations, MintCapStore &#123;<br/>    let maybe_index &#61; find_delegation(signer::address_of(account));<br/>    assert!(option::is_some(&amp;maybe_index), EDELEGATION_NOT_FOUND);<br/>    let idx &#61; &#42;option::borrow(&amp;maybe_index);<br/>    let delegations &#61; &amp;mut borrow_global_mut&lt;Delegations&gt;(@core_resources).inner;<br/>    let DelegatedMintCapability &#123; to: _ &#125; &#61; vector::swap_remove(delegations, idx);<br/><br/>    // Make a copy of mint cap and give it to the specified account.<br/>    let mint_cap &#61; borrow_global&lt;MintCapStore&gt;(@core_resources).mint_cap;<br/>    move_to(account, MintCapStore &#123; mint_cap &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_aptos_coin_find_delegation"></a>

## Function `find_delegation`



<pre><code>fun find_delegation(addr: address): option::Option&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun find_delegation(addr: address): Option&lt;u64&gt; acquires Delegations &#123;<br/>    let delegations &#61; &amp;borrow_global&lt;Delegations&gt;(@core_resources).inner;<br/>    let i &#61; 0;<br/>    let len &#61; vector::length(delegations);<br/>    let index &#61; option::none();<br/>    while (i &lt; len) &#123;<br/>        let element &#61; vector::borrow(delegations, i);<br/>        if (element.to &#61;&#61; addr) &#123;<br/>            index &#61; option::some(i);<br/>            break<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    index<br/>&#125;<br/></code></pre>



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
<td>The native token, APT, must be initialized during genesis.</td>
<td>Medium</td>
<td>The initialize function is only called once, during genesis.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The APT coin may only be created exactly once.</td>
<td>Medium</td>
<td>The initialization function may only be called once.</td>
<td>Enforced through the <a href="https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move">coin</a> module, which has been audited.</td>
</tr>

<tr>
<td>4</td>
<td>Any type of operation on the APT coin should fail if the user has not registered for the coin.</td>
<td>Medium</td>
<td>Coin operations may succeed only on valid user coin registration.</td>
<td>Enforced through the <a href="https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move">coin</a> module, which has been audited.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer): (coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;, coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if addr !&#61; @aptos_framework;<br/>aborts_if !string::spec_internal_check_utf8(b&quot;Aptos Coin&quot;);<br/>aborts_if !string::spec_internal_check_utf8(b&quot;APT&quot;);<br/>aborts_if exists&lt;MintCapStore&gt;(addr);<br/>aborts_if exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);<br/>aborts_if !exists&lt;aggregator_factory::AggregatorFactory&gt;(addr);<br/>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures exists&lt;MintCapStore&gt;(addr);<br/>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures global&lt;MintCapStore&gt;(addr).mint_cap &#61;&#61;  MintCapability&lt;AptosCoin&gt; &#123;&#125;;<br/>ensures exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);<br/>ensures result_1 &#61;&#61; BurnCapability&lt;AptosCoin&gt; &#123;&#125;;<br/>ensures result_2 &#61;&#61; MintCapability&lt;AptosCoin&gt; &#123;&#125;;<br/></code></pre>



<a id="@Specification_1_destroy_mint_cap"></a>

### Function `destroy_mint_cap`


<pre><code>public(friend) fun destroy_mint_cap(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if addr !&#61; @aptos_framework;<br/>aborts_if !exists&lt;MintCapStore&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_configure_accounts_for_test"></a>

### Function `configure_accounts_for_test`


<pre><code>public(friend) fun configure_accounts_for_test(aptos_framework: &amp;signer, core_resources: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code>public entry fun mint(account: &amp;signer, dst_addr: address, amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_delegate_mint_capability"></a>

### Function `delegate_mint_capability`


<pre><code>public entry fun delegate_mint_capability(account: signer, to: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_claim_mint_capability"></a>

### Function `claim_mint_capability`


<pre><code>public entry fun claim_mint_capability(account: &amp;signer)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_find_delegation"></a>

### Function `find_delegation`


<pre><code>fun find_delegation(addr: address): option::Option&lt;u64&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;Delegations&gt;(@core_resources);<br/></code></pre>




<a id="0x1_aptos_coin_ExistsAptosCoin"></a>


<pre><code>schema ExistsAptosCoin &#123;<br/>requires exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
