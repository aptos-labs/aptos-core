
<a id="0x1_resource_account"></a>

# Module `0x1::resource_account`

A resource account is used to manage resources independent of an account managed by a user.
This contains several utilities to make using resource accounts more effective.


<a id="@Resource_Accounts_to_manage_liquidity_pools_0"></a>

### Resource Accounts to manage liquidity pools


A dev wishing to use resource accounts for a liquidity pool, would likely do the following:

1. Create a new account using <code>resource_account::create_resource_account</code>. This creates the
account, stores the <code>signer_cap</code> within a <code>resource_account::Container</code>, and rotates the key to
the current account's authentication key or a provided authentication key.
2. Define the liquidity pool module's address to be the same as the resource account.
3. Construct a package-publishing transaction for the resource account using the
authentication key used in step 1.
4. In the liquidity pool module's <code>init_module</code> function, call <code>retrieve_resource_account_cap</code>
which will retrieve the <code>signer_cap</code> and rotate the resource account's authentication key to
<code>0x0</code>, effectively locking it off.
5. When adding a new coin, the liquidity pool will load the capability and hence the <code>signer</code> to
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
1. Create a new account using <code>resource_account::create_resource_account_and_publish_package</code>.
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


<pre><code>use 0x1::account;
use 0x1::aptos_coin;
use 0x1::code;
use 0x1::coin;
use 0x1::error;
use 0x1::signer;
use 0x1::simple_map;
use 0x1::vector;
</code></pre>



<a id="0x1_resource_account_Container"></a>

## Resource `Container`



<pre><code>struct Container has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>store: simple_map::SimpleMap&lt;address, account::SignerCapability&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_2"></a>

## Constants


<a id="0x1_resource_account_ZERO_AUTH_KEY"></a>



<pre><code>const ZERO_AUTH_KEY: vector&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a id="0x1_resource_account_ECONTAINER_NOT_PUBLISHED"></a>

Container resource not found in account


<pre><code>const ECONTAINER_NOT_PUBLISHED: u64 &#61; 1;
</code></pre>



<a id="0x1_resource_account_EUNAUTHORIZED_NOT_OWNER"></a>

The resource account was not created by the specified source account


<pre><code>const EUNAUTHORIZED_NOT_OWNER: u64 &#61; 2;
</code></pre>



<a id="0x1_resource_account_create_resource_account"></a>

## Function `create_resource_account`

Creates a new resource account and rotates the authentication key to either
the optional auth key if it is non-empty (though auth keys are 32-bytes)
or the source accounts current auth key.


<pre><code>public entry fun create_resource_account(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_resource_account(
    origin: &amp;signer,
    seed: vector&lt;u8&gt;,
    optional_auth_key: vector&lt;u8&gt;,
) acquires Container &#123;
    let (resource, resource_signer_cap) &#61; account::create_resource_account(origin, seed);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        resource_signer_cap,
        optional_auth_key,
    );
&#125;
</code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_fund"></a>

## Function `create_resource_account_and_fund`

Creates a new resource account, transfer the amount of coins from the origin to the resource
account, and rotates the authentication key to either the optional auth key if it is
non-empty (though auth keys are 32-bytes) or the source accounts current auth key. Note,
this function adds additional resource ownership to the resource account and should only be
used for resource accounts that need access to <code>Coin&lt;AptosCoin&gt;</code>.


<pre><code>public entry fun create_resource_account_and_fund(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;, fund_amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_resource_account_and_fund(
    origin: &amp;signer,
    seed: vector&lt;u8&gt;,
    optional_auth_key: vector&lt;u8&gt;,
    fund_amount: u64,
) acquires Container &#123;
    let (resource, resource_signer_cap) &#61; account::create_resource_account(origin, seed);
    coin::register&lt;AptosCoin&gt;(&amp;resource);
    coin::transfer&lt;AptosCoin&gt;(origin, signer::address_of(&amp;resource), fund_amount);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        resource_signer_cap,
        optional_auth_key,
    );
&#125;
</code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_publish_package"></a>

## Function `create_resource_account_and_publish_package`

Creates a new resource account, publishes the package under this account transaction under
this account and leaves the signer cap readily available for pickup.


