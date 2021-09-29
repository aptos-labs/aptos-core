
<a name="0x1_Capability"></a>

# Module `0x1::Capability`

A module which defines the basic concept of
[*capabilities*](https://en.wikipedia.org/wiki/Capability-based_security) for managing access control.


<a name="@Overview_0"></a>

## Overview


A capability is a unforgeable token which testifies that a signer has authorized a certain operation.
The token is valid during the transaction where it is obtained. Since the type <code><a href="Capability.md#0x1_Capability_Cap">Capability::Cap</a></code> has
no ability to be stored in global memory, capabilities cannot leak out of a transaction. For every function
called within a transaction which has a capability as a parameter, it is guaranteed that the capability
has been obtained via a proper signer-based authorization step previously in the transaction's execution.


<a name="@Basic_Usage_1"></a>

### Basic Usage


Capabilities are used typically as follows:

```
struct ProtectedFeature { ... } // this can be just a type tag, or actually some protected data

public fun initialize(s: &signer) {
// Create capability. This happens once at module initialization time.
Capability::create<ProtectedFeature>(s);
}

public fun do_something(s: &signer) {
// Acquire the capability. This is the authorization step. Must have a signer to do so.
let cap = Capability::acquire<ProtectedFeature>(s);
// Pass the capability on to functions which require authorization.
critical(cap);
}

fun critical(cap: Capability::Cap<ProtectedFeature>) {
// Authorization guaranteed by construction -- no verification needed!
...
}
```

Notice that a key feature of capabilities is that they do not require extra verification steps
to ensure authorization is valid.


<a name="@Advanced_Authorization_Scenarios_2"></a>

### Advanced Authorization Scenarios


In the basic usage above, in order to acquire <code><a href="Capability.md#0x1_Capability_Cap">Cap</a>&lt;ProtectedFeature&gt;</code>, we needed a signer
that owns this capability. Because <code>Capability::acquires</code> is a public function, everybody can
acquire the capability provided the right signer is presented. But what if there authorization
scenarios which go beyond having a signer?

The current way how to achieve this in Move is to build a wrapper around <code><a href="Capability.md#0x1_Capability_Cap">Capability::Cap</a>&lt;T&gt;</code>.
The wrapper type will be owned by a specific module, restricting how values of it can be created.
Below, we extend the example from above to illustrate this pattern:

```
struct ProtectedFeatureCap has copy, drop {
cap: Capability::Cap<ProtectedFeature>
}

public fun acquire_protected_feature_access(s: &signer): ProtectedFeatureCap {
let cap = Capability::acquire<ProtectedFeature>(s);
validate_authorization(s, cap); // Do any additional authorization validation
ProtectedFeatureCap{cap}
}
```


<a name="@Delegation_3"></a>

### Delegation


Capabilities come with the optional feature of *delegation*. Via delegation, an owner of a capability
can designate another signer to be also capable of acquiring the capability. Like the original owner,
the delegate needs to present his signer to obtain the capability in his transactions. Delegation can
be revoked, removing this access right from the delegate.

While the basic authorization mechanism for delegates is the same as with core capabilities, the
target of delegation might be subject of restrictions which need to be specified and verified. This can
be done via global invariants in the specification language. For example, in order to prevent delegation
all together for a capability, one can use the following invariant:

```
invariant forall a: address where exists<CapState<ProtectedFeature>>(addr):
len(Capability::spec_delegates<ProtectedFeature>(a)) == 0;
```

Similarly, the following invariant would enforce that delegates, if existent, must satisfy a certain
predicate:

```
invariant forall a: address where exists<CapState<ProtectedFeature>>(addr):
forall d in Capability::spec_delegates<ProtectedFeature>(a):
is_valid_delegate_for_protected_feature(d);
```


-  [Overview](#@Overview_0)
    -  [Basic Usage](#@Basic_Usage_1)
    -  [Advanced Authorization Scenarios](#@Advanced_Authorization_Scenarios_2)
    -  [Delegation](#@Delegation_3)
-  [Struct `Cap`](#0x1_Capability_Cap)
-  [Resource `CapState`](#0x1_Capability_CapState)
-  [Resource `CapDelegateState`](#0x1_Capability_CapDelegateState)
-  [Constants](#@Constants_4)
-  [Function `create`](#0x1_Capability_create)
-  [Function `acquire`](#0x1_Capability_acquire)
-  [Function `delegate`](#0x1_Capability_delegate)
-  [Function `revoke`](#0x1_Capability_revoke)
-  [Function `remove_element`](#0x1_Capability_remove_element)
-  [Function `add_element`](#0x1_Capability_add_element)
-  [Module Specification](#@Module_Specification_5)


<pre><code><b>use</b> <a href="">0x1::Errors</a>;
<b>use</b> <a href="">0x1::Signer</a>;
<b>use</b> <a href="">0x1::Vector</a>;
</code></pre>



<a name="0x1_Capability_Cap"></a>

## Struct `Cap`

The token representing an acquired capability. Cannot be stored in memory, but copied and dropped freely.


<pre><code><b>struct</b> <a href="Capability.md#0x1_Capability_Cap">Cap</a>&lt;Feature&gt; has <b>copy</b>, drop
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

<a name="0x1_Capability_CapState"></a>

## Resource `CapState`

An internal data structure for representing a configured capability.


<pre><code><b>struct</b> <a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt; has key
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

<a name="0x1_Capability_CapDelegateState"></a>

## Resource `CapDelegateState`

An internal data structure for representing a configured delegated capability.


<pre><code><b>struct</b> <a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt; has key
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

<a name="@Constants_4"></a>

## Constants


<a name="0x1_Capability_ECAP"></a>



<pre><code><b>const</b> <a href="Capability.md#0x1_Capability_ECAP">ECAP</a>: u64 = 0;
</code></pre>



<a name="0x1_Capability_EDELEGATE"></a>



<pre><code><b>const</b> <a href="Capability.md#0x1_Capability_EDELEGATE">EDELEGATE</a>: u64 = 1;
</code></pre>



<a name="0x1_Capability_create"></a>

## Function `create`

Creates a new capability class, owned by the passed signer.


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_create">create</a>&lt;Feature&gt;(owner: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_create">create</a>&lt;Feature&gt;(owner: &signer) {
    <b>let</b> addr = <a href="_address_of">Signer::address_of</a>(owner);
    <b>assert</b>(!<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(addr), <a href="_already_published">Errors::already_published</a>(<a href="Capability.md#0x1_Capability_ECAP">ECAP</a>));
    move_to&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(owner, <a href="Capability.md#0x1_Capability_CapState">CapState</a>{ delegates: <a href="_empty">Vector::empty</a>() });
}
</code></pre>



</details>

<a name="0x1_Capability_acquire"></a>

## Function `acquire`

Acquires a capability token. Only the owner of the capability class, or an authorized delegate,
can succeed with this operation.


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_acquire">acquire</a>&lt;Feature&gt;(requester: &signer): <a href="Capability.md#0x1_Capability_Cap">Capability::Cap</a>&lt;Feature&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_acquire">acquire</a>&lt;Feature&gt;(requester: &signer): <a href="Capability.md#0x1_Capability_Cap">Cap</a>&lt;Feature&gt;
<b>acquires</b> <a href="Capability.md#0x1_Capability_CapState">CapState</a>, <a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a> {
    <b>let</b> addr = <a href="_address_of">Signer::address_of</a>(requester);
    <b>if</b> (<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr)) {
        <b>let</b> root_addr = borrow_global&lt;<a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr).root;
        // double check that requester is actually registered <b>as</b> a delegate
        <b>assert</b>(<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(root_addr), <a href="_invalid_state">Errors::invalid_state</a>(<a href="Capability.md#0x1_Capability_EDELEGATE">EDELEGATE</a>));
        <b>assert</b>(<a href="_contains">Vector::contains</a>(&borrow_global&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(root_addr).delegates, &addr),
               <a href="_invalid_state">Errors::invalid_state</a>(<a href="Capability.md#0x1_Capability_EDELEGATE">EDELEGATE</a>));
        <a href="Capability.md#0x1_Capability_Cap">Cap</a>&lt;Feature&gt;{root: root_addr}
    } <b>else</b> {
        <b>assert</b>(<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(addr), <a href="_not_published">Errors::not_published</a>(<a href="Capability.md#0x1_Capability_ECAP">ECAP</a>));
        <a href="Capability.md#0x1_Capability_Cap">Cap</a>&lt;Feature&gt;{root: addr}
    }
}
</code></pre>



</details>

<a name="0x1_Capability_delegate"></a>

## Function `delegate`

Registers a delegation relation.


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_delegate">delegate</a>&lt;Feature&gt;(cap: <a href="Capability.md#0x1_Capability_Cap">Capability::Cap</a>&lt;Feature&gt;, <b>to</b>: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_delegate">delegate</a>&lt;Feature&gt;(cap: <a href="Capability.md#0x1_Capability_Cap">Cap</a>&lt;Feature&gt;, <b>to</b>: &signer)
<b>acquires</b> <a href="Capability.md#0x1_Capability_CapState">CapState</a> {
    <b>let</b> addr = <a href="_address_of">Signer::address_of</a>(<b>to</b>);
    <b>assert</b>(!<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(addr), <a href="_already_published">Errors::already_published</a>(<a href="Capability.md#0x1_Capability_EDELEGATE">EDELEGATE</a>));
    <b>assert</b>(<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(cap.root), <a href="_invalid_state">Errors::invalid_state</a>(<a href="Capability.md#0x1_Capability_ECAP">ECAP</a>));
    move_to(<b>to</b>, <a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;{root: cap.root});
    <a href="Capability.md#0x1_Capability_add_element">add_element</a>(&<b>mut</b> borrow_global_mut&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(cap.root).delegates, addr);
}
</code></pre>



</details>

<a name="0x1_Capability_revoke"></a>

## Function `revoke`

Revokes a delegation relation.


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_revoke">revoke</a>&lt;Feature&gt;(cap: <a href="Capability.md#0x1_Capability_Cap">Capability::Cap</a>&lt;Feature&gt;, from: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Capability.md#0x1_Capability_revoke">revoke</a>&lt;Feature&gt;(cap: <a href="Capability.md#0x1_Capability_Cap">Cap</a>&lt;Feature&gt;, from: address)
<b>acquires</b> <a href="Capability.md#0x1_Capability_CapState">CapState</a>, <a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>
{
    <b>assert</b>(<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(from), <a href="_not_published">Errors::not_published</a>(<a href="Capability.md#0x1_Capability_EDELEGATE">EDELEGATE</a>));
    <b>assert</b>(<b>exists</b>&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(cap.root), <a href="_invalid_state">Errors::invalid_state</a>(<a href="Capability.md#0x1_Capability_ECAP">ECAP</a>));
    <b>let</b> <a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>{root: _root} = move_from&lt;<a href="Capability.md#0x1_Capability_CapDelegateState">CapDelegateState</a>&lt;Feature&gt;&gt;(from);
    <a href="Capability.md#0x1_Capability_remove_element">remove_element</a>(&<b>mut</b> borrow_global_mut&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(cap.root).delegates, &from);
}
</code></pre>



</details>

<a name="0x1_Capability_remove_element"></a>

## Function `remove_element`

Helper to remove an element from a vector.


<pre><code><b>fun</b> <a href="Capability.md#0x1_Capability_remove_element">remove_element</a>&lt;E: drop&gt;(v: &<b>mut</b> vector&lt;E&gt;, x: &E)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Capability.md#0x1_Capability_remove_element">remove_element</a>&lt;E: drop&gt;(v: &<b>mut</b> vector&lt;E&gt;, x: &E) {
    <b>let</b> (found, index) = <a href="_index_of">Vector::index_of</a>(v, x);
    <b>if</b> (found) {
        <a href="_remove">Vector::remove</a>(v, index);
    }
}
</code></pre>



</details>

<a name="0x1_Capability_add_element"></a>

## Function `add_element`

Helper to add an element to a vector.


<pre><code><b>fun</b> <a href="Capability.md#0x1_Capability_add_element">add_element</a>&lt;E: drop&gt;(v: &<b>mut</b> vector&lt;E&gt;, x: E)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Capability.md#0x1_Capability_add_element">add_element</a>&lt;E: drop&gt;(v: &<b>mut</b> vector&lt;E&gt;, x: E) {
    <b>if</b> (!<a href="_contains">Vector::contains</a>(v, &x)) {
        <a href="_push_back">Vector::push_back</a>(v, x)
    }
}
</code></pre>



</details>

<a name="@Module_Specification_5"></a>

## Module Specification

Helper specification function to obtain the delegates of a capability.


<a name="0x1_Capability_spec_delegates"></a>


<pre><code><b>fun</b> <a href="Capability.md#0x1_Capability_spec_delegates">spec_delegates</a>&lt;Feature&gt;(addr: address): vector&lt;address&gt; {
   <b>global</b>&lt;<a href="Capability.md#0x1_Capability_CapState">CapState</a>&lt;Feature&gt;&gt;(addr).delegates
}
</code></pre>
