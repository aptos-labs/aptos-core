
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


<pre><code>use 0x1::coin;
use 0x1::error;
use 0x1::signer;
use 0x1::string;
</code></pre>



<a id="0x1_managed_coin_Capabilities"></a>

## Resource `Capabilities`

Capabilities resource storing mint and burn capabilities.
The resource is stored on the account that initialized coin <code>CoinType</code>.


<pre><code>struct Capabilities&lt;CoinType&gt; has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_cap: coin::BurnCapability&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>freeze_cap: coin::FreezeCapability&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mint_cap: coin::MintCapability&lt;CoinType&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_managed_coin_ENO_CAPABILITIES"></a>

Account has no capabilities (burn/mint).


<pre><code>const ENO_CAPABILITIES: u64 &#61; 1;
</code></pre>



<a id="0x1_managed_coin_burn"></a>

## Function `burn`

Withdraw an <code>amount</code> of coin <code>CoinType</code> from <code>account</code> and burn it.


<pre><code>public entry fun burn&lt;CoinType&gt;(account: &amp;signer, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn&lt;CoinType&gt;(
    account: &amp;signer,
    amount: u64,
) acquires Capabilities &#123;
    let account_addr &#61; signer::address_of(account);

    assert!(
        exists&lt;Capabilities&lt;CoinType&gt;&gt;(account_addr),
        error::not_found(ENO_CAPABILITIES),
    );

    let capabilities &#61; borrow_global&lt;Capabilities&lt;CoinType&gt;&gt;(account_addr);

    let to_burn &#61; coin::withdraw&lt;CoinType&gt;(account, amount);
    coin::burn(to_burn, &amp;capabilities.burn_cap);
&#125;
</code></pre>



</details>

<a id="0x1_managed_coin_initialize"></a>

## Function `initialize`

Initialize new coin <code>CoinType</code> in Aptos Blockchain.
Mint and Burn Capabilities will be stored under <code>account</code> in <code>Capabilities</code> resource.


<pre><code>public entry fun initialize&lt;CoinType&gt;(account: &amp;signer, name: vector&lt;u8&gt;, symbol: vector&lt;u8&gt;, decimals: u8, monitor_supply: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize&lt;CoinType&gt;(
    account: &amp;signer,
    name: vector&lt;u8&gt;,
    symbol: vector&lt;u8&gt;,
    decimals: u8,
    monitor_supply: bool,
) &#123;
    let (burn_cap, freeze_cap, mint_cap) &#61; coin::initialize&lt;CoinType&gt;(
        account,
        string::utf8(name),
        string::utf8(symbol),
        decimals,
        monitor_supply,
    );

    move_to(account, Capabilities&lt;CoinType&gt; &#123;
        burn_cap,
        freeze_cap,
        mint_cap,
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_managed_coin_mint"></a>

## Function `mint`

Create new coins <code>CoinType</code> and deposit them into dst_addr's account.


<pre><code>public entry fun mint&lt;CoinType&gt;(account: &amp;signer, dst_addr: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint&lt;CoinType&gt;(
    account: &amp;signer,
    dst_addr: address,
    amount: u64,
) acquires Capabilities &#123;
    let account_addr &#61; signer::address_of(account);

    assert!(
        exists&lt;Capabilities&lt;CoinType&gt;&gt;(account_addr),
        error::not_found(ENO_CAPABILITIES),
    );

    let capabilities &#61; borrow_global&lt;Capabilities&lt;CoinType&gt;&gt;(account_addr);
    let coins_minted &#61; coin::mint(amount, &amp;capabilities.mint_cap);
    coin::deposit(dst_addr, coins_minted);
&#125;
</code></pre>



</details>

<a id="0x1_managed_coin_register"></a>

## Function `register`

Creating a resource that stores balance of <code>CoinType</code> on user's account, withdraw and deposit event handlers.
Required if user wants to start accepting deposits of <code>CoinType</code> in his account.


<pre><code>public entry fun register&lt;CoinType&gt;(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun register&lt;CoinType&gt;(account: &amp;signer) &#123;
    coin::register&lt;CoinType&gt;(account);
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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>public entry fun burn&lt;CoinType&gt;(account: &amp;signer, amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let account_addr &#61; signer::address_of(account);
aborts_if !exists&lt;Capabilities&lt;CoinType&gt;&gt;(account_addr);
let coin_store &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr);
let balance &#61; coin_store.coin.value;
// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a> and <a id="high-level-req-4.1" href="#high-level-req">high-level requirement 4</a>:
aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr);
aborts_if coin_store.frozen;
aborts_if balance &lt; amount;
let addr &#61;  type_info::type_of&lt;CoinType&gt;().account_address;
let maybe_supply &#61; global&lt;coin::CoinInfo&lt;CoinType&gt;&gt;(addr).supply;
aborts_if amount &#61;&#61; 0;
aborts_if !exists&lt;coin::CoinInfo&lt;CoinType&gt;&gt;(addr);
include coin::CoinSubAbortsIf&lt;CoinType&gt; &#123; amount:amount &#125;;
ensures coin::supply&lt;CoinType&gt; &#61;&#61; old(coin::supply&lt;CoinType&gt;) &#45; amount;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public entry fun initialize&lt;CoinType&gt;(account: &amp;signer, name: vector&lt;u8&gt;, symbol: vector&lt;u8&gt;, decimals: u8, monitor_supply: bool)
</code></pre>


Make sure <code>name</code> and <code>symbol</code> are legal length.
Only the creator of <code>CoinType</code> can initialize.
The 'name' and 'symbol' should be valid utf8 bytes
The Capabilities<CoinType> should not be under the signer before creating;
The Capabilities<CoinType> should be under the signer after creating;


<pre><code>include coin::InitializeInternalSchema&lt;CoinType&gt;;
aborts_if !string::spec_internal_check_utf8(name);
aborts_if !string::spec_internal_check_utf8(symbol);
aborts_if exists&lt;Capabilities&lt;CoinType&gt;&gt;(signer::address_of(account));
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a> and <a id="high-level-req-3.1" href="#high-level-req">high-level requirement 3</a>:
ensures exists&lt;Capabilities&lt;CoinType&gt;&gt;(signer::address_of(account));
</code></pre>



<a id="@Specification_1_mint"></a>

### Function `mint`


<pre><code>public entry fun mint&lt;CoinType&gt;(account: &amp;signer, dst_addr: address, amount: u64)
</code></pre>


The Capabilities<CoinType> should not exist in the signer address.
The <code>dst_addr</code> should not be frozen.


<pre><code>pragma verify &#61; false;
let account_addr &#61; signer::address_of(account);
// This enforces <a id="high-level-req-3.3" href="#high-level-req">high-level requirement 3</a>:
aborts_if !exists&lt;Capabilities&lt;CoinType&gt;&gt;(account_addr);
let addr &#61; type_info::type_of&lt;CoinType&gt;().account_address;
aborts_if (amount !&#61; 0) &amp;&amp; !exists&lt;coin::CoinInfo&lt;CoinType&gt;&gt;(addr);
let coin_store &#61; global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(dst_addr);
aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(dst_addr);
aborts_if coin_store.frozen;
include coin::CoinAddAbortsIf&lt;CoinType&gt;;
ensures coin::supply&lt;CoinType&gt; &#61;&#61; old(coin::supply&lt;CoinType&gt;) &#43; amount;
// This enforces <a id="high-level-req-6" href="#high-level-req">high-level requirement 6</a>:
ensures global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(dst_addr).coin.value &#61;&#61; old(global&lt;coin::CoinStore&lt;CoinType&gt;&gt;(dst_addr)).coin.value &#43; amount;
</code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code>public entry fun register&lt;CoinType&gt;(account: &amp;signer)
</code></pre>


An account can only be registered once.
Updating <code>Account.guid_creation_num</code> will not overflow.


<pre><code>pragma verify &#61; false;
let account_addr &#61; signer::address_of(account);
let acc &#61; global&lt;account::Account&gt;(account_addr);
aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; acc.guid_creation_num &#43; 2 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; acc.guid_creation_num &#43; 2 &gt; MAX_U64;
aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; !exists&lt;account::Account&gt;(account_addr);
aborts_if !exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr) &amp;&amp; !type_info::spec_is_struct&lt;CoinType&gt;();
ensures exists&lt;coin::CoinStore&lt;CoinType&gt;&gt;(account_addr);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
