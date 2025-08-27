
<a id="0x1_resource_account"></a>

# Module `0x1::resource_account`

A resource account is used to manage resources independent of an account managed by a user.
This contains several utilities to make using resource accounts more effective.


<a id="@Resource_Accounts_to_manage_liquidity_pools_0"></a>

### Resource Accounts to manage liquidity pools


A dev wishing to use resource accounts for a liquidity pool, would likely do the following:

1. Create a new account using <code><a href="resource_account.md#0x1_resource_account_create_resource_account">resource_account::create_resource_account</a></code>. This creates the
account, stores the <code>signer_cap</code> within a <code><a href="resource_account.md#0x1_resource_account_Container">resource_account::Container</a></code>, and rotates the key to
the current account's authentication key or a provided authentication key.
2. Define the liquidity pool module's address to be the same as the resource account.
3. Construct a package-publishing transaction for the resource account using the
authentication key used in step 1.
4. In the liquidity pool module's <code>init_module</code> function, call <code>retrieve_resource_account_cap</code>
which will retrieve the <code>signer_cap</code> and rotate the resource account's authentication key to
<code>0x0</code>, effectively locking it off.
5. When adding a new coin, the liquidity pool will load the capability and hence the <code><a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a></code> to
register and store new <code>LiquidityCoin</code> resources.

Code snippets to help:

```
fun init_module(resource_account: &signer) {
let dev_address = @DEV_ADDR;
let signer_cap = retrieve_resource_account_cap(resource_account, dev_address);
let lp = LiquidityPoolInfo { signer_cap: signer_cap, ... };
move_to(resource_account, lp);
}
```

Later on during a coin registration:
```
public fun add_coin<X, Y>(lp: &LP, x: Coin<x>, y: Coin<y>) {
if(!exists<LiquidityCoin<X, Y>(LP::Address(lp), LiquidityCoin<X, Y>)) {
let mint, burn = Coin::initialize<LiquidityCoin<X, Y>>(...);
move_to(&create_signer_with_capability(&lp.cap), LiquidityCoin<X, Y>{ mint, burn });
}
...
}
```

<a id="@Resource_accounts_to_manage_an_account_for_module_publishing_(i.e.,_contract_account)_1"></a>

### Resource accounts to manage an account for module publishing (i.e., contract account)


A dev wishes to have an account dedicated to managing a contract. The contract itself does not
require signer post initialization. The dev could do the following:
1. Create a new account using <code><a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">resource_account::create_resource_account_and_publish_package</a></code>.
This creates the account and publishes the package for that account.
2. At a later point in time, the account creator can move the signer capability to the module.

