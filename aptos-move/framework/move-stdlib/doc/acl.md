
<a id="0x1_acl"></a>

# Module `0x1::acl`

Access control list (acl) module. An acl is a list of account addresses who<br/> have the access permission to a certain object.<br/> This module uses a <code>vector</code> to represent the list, but can be refactored to<br/> use a &quot;set&quot; instead when it&apos;s available in the language in the future.


-  [Struct `ACL`](#0x1_acl_ACL)
-  [Constants](#@Constants_0)
-  [Function `empty`](#0x1_acl_empty)
-  [Function `add`](#0x1_acl_add)
-  [Function `remove`](#0x1_acl_remove)
-  [Function `contains`](#0x1_acl_contains)
-  [Function `assert_contains`](#0x1_acl_assert_contains)
-  [Specification](#@Specification_1)
    -  [Struct `ACL`](#@Specification_1_ACL)
    -  [Function `add`](#@Specification_1_add)
    -  [Function `remove`](#@Specification_1_remove)
    -  [Function `contains`](#@Specification_1_contains)
    -  [Function `assert_contains`](#@Specification_1_assert_contains)


<pre><code>use 0x1::error;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_acl_ACL"></a>

## Struct `ACL`



<pre><code>struct ACL has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>list: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_acl_ECONTAIN"></a>

The ACL already contains the address.


<pre><code>const ECONTAIN: u64 &#61; 0;<br/></code></pre>



<a id="0x1_acl_ENOT_CONTAIN"></a>

The ACL does not contain the address.


<pre><code>const ENOT_CONTAIN: u64 &#61; 1;<br/></code></pre>



<a id="0x1_acl_empty"></a>

## Function `empty`

Return an empty ACL.


<pre><code>public fun empty(): acl::ACL<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun empty(): ACL &#123;<br/>    ACL&#123; list: vector::empty&lt;address&gt;() &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_acl_add"></a>

## Function `add`

Add the address to the ACL.


<pre><code>public fun add(acl: &amp;mut acl::ACL, addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add(acl: &amp;mut ACL, addr: address) &#123;<br/>    assert!(!vector::contains(&amp;mut acl.list, &amp;addr), error::invalid_argument(ECONTAIN));<br/>    vector::push_back(&amp;mut acl.list, addr);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_acl_remove"></a>

## Function `remove`

Remove the address from the ACL.


<pre><code>public fun remove(acl: &amp;mut acl::ACL, addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove(acl: &amp;mut ACL, addr: address) &#123;<br/>    let (found, index) &#61; vector::index_of(&amp;mut acl.list, &amp;addr);<br/>    assert!(found, error::invalid_argument(ENOT_CONTAIN));<br/>    vector::remove(&amp;mut acl.list, index);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_acl_contains"></a>

## Function `contains`

Return true iff the ACL contains the address.


<pre><code>public fun contains(acl: &amp;acl::ACL, addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains(acl: &amp;ACL, addr: address): bool &#123;<br/>    vector::contains(&amp;acl.list, &amp;addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_acl_assert_contains"></a>

## Function `assert_contains`

assert! that the ACL has the address.


<pre><code>public fun assert_contains(acl: &amp;acl::ACL, addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_contains(acl: &amp;ACL, addr: address) &#123;<br/>    assert!(contains(acl, addr), error::invalid_argument(ENOT_CONTAIN));<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_ACL"></a>

### Struct `ACL`


<pre><code>struct ACL has copy, drop, store<br/></code></pre>



<dl>
<dt>
<code>list: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant forall i in 0..len(list), j in 0..len(list): list[i] &#61;&#61; list[j] &#61;&#61;&gt; i &#61;&#61; j;<br/></code></pre>




<a id="0x1_acl_spec_contains"></a>


<pre><code>fun spec_contains(acl: ACL, addr: address): bool &#123;<br/>   exists a in acl.list: a &#61;&#61; addr<br/>&#125;<br/></code></pre>



<a id="@Specification_1_add"></a>

### Function `add`


<pre><code>public fun add(acl: &amp;mut acl::ACL, addr: address)<br/></code></pre>




<pre><code>aborts_if spec_contains(acl, addr) with error::INVALID_ARGUMENT;<br/>ensures spec_contains(acl, addr);<br/></code></pre>



<a id="@Specification_1_remove"></a>

### Function `remove`


<pre><code>public fun remove(acl: &amp;mut acl::ACL, addr: address)<br/></code></pre>




<pre><code>aborts_if !spec_contains(acl, addr) with error::INVALID_ARGUMENT;<br/>ensures !spec_contains(acl, addr);<br/></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code>public fun contains(acl: &amp;acl::ACL, addr: address): bool<br/></code></pre>




<pre><code>ensures result &#61;&#61; spec_contains(acl, addr);<br/></code></pre>



<a id="@Specification_1_assert_contains"></a>

### Function `assert_contains`


<pre><code>public fun assert_contains(acl: &amp;acl::ACL, addr: address)<br/></code></pre>




<pre><code>aborts_if !spec_contains(acl, addr) with error::INVALID_ARGUMENT;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
