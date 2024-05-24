
<a id="0x1_resource_account"></a>

# Module `0x1::resource_account`

A resource account is used to manage resources independent of an account managed by a user.
This contains several utilities to make using resource accounts more effective.


<a id="@Resource_Accounts_to_manage_liquidity_pools_0"></a>

### Resource Accounts to manage liquidity pools


A dev wishing to use resource accounts for a liquidity pool, would likely do the following:

1. Create a new account using <code><a href="resource_account.md#0x1_resource_account_create_resource_account">resource_account::create_resource_account</a></code>. This creates the
account, stores the <code>signer_cap</code> within a <code><a href="resource_account.md#0x1_resource_account_Container">resource_account::Container</a></code>, and rotates the key to
the current account&apos;s authentication key or a provided authentication key.
2. Define the liquidity pool module&apos;s address to be the same as the resource account.
3. Construct a package&#45;publishing transaction for the resource account using the
authentication key used in step 1.
4. In the liquidity pool module&apos;s <code>init_module</code> function, call <code>retrieve_resource_account_cap</code>
which will retrieve the <code>signer_cap</code> and rotate the resource account&apos;s authentication key to
<code>0x0</code>, effectively locking it off.
5. When adding a new coin, the liquidity pool will load the capability and hence the <code><a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a></code> to
register and store new <code>LiquidityCoin</code> resources.

Code snippets to help:

```
fun init_module(resource_account: &amp;signer) &#123;
let dev_address &#61; @DEV_ADDR;
let signer_cap &#61; retrieve_resource_account_cap(resource_account, dev_address);
let lp &#61; LiquidityPoolInfo &#123; signer_cap: signer_cap, ... &#125;;
move_to(resource_account, lp);
&#125;
```

Later on during a coin registration:
```
public fun add_coin&lt;X, Y&gt;(lp: &amp;LP, x: Coin&lt;x&gt;, y: Coin&lt;y&gt;) &#123;
if(!exists&lt;LiquidityCoin&lt;X, Y&gt;(LP::Address(lp), LiquidityCoin&lt;X, Y&gt;)) &#123;
let mint, burn &#61; Coin::initialize&lt;LiquidityCoin&lt;X, Y&gt;&gt;(...);
move_to(&amp;create_signer_with_capability(&amp;lp.cap), LiquidityCoin&lt;X, Y&gt;&#123; mint, burn &#125;);
&#125;
...
&#125;
```

<a id="@Resource_accounts_to_manage_an_account_for_module_publishing_(i.e.,_contract_account)_1"></a>

### Resource accounts to manage an account for module publishing (i.e., contract account)


A dev wishes to have an account dedicated to managing a contract. The contract itself does not
require signer post initialization. The dev could do the following:
1. Create a new account using <code><a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">resource_account::create_resource_account_and_publish_package</a></code>.
This creates the account and publishes the package for that account.
2. At a later point in time, the account creator can move the signer capability to the module.

