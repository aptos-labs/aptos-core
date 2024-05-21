
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


<pre><code>use 0x1::coin;
use 0x1::error;
use 0x1::option;
use 0x1::signer;
use 0x1::string;
use 0x1::system_addresses;
use 0x1::vector;
</code></pre>



<a id="0x1_aptos_coin_AptosCoin"></a>

## Resource `AptosCoin`



<pre><code>struct AptosCoin has key
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

<a id="0x1_aptos_coin_MintCapStore"></a>

## Resource `MintCapStore`



<pre><code>struct MintCapStore has key
</code></pre>



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


<pre><code>struct DelegatedMintCapability has store
</code></pre>



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


<pre><code>struct Delegations has key
</code></pre>



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


<pre><code>const EALREADY_DELEGATED: u64 &#61; 2;
</code></pre>



<a id="0x1_aptos_coin_EDELEGATION_NOT_FOUND"></a>

Cannot find delegation of mint capability to this account


<pre><code>const EDELEGATION_NOT_FOUND: u64 &#61; 3;
</code></pre>



<a id="0x1_aptos_coin_ENO_CAPABILITIES"></a>

Account does not have mint capability


<pre><code>const ENO_CAPABILITIES: u64 &#61; 1;
</code></pre>



<a id="0x1_aptos_coin_initialize"></a>

## Function `initialize`

