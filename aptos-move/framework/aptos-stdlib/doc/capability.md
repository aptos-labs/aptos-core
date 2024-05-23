
<a id="0x1_capability"></a>

# Module `0x1::capability`

A module which defines the basic concept of<br/> [&#42;capabilities&#42;](https://en.wikipedia.org/wiki/Capability&#45;based_security) for managing access control.<br/><br/> EXPERIMENTAL<br/><br/> &#35; Overview<br/><br/> A capability is a unforgeable token which testifies that a signer has authorized a certain operation.<br/> The token is valid during the transaction where it is obtained. Since the type <code>capability::Cap</code> has<br/> no ability to be stored in global memory, capabilities cannot leak out of a transaction. For every function<br/> called within a transaction which has a capability as a parameter, it is guaranteed that the capability<br/> has been obtained via a proper signer&#45;based authorization step previously in the transaction&apos;s execution.<br/><br/> &#35;&#35; Usage<br/><br/> Initializing and acquiring capabilities is usually encapsulated in a module with a type<br/> tag which can only be constructed by this module.<br/><br/> ```<br/> module Pkg::Feature &#123;<br/>   use std::capability::Cap;<br/><br/>   /// A type tag used in Cap&lt;Feature&gt;. Only this module can create an instance,<br/>   /// and there is no public function other than Self::acquire which returns a value of this type.<br/>   /// This way, this module has full control how Cap&lt;Feature&gt; is given out.<br/>   struct Feature has drop &#123;&#125;<br/><br/>   /// Initializes this module.<br/>   public fun initialize(s: &amp;signer) &#123;<br/>     // Create capability. This happens once at module initialization time.<br/>     // One needs to provide a witness for being the owner of Feature<br/>     // in the 2nd parameter.<br/>     &lt;&lt;additional conditions allowing to initialize this capability&gt;&gt;<br/>     capability::create&lt;Feature&gt;(s, &amp;Feature&#123;&#125;);<br/>   &#125;<br/><br/>   /// Acquires the capability to work with this feature.<br/>   public fun acquire(s: &amp;signer): Cap&lt;Feature&gt; &#123;<br/>     &lt;&lt;additional conditions allowing to acquire this capability&gt;&gt;<br/>     capability::acquire&lt;Feature&gt;(s, &amp;Feature&#123;&#125;);<br/>   &#125;<br/><br/>   /// Does something related to the feature. The caller must pass a Cap&lt;Feature&gt;.<br/>   public fun do_something(_cap: Cap&lt;Feature&gt;) &#123; ... &#125;<br/> &#125;<br/> ```<br/><br/> &#35;&#35; Delegation<br/><br/> Capabilities come with the optional feature of &#42;delegation&#42;. Via <code>Self::delegate</code>, an owner of a capability<br/> can designate another signer to be also capable of acquiring the capability. Like the original creator,<br/> the delegate needs to present his signer to obtain the capability in his transactions. Delegation can<br/> be revoked via <code>Self::revoke</code>, removing this access right from the delegate.<br/><br/> While the basic authorization mechanism for delegates is the same as with core capabilities, the<br/> target of delegation might be subject of restrictions which need to be specified and verified. This can<br/> be done via global invariants in the specification language. For example, in order to prevent delegation<br/> all together for a capability, one can use the following invariant:<br/><br/> ```<br/>   invariant forall a: address where capability::spec_has_cap&lt;Feature&gt;(a):<br/>               len(capability::spec_delegates&lt;Feature&gt;(a)) &#61;&#61; 0;<br/> ```<br/><br/> Similarly, the following invariant would enforce that delegates, if existent, must satisfy a certain<br/> predicate:<br/><br/> ```<br/>   invariant forall a: address where capability::spec_has_cap&lt;Feature&gt;(a):<br/>               forall d in capability::spec_delegates&lt;Feature&gt;(a):<br/>                  is_valid_delegate_for_feature(d);<br/> ```<br/>


-  [Struct `Cap`](#0x1_capability_Cap)
-  [Struct `LinearCap`](#0x1_capability_LinearCap)
-  [Resource `CapState`](#0x1_capability_CapState)
-  [Resource `CapDelegateState`](#0x1_capability_CapDelegateState)
-  [Constants](#@Constants_0)
-  [Function `create`](#0x1_capability_create)
-  [Function `acquire`](#0x1_capability_acquire)
-  [Function `acquire_linear`](#0x1_capability_acquire_linear)
-  [Function `validate_acquire`](#0x1_capability_validate_acquire)
-  [Function `root_addr`](#0x1_capability_root_addr)
-  [Function `linear_root_addr`](#0x1_capability_linear_root_addr)
-  [Function `delegate`](#0x1_capability_delegate)
-  [Function `revoke`](#0x1_capability_revoke)
-  [Function `remove_element`](#0x1_capability_remove_element)
-  [Function `add_element`](#0x1_capability_add_element)
-  [Specification](#@Specification_1)
    -  [Function `create`](#@Specification_1_create)
    -  [Function `acquire`](#@Specification_1_acquire)
    -  [Function `acquire_linear`](#@Specification_1_acquire_linear)
    -  [Function `delegate`](#@Specification_1_delegate)
    -  [Function `revoke`](#@Specification_1_revoke)
    -  [Function `remove_element`](#@Specification_1_remove_element)
    -  [Function `add_element`](#@Specification_1_add_element)


<pre><code>use 0x1::error;<br/>use 0x1::signer;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_capability_Cap"></a>

## Struct `Cap`

The token representing an acquired capability. Cannot be stored in memory, but copied and dropped freely.


<pre><code>struct Cap&lt;Feature&gt; has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_capability_LinearCap"></a>

## Struct `LinearCap`

A linear version of a capability token. This can be used if an acquired capability should be enforced<br/> to be used only once for an authorization.


<pre><code>struct LinearCap&lt;Feature&gt; has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_capability_CapState"></a>

## Resource `CapState`

An internal data structure for representing a configured capability.


<pre><code>struct CapState&lt;Feature&gt; has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>delegates: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_capability_CapDelegateState"></a>

## Resource `CapDelegateState`

An internal data structure for representing a configured delegated capability.


<pre><code>struct CapDelegateState&lt;Feature&gt; has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_capability_ECAPABILITY_ALREADY_EXISTS"></a>

Capability resource already exists on the specified account


<pre><code>const ECAPABILITY_ALREADY_EXISTS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_capability_ECAPABILITY_NOT_FOUND"></a>

Capability resource not found


<pre><code>const ECAPABILITY_NOT_FOUND: u64 &#61; 2;<br/></code></pre>



<a id="0x1_capability_EDELEGATE"></a>

Account does not have delegated permissions


<pre><code>const EDELEGATE: u64 &#61; 3;<br/></code></pre>



<a id="0x1_capability_create"></a>

## Function `create`

Creates a new capability class, owned by the passed signer. A caller must pass a witness that<br/> they own the <code>Feature</code> type parameter.


<pre><code>public fun create&lt;Feature&gt;(owner: &amp;signer, _feature_witness: &amp;Feature)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create&lt;Feature&gt;(owner: &amp;signer, _feature_witness: &amp;Feature) &#123;<br/>    let addr &#61; signer::address_of(owner);<br/>    assert!(!exists&lt;CapState&lt;Feature&gt;&gt;(addr), error::already_exists(ECAPABILITY_ALREADY_EXISTS));<br/>    move_to&lt;CapState&lt;Feature&gt;&gt;(owner, CapState &#123; delegates: vector::empty() &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_acquire"></a>

## Function `acquire`

Acquires a capability token. Only the owner of the capability class, or an authorized delegate,<br/> can succeed with this operation. A caller must pass a witness that they own the <code>Feature</code> type<br/> parameter.


<pre><code>public fun acquire&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::Cap&lt;Feature&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun acquire&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): Cap&lt;Feature&gt;<br/>acquires CapState, CapDelegateState &#123;<br/>    Cap&lt;Feature&gt; &#123; root: validate_acquire&lt;Feature&gt;(requester) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_acquire_linear"></a>

## Function `acquire_linear`

Acquires a linear capability token. It is up to the module which owns <code>Feature</code> to decide<br/> whether to expose a linear or non&#45;linear capability.


<pre><code>public fun acquire_linear&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::LinearCap&lt;Feature&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun acquire_linear&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): LinearCap&lt;Feature&gt;<br/>acquires CapState, CapDelegateState &#123;<br/>    LinearCap&lt;Feature&gt; &#123; root: validate_acquire&lt;Feature&gt;(requester) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_validate_acquire"></a>

## Function `validate_acquire`

Helper to validate an acquire. Returns the root address of the capability.


<pre><code>fun validate_acquire&lt;Feature&gt;(requester: &amp;signer): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_acquire&lt;Feature&gt;(requester: &amp;signer): address<br/>acquires CapState, CapDelegateState &#123;<br/>    let addr &#61; signer::address_of(requester);<br/>    if (exists&lt;CapDelegateState&lt;Feature&gt;&gt;(addr)) &#123;<br/>        let root_addr &#61; borrow_global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root;<br/>        // double check that requester is actually registered as a delegate<br/>        assert!(exists&lt;CapState&lt;Feature&gt;&gt;(root_addr), error::invalid_state(EDELEGATE));<br/>        assert!(vector::contains(&amp;borrow_global&lt;CapState&lt;Feature&gt;&gt;(root_addr).delegates, &amp;addr),<br/>            error::invalid_state(EDELEGATE));<br/>        root_addr<br/>    &#125; else &#123;<br/>        assert!(exists&lt;CapState&lt;Feature&gt;&gt;(addr), error::not_found(ECAPABILITY_NOT_FOUND));<br/>        addr<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_root_addr"></a>

## Function `root_addr`

Returns the root address associated with the given capability token. Only the owner<br/> of the feature can do this.


<pre><code>public fun root_addr&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun root_addr&lt;Feature&gt;(cap: Cap&lt;Feature&gt;, _feature_witness: &amp;Feature): address &#123;<br/>    cap.root<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_linear_root_addr"></a>

## Function `linear_root_addr`

Returns the root address associated with the given linear capability token.


<pre><code>public fun linear_root_addr&lt;Feature&gt;(cap: capability::LinearCap&lt;Feature&gt;, _feature_witness: &amp;Feature): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun linear_root_addr&lt;Feature&gt;(cap: LinearCap&lt;Feature&gt;, _feature_witness: &amp;Feature): address &#123;<br/>    cap.root<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_delegate"></a>

## Function `delegate`

Registers a delegation relation. If the relation already exists, this function does<br/> nothing.


<pre><code>public fun delegate&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, to: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegate&lt;Feature&gt;(cap: Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, to: &amp;signer)<br/>acquires CapState &#123;<br/>    let addr &#61; signer::address_of(to);<br/>    if (exists&lt;CapDelegateState&lt;Feature&gt;&gt;(addr)) return;<br/>    move_to(to, CapDelegateState&lt;Feature&gt; &#123; root: cap.root &#125;);<br/>    add_element(&amp;mut borrow_global_mut&lt;CapState&lt;Feature&gt;&gt;(cap.root).delegates, addr);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_revoke"></a>

## Function `revoke`

Revokes a delegation relation. If no relation exists, this function does nothing.


<pre><code>public fun revoke&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, from: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun revoke&lt;Feature&gt;(cap: Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, from: address)<br/>acquires CapState, CapDelegateState<br/>&#123;<br/>    if (!exists&lt;CapDelegateState&lt;Feature&gt;&gt;(from)) return;<br/>    let CapDelegateState &#123; root: _root &#125; &#61; move_from&lt;CapDelegateState&lt;Feature&gt;&gt;(from);<br/>    remove_element(&amp;mut borrow_global_mut&lt;CapState&lt;Feature&gt;&gt;(cap.root).delegates, &amp;from);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_remove_element"></a>

## Function `remove_element`

Helper to remove an element from a vector.


<pre><code>fun remove_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: &amp;E)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun remove_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: &amp;E) &#123;<br/>    let (found, index) &#61; vector::index_of(v, x);<br/>    if (found) &#123;<br/>        vector::remove(v, index);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_capability_add_element"></a>

## Function `add_element`

Helper to add an element to a vector.


<pre><code>fun add_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: E)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun add_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: E) &#123;<br/>    if (!vector::contains(v, &amp;x)) &#123;<br/>        vector::push_back(v, x)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification

Helper specification function to check whether a capability exists at address.


<a id="0x1_capability_spec_has_cap"></a>


<pre><code>fun spec_has_cap&lt;Feature&gt;(addr: address): bool &#123;<br/>   exists&lt;CapState&lt;Feature&gt;&gt;(addr)<br/>&#125;<br/></code></pre>


Helper specification function to obtain the delegates of a capability.


<a id="0x1_capability_spec_delegates"></a>


<pre><code>fun spec_delegates&lt;Feature&gt;(addr: address): vector&lt;address&gt; &#123;<br/>   global&lt;CapState&lt;Feature&gt;&gt;(addr).delegates<br/>&#125;<br/></code></pre>


Helper specification function to check whether a delegated capability exists at address.


<a id="0x1_capability_spec_has_delegate_cap"></a>


<pre><code>fun spec_has_delegate_cap&lt;Feature&gt;(addr: address): bool &#123;<br/>   exists&lt;CapDelegateState&lt;Feature&gt;&gt;(addr)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create"></a>

### Function `create`


<pre><code>public fun create&lt;Feature&gt;(owner: &amp;signer, _feature_witness: &amp;Feature)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(owner);<br/>aborts_if spec_has_cap&lt;Feature&gt;(addr);<br/>ensures spec_has_cap&lt;Feature&gt;(addr);<br/></code></pre>



<a id="@Specification_1_acquire"></a>

### Function `acquire`


<pre><code>public fun acquire&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::Cap&lt;Feature&gt;<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(requester);<br/>let root_addr &#61; global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root;<br/>include AcquireSchema&lt;Feature&gt;;<br/>ensures spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; root_addr;<br/>ensures !spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; addr;<br/></code></pre>



<a id="@Specification_1_acquire_linear"></a>

### Function `acquire_linear`


<pre><code>public fun acquire_linear&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::LinearCap&lt;Feature&gt;<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(requester);<br/>let root_addr &#61; global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root;<br/>include AcquireSchema&lt;Feature&gt;;<br/>ensures spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; root_addr;<br/>ensures !spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; addr;<br/></code></pre>




<a id="0x1_capability_AcquireSchema"></a>


<pre><code>schema AcquireSchema&lt;Feature&gt; &#123;<br/>addr: address;<br/>root_addr: address;<br/>aborts_if spec_has_delegate_cap&lt;Feature&gt;(addr) &amp;&amp; !spec_has_cap&lt;Feature&gt;(root_addr);<br/>aborts_if spec_has_delegate_cap&lt;Feature&gt;(addr) &amp;&amp; !vector::spec_contains(spec_delegates&lt;Feature&gt;(root_addr), addr);<br/>aborts_if !spec_has_delegate_cap&lt;Feature&gt;(addr) &amp;&amp; !spec_has_cap&lt;Feature&gt;(addr);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_delegate"></a>

### Function `delegate`


<pre><code>public fun delegate&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, to: &amp;signer)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(to);<br/>ensures spec_has_delegate_cap&lt;Feature&gt;(addr);<br/>ensures !old(spec_has_delegate_cap&lt;Feature&gt;(addr)) &#61;&#61;&gt; global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root &#61;&#61; cap.root;<br/>ensures !old(spec_has_delegate_cap&lt;Feature&gt;(addr)) &#61;&#61;&gt; vector::spec_contains(spec_delegates&lt;Feature&gt;(cap.root), addr);<br/></code></pre>



<a id="@Specification_1_revoke"></a>

### Function `revoke`


<pre><code>public fun revoke&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, from: address)<br/></code></pre>




<pre><code>ensures !spec_has_delegate_cap&lt;Feature&gt;(from);<br/></code></pre>



<a id="@Specification_1_remove_element"></a>

### Function `remove_element`


<pre><code>fun remove_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: &amp;E)<br/></code></pre>




<a id="@Specification_1_add_element"></a>

### Function `add_element`


<pre><code>fun add_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: E)<br/></code></pre>




<pre><code>ensures vector::spec_contains(v, x);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