```
struct MyModuleResource has key &#123;
...
resource_signer_cap: Option&lt;SignerCapability&gt;,
&#125;

public fun provide_signer_capability(resource_signer_cap: SignerCapability) &#123;
let account_addr &#61; account::get_signer_capability_address(resource_signer_cap);
let resource_addr &#61; type_info::account_address(&amp;type_info::type_of&lt;MyModuleResource&gt;());
assert!(account_addr &#61;&#61; resource_addr, EADDRESS_MISMATCH);
let module &#61; borrow_global_mut&lt;MyModuleResource&gt;(account_addr);
module.resource_signer_cap &#61; option::some(resource_signer_cap);
&#125;
```


    -  [Resource Accounts to manage liquidity pools](#@Resource_Accounts_to_manage_liquidity_pools_0)
    -  [Resource accounts to manage an account for module publishing (i.e., contract account)](#@Resource_accounts_to_manage_an_account_for_module_publishing_(i.e.,_contract_account)_1)
-  [Resource `Container`](#0x1_resource_account_Container)
-  [Constants](#@Constants_2)
-  [Function `create_resource_account`](#0x1_resource_account_create_resource_account)
-  [Function `create_resource_account_and_fund`](#0x1_resource_account_create_resource_account_and_fund)
-  [Function `create_resource_account_and_publish_package`](#0x1_resource_account_create_resource_account_and_publish_package)
-  [Function `rotate_account_authentication_key_and_store_capability`](#0x1_resource_account_rotate_account_authentication_key_and_store_capability)
-  [Function `retrieve_resource_account_cap`](#0x1_resource_account_retrieve_resource_account_cap)
-  [Specification](#@Specification_3)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create_resource_account`](#@Specification_3_create_resource_account)
    -  [Function `create_resource_account_and_fund`](#@Specification_3_create_resource_account_and_fund)
    -  [Function `create_resource_account_and_publish_package`](#@Specification_3_create_resource_account_and_publish_package)
    -  [Function `rotate_account_authentication_key_and_store_capability`](#@Specification_3_rotate_account_authentication_key_and_store_capability)
    -  [Function `retrieve_resource_account_cap`](#@Specification_3_retrieve_resource_account_cap)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="code.md#0x1_code">0x1::code</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_resource_account_Container"></a>

## Resource `Container`



<pre><code><b>struct</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_2"></a>

## Constants


<a id="0x1_resource_account_ZERO_AUTH_KEY"></a>



<pre><code><b>const</b> <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];<br /></code></pre>



<a id="0x1_resource_account_ECONTAINER_NOT_PUBLISHED"></a>

Container resource not found in account


<pre><code><b>const</b> <a href="resource_account.md#0x1_resource_account_ECONTAINER_NOT_PUBLISHED">ECONTAINER_NOT_PUBLISHED</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_resource_account_EUNAUTHORIZED_NOT_OWNER"></a>

The resource account was not created by the specified source account


<pre><code><b>const</b> <a href="resource_account.md#0x1_resource_account_EUNAUTHORIZED_NOT_OWNER">EUNAUTHORIZED_NOT_OWNER</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_resource_account_create_resource_account"></a>

## Function `create_resource_account`

Creates a new resource account and rotates the authentication key to either
the optional auth key if it is non&#45;empty (though auth keys are 32&#45;bytes)
or the source accounts current auth key.


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account">create_resource_account</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account">create_resource_account</a>(<br />    origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> &#123;<br />    <b>let</b> (resource, resource_signer_cap) &#61; <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(origin, seed);<br />    <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(<br />        origin,<br />        resource,<br />        resource_signer_cap,<br />        optional_auth_key,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_fund"></a>

## Function `create_resource_account_and_fund`

Creates a new resource account, transfer the amount of coins from the origin to the resource
account, and rotates the authentication key to either the optional auth key if it is
non&#45;empty (though auth keys are 32&#45;bytes) or the source accounts current auth key. Note,
this function adds additional resource ownership to the resource account and should only be
used for resource accounts that need access to <code>Coin&lt;AptosCoin&gt;</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_fund">create_resource_account_and_fund</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fund_amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_fund">create_resource_account_and_fund</a>(<br />    origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    fund_amount: u64,<br />) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> &#123;<br />    <b>let</b> (resource, resource_signer_cap) &#61; <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(origin, seed);<br />    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&amp;resource);<br />    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(origin, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;resource), fund_amount);<br />    <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(<br />        origin,<br />        resource,<br />        resource_signer_cap,<br />        optional_auth_key,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_publish_package"></a>

## Function `create_resource_account_and_publish_package`

Creates a new resource account, publishes the package under this account transaction under
this account and leaves the signer cap readily available for pickup.


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">create_resource_account_and_publish_package</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">create_resource_account_and_publish_package</a>(<br />    origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> &#123;<br />    <b>let</b> (resource, resource_signer_cap) &#61; <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(origin, seed);<br />    aptos_framework::code::publish_package_txn(&amp;resource, metadata_serialized, <a href="code.md#0x1_code">code</a>);<br />    <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(<br />        origin,<br />        resource,<br />        resource_signer_cap,<br />        <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_resource_account_rotate_account_authentication_key_and_store_capability"></a>

## Function `rotate_account_authentication_key_and_store_capability`



<pre><code><b>fun</b> <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(<br />    origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    resource: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    resource_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>,<br />    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> &#123;<br />    <b>let</b> origin_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(origin_addr)) &#123;<br />        <b>move_to</b>(origin, <a href="resource_account.md#0x1_resource_account_Container">Container</a> &#123; store: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>() &#125;)<br />    &#125;;<br /><br />    <b>let</b> container &#61; <b>borrow_global_mut</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(origin_addr);<br />    <b>let</b> resource_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&amp;resource);<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> container.store, resource_addr, resource_signer_cap);<br /><br />    <b>let</b> auth_key &#61; <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;optional_auth_key)) &#123;<br />        <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(origin_addr)<br />    &#125; <b>else</b> &#123;<br />        optional_auth_key<br />    &#125;;<br />    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(&amp;resource, auth_key);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_resource_account_retrieve_resource_account_cap"></a>

## Function `retrieve_resource_account_cap`

When called by the resource account, it will retrieve the capability associated with that
account and rotate the account&apos;s auth key to 0x0 making the account inaccessible without
the SignerCapability.


<pre><code><b>public</b> <b>fun</b> <a href="resource_account.md#0x1_resource_account_retrieve_resource_account_cap">retrieve_resource_account_cap</a>(resource: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, source_addr: <b>address</b>): <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="resource_account.md#0x1_resource_account_retrieve_resource_account_cap">retrieve_resource_account_cap</a>(<br />    resource: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    source_addr: <b>address</b>,<br />): <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a> <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="resource_account.md#0x1_resource_account_ECONTAINER_NOT_PUBLISHED">ECONTAINER_NOT_PUBLISHED</a>));<br /><br />    <b>let</b> resource_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(resource);<br />    <b>let</b> (resource_signer_cap, empty_container) &#61; &#123;<br />        <b>let</b> container &#61; <b>borrow_global_mut</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;container.store, &amp;resource_addr),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="resource_account.md#0x1_resource_account_EUNAUTHORIZED_NOT_OWNER">EUNAUTHORIZED_NOT_OWNER</a>)<br />        );<br />        <b>let</b> (_resource_addr, signer_cap) &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&amp;<b>mut</b> container.store, &amp;resource_addr);<br />        (signer_cap, <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&amp;container.store) &#61;&#61; 0)<br />    &#125;;<br /><br />    <b>if</b> (empty_container) &#123;<br />        <b>let</b> container &#61; <b>move_from</b>(source_addr);<br />        <b>let</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> &#123; store &#125; &#61; container;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_destroy_empty">simple_map::destroy_empty</a>(store);<br />    &#125;;<br /><br />    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(resource, <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>);<br />    resource_signer_cap<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_3"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The length of the authentication key must be 32 bytes.</td>
<td>Medium</td>
<td>The rotate_authentication_key_internal function ensures that the authentication key passed to it is of 32 bytes.</td>
<td>Formally verified via <a href="#high-level-req-1">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The Container structure must exist in the origin account in order to rotate the authentication key of a resource account and to store its signer capability.</td>
<td>High</td>
<td>The rotate_account_authentication_key_and_store_capability function makes sure the Container structure exists under the origin account.</td>
<td>Formally verified via <a href="#high-level-req-2">rotate_account_authentication_key_and_store_capability</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The resource account is registered for the Aptos coin.</td>
<td>High</td>
<td>The create_resource_account_and_fund ensures the newly created resource account is registered to receive the AptosCoin.</td>
<td>Formally verified via <a href="#high-level-req-3">create_resource_account_and_fund</a>.</td>
</tr>

<tr>
<td>4</td>
<td>It is not possible to store two capabilities for the same resource address.</td>
<td>Medium</td>
<td>The rotate_account_authentication_key_and_store_capability will abort if the resource signer capability for the given resource address already exists in container.store.</td>
<td>Formally verified via <a href="#high-level-req-4">rotate_account_authentication_key_and_store_capability</a>.</td>
</tr>

<tr>
<td>5</td>
<td>If provided, the optional authentication key is used for key rotation.</td>
<td>Low</td>
<td>The rotate_account_authentication_key_and_store_capability function will use optional_auth_key if it is provided as a parameter.</td>
<td>Formally verified via <a href="#high-level-req-5">rotate_account_authentication_key_and_store_capability</a>.</td>
</tr>

<tr>
<td>6</td>
<td>The container stores the resource accounts&apos; signer capabilities.</td>
<td>Low</td>
<td>retrieve_resource_account_cap will abort if there is no Container structure assigned to source_addr.</td>
<td>Formally verified via <a href="#high-level-req-6">retreive_resource_account_cap</a>.</td>
</tr>

<tr>
<td>7</td>
<td>Resource account may retrieve the signer capability if it was previously added to its container.</td>
<td>High</td>
<td>retrieve_resource_account_cap will abort if the container of source_addr doesn&apos;t store the signer capability for the given resource.</td>
<td>Formally verified via <a href="#high-level-req-7">retrieve_resource_account_cap</a>.</td>
</tr>

<tr>
<td>8</td>
<td>Retrieving the last signer capability from the container must result in the container being removed.</td>
<td>Low</td>
<td>retrieve_resource_account_cap will remove the container if the retrieved signer_capability was the last one stored under it.</td>
<td>Formally verified via <a href="#high-level-req-8">retrieve_resource_account_cap</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_3_create_resource_account"></a>

### Function `create_resource_account`


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account">create_resource_account</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>let</b> source_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);<br /><b>let</b> resource_addr &#61; <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(source_addr, seed);<br /><b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a>;<br /></code></pre>



<a id="@Specification_3_create_resource_account_and_fund"></a>

### Function `create_resource_account_and_fund`


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_fund">create_resource_account_and_fund</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fund_amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> source_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);<br /><b>let</b> resource_addr &#61; <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(source_addr, seed);<br /><b>let</b> coin_store_resource &#61; <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr);<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">aptos_account::WithdrawAbortsIf</a>&lt;AptosCoin&gt;&#123;from: origin, amount: fund_amount&#125;;<br /><b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">aptos_account::GuidAbortsIf</a>&lt;AptosCoin&gt;&#123;<b>to</b>: resource_addr&#125;;<br /><b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a>;<br /><b>aborts_if</b> <a href="coin.md#0x1_coin_spec_is_account_registered">coin::spec_is_account_registered</a>&lt;AptosCoin&gt;(resource_addr) &amp;&amp; coin_store_resource.frozen;<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> <b>exists</b>&lt;aptos_framework::coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);<br /></code></pre>



<a id="@Specification_3_create_resource_account_and_publish_package"></a>

### Function `create_resource_account_and_publish_package`


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">create_resource_account_and_publish_package</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> source_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);<br /><b>let</b> resource_addr &#61; <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(source_addr, seed);<br /><b>let</b> optional_auth_key &#61; <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>;<br /><b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a>;<br /></code></pre>



<a id="@Specification_3_rotate_account_authentication_key_and_store_capability"></a>

### Function `rotate_account_authentication_key_and_store_capability`


<pre><code><b>fun</b> <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(origin: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>let</b> resource_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(resource);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf</a>;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin));<br />// This enforces <a id="high-level-req-5" href="#high-level-req">high&#45;level requirement 5</a>:
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(optional_auth_key) !&#61; 0 &#61;&#61;&gt;<br />    <b>global</b>&lt;aptos_framework::account::Account&gt;(resource_addr).authentication_key &#61;&#61; optional_auth_key;<br /></code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf"></a>


<pre><code><b>schema</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf</a> &#123;<br />origin: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />resource_addr: <b>address</b>;<br />optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>let</b> source_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);<br /><b>let</b> container &#61; <b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br /><b>let</b> get &#61; len(optional_auth_key) &#61;&#61; 0;<br /><b>aborts_if</b> get &amp;&amp; !<b>exists</b>&lt;Account&gt;(source_addr);<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
    <b>aborts_if</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr) &amp;&amp; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(container.store, resource_addr);<br /><b>aborts_if</b> get &amp;&amp; !(<b>exists</b>&lt;Account&gt;(resource_addr) &amp;&amp; len(<b>global</b>&lt;Account&gt;(source_addr).authentication_key) &#61;&#61; 32);<br /><b>aborts_if</b> !get &amp;&amp; !(<b>exists</b>&lt;Account&gt;(resource_addr) &amp;&amp; len(optional_auth_key) &#61;&#61; 32);<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr).store, resource_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br />&#125;<br /></code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit"></a>


<pre><code><b>schema</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a> &#123;<br />source_addr: <b>address</b>;<br />optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />resource_addr: <b>address</b>;<br /><b>let</b> container &#61; <b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br /><b>let</b> get &#61; len(optional_auth_key) &#61;&#61; 0;<br /><b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(source_addr);<br /><b>requires</b> source_addr !&#61; resource_addr;<br /><b>aborts_if</b> len(<a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>) !&#61; 32;<br /><b>include</b> <a href="account.md#0x1_account_exists_at">account::exists_at</a>(resource_addr) &#61;&#61;&gt; <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">account::CreateResourceAccountAbortsIf</a>;<br /><b>include</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(resource_addr) &#61;&#61;&gt; <a href="account.md#0x1_account_CreateAccountAbortsIf">account::CreateAccountAbortsIf</a> &#123;addr: resource_addr&#125;;<br /><b>aborts_if</b> get &amp;&amp; !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(source_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr) &amp;&amp; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(container.store, resource_addr);<br /><b>aborts_if</b> get &amp;&amp; len(<b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(source_addr).authentication_key) !&#61; 32;<br /><b>aborts_if</b> !get &amp;&amp; len(optional_auth_key) !&#61; 32;<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr).store, resource_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br />&#125;<br /></code></pre>



<a id="@Specification_3_retrieve_resource_account_cap"></a>

### Function `retrieve_resource_account_cap`


<pre><code><b>public</b> <b>fun</b> <a href="resource_account.md#0x1_resource_account_retrieve_resource_account_cap">retrieve_resource_account_cap</a>(resource: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, source_addr: <b>address</b>): <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a><br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-6" href="#high-level-req">high&#45;level requirement 6</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br /><b>let</b> resource_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(resource);<br /><b>let</b> container &#61; <b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br />// This enforces <a id="high-level-req-7" href="#high-level-req">high&#45;level requirement 7</a>:
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(container.store, resource_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);<br />// This enforces <a id="high-level-req-8" href="#high-level-req">high&#45;level requirement 8</a>:
<b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>old</b>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr)).store, resource_addr) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_len">simple_map::spec_len</a>(<b>old</b>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr)).store) &#61;&#61; 1 &#61;&#61;&gt; !<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr) &#61;&#61;&gt; !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr).store, resource_addr);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
