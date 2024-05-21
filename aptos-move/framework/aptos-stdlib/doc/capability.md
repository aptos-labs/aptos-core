
<a id="0x1_capability"></a>

# Module `0x1::capability`

A module which defines the basic concept of
[*capabilities*](https://en.wikipedia.org/wiki/Capability-based_security) for managing access control.

EXPERIMENTAL


<a id="@Overview_0"></a>

## Overview


A capability is a unforgeable token which testifies that a signer has authorized a certain operation.
The token is valid during the transaction where it is obtained. Since the type <code>capability::Cap</code> has
no ability to be stored in global memory, capabilities cannot leak out of a transaction. For every function
called within a transaction which has a capability as a parameter, it is guaranteed that the capability
has been obtained via a proper signer-based authorization step previously in the transaction's execution.


<a id="@Usage_1"></a>

### Usage


Initializing and acquiring capabilities is usually encapsulated in a module with a type
tag which can only be constructed by this module.

```
module Pkg::Feature {
use std::capability::Cap;

/// A type tag used in Cap<Feature>. Only this module can create an instance,
/// and there is no public function other than Self::acquire which returns a value of this type.
/// This way, this module has full control how Cap<Feature> is given out.
struct Feature has drop {}

/// Initializes this module.
public fun initialize(s: &signer) {
// Create capability. This happens once at module initialization time.
// One needs to provide a witness for being the owner of Feature
// in the 2nd parameter.
<<additional conditions allowing to initialize this capability>>
capability::create<Feature>(s, &Feature{});
}

/// Acquires the capability to work with this feature.
public fun acquire(s: &signer): Cap<Feature> {
<<additional conditions allowing to acquire this capability>>
capability::acquire<Feature>(s, &Feature{});
}

/// Does something related to the feature. The caller must pass a Cap<Feature>.
public fun do_something(_cap: Cap<Feature>) { ... }
}
```


<a id="@Delegation_2"></a>

### Delegation


Capabilities come with the optional feature of *delegation*. Via <code>Self::delegate</code>, an owner of a capability
can designate another signer to be also capable of acquiring the capability. Like the original creator,
the delegate needs to present his signer to obtain the capability in his transactions. Delegation can
be revoked via <code>Self::revoke</code>, removing this access right from the delegate.

While the basic authorization mechanism for delegates is the same as with core capabilities, the
target of delegation might be subject of restrictions which need to be specified and verified. This can
be done via global invariants in the specification language. For example, in order to prevent delegation
all together for a capability, one can use the following invariant:

```
invariant forall a: address where capability::spec_has_cap<Feature>(a):
len(capability::spec_delegates<Feature>(a)) == 0;
```

Similarly, the following invariant would enforce that delegates, if existent, must satisfy a certain
predicate:

```
invariant forall a: address where capability::spec_has_cap<Feature>(a):
forall d in capability::spec_delegates<Feature>(a):
is_valid_delegate_for_feature(d);
```


-  [Overview](#@Overview_0)
    -  [Usage](#@Usage_1)
    -  [Delegation](#@Delegation_2)
-  [Struct `Cap`](#0x1_capability_Cap)
-  [Struct `LinearCap`](#0x1_capability_LinearCap)
-  [Resource `CapState`](#0x1_capability_CapState)
-  [Resource `CapDelegateState`](#0x1_capability_CapDelegateState)
-  [Constants](#@Constants_3)
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
-  [Specification](#@Specification_4)
    -  [Function `create`](#@Specification_4_create)
    -  [Function `acquire`](#@Specification_4_acquire)
    -  [Function `acquire_linear`](#@Specification_4_acquire_linear)
    -  [Function `delegate`](#@Specification_4_delegate)
    -  [Function `revoke`](#@Specification_4_revoke)
    -  [Function `remove_element`](#@Specification_4_remove_element)
    -  [Function `add_element`](#@Specification_4_add_element)


<pre><code>use 0x1::error;
use 0x1::signer;
use 0x1::vector;
</code></pre>



<a id="0x1_capability_Cap"></a>

## Struct `Cap`

The token representing an acquired capability. Cannot be stored in memory, but copied and dropped freely.


<pre><code>struct Cap&lt;Feature&gt; has copy, drop
</code></pre>



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

A linear version of a capability token. This can be used if an acquired capability should be enforced
to be used only once for an authorization.


<pre><code>struct LinearCap&lt;Feature&gt; has drop
</code></pre>



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


<pre><code>struct CapState&lt;Feature&gt; has key
</code></pre>



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


<pre><code>struct CapDelegateState&lt;Feature&gt; has key
</code></pre>



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

<a id="@Constants_3"></a>

## Constants


<a id="0x1_capability_ECAPABILITY_ALREADY_EXISTS"></a>

Capability resource already exists on the specified account


<pre><code>const ECAPABILITY_ALREADY_EXISTS: u64 &#61; 1;
</code></pre>



<a id="0x1_capability_ECAPABILITY_NOT_FOUND"></a>

Capability resource not found


<pre><code>const ECAPABILITY_NOT_FOUND: u64 &#61; 2;
</code></pre>



<a id="0x1_capability_EDELEGATE"></a>

Account does not have delegated permissions


<pre><code>const EDELEGATE: u64 &#61; 3;
</code></pre>



<a id="0x1_capability_create"></a>

## Function `create`

Creates a new capability class, owned by the passed signer. A caller must pass a witness that
they own the <code>Feature</code> type parameter.


<pre><code>public fun create&lt;Feature&gt;(owner: &amp;signer, _feature_witness: &amp;Feature)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create&lt;Feature&gt;(owner: &amp;signer, _feature_witness: &amp;Feature) &#123;
    let addr &#61; signer::address_of(owner);
    assert!(!exists&lt;CapState&lt;Feature&gt;&gt;(addr), error::already_exists(ECAPABILITY_ALREADY_EXISTS));
    move_to&lt;CapState&lt;Feature&gt;&gt;(owner, CapState &#123; delegates: vector::empty() &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_capability_acquire"></a>

## Function `acquire`

Acquires a capability token. Only the owner of the capability class, or an authorized delegate,
can succeed with this operation. A caller must pass a witness that they own the <code>Feature</code> type
parameter.


<pre><code>public fun acquire&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::Cap&lt;Feature&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun acquire&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): Cap&lt;Feature&gt;
acquires CapState, CapDelegateState &#123;
    Cap&lt;Feature&gt; &#123; root: validate_acquire&lt;Feature&gt;(requester) &#125;
&#125;
</code></pre>



</details>

<a id="0x1_capability_acquire_linear"></a>

## Function `acquire_linear`

Acquires a linear capability token. It is up to the module which owns <code>Feature</code> to decide
whether to expose a linear or non-linear capability.


<pre><code>public fun acquire_linear&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::LinearCap&lt;Feature&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun acquire_linear&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): LinearCap&lt;Feature&gt;
acquires CapState, CapDelegateState &#123;
    LinearCap&lt;Feature&gt; &#123; root: validate_acquire&lt;Feature&gt;(requester) &#125;
&#125;
</code></pre>



</details>

<a id="0x1_capability_validate_acquire"></a>

## Function `validate_acquire`

Helper to validate an acquire. Returns the root address of the capability.


<pre><code>fun validate_acquire&lt;Feature&gt;(requester: &amp;signer): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_acquire&lt;Feature&gt;(requester: &amp;signer): address
acquires CapState, CapDelegateState &#123;
    let addr &#61; signer::address_of(requester);
    if (exists&lt;CapDelegateState&lt;Feature&gt;&gt;(addr)) &#123;
        let root_addr &#61; borrow_global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root;
        // double check that requester is actually registered as a delegate
        assert!(exists&lt;CapState&lt;Feature&gt;&gt;(root_addr), error::invalid_state(EDELEGATE));
        assert!(vector::contains(&amp;borrow_global&lt;CapState&lt;Feature&gt;&gt;(root_addr).delegates, &amp;addr),
            error::invalid_state(EDELEGATE));
        root_addr
    &#125; else &#123;
        assert!(exists&lt;CapState&lt;Feature&gt;&gt;(addr), error::not_found(ECAPABILITY_NOT_FOUND));
        addr
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_capability_root_addr"></a>

## Function `root_addr`

Returns the root address associated with the given capability token. Only the owner
of the feature can do this.


<pre><code>public fun root_addr&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun root_addr&lt;Feature&gt;(cap: Cap&lt;Feature&gt;, _feature_witness: &amp;Feature): address &#123;
    cap.root
&#125;
</code></pre>



</details>

<a id="0x1_capability_linear_root_addr"></a>

## Function `linear_root_addr`

Returns the root address associated with the given linear capability token.


<pre><code>public fun linear_root_addr&lt;Feature&gt;(cap: capability::LinearCap&lt;Feature&gt;, _feature_witness: &amp;Feature): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun linear_root_addr&lt;Feature&gt;(cap: LinearCap&lt;Feature&gt;, _feature_witness: &amp;Feature): address &#123;
    cap.root
&#125;
</code></pre>



</details>

<a id="0x1_capability_delegate"></a>

## Function `delegate`

Registers a delegation relation. If the relation already exists, this function does
nothing.


<pre><code>public fun delegate&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, to: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegate&lt;Feature&gt;(cap: Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, to: &amp;signer)
acquires CapState &#123;
    let addr &#61; signer::address_of(to);
    if (exists&lt;CapDelegateState&lt;Feature&gt;&gt;(addr)) return;
    move_to(to, CapDelegateState&lt;Feature&gt; &#123; root: cap.root &#125;);
    add_element(&amp;mut borrow_global_mut&lt;CapState&lt;Feature&gt;&gt;(cap.root).delegates, addr);
&#125;
</code></pre>



</details>

<a id="0x1_capability_revoke"></a>

## Function `revoke`

Revokes a delegation relation. If no relation exists, this function does nothing.


<pre><code>public fun revoke&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, from: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun revoke&lt;Feature&gt;(cap: Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, from: address)
acquires CapState, CapDelegateState
&#123;
    if (!exists&lt;CapDelegateState&lt;Feature&gt;&gt;(from)) return;
    let CapDelegateState &#123; root: _root &#125; &#61; move_from&lt;CapDelegateState&lt;Feature&gt;&gt;(from);
    remove_element(&amp;mut borrow_global_mut&lt;CapState&lt;Feature&gt;&gt;(cap.root).delegates, &amp;from);
&#125;
</code></pre>



</details>

<a id="0x1_capability_remove_element"></a>

## Function `remove_element`

Helper to remove an element from a vector.


<pre><code>fun remove_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: &amp;E)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun remove_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: &amp;E) &#123;
    let (found, index) &#61; vector::index_of(v, x);
    if (found) &#123;
        vector::remove(v, index);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_capability_add_element"></a>

## Function `add_element`

Helper to add an element to a vector.


<pre><code>fun add_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: E)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun add_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: E) &#123;
    if (!vector::contains(v, &amp;x)) &#123;
        vector::push_back(v, x)
    &#125;
&#125;
</code></pre>



</details>

<a id="@Specification_4"></a>

## Specification

Helper specification function to check whether a capability exists at address.


<a id="0x1_capability_spec_has_cap"></a>


<pre><code>fun spec_has_cap&lt;Feature&gt;(addr: address): bool &#123;
   exists&lt;CapState&lt;Feature&gt;&gt;(addr)
&#125;
</code></pre>


Helper specification function to obtain the delegates of a capability.


<a id="0x1_capability_spec_delegates"></a>


<pre><code>fun spec_delegates&lt;Feature&gt;(addr: address): vector&lt;address&gt; &#123;
   global&lt;CapState&lt;Feature&gt;&gt;(addr).delegates
&#125;
</code></pre>


Helper specification function to check whether a delegated capability exists at address.


<a id="0x1_capability_spec_has_delegate_cap"></a>


<pre><code>fun spec_has_delegate_cap&lt;Feature&gt;(addr: address): bool &#123;
   exists&lt;CapDelegateState&lt;Feature&gt;&gt;(addr)
&#125;
</code></pre>



<a id="@Specification_4_create"></a>

### Function `create`


<pre><code>public fun create&lt;Feature&gt;(owner: &amp;signer, _feature_witness: &amp;Feature)
</code></pre>




<pre><code>let addr &#61; signer::address_of(owner);
aborts_if spec_has_cap&lt;Feature&gt;(addr);
ensures spec_has_cap&lt;Feature&gt;(addr);
</code></pre>



<a id="@Specification_4_acquire"></a>

### Function `acquire`


<pre><code>public fun acquire&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::Cap&lt;Feature&gt;
</code></pre>




<pre><code>let addr &#61; signer::address_of(requester);
let root_addr &#61; global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root;
include AcquireSchema&lt;Feature&gt;;
ensures spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; root_addr;
ensures !spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; addr;
</code></pre>



<a id="@Specification_4_acquire_linear"></a>

### Function `acquire_linear`


<pre><code>public fun acquire_linear&lt;Feature&gt;(requester: &amp;signer, _feature_witness: &amp;Feature): capability::LinearCap&lt;Feature&gt;
</code></pre>




<pre><code>let addr &#61; signer::address_of(requester);
let root_addr &#61; global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root;
include AcquireSchema&lt;Feature&gt;;
ensures spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; root_addr;
ensures !spec_has_delegate_cap&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; addr;
</code></pre>




<a id="0x1_capability_AcquireSchema"></a>


<pre><code>schema AcquireSchema&lt;Feature&gt; &#123;
    addr: address;
    root_addr: address;
    aborts_if spec_has_delegate_cap&lt;Feature&gt;(addr) &amp;&amp; !spec_has_cap&lt;Feature&gt;(root_addr);
    aborts_if spec_has_delegate_cap&lt;Feature&gt;(addr) &amp;&amp; !vector::spec_contains(spec_delegates&lt;Feature&gt;(root_addr), addr);
    aborts_if !spec_has_delegate_cap&lt;Feature&gt;(addr) &amp;&amp; !spec_has_cap&lt;Feature&gt;(addr);
&#125;
</code></pre>



<a id="@Specification_4_delegate"></a>

### Function `delegate`


<pre><code>public fun delegate&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, to: &amp;signer)
</code></pre>




<pre><code>let addr &#61; signer::address_of(to);
ensures spec_has_delegate_cap&lt;Feature&gt;(addr);
ensures !old(spec_has_delegate_cap&lt;Feature&gt;(addr)) &#61;&#61;&gt; global&lt;CapDelegateState&lt;Feature&gt;&gt;(addr).root &#61;&#61; cap.root;
ensures !old(spec_has_delegate_cap&lt;Feature&gt;(addr)) &#61;&#61;&gt; vector::spec_contains(spec_delegates&lt;Feature&gt;(cap.root), addr);
</code></pre>



<a id="@Specification_4_revoke"></a>

### Function `revoke`


<pre><code>public fun revoke&lt;Feature&gt;(cap: capability::Cap&lt;Feature&gt;, _feature_witness: &amp;Feature, from: address)
</code></pre>




<pre><code>ensures !spec_has_delegate_cap&lt;Feature&gt;(from);
</code></pre>



<a id="@Specification_4_remove_element"></a>

### Function `remove_element`


<pre><code>fun remove_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: &amp;E)
</code></pre>




<a id="@Specification_4_add_element"></a>

### Function `add_element`


<pre><code>fun add_element&lt;E: drop&gt;(v: &amp;mut vector&lt;E&gt;, x: E)
</code></pre>




<pre><code>ensures vector::spec_contains(v, x);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
