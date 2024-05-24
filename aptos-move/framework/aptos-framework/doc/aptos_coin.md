
<a id="0x1_aptos_coin"></a>

# Module `0x1::aptos_coin`

This module defines a minimal and generic Coin and Balance.
modified from https://github.com/move&#45;language/move/tree/main/language/documentation/tutorial


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


<pre><code><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_aptos_coin_AptosCoin"></a>

## Resource `AptosCoin`



<pre><code><b>struct</b> <a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a> <b>has</b> key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_coin_DelegatedMintCapability"></a>

## Struct `DelegatedMintCapability`

Delegation token created by delegator and can be claimed by the delegatee as MintCapability.


<pre><code><b>struct</b> <a href="aptos_coin.md#0x1_aptos_coin_DelegatedMintCapability">DelegatedMintCapability</a> <b>has</b> store<br /></code></pre>



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

<a id="0x1_aptos_coin_Delegations"></a>

## Resource `Delegations`

The container stores the current pending delegations.


<pre><code><b>struct</b> <a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_DelegatedMintCapability">aptos_coin::DelegatedMintCapability</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aptos_coin_EALREADY_DELEGATED"></a>

Mint capability has already been delegated to this specified address


<pre><code><b>const</b> <a href="aptos_coin.md#0x1_aptos_coin_EALREADY_DELEGATED">EALREADY_DELEGATED</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_aptos_coin_EDELEGATION_NOT_FOUND"></a>

Cannot find delegation of mint capability to this account


<pre><code><b>const</b> <a href="aptos_coin.md#0x1_aptos_coin_EDELEGATION_NOT_FOUND">EDELEGATION_NOT_FOUND</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_aptos_coin_ENO_CAPABILITIES"></a>

Account does not have mint capability


<pre><code><b>const</b> <a href="aptos_coin.md#0x1_aptos_coin_ENO_CAPABILITIES">ENO_CAPABILITIES</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_aptos_coin_initialize"></a>

## Function `initialize`

Can only called during genesis to initialize the Aptos coin.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): (BurnCapability&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;, MintCapability&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <b>let</b> (burn_cap, freeze_cap, mint_cap) &#61; <a href="coin.md#0x1_coin_initialize_with_parallelizable_supply">coin::initialize_with_parallelizable_supply</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;(<br />        aptos_framework,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;Aptos Coin&quot;),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;APT&quot;),<br />        8, // decimals<br />        <b>true</b>, // monitor_supply<br />    );<br /><br />    // Aptos framework needs mint cap <b>to</b> mint coins <b>to</b> initial validators. This will be revoked once the validators<br />    // have been initialized.<br />    <b>move_to</b>(aptos_framework, <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> &#123; mint_cap &#125;);<br /><br />    <a href="coin.md#0x1_coin_destroy_freeze_cap">coin::destroy_freeze_cap</a>(freeze_cap);<br />    (burn_cap, mint_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_coin_has_mint_capability"></a>

## Function `has_mint_capability`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_has_mint_capability">has_mint_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_has_mint_capability">has_mint_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool &#123;<br />    <b>exists</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_coin_destroy_mint_cap"></a>

## Function `destroy_mint_cap`

Only called during genesis to destroy the aptos framework account&apos;s mint capability once all initial validators
and accounts have been initialized during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_destroy_mint_cap">destroy_mint_cap</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_destroy_mint_cap">destroy_mint_cap</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>let</b> <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> &#123; mint_cap &#125; &#61; <b>move_from</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(@aptos_framework);<br />    <a href="coin.md#0x1_coin_destroy_mint_cap">coin::destroy_mint_cap</a>(mint_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_coin_configure_accounts_for_test"></a>

## Function `configure_accounts_for_test`

Can only be called during genesis for tests to grant mint capability to aptos framework and core resources
accounts.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_configure_accounts_for_test">configure_accounts_for_test</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_configure_accounts_for_test">configure_accounts_for_test</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    mint_cap: MintCapability&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;,<br />) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    // Mint the core resource <a href="account.md#0x1_account">account</a> <a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a> for gas so it can execute system transactions.<br />    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;(core_resources);<br />    <b>let</b> coins &#61; <a href="coin.md#0x1_coin_mint">coin::mint</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;(<br />        18446744073709551615,<br />        &amp;mint_cap,<br />    );<br />    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(core_resources), coins);<br /><br />    <b>move_to</b>(core_resources, <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> &#123; mint_cap &#125;);<br />    <b>move_to</b>(core_resources, <a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a> &#123; inner: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>() &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_coin_mint"></a>

## Function `mint`

