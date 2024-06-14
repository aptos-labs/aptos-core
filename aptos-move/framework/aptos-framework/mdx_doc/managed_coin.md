
<a id="0x1_managed_coin"></a>

# Module `0x1::managed_coin`

ManagedCoin is built to make a simple walkthrough of the Coins module.
It contains scripts you will need to initialize, mint, burn, transfer coins.
By utilizing this current module, a developer can create his own coin and care less about mint and burn capabilities,


-  [Resource `Capabilities`](#0x1_managed_coin_Capabilities)
-  [Constants](#@Constants_0)
-  [Function `burn`](#0x1_managed_coin_burn)
-  [Function `initialize`](#0x1_managed_coin_initialize)
-  [Function `mint`](#0x1_managed_coin_mint)
-  [Function `register`](#0x1_managed_coin_register)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `burn`](#@Specification_1_burn)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `mint`](#@Specification_1_mint)
    -  [Function `register`](#@Specification_1_register)


<pre><code><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /></code></pre>



<a id="0x1_managed_coin_Capabilities"></a>

## Resource `Capabilities`

Capabilities resource storing mint and burn capabilities.
The resource is stored on the account that initialized coin <code>CoinType</code>.


<pre><code><b>struct</b> <a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt; <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>freeze_cap: <a href="coin.md#0x1_coin_FreezeCapability">coin::FreezeCapability</a>&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_managed_coin_ENO_CAPABILITIES"></a>

Account has no capabilities (burn/mint).


<pre><code><b>const</b> <a href="managed_coin.md#0x1_managed_coin_ENO_CAPABILITIES">ENO_CAPABILITIES</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_managed_coin_burn"></a>

## Function `burn`

Withdraw an <code>amount</code> of coin <code>CoinType</code> from <code><a href="account.md#0x1_account">account</a></code> and burn it.


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_burn">burn</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_burn">burn</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    amount: u64,<br />) <b>acquires</b> <a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(account_addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_coin.md#0x1_managed_coin_ENO_CAPABILITIES">ENO_CAPABILITIES</a>),<br />    );<br /><br />    <b>let</b> capabilities &#61; <b>borrow_global</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(account_addr);<br /><br />    <b>let</b> to_burn &#61; <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>, amount);<br />    <a href="coin.md#0x1_coin_burn">coin::burn</a>(to_burn, &amp;capabilities.burn_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_managed_coin_initialize"></a>

## Function `initialize`

Initialize new coin <code>CoinType</code> in Aptos Blockchain.
Mint and Burn Capabilities will be stored under <code><a href="account.md#0x1_account">account</a></code> in <code><a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a></code> resource.


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_initialize">initialize</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, decimals: u8, monitor_supply: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_initialize">initialize</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    decimals: u8,<br />    monitor_supply: bool,<br />) &#123;<br />    <b>let</b> (burn_cap, freeze_cap, mint_cap) &#61; <a href="coin.md#0x1_coin_initialize">coin::initialize</a>&lt;CoinType&gt;(<br />        <a href="account.md#0x1_account">account</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(name),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(symbol),<br />        decimals,<br />        monitor_supply,<br />    );<br /><br />    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt; &#123;<br />        burn_cap,<br />        freeze_cap,<br />        mint_cap,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_managed_coin_mint"></a>

## Function `mint`

Create new coins <code>CoinType</code> and deposit them into dst_addr&apos;s account.


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_mint">mint</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dst_addr: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_mint">mint</a>&lt;CoinType&gt;(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    dst_addr: <b>address</b>,<br />    amount: u64,<br />) <b>acquires</b> <a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(account_addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="managed_coin.md#0x1_managed_coin_ENO_CAPABILITIES">ENO_CAPABILITIES</a>),<br />    );<br /><br />    <b>let</b> capabilities &#61; <b>borrow_global</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(account_addr);<br />    <b>let</b> coins_minted &#61; <a href="coin.md#0x1_coin_mint">coin::mint</a>(amount, &amp;capabilities.mint_cap);<br />    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(dst_addr, coins_minted);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_managed_coin_register"></a>

## Function `register`

Creating a resource that stores balance of <code>CoinType</code> on user&apos;s account, withdraw and deposit event handlers.
Required if user wants to start accepting deposits of <code>CoinType</code> in his account.


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_register">register</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_register">register</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>);<br />&#125;<br /></code></pre>



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
<td>The initializing account should hold the capabilities to operate the coin.</td>
<td>Critical</td>
<td>The capabilities are stored under the initializing account under the Capabilities resource, which is distinct for a distinct type of coin.</td>
<td>Enforced via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>A new coin should be properly initialized.</td>
<td>High</td>
<td>In the initialize function, a new coin is initialized via the coin module with the specified properties.</td>
<td>Enforced via <a href="coin.md#high-level-req-2">initialize_internal</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Minting/Burning should only be done by the account who hold the valid capabilities.</td>
<td>High</td>
<td>The mint and burn capabilities are moved under the initializing account and retrieved, while minting/burning</td>
<td>Enforced via: <a href="#high-level-req-3.1">initialize</a>, <a href="#high-level-req-3.2">burn</a>, <a href="#high-level-req-3.3">mint</a>.</td>
</tr>

<tr>
<td>4</td>
<td>If the total supply of coins is being monitored, burn and mint operations will appropriately adjust the total supply.</td>
<td>High</td>
<td>The coin::burn and coin::mint functions, when tracking the supply, adjusts the total coin supply accordingly.</td>
<td>Enforced via <a href="coin.md#high-level-req-4">TotalSupplyNoChange</a>.</td>
</tr>

<tr>
<td>5</td>
<td>Before burning coins, exact amount of coins are withdrawn.</td>
<td>High</td>
<td>After utilizing the coin::withdraw function to withdraw coins, they are then burned, and the function ensures the precise return of the initially specified coin amount.</td>
<td>Enforced via <a href="coin.md#high-level-req-5">burn_from</a>.</td>
</tr>

<tr>
<td>6</td>
<td>Minted coins are deposited to the provided destination address.</td>
<td>High</td>
<td>After the coins are minted via coin::mint they are deposited into the coinstore of the destination address.</td>
<td>Enforced via <a href="#high-level-req-6">mint</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_burn">burn</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> balance &#61; coin_store.<a href="coin.md#0x1_coin">coin</a>.value;<br />// This enforces <a id="high-level-req-3.2" href="#high-level-req">high&#45;level requirement 3</a> and <a id="high-level-req-4.1" href="#high-level-req">high&#45;level requirement 4</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>aborts_if</b> coin_store.frozen;<br /><b>aborts_if</b> balance &lt; amount;<br /><b>let</b> addr &#61;  <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>let</b> maybe_supply &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;CoinType&gt;&gt;(addr).supply;<br /><b>aborts_if</b> amount &#61;&#61; 0;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /><b>include</b> <a href="coin.md#0x1_coin_CoinSubAbortsIf">coin::CoinSubAbortsIf</a>&lt;CoinType&gt; &#123; amount:amount &#125;;<br /><b>ensures</b> <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;CoinType&gt; &#61;&#61; <b>old</b>(<a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;CoinType&gt;) &#45; amount;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_initialize">initialize</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, decimals: u8, monitor_supply: bool)<br /></code></pre>


Make sure <code>name</code> and <code>symbol</code> are legal length.
Only the creator of <code>CoinType</code> can initialize.
The &apos;name&apos; and &apos;symbol&apos; should be valid utf8 bytes
The Capabilities&lt;CoinType&gt; should not be under the signer before creating;
The Capabilities&lt;CoinType&gt; should be under the signer after creating;


<pre><code><b>include</b> <a href="coin.md#0x1_coin_InitializeInternalSchema">coin::InitializeInternalSchema</a>&lt;CoinType&gt;;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(name);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(symbol);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a> and <a id="high-level-req-3.1" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br /></code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_mint">mint</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, dst_addr: <b>address</b>, amount: u64)<br /></code></pre>


The Capabilities&lt;CoinType&gt; should not exist in the signer address.
The <code>dst_addr</code> should not be frozen.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />// This enforces <a id="high-level-req-3.3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="managed_coin.md#0x1_managed_coin_Capabilities">Capabilities</a>&lt;CoinType&gt;&gt;(account_addr);<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;().account_address;<br /><b>aborts_if</b> (amount !&#61; 0) &amp;&amp; !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinInfo">coin::CoinInfo</a>&lt;CoinType&gt;&gt;(addr);<br /><b>let</b> coin_store &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(dst_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(dst_addr);<br /><b>aborts_if</b> coin_store.frozen;<br /><b>include</b> <a href="coin.md#0x1_coin_CoinAddAbortsIf">coin::CoinAddAbortsIf</a>&lt;CoinType&gt;;<br /><b>ensures</b> <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;CoinType&gt; &#61;&#61; <b>old</b>(<a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;CoinType&gt;) &#43; amount;<br />// This enforces <a id="high-level-req-6" href="#high-level-req">high&#45;level requirement 6</a>:
<b>ensures</b> <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(dst_addr).<a href="coin.md#0x1_coin">coin</a>.value &#61;&#61; <b>old</b>(<b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(dst_addr)).<a href="coin.md#0x1_coin">coin</a>.value &#43; amount;<br /></code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code><b>public</b> entry <b>fun</b> <a href="managed_coin.md#0x1_managed_coin_register">register</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


An account can only be registered once.
Updating <code>Account.guid_creation_num</code> will not overflow.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> account_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> acc &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(account_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(account_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;CoinType&gt;();<br /><b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;&gt;(account_addr);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
