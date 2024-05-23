
<a id="0x1_resource_account"></a>

# Module `0x1::resource_account`

A resource account is used to manage resources independent of an account managed by a user.<br/> This contains several utilities to make using resource accounts more effective.<br/><br/> &#35;&#35; Resource Accounts to manage liquidity pools<br/><br/> A dev wishing to use resource accounts for a liquidity pool, would likely do the following:<br/><br/>  1. Create a new account using <code>resource_account::create_resource_account</code>. This creates the<br/>     account, stores the <code>signer_cap</code> within a <code>resource_account::Container</code>, and rotates the key to<br/>     the current account&apos;s authentication key or a provided authentication key.<br/>  2. Define the liquidity pool module&apos;s address to be the same as the resource account.<br/>  3. Construct a package&#45;publishing transaction for the resource account using the<br/>     authentication key used in step 1.<br/>  4. In the liquidity pool module&apos;s <code>init_module</code> function, call <code>retrieve_resource_account_cap</code><br/>     which will retrieve the <code>signer_cap</code> and rotate the resource account&apos;s authentication key to<br/>     <code>0x0</code>, effectively locking it off.<br/>  5. When adding a new coin, the liquidity pool will load the capability and hence the <code>signer</code> to<br/>     register and store new <code>LiquidityCoin</code> resources.<br/><br/> Code snippets to help:<br/><br/> ```<br/> fun init_module(resource_account: &amp;signer) &#123;<br/>   let dev_address &#61; @DEV_ADDR;<br/>   let signer_cap &#61; retrieve_resource_account_cap(resource_account, dev_address);<br/>   let lp &#61; LiquidityPoolInfo &#123; signer_cap: signer_cap, ... &#125;;<br/>   move_to(resource_account, lp);<br/> &#125;<br/> ```<br/><br/> Later on during a coin registration:<br/> ```<br/> public fun add_coin&lt;X, Y&gt;(lp: &amp;LP, x: Coin&lt;x&gt;, y: Coin&lt;y&gt;) &#123;<br/>     if(!exists&lt;LiquidityCoin&lt;X, Y&gt;(LP::Address(lp), LiquidityCoin&lt;X, Y&gt;)) &#123;<br/>         let mint, burn &#61; Coin::initialize&lt;LiquidityCoin&lt;X, Y&gt;&gt;(...);<br/>         move_to(&amp;create_signer_with_capability(&amp;lp.cap), LiquidityCoin&lt;X, Y&gt;&#123; mint, burn &#125;);<br/>     &#125;<br/>     ...<br/> &#125;<br/> ```<br/> &#35;&#35; Resource accounts to manage an account for module publishing (i.e., contract account)<br/><br/> A dev wishes to have an account dedicated to managing a contract. The contract itself does not<br/> require signer post initialization. The dev could do the following:<br/> 1. Create a new account using <code>resource_account::create_resource_account_and_publish_package</code>.<br/> This creates the account and publishes the package for that account.<br/> 2. At a later point in time, the account creator can move the signer capability to the module.<br/><br/> ```<br/> struct MyModuleResource has key &#123;<br/>     ...<br/>     resource_signer_cap: Option&lt;SignerCapability&gt;,<br/> &#125;<br/><br/> public fun provide_signer_capability(resource_signer_cap: SignerCapability) &#123;<br/>    let account_addr &#61; account::get_signer_capability_address(resource_signer_cap);<br/>    let resource_addr &#61; type_info::account_address(&amp;type_info::type_of&lt;MyModuleResource&gt;());<br/>    assert!(account_addr &#61;&#61; resource_addr, EADDRESS_MISMATCH);<br/>    let module &#61; borrow_global_mut&lt;MyModuleResource&gt;(account_addr);<br/>    module.resource_signer_cap &#61; option::some(resource_signer_cap);<br/> &#125;<br/> ```