<pre><code>public entry fun create_resource_account_and_publish_package(origin: &amp;signer, seed: vector&lt;u8&gt;, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_resource_account_and_publish_package(
    origin: &amp;signer,
    seed: vector&lt;u8&gt;,
    metadata_serialized: vector&lt;u8&gt;,
    code: vector&lt;vector&lt;u8&gt;&gt;,
) acquires Container &#123;
    let (resource, resource_signer_cap) &#61; account::create_resource_account(origin, seed);
    aptos_framework::code::publish_package_txn(&amp;resource, metadata_serialized, code);
    rotate_account_authentication_key_and_store_capability(
        origin,
        resource,
        resource_signer_cap,
        ZERO_AUTH_KEY,
    );
&#125;
</code></pre>



</details>

<a id="0x1_resource_account_rotate_account_authentication_key_and_store_capability"></a>

## Function `rotate_account_authentication_key_and_store_capability`



<pre><code>fun rotate_account_authentication_key_and_store_capability(origin: &amp;signer, resource: signer, resource_signer_cap: account::SignerCapability, optional_auth_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun rotate_account_authentication_key_and_store_capability(
    origin: &amp;signer,
    resource: signer,
    resource_signer_cap: account::SignerCapability,
    optional_auth_key: vector&lt;u8&gt;,
) acquires Container &#123;
    let origin_addr &#61; signer::address_of(origin);
    if (!exists&lt;Container&gt;(origin_addr)) &#123;
        move_to(origin, Container &#123; store: simple_map::create() &#125;)
    &#125;;

    let container &#61; borrow_global_mut&lt;Container&gt;(origin_addr);
    let resource_addr &#61; signer::address_of(&amp;resource);
    simple_map::add(&amp;mut container.store, resource_addr, resource_signer_cap);

    let auth_key &#61; if (vector::is_empty(&amp;optional_auth_key)) &#123;
        account::get_authentication_key(origin_addr)
    &#125; else &#123;
        optional_auth_key
    &#125;;
    account::rotate_authentication_key_internal(&amp;resource, auth_key);
&#125;
</code></pre>



</details>

<a id="0x1_resource_account_retrieve_resource_account_cap"></a>

## Function `retrieve_resource_account_cap`

When called by the resource account, it will retrieve the capability associated with that
account and rotate the account's auth key to 0x0 making the account inaccessible without
the SignerCapability.


<pre><code>public fun retrieve_resource_account_cap(resource: &amp;signer, source_addr: address): account::SignerCapability
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun retrieve_resource_account_cap(
    resource: &amp;signer,
    source_addr: address,
): account::SignerCapability acquires Container &#123;
    assert!(exists&lt;Container&gt;(source_addr), error::not_found(ECONTAINER_NOT_PUBLISHED));

    let resource_addr &#61; signer::address_of(resource);
    let (resource_signer_cap, empty_container) &#61; &#123;
        let container &#61; borrow_global_mut&lt;Container&gt;(source_addr);
        assert!(
            simple_map::contains_key(&amp;container.store, &amp;resource_addr),
            error::invalid_argument(EUNAUTHORIZED_NOT_OWNER)
        );
        let (_resource_addr, signer_cap) &#61; simple_map::remove(&amp;mut container.store, &amp;resource_addr);
        (signer_cap, simple_map::length(&amp;container.store) &#61;&#61; 0)
    &#125;;

    if (empty_container) &#123;
        let container &#61; move_from(source_addr);
        let Container &#123; store &#125; &#61; container;
        simple_map::destroy_empty(store);
    &#125;;

    account::rotate_authentication_key_internal(resource, ZERO_AUTH_KEY);
    resource_signer_cap
&#125;
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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_3_create_resource_account"></a>

### Function `create_resource_account`


<pre><code>public entry fun create_resource_account(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;)
</code></pre>




<pre><code>let source_addr &#61; signer::address_of(origin);
let resource_addr &#61; account::spec_create_resource_address(source_addr, seed);
include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
</code></pre>



<a id="@Specification_3_create_resource_account_and_fund"></a>

### Function `create_resource_account_and_fund`


<pre><code>public entry fun create_resource_account_and_fund(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;, fund_amount: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let source_addr &#61; signer::address_of(origin);
let resource_addr &#61; account::spec_create_resource_address(source_addr, seed);
let coin_store_resource &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);
include aptos_account::WithdrawAbortsIf&lt;AptosCoin&gt;&#123;from: origin, amount: fund_amount&#125;;
include aptos_account::GuidAbortsIf&lt;AptosCoin&gt;&#123;to: resource_addr&#125;;
include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
aborts_if coin::spec_is_account_registered&lt;AptosCoin&gt;(resource_addr) &amp;&amp; coin_store_resource.frozen;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures exists&lt;aptos_framework::coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);
</code></pre>



<a id="@Specification_3_create_resource_account_and_publish_package"></a>

### Function `create_resource_account_and_publish_package`


<pre><code>public entry fun create_resource_account_and_publish_package(origin: &amp;signer, seed: vector&lt;u8&gt;, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
let source_addr &#61; signer::address_of(origin);
let resource_addr &#61; account::spec_create_resource_address(source_addr, seed);
let optional_auth_key &#61; ZERO_AUTH_KEY;
include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
</code></pre>



<a id="@Specification_3_rotate_account_authentication_key_and_store_capability"></a>

### Function `rotate_account_authentication_key_and_store_capability`


<pre><code>fun rotate_account_authentication_key_and_store_capability(origin: &amp;signer, resource: signer, resource_signer_cap: account::SignerCapability, optional_auth_key: vector&lt;u8&gt;)
</code></pre>




<pre><code>let resource_addr &#61; signer::address_of(resource);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
ensures exists&lt;Container&gt;(signer::address_of(origin));
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a>:
ensures vector::length(optional_auth_key) !&#61; 0 &#61;&#61;&gt;
    global&lt;aptos_framework::account::Account&gt;(resource_addr).authentication_key &#61;&#61; optional_auth_key;
</code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf"></a>


<pre><code>schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf &#123;
    origin: signer;
    resource_addr: address;
    optional_auth_key: vector&lt;u8&gt;;
    let source_addr &#61; signer::address_of(origin);
    let container &#61; global&lt;Container&gt;(source_addr);
    let get &#61; len(optional_auth_key) &#61;&#61; 0;
    aborts_if get &amp;&amp; !exists&lt;Account&gt;(source_addr);
    // This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
    aborts_if exists&lt;Container&gt;(source_addr) &amp;&amp; simple_map::spec_contains_key(container.store, resource_addr);
    aborts_if get &amp;&amp; !(exists&lt;Account&gt;(resource_addr) &amp;&amp; len(global&lt;Account&gt;(source_addr).authentication_key) &#61;&#61; 32);
    aborts_if !get &amp;&amp; !(exists&lt;Account&gt;(resource_addr) &amp;&amp; len(optional_auth_key) &#61;&#61; 32);
    ensures simple_map::spec_contains_key(global&lt;Container&gt;(source_addr).store, resource_addr);
    ensures exists&lt;Container&gt;(source_addr);
&#125;
</code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit"></a>


<pre><code>schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit &#123;
    source_addr: address;
    optional_auth_key: vector&lt;u8&gt;;
    resource_addr: address;
    let container &#61; global&lt;Container&gt;(source_addr);
    let get &#61; len(optional_auth_key) &#61;&#61; 0;
    let account &#61; global&lt;account::Account&gt;(source_addr);
    requires source_addr !&#61; resource_addr;
    aborts_if len(ZERO_AUTH_KEY) !&#61; 32;
    include account::exists_at(resource_addr) &#61;&#61;&gt; account::CreateResourceAccountAbortsIf;
    include !account::exists_at(resource_addr) &#61;&#61;&gt; account::CreateAccountAbortsIf &#123;addr: resource_addr&#125;;
    aborts_if get &amp;&amp; !exists&lt;account::Account&gt;(source_addr);
    aborts_if exists&lt;Container&gt;(source_addr) &amp;&amp; simple_map::spec_contains_key(container.store, resource_addr);
    aborts_if get &amp;&amp; len(global&lt;account::Account&gt;(source_addr).authentication_key) !&#61; 32;
    aborts_if !get &amp;&amp; len(optional_auth_key) !&#61; 32;
    ensures simple_map::spec_contains_key(global&lt;Container&gt;(source_addr).store, resource_addr);
    ensures exists&lt;Container&gt;(source_addr);
&#125;
</code></pre>



<a id="@Specification_3_retrieve_resource_account_cap"></a>

### Function `retrieve_resource_account_cap`


<pre><code>public fun retrieve_resource_account_cap(resource: &amp;signer, source_addr: address): account::SignerCapability
</code></pre>




<pre><code>// This enforces <a id="high-level-req-6" href="#high-level-req">high-level requirement 6</a>:
aborts_if !exists&lt;Container&gt;(source_addr);
let resource_addr &#61; signer::address_of(resource);
let container &#61; global&lt;Container&gt;(source_addr);
// This enforces <a id="high-level-req-7" href="#high-level-req">high-level requirement 7</a>:
aborts_if !simple_map::spec_contains_key(container.store, resource_addr);
aborts_if !exists&lt;account::Account&gt;(resource_addr);
// This enforces <a id="high-level-req-8" href="#high-level-req">high-level requirement 8</a>:
ensures simple_map::spec_contains_key(old(global&lt;Container&gt;(source_addr)).store, resource_addr) &amp;&amp;
    simple_map::spec_len(old(global&lt;Container&gt;(source_addr)).store) &#61;&#61; 1 &#61;&#61;&gt; !exists&lt;Container&gt;(source_addr);
ensures exists&lt;Container&gt;(source_addr) &#61;&#61;&gt; !simple_map::spec_contains_key(global&lt;Container&gt;(source_addr).store, resource_addr);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
