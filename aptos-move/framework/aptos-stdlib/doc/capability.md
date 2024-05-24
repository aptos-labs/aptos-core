
<a id="0x1_capability"></a>

# Module `0x1::capability`

A module which defines the basic concept of
[&#42;capabilities&#42;](https://en.wikipedia.org/wiki/Capability&#45;based_security) for managing access control.

EXPERIMENTAL


<a id="@Overview_0"></a>

## Overview


A capability is a unforgeable token which testifies that a signer has authorized a certain operation.
The token is valid during the transaction where it is obtained. Since the type <code><a href="capability.md#0x1_capability_Cap">capability::Cap</a></code> has
no ability to be stored in global memory, capabilities cannot leak out of a transaction. For every function
called within a transaction which has a capability as a parameter, it is guaranteed that the capability
has been obtained via a proper signer&#45;based authorization step previously in the transaction&apos;s execution.


<a id="@Usage_1"></a>

### Usage


Initializing and acquiring capabilities is usually encapsulated in a module with a type
tag which can only be constructed by this module.

```
module Pkg::Feature &#123;
use std::capability::Cap;

/// A type tag used in Cap&lt;Feature&gt;. Only this module can create an instance,
/// and there is no public function other than Self::acquire which returns a value of this type.
/// This way, this module has full control how Cap&lt;Feature&gt; is given out.
struct Feature has drop &#123;&#125;

/// Initializes this module.
public fun initialize(s: &amp;signer) &#123;
// Create capability. This happens once at module initialization time.
// One needs to provide a witness for being the owner of Feature
// in the 2nd parameter.
&lt;&lt;additional conditions allowing to initialize this capability&gt;&gt;
capability::create&lt;Feature&gt;(s, &amp;Feature&#123;&#125;);
&#125;

/// Acquires the capability to work with this feature.
public fun acquire(s: &amp;signer): Cap&lt;Feature&gt; &#123;
&lt;&lt;additional conditions allowing to acquire this capability&gt;&gt;
capability::acquire&lt;Feature&gt;(s, &amp;Feature&#123;&#125;);
&#125;

/// Does something related to the feature. The caller must pass a Cap&lt;Feature&gt;.
public fun do_something(_cap: Cap&lt;Feature&gt;) &#123; ... &#125;
&#125;
```


<a id="@Delegation_2"></a>

### Delegation


Capabilities come with the optional feature of &#42;delegation&#42;. Via <code><a href="capability.md#0x1_capability_delegate">Self::delegate</a></code>, an owner of a capability
can designate another signer to be also capable of acquiring the capability. Like the original creator,
the delegate needs to present his signer to obtain the capability in his transactions. Delegation can
be revoked via <code><a href="capability.md#0x1_capability_revoke">Self::revoke</a></code>, removing this access right from the delegate.

While the basic authorization mechanism for delegates is the same as with core capabilities, the
target of delegation might be subject of restrictions which need to be specified and verified. This can
be done via global invariants in the specification language. For example, in order to prevent delegation
all together for a capability, one can use the following invariant:

```
invariant forall a: address where capability::spec_has_cap&lt;Feature&gt;(a):
len(capability::spec_delegates&lt;Feature&gt;(a)) &#61;&#61; 0;
```

Similarly, the following invariant would enforce that delegates, if existent, must satisfy a certain
predicate:

```
invariant forall a: address where capability::spec_has_cap&lt;Feature&gt;(a):
forall d in capability::spec_delegates&lt;Feature&gt;(a):
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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_capability_Cap"></a>

## Struct `Cap`

The token representing an acquired capability. Cannot be stored in memory, but copied and dropped freely.


<pre><code><b>struct</b> <a href="capability.md#0x1_capability_Cap">Cap</a>&lt;Feature&gt; <b>has</b> <b>copy</b>, drop<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_capability_LinearCap"></a>

## Struct `LinearCap`

A linear version of a capability token. This can be used if an acquired capability should be enforced
to be used only once for an authorization.


<pre><code><b>struct</b> <a href="capability.md#0x1_capability_LinearCap">LinearCap</a>&lt;Feature&gt; <b>has</b> drop<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_capability_CapState"></a>

## Resource `CapState`

An internal data structure for representing a configured capability.


<pre><code><b>struct</b> <a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt; <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>delegates: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_capability_CapDelegateState"></a>

## Resource `CapDelegateState`

An internal data structure for representing a configured delegated capability.


<pre><code><b>struct</b> <a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt; <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_3"></a>

## Constants


<a id="0x1_capability_ECAPABILITY_ALREADY_EXISTS"></a>

Capability resource already exists on the specified account


<pre><code><b>const</b> <a href="capability.md#0x1_capability_ECAPABILITY_ALREADY_EXISTS">ECAPABILITY_ALREADY_EXISTS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_capability_ECAPABILITY_NOT_FOUND"></a>

Capability resource not found


<pre><code><b>const</b> <a href="capability.md#0x1_capability_ECAPABILITY_NOT_FOUND">ECAPABILITY_NOT_FOUND</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_capability_EDELEGATE"></a>

Account does not have delegated permissions


<pre><code><b>const</b> <a href="capability.md#0x1_capability_EDELEGATE">EDELEGATE</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_capability_create"></a>

## Function `create`

Creates a new capability class, owned by the passed signer. A caller must pass a witness that
they own the <code>Feature</code> type parameter.


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_create">create</a>&lt;Feature&gt;(owner: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_create">create</a>&lt;Feature&gt;(owner: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature) &#123;<br />    <b>let</b> addr &#61; <a href="../../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>assert</b>!(!<b>exists</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(addr), <a href="../../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="capability.md#0x1_capability_ECAPABILITY_ALREADY_EXISTS">ECAPABILITY_ALREADY_EXISTS</a>));<br />    <b>move_to</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(owner, <a href="capability.md#0x1_capability_CapState">CapState</a> &#123; delegates: <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>() &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_acquire"></a>

## Function `acquire`

Acquires a capability token. Only the owner of the capability class, or an authorized delegate,
can succeed with this operation. A caller must pass a witness that they own the <code>Feature</code> type
parameter.


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_acquire">acquire</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature): <a href="capability.md#0x1_capability_Cap">capability::Cap</a>&lt;Feature&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_acquire">acquire</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature): <a href="capability.md#0x1_capability_Cap">Cap</a>&lt;Feature&gt;<br /><b>acquires</b> <a href="capability.md#0x1_capability_CapState">CapState</a>, <a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a> &#123;<br />    <a href="capability.md#0x1_capability_Cap">Cap</a>&lt;Feature&gt; &#123; root: <a href="capability.md#0x1_capability_validate_acquire">validate_acquire</a>&lt;Feature&gt;(requester) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_acquire_linear"></a>

## Function `acquire_linear`

Acquires a linear capability token. It is up to the module which owns <code>Feature</code> to decide
whether to expose a linear or non&#45;linear capability.


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_acquire_linear">acquire_linear</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature): <a href="capability.md#0x1_capability_LinearCap">capability::LinearCap</a>&lt;Feature&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_acquire_linear">acquire_linear</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature): <a href="capability.md#0x1_capability_LinearCap">LinearCap</a>&lt;Feature&gt;<br /><b>acquires</b> <a href="capability.md#0x1_capability_CapState">CapState</a>, <a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a> &#123;<br />    <a href="capability.md#0x1_capability_LinearCap">LinearCap</a>&lt;Feature&gt; &#123; root: <a href="capability.md#0x1_capability_validate_acquire">validate_acquire</a>&lt;Feature&gt;(requester) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_validate_acquire"></a>

## Function `validate_acquire`

Helper to validate an acquire. Returns the root address of the capability.


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_validate_acquire">validate_acquire</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_validate_acquire">validate_acquire</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b><br /><b>acquires</b> <a href="capability.md#0x1_capability_CapState">CapState</a>, <a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(requester);<br />    <b>if</b> (<b>exists</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr)) &#123;<br />        <b>let</b> root_addr &#61; <b>borrow_global</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr).root;<br />        // double check that requester is actually registered <b>as</b> a delegate<br />        <b>assert</b>!(<b>exists</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(root_addr), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="capability.md#0x1_capability_EDELEGATE">EDELEGATE</a>));<br />        <b>assert</b>!(<a href="../../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&amp;<b>borrow_global</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(root_addr).delegates, &amp;addr),<br />            <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="capability.md#0x1_capability_EDELEGATE">EDELEGATE</a>));<br />        root_addr<br />    &#125; <b>else</b> &#123;<br />        <b>assert</b>!(<b>exists</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(addr), <a href="../../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="capability.md#0x1_capability_ECAPABILITY_NOT_FOUND">ECAPABILITY_NOT_FOUND</a>));<br />        addr<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_root_addr"></a>

## Function `root_addr`

Returns the root address associated with the given capability token. Only the owner
of the feature can do this.


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_root_addr">root_addr</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">capability::Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_root_addr">root_addr</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature): <b>address</b> &#123;<br />    cap.root<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_linear_root_addr"></a>

## Function `linear_root_addr`

Returns the root address associated with the given linear capability token.


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_linear_root_addr">linear_root_addr</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_LinearCap">capability::LinearCap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_linear_root_addr">linear_root_addr</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_LinearCap">LinearCap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature): <b>address</b> &#123;<br />    cap.root<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_delegate"></a>

## Function `delegate`

Registers a delegation relation. If the relation already exists, this function does
nothing.


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_delegate">delegate</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">capability::Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature, <b>to</b>: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_delegate">delegate</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature, <b>to</b>: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /><b>acquires</b> <a href="capability.md#0x1_capability_CapState">CapState</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<b>to</b>);<br />    <b>if</b> (<b>exists</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr)) <b>return</b>;<br />    <b>move_to</b>(<b>to</b>, <a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt; &#123; root: cap.root &#125;);<br />    <a href="capability.md#0x1_capability_add_element">add_element</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(cap.root).delegates, addr);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_revoke"></a>

## Function `revoke`

Revokes a delegation relation. If no relation exists, this function does nothing.


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_revoke">revoke</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">capability::Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature, from: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_revoke">revoke</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature, from: <b>address</b>)<br /><b>acquires</b> <a href="capability.md#0x1_capability_CapState">CapState</a>, <a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a><br />&#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(from)) <b>return</b>;<br />    <b>let</b> <a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a> &#123; root: _root &#125; &#61; <b>move_from</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(from);<br />    <a href="capability.md#0x1_capability_remove_element">remove_element</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(cap.root).delegates, &amp;from);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_remove_element"></a>

## Function `remove_element`

Helper to remove an element from a vector.


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_remove_element">remove_element</a>&lt;E: drop&gt;(v: &amp;<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;, x: &amp;E)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_remove_element">remove_element</a>&lt;E: drop&gt;(v: &amp;<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;, x: &amp;E) &#123;<br />    <b>let</b> (found, index) &#61; <a href="../../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(v, x);<br />    <b>if</b> (found) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(v, index);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_capability_add_element"></a>

## Function `add_element`

Helper to add an element to a vector.


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_add_element">add_element</a>&lt;E: drop&gt;(v: &amp;<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;, x: E)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_add_element">add_element</a>&lt;E: drop&gt;(v: &amp;<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;, x: E) &#123;<br />    <b>if</b> (!<a href="../../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(v, &amp;x)) &#123;<br />        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(v, x)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_4"></a>

## Specification

Helper specification function to check whether a capability exists at address.


<a id="0x1_capability_spec_has_cap"></a>


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_spec_has_cap">spec_has_cap</a>&lt;Feature&gt;(addr: <b>address</b>): bool &#123;<br />   <b>exists</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(addr)<br />&#125;<br /></code></pre>


Helper specification function to obtain the delegates of a capability.


<a id="0x1_capability_spec_delegates"></a>


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_spec_delegates">spec_delegates</a>&lt;Feature&gt;(addr: <b>address</b>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; &#123;<br />   <b>global</b>&lt;<a href="capability.md#0x1_capability_CapState">CapState</a>&lt;Feature&gt;&gt;(addr).delegates<br />&#125;<br /></code></pre>


Helper specification function to check whether a delegated capability exists at address.


<a id="0x1_capability_spec_has_delegate_cap"></a>


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr: <b>address</b>): bool &#123;<br />   <b>exists</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr)<br />&#125;<br /></code></pre>



<a id="@Specification_4_create"></a>

### Function `create`


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_create">create</a>&lt;Feature&gt;(owner: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>aborts_if</b> <a href="capability.md#0x1_capability_spec_has_cap">spec_has_cap</a>&lt;Feature&gt;(addr);<br /><b>ensures</b> <a href="capability.md#0x1_capability_spec_has_cap">spec_has_cap</a>&lt;Feature&gt;(addr);<br /></code></pre>



<a id="@Specification_4_acquire"></a>

### Function `acquire`


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_acquire">acquire</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature): <a href="capability.md#0x1_capability_Cap">capability::Cap</a>&lt;Feature&gt;<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(requester);<br /><b>let</b> root_addr &#61; <b>global</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr).root;<br /><b>include</b> <a href="capability.md#0x1_capability_AcquireSchema">AcquireSchema</a>&lt;Feature&gt;;<br /><b>ensures</b> <a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; root_addr;<br /><b>ensures</b> !<a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; addr;<br /></code></pre>



<a id="@Specification_4_acquire_linear"></a>

### Function `acquire_linear`


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_acquire_linear">acquire_linear</a>&lt;Feature&gt;(requester: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _feature_witness: &amp;Feature): <a href="capability.md#0x1_capability_LinearCap">capability::LinearCap</a>&lt;Feature&gt;<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(requester);<br /><b>let</b> root_addr &#61; <b>global</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr).root;<br /><b>include</b> <a href="capability.md#0x1_capability_AcquireSchema">AcquireSchema</a>&lt;Feature&gt;;<br /><b>ensures</b> <a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; root_addr;<br /><b>ensures</b> !<a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr) &#61;&#61;&gt; result.root &#61;&#61; addr;<br /></code></pre>




<a id="0x1_capability_AcquireSchema"></a>


<pre><code><b>schema</b> <a href="capability.md#0x1_capability_AcquireSchema">AcquireSchema</a>&lt;Feature&gt; &#123;<br />addr: <b>address</b>;<br />root_addr: <b>address</b>;<br /><b>aborts_if</b> <a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr) &amp;&amp; !<a href="capability.md#0x1_capability_spec_has_cap">spec_has_cap</a>&lt;Feature&gt;(root_addr);<br /><b>aborts_if</b> <a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr) &amp;&amp; !<a href="../../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(<a href="capability.md#0x1_capability_spec_delegates">spec_delegates</a>&lt;Feature&gt;(root_addr), addr);<br /><b>aborts_if</b> !<a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr) &amp;&amp; !<a href="capability.md#0x1_capability_spec_has_cap">spec_has_cap</a>&lt;Feature&gt;(addr);<br />&#125;<br /></code></pre>



<a id="@Specification_4_delegate"></a>

### Function `delegate`


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_delegate">delegate</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">capability::Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature, <b>to</b>: &amp;<a href="../../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<b>to</b>);<br /><b>ensures</b> <a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr);<br /><b>ensures</b> !<b>old</b>(<a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr)) &#61;&#61;&gt; <b>global</b>&lt;<a href="capability.md#0x1_capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr).root &#61;&#61; cap.root;<br /><b>ensures</b> !<b>old</b>(<a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(addr)) &#61;&#61;&gt; <a href="../../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(<a href="capability.md#0x1_capability_spec_delegates">spec_delegates</a>&lt;Feature&gt;(cap.root), addr);<br /></code></pre>



<a id="@Specification_4_revoke"></a>

### Function `revoke`


<pre><code><b>public</b> <b>fun</b> <a href="capability.md#0x1_capability_revoke">revoke</a>&lt;Feature&gt;(cap: <a href="capability.md#0x1_capability_Cap">capability::Cap</a>&lt;Feature&gt;, _feature_witness: &amp;Feature, from: <b>address</b>)<br /></code></pre>




<pre><code><b>ensures</b> !<a href="capability.md#0x1_capability_spec_has_delegate_cap">spec_has_delegate_cap</a>&lt;Feature&gt;(from);<br /></code></pre>



<a id="@Specification_4_remove_element"></a>

### Function `remove_element`


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_remove_element">remove_element</a>&lt;E: drop&gt;(v: &amp;<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;, x: &amp;E)<br /></code></pre>




<a id="@Specification_4_add_element"></a>

### Function `add_element`


<pre><code><b>fun</b> <a href="capability.md#0x1_capability_add_element">add_element</a>&lt;E: drop&gt;(v: &amp;<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;E&gt;, x: E)<br /></code></pre>




<pre><code><b>ensures</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(v, x);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