Can only called during genesis to initialize the Aptos coin.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer): (coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;, coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer): (BurnCapability&lt;AptosCoin&gt;, MintCapability&lt;AptosCoin&gt;) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);

    let (burn_cap, freeze_cap, mint_cap) &#61; coin::initialize_with_parallelizable_supply&lt;AptosCoin&gt;(
        aptos_framework,
        string::utf8(b&quot;Aptos Coin&quot;),
        string::utf8(b&quot;APT&quot;),
        8, // decimals
        true, // monitor_supply
    );

    // Aptos framework needs mint cap to mint coins to initial validators. This will be revoked once the validators
    // have been initialized.
    move_to(aptos_framework, MintCapStore &#123; mint_cap &#125;);

    coin::destroy_freeze_cap(freeze_cap);
    (burn_cap, mint_cap)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_coin_has_mint_capability"></a>

## Function `has_mint_capability`



<pre><code>public fun has_mint_capability(account: &amp;signer): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun has_mint_capability(account: &amp;signer): bool &#123;
    exists&lt;MintCapStore&gt;(signer::address_of(account))
&#125;
</code></pre>



</details>

<a id="0x1_aptos_coin_destroy_mint_cap"></a>

## Function `destroy_mint_cap`

Only called during genesis to destroy the aptos framework account's mint capability once all initial validators
and accounts have been initialized during genesis.


<pre><code>public(friend) fun destroy_mint_cap(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun destroy_mint_cap(aptos_framework: &amp;signer) acquires MintCapStore &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    let MintCapStore &#123; mint_cap &#125; &#61; move_from&lt;MintCapStore&gt;(@aptos_framework);
    coin::destroy_mint_cap(mint_cap);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_coin_configure_accounts_for_test"></a>

## Function `configure_accounts_for_test`

Can only be called during genesis for tests to grant mint capability to aptos framework and core resources
accounts.


<pre><code>public(friend) fun configure_accounts_for_test(aptos_framework: &amp;signer, core_resources: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun configure_accounts_for_test(
    aptos_framework: &amp;signer,
    core_resources: &amp;signer,
    mint_cap: MintCapability&lt;AptosCoin&gt;,
) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);

    // Mint the core resource account AptosCoin for gas so it can execute system transactions.
    coin::register&lt;AptosCoin&gt;(core_resources);
    let coins &#61; coin::mint&lt;AptosCoin&gt;(
        18446744073709551615,
        &amp;mint_cap,
    );
    coin::deposit&lt;AptosCoin&gt;(signer::address_of(core_resources), coins);

    move_to(core_resources, MintCapStore &#123; mint_cap &#125;);
    move_to(core_resources, Delegations &#123; inner: vector::empty() &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_coin_mint"></a>

## Function `mint`

Only callable in tests and testnets where the core resources account exists.
Create new coins and deposit them into dst_addr's account.


<pre><code>public entry fun mint(account: &amp;signer, dst_addr: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint(
    account: &amp;signer,
    dst_addr: address,
    amount: u64,
) acquires MintCapStore &#123;
    let account_addr &#61; signer::address_of(account);

    assert!(
        exists&lt;MintCapStore&gt;(account_addr),
        error::not_found(ENO_CAPABILITIES),
    );

    let mint_cap &#61; &amp;borrow_global&lt;MintCapStore&gt;(account_addr).mint_cap;
    let coins_minted &#61; coin::mint&lt;AptosCoin&gt;(amount, mint_cap);
    coin::deposit&lt;AptosCoin&gt;(dst_addr, coins_minted);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_coin_delegate_mint_capability"></a>

## Function `delegate_mint_capability`

Only callable in tests and testnets where the core resources account exists.
Create delegated token for the address so the account could claim MintCapability later.


<pre><code>public entry fun delegate_mint_capability(account: signer, to: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun delegate_mint_capability(account: signer, to: address) acquires Delegations &#123;
    system_addresses::assert_core_resource(&amp;account);
    let delegations &#61; &amp;mut borrow_global_mut&lt;Delegations&gt;(@core_resources).inner;
    vector::for_each_ref(delegations, &#124;element&#124; &#123;
        let element: &amp;DelegatedMintCapability &#61; element;
        assert!(element.to !&#61; to, error::invalid_argument(EALREADY_DELEGATED));
    &#125;);
    vector::push_back(delegations, DelegatedMintCapability &#123; to &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_coin_claim_mint_capability"></a>

## Function `claim_mint_capability`

Only callable in tests and testnets where the core resources account exists.
Claim the delegated mint capability and destroy the delegated token.


<pre><code>public entry fun claim_mint_capability(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun claim_mint_capability(account: &amp;signer) acquires Delegations, MintCapStore &#123;
    let maybe_index &#61; find_delegation(signer::address_of(account));
    assert!(option::is_some(&amp;maybe_index), EDELEGATION_NOT_FOUND);
    let idx &#61; &#42;option::borrow(&amp;maybe_index);
    let delegations &#61; &amp;mut borrow_global_mut&lt;Delegations&gt;(@core_resources).inner;
    let DelegatedMintCapability &#123; to: _ &#125; &#61; vector::swap_remove(delegations, idx);

    // Make a copy of mint cap and give it to the specified account.
    let mint_cap &#61; borrow_global&lt;MintCapStore&gt;(@core_resources).mint_cap;
    move_to(account, MintCapStore &#123; mint_cap &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_coin_find_delegation"></a>

## Function `find_delegation`



<pre><code>fun find_delegation(addr: address): option::Option&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun find_delegation(addr: address): Option&lt;u64&gt; acquires Delegations &#123;
    let delegations &#61; &amp;borrow_global&lt;Delegations&gt;(@core_resources).inner;
    let i &#61; 0;
    let len &#61; vector::length(delegations);
    let index &#61; option::none();
    while (i &lt; len) &#123;
        let element &#61; vector::borrow(delegations, i);
        if (element.to &#61;&#61; addr) &#123;
            index &#61; option::some(i);
            break
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    index
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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer): (coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;, coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>




<pre><code>let addr &#61; signer::address_of(aptos_framework);
aborts_if addr !&#61; @aptos_framework;
aborts_if !string::spec_internal_check_utf8(b&quot;Aptos Coin&quot;);
aborts_if !string::spec_internal_check_utf8(b&quot;APT&quot;);
aborts_if exists&lt;MintCapStore&gt;(addr);
aborts_if exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);
aborts_if !exists&lt;aggregator_factory::AggregatorFactory&gt;(addr);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures exists&lt;MintCapStore&gt;(addr);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures global&lt;MintCapStore&gt;(addr).mint_cap &#61;&#61;  MintCapability&lt;AptosCoin&gt; &#123;&#125;;
ensures exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(addr);
ensures result_1 &#61;&#61; BurnCapability&lt;AptosCoin&gt; &#123;&#125;;
ensures result_2 &#61;&#61; MintCapability&lt;AptosCoin&gt; &#123;&#125;;
</code></pre>



<a id="@Specification_1_destroy_mint_cap"></a>

### Function `destroy_mint_cap`


<pre><code>public(friend) fun destroy_mint_cap(aptos_framework: &amp;signer)
</code></pre>




<pre><code>let addr &#61; signer::address_of(aptos_framework);
aborts_if addr !&#61; @aptos_framework;
aborts_if !exists&lt;MintCapStore&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_configure_accounts_for_test"></a>

### Function `configure_accounts_for_test`


<pre><code>public(friend) fun configure_accounts_for_test(aptos_framework: &amp;signer, core_resources: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code>public entry fun mint(account: &amp;signer, dst_addr: address, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_delegate_mint_capability"></a>

### Function `delegate_mint_capability`


<pre><code>public entry fun delegate_mint_capability(account: signer, to: address)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_claim_mint_capability"></a>

### Function `claim_mint_capability`


<pre><code>public entry fun claim_mint_capability(account: &amp;signer)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_find_delegation"></a>

### Function `find_delegation`


<pre><code>fun find_delegation(addr: address): option::Option&lt;u64&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;Delegations&gt;(@core_resources);
</code></pre>




<a id="0x1_aptos_coin_ExistsAptosCoin"></a>


<pre><code>schema ExistsAptosCoin &#123;
    requires exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