-  [Resource `Container`](#0x1_resource_account_Container)
-  [Constants](#@Constants_0)
-  [Function `create_resource_account`](#0x1_resource_account_create_resource_account)
-  [Function `create_resource_account_and_fund`](#0x1_resource_account_create_resource_account_and_fund)
-  [Function `create_resource_account_and_publish_package`](#0x1_resource_account_create_resource_account_and_publish_package)
-  [Function `rotate_account_authentication_key_and_store_capability`](#0x1_resource_account_rotate_account_authentication_key_and_store_capability)
-  [Function `retrieve_resource_account_cap`](#0x1_resource_account_retrieve_resource_account_cap)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create_resource_account`](#@Specification_1_create_resource_account)
    -  [Function `create_resource_account_and_fund`](#@Specification_1_create_resource_account_and_fund)
    -  [Function `create_resource_account_and_publish_package`](#@Specification_1_create_resource_account_and_publish_package)
    -  [Function `rotate_account_authentication_key_and_store_capability`](#@Specification_1_rotate_account_authentication_key_and_store_capability)
    -  [Function `retrieve_resource_account_cap`](#@Specification_1_retrieve_resource_account_cap)


<pre><code>use 0x1::account;<br/>use 0x1::aptos_coin;<br/>use 0x1::code;<br/>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::signer;<br/>use 0x1::simple_map;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_resource_account_Container"></a>

## Resource `Container`



<pre><code>struct Container has key<br/></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_resource_account_ZERO_AUTH_KEY"></a>



<pre><code>const ZERO_AUTH_KEY: vector&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];<br/></code></pre>



<a id="0x1_resource_account_ECONTAINER_NOT_PUBLISHED"></a>

Container resource not found in account


<pre><code>const ECONTAINER_NOT_PUBLISHED: u64 &#61; 1;<br/></code></pre>



<a id="0x1_resource_account_EUNAUTHORIZED_NOT_OWNER"></a>

The resource account was not created by the specified source account


<pre><code>const EUNAUTHORIZED_NOT_OWNER: u64 &#61; 2;<br/></code></pre>



<a id="0x1_resource_account_create_resource_account"></a>

## Function `create_resource_account`

Creates a new resource account and rotates the authentication key to either<br/> the optional auth key if it is non&#45;empty (though auth keys are 32&#45;bytes)<br/> or the source accounts current auth key.


<pre><code>public entry fun create_resource_account(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_resource_account(<br/>    origin: &amp;signer,<br/>    seed: vector&lt;u8&gt;,<br/>    optional_auth_key: vector&lt;u8&gt;,<br/>) acquires Container &#123;<br/>    let (resource, resource_signer_cap) &#61; account::create_resource_account(origin, seed);<br/>    rotate_account_authentication_key_and_store_capability(<br/>        origin,<br/>        resource,<br/>        resource_signer_cap,<br/>        optional_auth_key,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_fund"></a>

## Function `create_resource_account_and_fund`

Creates a new resource account, transfer the amount of coins from the origin to the resource<br/> account, and rotates the authentication key to either the optional auth key if it is<br/> non&#45;empty (though auth keys are 32&#45;bytes) or the source accounts current auth key. Note,<br/> this function adds additional resource ownership to the resource account and should only be<br/> used for resource accounts that need access to <code>Coin&lt;AptosCoin&gt;</code>.


<pre><code>public entry fun create_resource_account_and_fund(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;, fund_amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_resource_account_and_fund(<br/>    origin: &amp;signer,<br/>    seed: vector&lt;u8&gt;,<br/>    optional_auth_key: vector&lt;u8&gt;,<br/>    fund_amount: u64,<br/>) acquires Container &#123;<br/>    let (resource, resource_signer_cap) &#61; account::create_resource_account(origin, seed);<br/>    coin::register&lt;AptosCoin&gt;(&amp;resource);<br/>    coin::transfer&lt;AptosCoin&gt;(origin, signer::address_of(&amp;resource), fund_amount);<br/>    rotate_account_authentication_key_and_store_capability(<br/>        origin,<br/>        resource,<br/>        resource_signer_cap,<br/>        optional_auth_key,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_resource_account_create_resource_account_and_publish_package"></a>

## Function `create_resource_account_and_publish_package`

Creates a new resource account, publishes the package under this account transaction under<br/> this account and leaves the signer cap readily available for pickup.


<pre><code>public entry fun create_resource_account_and_publish_package(origin: &amp;signer, seed: vector&lt;u8&gt;, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_resource_account_and_publish_package(<br/>    origin: &amp;signer,<br/>    seed: vector&lt;u8&gt;,<br/>    metadata_serialized: vector&lt;u8&gt;,<br/>    code: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires Container &#123;<br/>    let (resource, resource_signer_cap) &#61; account::create_resource_account(origin, seed);<br/>    aptos_framework::code::publish_package_txn(&amp;resource, metadata_serialized, code);<br/>    rotate_account_authentication_key_and_store_capability(<br/>        origin,<br/>        resource,<br/>        resource_signer_cap,<br/>        ZERO_AUTH_KEY,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_resource_account_rotate_account_authentication_key_and_store_capability"></a>

## Function `rotate_account_authentication_key_and_store_capability`



<pre><code>fun rotate_account_authentication_key_and_store_capability(origin: &amp;signer, resource: signer, resource_signer_cap: account::SignerCapability, optional_auth_key: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun rotate_account_authentication_key_and_store_capability(<br/>    origin: &amp;signer,<br/>    resource: signer,<br/>    resource_signer_cap: account::SignerCapability,<br/>    optional_auth_key: vector&lt;u8&gt;,<br/>) acquires Container &#123;<br/>    let origin_addr &#61; signer::address_of(origin);<br/>    if (!exists&lt;Container&gt;(origin_addr)) &#123;<br/>        move_to(origin, Container &#123; store: simple_map::create() &#125;)<br/>    &#125;;<br/><br/>    let container &#61; borrow_global_mut&lt;Container&gt;(origin_addr);<br/>    let resource_addr &#61; signer::address_of(&amp;resource);<br/>    simple_map::add(&amp;mut container.store, resource_addr, resource_signer_cap);<br/><br/>    let auth_key &#61; if (vector::is_empty(&amp;optional_auth_key)) &#123;<br/>        account::get_authentication_key(origin_addr)<br/>    &#125; else &#123;<br/>        optional_auth_key<br/>    &#125;;<br/>    account::rotate_authentication_key_internal(&amp;resource, auth_key);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_resource_account_retrieve_resource_account_cap"></a>

## Function `retrieve_resource_account_cap`

When called by the resource account, it will retrieve the capability associated with that<br/> account and rotate the account&apos;s auth key to 0x0 making the account inaccessible without<br/> the SignerCapability.


<pre><code>public fun retrieve_resource_account_cap(resource: &amp;signer, source_addr: address): account::SignerCapability<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun retrieve_resource_account_cap(<br/>    resource: &amp;signer,<br/>    source_addr: address,<br/>): account::SignerCapability acquires Container &#123;<br/>    assert!(exists&lt;Container&gt;(source_addr), error::not_found(ECONTAINER_NOT_PUBLISHED));<br/><br/>    let resource_addr &#61; signer::address_of(resource);<br/>    let (resource_signer_cap, empty_container) &#61; &#123;<br/>        let container &#61; borrow_global_mut&lt;Container&gt;(source_addr);<br/>        assert!(<br/>            simple_map::contains_key(&amp;container.store, &amp;resource_addr),<br/>            error::invalid_argument(EUNAUTHORIZED_NOT_OWNER)<br/>        );<br/>        let (_resource_addr, signer_cap) &#61; simple_map::remove(&amp;mut container.store, &amp;resource_addr);<br/>        (signer_cap, simple_map::length(&amp;container.store) &#61;&#61; 0)<br/>    &#125;;<br/><br/>    if (empty_container) &#123;<br/>        let container &#61; move_from(source_addr);<br/>        let Container &#123; store &#125; &#61; container;<br/>        simple_map::destroy_empty(store);<br/>    &#125;;<br/><br/>    account::rotate_authentication_key_internal(resource, ZERO_AUTH_KEY);<br/>    resource_signer_cap<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The length of the authentication key must be 32 bytes.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The rotate_authentication_key_internal function ensures that the authentication key passed to it is of 32 bytes.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The Container structure must exist in the origin account in order to rotate the authentication key of a resource account and to store its signer capability.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The rotate_account_authentication_key_and_store_capability function makes sure the Container structure exists under the origin account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;rotate_account_authentication_key_and_store_capability&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The resource account is registered for the Aptos coin.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The create_resource_account_and_fund ensures the newly created resource account is registered to receive the AptosCoin.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;create_resource_account_and_fund&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;It is not possible to store two capabilities for the same resource address.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The rotate_account_authentication_key_and_store_capability will abort if the resource signer capability for the given resource address already exists in container.store.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;rotate_account_authentication_key_and_store_capability&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;If provided, the optional authentication key is used for key rotation.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The rotate_account_authentication_key_and_store_capability function will use optional_auth_key if it is provided as a parameter.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5&quot;&gt;rotate_account_authentication_key_and_store_capability&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;The container stores the resource accounts&apos; signer capabilities.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;retrieve_resource_account_cap will abort if there is no Container structure assigned to source_addr.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;6&quot;&gt;retreive_resource_account_cap&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;Resource account may retrieve the signer capability if it was previously added to its container.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;retrieve_resource_account_cap will abort if the container of source_addr doesn&apos;t store the signer capability for the given resource.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;7&quot;&gt;retrieve_resource_account_cap&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;8&lt;/td&gt;<br/>&lt;td&gt;Retrieving the last signer capability from the container must result in the container being removed.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;retrieve_resource_account_cap will remove the container if the retrieved signer_capability was the last one stored under it.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;8&quot;&gt;retrieve_resource_account_cap&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_create_resource_account"></a>

### Function `create_resource_account`


<pre><code>public entry fun create_resource_account(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>let source_addr &#61; signer::address_of(origin);<br/>let resource_addr &#61; account::spec_create_resource_address(source_addr, seed);<br/>include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;<br/></code></pre>



<a id="@Specification_1_create_resource_account_and_fund"></a>

### Function `create_resource_account_and_fund`


<pre><code>public entry fun create_resource_account_and_fund(origin: &amp;signer, seed: vector&lt;u8&gt;, optional_auth_key: vector&lt;u8&gt;, fund_amount: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let source_addr &#61; signer::address_of(origin);<br/>let resource_addr &#61; account::spec_create_resource_address(source_addr, seed);<br/>let coin_store_resource &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);<br/>include aptos_account::WithdrawAbortsIf&lt;AptosCoin&gt;&#123;from: origin, amount: fund_amount&#125;;<br/>include aptos_account::GuidAbortsIf&lt;AptosCoin&gt;&#123;to: resource_addr&#125;;<br/>include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;<br/>aborts_if coin::spec_is_account_registered&lt;AptosCoin&gt;(resource_addr) &amp;&amp; coin_store_resource.frozen;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
ensures exists&lt;aptos_framework::coin::CoinStore&lt;AptosCoin&gt;&gt;(resource_addr);<br/></code></pre>



<a id="@Specification_1_create_resource_account_and_publish_package"></a>

### Function `create_resource_account_and_publish_package`


<pre><code>public entry fun create_resource_account_and_publish_package(origin: &amp;signer, seed: vector&lt;u8&gt;, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let source_addr &#61; signer::address_of(origin);<br/>let resource_addr &#61; account::spec_create_resource_address(source_addr, seed);<br/>let optional_auth_key &#61; ZERO_AUTH_KEY;<br/>include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;<br/></code></pre>



<a id="@Specification_1_rotate_account_authentication_key_and_store_capability"></a>

### Function `rotate_account_authentication_key_and_store_capability`


<pre><code>fun rotate_account_authentication_key_and_store_capability(origin: &amp;signer, resource: signer, resource_signer_cap: account::SignerCapability, optional_auth_key: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>let resource_addr &#61; signer::address_of(resource);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
ensures exists&lt;Container&gt;(signer::address_of(origin));<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
ensures vector::length(optional_auth_key) !&#61; 0 &#61;&#61;&gt;<br/>    global&lt;aptos_framework::account::Account&gt;(resource_addr).authentication_key &#61;&#61; optional_auth_key;<br/></code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf"></a>


<pre><code>schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf &#123;<br/>origin: signer;<br/>resource_addr: address;<br/>optional_auth_key: vector&lt;u8&gt;;<br/>let source_addr &#61; signer::address_of(origin);<br/>let container &#61; global&lt;Container&gt;(source_addr);<br/>let get &#61; len(optional_auth_key) &#61;&#61; 0;<br/>aborts_if get &amp;&amp; !exists&lt;Account&gt;(source_addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
    aborts_if exists&lt;Container&gt;(source_addr) &amp;&amp; simple_map::spec_contains_key(container.store, resource_addr);<br/>aborts_if get &amp;&amp; !(exists&lt;Account&gt;(resource_addr) &amp;&amp; len(global&lt;Account&gt;(source_addr).authentication_key) &#61;&#61; 32);<br/>aborts_if !get &amp;&amp; !(exists&lt;Account&gt;(resource_addr) &amp;&amp; len(optional_auth_key) &#61;&#61; 32);<br/>ensures simple_map::spec_contains_key(global&lt;Container&gt;(source_addr).store, resource_addr);<br/>ensures exists&lt;Container&gt;(source_addr);<br/>&#125;<br/></code></pre>




<a id="0x1_resource_account_RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit"></a>


<pre><code>schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit &#123;<br/>source_addr: address;<br/>optional_auth_key: vector&lt;u8&gt;;<br/>resource_addr: address;<br/>let container &#61; global&lt;Container&gt;(source_addr);<br/>let get &#61; len(optional_auth_key) &#61;&#61; 0;<br/>let account &#61; global&lt;account::Account&gt;(source_addr);<br/>requires source_addr !&#61; resource_addr;<br/>aborts_if len(ZERO_AUTH_KEY) !&#61; 32;<br/>include account::exists_at(resource_addr) &#61;&#61;&gt; account::CreateResourceAccountAbortsIf;<br/>include !account::exists_at(resource_addr) &#61;&#61;&gt; account::CreateAccountAbortsIf &#123;addr: resource_addr&#125;;<br/>aborts_if get &amp;&amp; !exists&lt;account::Account&gt;(source_addr);<br/>aborts_if exists&lt;Container&gt;(source_addr) &amp;&amp; simple_map::spec_contains_key(container.store, resource_addr);<br/>aborts_if get &amp;&amp; len(global&lt;account::Account&gt;(source_addr).authentication_key) !&#61; 32;<br/>aborts_if !get &amp;&amp; len(optional_auth_key) !&#61; 32;<br/>ensures simple_map::spec_contains_key(global&lt;Container&gt;(source_addr).store, resource_addr);<br/>ensures exists&lt;Container&gt;(source_addr);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_retrieve_resource_account_cap"></a>

### Function `retrieve_resource_account_cap`


<pre><code>public fun retrieve_resource_account_cap(resource: &amp;signer, source_addr: address): account::SignerCapability<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;6&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 6&lt;/a&gt;:
aborts_if !exists&lt;Container&gt;(source_addr);<br/>let resource_addr &#61; signer::address_of(resource);<br/>let container &#61; global&lt;Container&gt;(source_addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;7&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 7&lt;/a&gt;:
aborts_if !simple_map::spec_contains_key(container.store, resource_addr);<br/>aborts_if !exists&lt;account::Account&gt;(resource_addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;8&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 8&lt;/a&gt;:
ensures simple_map::spec_contains_key(old(global&lt;Container&gt;(source_addr)).store, resource_addr) &amp;&amp;<br/>    simple_map::spec_len(old(global&lt;Container&gt;(source_addr)).store) &#61;&#61; 1 &#61;&#61;&gt; !exists&lt;Container&gt;(source_addr);<br/>ensures exists&lt;Container&gt;(source_addr) &#61;&#61;&gt; !simple_map::spec_contains_key(global&lt;Container&gt;(source_addr).store, resource_addr);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