```
struct MyModuleResource has key {
...
resource_signer_cap: Option<SignerCapability>,
}

public fun provide_signer_capability(resource_signer_cap: SignerCapability) {
let account_addr = account::get_signer_capability_address(resource_signer_cap);
let resource_addr = type_info::account_address(&type_info::type_of<MyModuleResource>());
assert!(account_addr == resource_addr, EADDRESS_MISMATCH);
let module = borrow_global_mut<MyModuleResource>(account_addr);
module.resource_signer_cap = option::some(resource_signer_cap);
}
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="code.md#0x1_code">0x1::code</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
</code></pre>



<a id="0x1_resource_account_Container"></a>

## Resource `Container`



<pre><code><b>struct</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> <b>has</b> key
</code></pre>



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



<pre><code><b>const</b> <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a id="0x1_resource_account_ECONTAINER_NOT_PUBLISHED"></a>

Container resource not found in account


<pre><code><b>const</b> <a href="resource_account.md#0x1_resource_account_ECONTAINER_NOT_PUBLISHED">ECONTAINER_NOT_PUBLISHED</a>: u64 = 1;
</code></pre>



<a id="0x1_resource_account_EUNAUTHORIZED_NOT_OWNER"></a>

The resource account was not created by the specified source account


<pre><code><b>const</b> <a href="resource_account.md#0x1_resource_account_EUNAUTHORIZED_NOT_OWNER">EUNAUTHORIZED_NOT_OWNER</a>: u64 = 2;
</code></pre>



<a id="0x1_resource_account_create_resource_account"></a>

## Function `create_resource_account`

Creates a new resource account and rotates the authentication key to either
the optional auth key if it is non-empty (though auth keys are 32-bytes)
or the source accounts current auth key.


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account">create_resource_account</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account">create_resource_account</a>(
    origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> {
    <b>let</b> (resource, resource_signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(origin, seed);
    <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(
        origin,
        resource,
        resource_signer_cap,
        optional_auth_key,
    );
}
</code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_fund"></a>

## Function `create_resource_account_and_fund`

Creates a new resource account, transfer the amount of coins from the origin to the resource
account, and rotates the authentication key to either the optional auth key if it is
non-empty (though auth keys are 32-bytes) or the source accounts current auth key. Note,
this function adds additional resource ownership to the resource account and should only be
used for resource accounts that need access to <code>Coin&lt;AptosCoin&gt;</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_fund">create_resource_account_and_fund</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fund_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_fund">create_resource_account_and_fund</a>(
    origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    fund_amount: u64,
) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> {
    <b>let</b> (resource, resource_signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(origin, seed);
    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&resource);
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(origin, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&resource), fund_amount);
    <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(
        origin,
        resource,
        resource_signer_cap,
        optional_auth_key,
    );
}
</code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_publish_package"></a>

## Function `create_resource_account_and_publish_package`

Creates a new resource account, publishes the package under this account transaction under
this account and leaves the signer cap readily available for pickup.


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">create_resource_account_and_publish_package</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">create_resource_account_and_publish_package</a>(
    origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> {
    <b>let</b> (resource, resource_signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(origin, seed);
    aptos_framework::code::publish_package_txn(&resource, metadata_serialized, <a href="code.md#0x1_code">code</a>);
    <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(
        origin,
        resource,
        resource_signer_cap,
        <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>,
    );
}
</code></pre>



</details>

<a id="0x1_resource_account_rotate_account_authentication_key_and_store_capability"></a>

## Function `rotate_account_authentication_key_and_store_capability`



<pre><code><b>fun</b> <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(
    origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    resource: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    resource_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>,
    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> {
    <b>let</b> origin_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);
    <b>if</b> (!<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(origin_addr)) {
        <b>move_to</b>(origin, <a href="resource_account.md#0x1_resource_account_Container">Container</a> { store: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>() })
    };

    <b>let</b> container = <b>borrow_global_mut</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(origin_addr);
    <b>let</b> resource_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&resource);
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> container.store, resource_addr, resource_signer_cap);

    <b>let</b> auth_key = <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&optional_auth_key)) {
        <a href="account.md#0x1_account_get_authentication_key">account::get_authentication_key</a>(origin_addr)
    } <b>else</b> {
        optional_auth_key
    };
    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(&resource, auth_key);
}
</code></pre>



</details>

<a id="0x1_resource_account_retrieve_resource_account_cap"></a>

## Function `retrieve_resource_account_cap`

When called by the resource account, it will retrieve the capability associated with that
account and rotate the account's auth key to 0x0 making the account inaccessible without
the SignerCapability.


<pre><code><b>public</b> <b>fun</b> <a href="resource_account.md#0x1_resource_account_retrieve_resource_account_cap">retrieve_resource_account_cap</a>(resource: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, source_addr: <b>address</b>): <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="resource_account.md#0x1_resource_account_retrieve_resource_account_cap">retrieve_resource_account_cap</a>(
    resource: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    source_addr: <b>address</b>,
): <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a> <b>acquires</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="resource_account.md#0x1_resource_account_ECONTAINER_NOT_PUBLISHED">ECONTAINER_NOT_PUBLISHED</a>));

    <b>let</b> resource_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(resource);
    <b>let</b> (resource_signer_cap, empty_container) = {
        <b>let</b> container = <b>borrow_global_mut</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&container.store, &resource_addr),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="resource_account.md#0x1_resource_account_EUNAUTHORIZED_NOT_OWNER">EUNAUTHORIZED_NOT_OWNER</a>)
        );
        <b>let</b> (_resource_addr, signer_cap) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> container.store, &resource_addr);
        (signer_cap, <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_length">simple_map::length</a>(&container.store) == 0)
    };

    <b>if</b> (empty_container) {
        <b>let</b> container = <b>move_from</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
        <b>let</b> <a href="resource_account.md#0x1_resource_account_Container">Container</a> { store } = container;
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_destroy_empty">simple_map::destroy_empty</a>(store);
    };

    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(resource, <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>);
    resource_signer_cap
}
</code></pre>



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
<td>The container stores the resource accounts' signer capabilities.</td>
<td>Low</td>
<td>retrieve_resource_account_cap will abort if there is no Container structure assigned to source_addr.</td>
<td>Formally verified via <a href="#high-level-req-6">retreive_resource_account_cap</a>.</td>
</tr>

<tr>
<td>7</td>
<td>Resource account may retrieve the signer capability if it was previously added to its container.</td>
<td>High</td>
<td>retrieve_resource_account_cap will abort if the container of source_addr doesn't store the signer capability for the given resource.</td>
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


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_partial;
</code></pre>



<a id="@Specification_3_create_resource_account"></a>

### Function `create_resource_account`


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account">create_resource_account</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>let</b> source_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);
<b>let</b> resource_addr = <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(source_addr, seed);
<b>let</b> resource = <a href="create_signer.md#0x1_create_signer_spec_create_signer">create_signer::spec_create_signer</a>(resource_addr);
<b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a>;
</code></pre>



<a id="@Specification_3_create_resource_account_and_fund"></a>

### Function `create_resource_account_and_fund`


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_fund">create_resource_account_and_fund</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, fund_amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> source_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);
<b>let</b> resource_addr = <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(source_addr, seed);
<b>let</b> coin_store_resource = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(resource_addr);
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_WithdrawAbortsIf">aptos_account::WithdrawAbortsIf</a>&lt;AptosCoin&gt;{from: origin, amount: fund_amount};
<b>include</b> <a href="aptos_account.md#0x1_aptos_account_GuidAbortsIf">aptos_account::GuidAbortsIf</a>&lt;AptosCoin&gt;{<b>to</b>: resource_addr};
<b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a>;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>ensures</b> <b>exists</b>&lt;aptos_framework::coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);
</code></pre>



<a id="@Specification_3_create_resource_account_and_publish_package"></a>

### Function `create_resource_account_and_publish_package`


<pre><code><b>public</b> entry <b>fun</b> <a href="resource_account.md#0x1_resource_account_create_resource_account_and_publish_package">create_resource_account_and_publish_package</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> source_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);
<b>let</b> resource_addr = <a href="account.md#0x1_account_spec_create_resource_address">account::spec_create_resource_address</a>(source_addr, seed);
<b>let</b> optional_auth_key = <a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>;
<b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a>;
</code></pre>



<a id="@Specification_3_rotate_account_authentication_key_and_store_capability"></a>

### Function `rotate_account_authentication_key_and_store_capability`


<pre><code><b>fun</b> <a href="resource_account.md#0x1_resource_account_rotate_account_authentication_key_and_store_capability">rotate_account_authentication_key_and_store_capability</a>(origin: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, resource_signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>, optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> resource_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(resource);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>include</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf</a>;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin));
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(optional_auth_key) != 0 ==&gt;
    <b>global</b>&lt;aptos_framework::account::Account&gt;(resource_addr).authentication_key == optional_auth_key;
</code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf"></a>


<pre><code><b>schema</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf</a> {
    origin: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    resource_addr: <b>address</b>;
    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    <b>let</b> source_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(origin);
    <b>let</b> container = <b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
    <b>let</b> get = len(optional_auth_key) == 0;
    <b>aborts_if</b> get && !<b>exists</b>&lt;Account&gt;(source_addr);
    // This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
    <b>aborts_if</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr) && <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(container.store, resource_addr);
    <b>aborts_if</b> get && !(<b>exists</b>&lt;Account&gt;(resource_addr) && len(<b>global</b>&lt;Account&gt;(source_addr).authentication_key) == 32);
    <b>aborts_if</b> !get && !(<b>exists</b>&lt;Account&gt;(resource_addr) && len(optional_auth_key) == 32);
    <b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr).store, resource_addr);
    <b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
}
</code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit"></a>


<pre><code><b>schema</b> <a href="resource_account.md#0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit">RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit</a> {
    source_addr: <b>address</b>;
    optional_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    resource_addr: <b>address</b>;
    <b>let</b> container = <b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
    <b>let</b> get = len(optional_auth_key) == 0;
    <b>let</b> <a href="account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(source_addr);
    <b>aborts_if</b> len(<a href="resource_account.md#0x1_resource_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>) != 32;
    <b>include</b> <a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(resource_addr) ==&gt; <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">account::CreateResourceAccountAbortsIf</a>;
    <b>include</b> !<a href="account.md#0x1_account_spec_exists_at">account::spec_exists_at</a>(resource_addr) ==&gt; <a href="account.md#0x1_account_CreateAccountAbortsIf">account::CreateAccountAbortsIf</a> {addr: resource_addr};
    <b>aborts_if</b> get && !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(source_addr);
    <b>aborts_if</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr) && <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(container.store, resource_addr);
    <b>aborts_if</b> get && len(<b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(source_addr).authentication_key) != 32;
    <b>aborts_if</b> !get && len(optional_auth_key) != 32;
    <b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr).store, resource_addr);
    <b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
}
</code></pre>



<a id="@Specification_3_retrieve_resource_account_cap"></a>

### Function `retrieve_resource_account_cap`


<pre><code><b>public</b> <b>fun</b> <a href="resource_account.md#0x1_resource_account_retrieve_resource_account_cap">retrieve_resource_account_cap</a>(resource: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, source_addr: <b>address</b>): <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
// This enforces <a id="high-level-req-6" href="#high-level-req">high-level requirement 6</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
<b>let</b> resource_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(resource);
<b>let</b> container = <b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
// This enforces <a id="high-level-req-7" href="#high-level-req">high-level requirement 7</a>:
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(container.store, resource_addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(resource_addr);
// This enforces <a id="high-level-req-8" href="#high-level-req">high-level requirement 8</a>:
<b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>old</b>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr)).store, resource_addr) &&
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_len">simple_map::spec_len</a>(<b>old</b>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr)).store) == 1 ==&gt; !<b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr);
<b>ensures</b> <b>exists</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr) ==&gt; !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(<b>global</b>&lt;<a href="resource_account.md#0x1_resource_account_Container">Container</a>&gt;(source_addr).store, resource_addr);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