Only callable in tests and testnets where the core resources account exists.
Create new coins and deposit them into dst_addr&apos;s account.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_mint">mint</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dst_addr: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_mint">mint</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    dst_addr: <b>address</b>,<br />    amount: u64,<br />) <b>acquires</b> <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(account_addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_coin.md#0x1_aptos_coin_ENO_CAPABILITIES">ENO_CAPABILITIES</a>),<br />    );<br /><br />    <b>let</b> mint_cap &#61; &amp;<b>borrow_global</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(account_addr).mint_cap;<br />    <b>let</b> coins_minted &#61; <a href="coin.md#0x1_coin_mint">coin::mint</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;(amount, mint_cap);<br />    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;(dst_addr, coins_minted);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_coin_delegate_mint_capability"></a>

## Function `delegate_mint_capability`

Only callable in tests and testnets where the core resources account exists.
Create delegated token for the address so the account could claim MintCapability later.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_delegate_mint_capability">delegate_mint_capability</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_delegate_mint_capability">delegate_mint_capability</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>) <b>acquires</b> <a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_core_resource">system_addresses::assert_core_resource</a>(&amp;<a href="account.md#0x1_account">account</a>);<br />    <b>let</b> delegations &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a>&gt;(@core_resources).inner;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(delegations, &#124;element&#124; &#123;<br />        <b>let</b> element: &amp;<a href="aptos_coin.md#0x1_aptos_coin_DelegatedMintCapability">DelegatedMintCapability</a> &#61; element;<br />        <b>assert</b>!(element.<b>to</b> !&#61; <b>to</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_coin.md#0x1_aptos_coin_EALREADY_DELEGATED">EALREADY_DELEGATED</a>));<br />    &#125;);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(delegations, <a href="aptos_coin.md#0x1_aptos_coin_DelegatedMintCapability">DelegatedMintCapability</a> &#123; <b>to</b> &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_coin_claim_mint_capability"></a>

## Function `claim_mint_capability`

Only callable in tests and testnets where the core resources account exists.
Claim the delegated mint capability and destroy the delegated token.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_claim_mint_capability">claim_mint_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_claim_mint_capability">claim_mint_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a>, <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> &#123;<br />    <b>let</b> maybe_index &#61; <a href="aptos_coin.md#0x1_aptos_coin_find_delegation">find_delegation</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;maybe_index), <a href="aptos_coin.md#0x1_aptos_coin_EDELEGATION_NOT_FOUND">EDELEGATION_NOT_FOUND</a>);<br />    <b>let</b> idx &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;maybe_index);<br />    <b>let</b> delegations &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a>&gt;(@core_resources).inner;<br />    <b>let</b> <a href="aptos_coin.md#0x1_aptos_coin_DelegatedMintCapability">DelegatedMintCapability</a> &#123; <b>to</b>: _ &#125; &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(delegations, idx);<br /><br />    // Make a <b>copy</b> of mint cap and give it <b>to</b> the specified <a href="account.md#0x1_account">account</a>.<br />    <b>let</b> mint_cap &#61; <b>borrow_global</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(@core_resources).mint_cap;<br />    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a> &#123; mint_cap &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_coin_find_delegation"></a>

## Function `find_delegation`



<pre><code><b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_find_delegation">find_delegation</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_find_delegation">find_delegation</a>(addr: <b>address</b>): Option&lt;u64&gt; <b>acquires</b> <a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a> &#123;<br />    <b>let</b> delegations &#61; &amp;<b>borrow_global</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a>&gt;(@core_resources).inner;<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(delegations);<br />    <b>let</b> index &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();<br />    <b>while</b> (i &lt; len) &#123;<br />        <b>let</b> element &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(delegations, i);<br />        <b>if</b> (element.<b>to</b> &#61;&#61; addr) &#123;<br />            index &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(i);<br />            <b>break</b><br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    index<br />&#125;<br /></code></pre>



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


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): (<a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;, <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;Aptos Coin&quot;);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(b&quot;APT&quot;);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aggregator_factory.md#0x1_aggregator_factory_AggregatorFactory">aggregator_factory::AggregatorFactory</a>&gt;(addr);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(addr);<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> <b>global</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(addr).mint_cap &#61;&#61;  MintCapability&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt; &#123;&#125;;<br /><b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;&gt;(addr);<br /><b>ensures</b> result_1 &#61;&#61; BurnCapability&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt; &#123;&#125;;<br /><b>ensures</b> result_2 &#61;&#61; MintCapability&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt; &#123;&#125;;<br /></code></pre>



<a id="@Specification_1_destroy_mint_cap"></a>

### Function `destroy_mint_cap`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_destroy_mint_cap">destroy_mint_cap</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">MintCapStore</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_configure_accounts_for_test"></a>

### Function `configure_accounts_for_test`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_configure_accounts_for_test">configure_accounts_for_test</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_mint">mint</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dst_addr: <b>address</b>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_delegate_mint_capability"></a>

### Function `delegate_mint_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_delegate_mint_capability">delegate_mint_capability</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_claim_mint_capability"></a>

### Function `claim_mint_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_claim_mint_capability">claim_mint_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_find_delegation"></a>

### Function `find_delegation`


<pre><code><b>fun</b> <a href="aptos_coin.md#0x1_aptos_coin_find_delegation">find_delegation</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_Delegations">Delegations</a>&gt;(@core_resources);<br /></code></pre>




<a id="0x1_aptos_coin_ExistsAptosCoin"></a>


<pre><code><b>schema</b> <a href="aptos_coin.md#0x1_aptos_coin_ExistsAptosCoin">ExistsAptosCoin</a> &#123;<br /><b>requires</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">AptosCoin</a>&gt;&gt;(@aptos_framework);<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
