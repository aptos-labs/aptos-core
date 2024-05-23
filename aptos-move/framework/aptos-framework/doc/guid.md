
<a id="0x1_guid"></a>

# Module `0x1::guid`

A module for generating globally unique identifiers


-  [Struct `GUID`](#0x1_guid_GUID)
-  [Struct `ID`](#0x1_guid_ID)
-  [Constants](#@Constants_0)
-  [Function `create`](#0x1_guid_create)
-  [Function `create_id`](#0x1_guid_create_id)
-  [Function `id`](#0x1_guid_id)
-  [Function `creator_address`](#0x1_guid_creator_address)
-  [Function `id_creator_address`](#0x1_guid_id_creator_address)
-  [Function `creation_num`](#0x1_guid_creation_num)
-  [Function `id_creation_num`](#0x1_guid_id_creation_num)
-  [Function `eq_id`](#0x1_guid_eq_id)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create`](#@Specification_1_create)
    -  [Function `create_id`](#@Specification_1_create_id)
    -  [Function `id`](#@Specification_1_id)
    -  [Function `creator_address`](#@Specification_1_creator_address)
    -  [Function `id_creator_address`](#@Specification_1_id_creator_address)
    -  [Function `creation_num`](#@Specification_1_creation_num)
    -  [Function `id_creation_num`](#@Specification_1_id_creation_num)
    -  [Function `eq_id`](#@Specification_1_eq_id)


<pre><code></code></pre>



<a id="0x1_guid_GUID"></a>

## Struct `GUID`

A globally unique identifier derived from the sender&apos;s address and a counter


<pre><code>struct GUID has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: guid::ID</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_guid_ID"></a>

## Struct `ID`

A non&#45;privileged identifier that can be freely created by anyone. Useful for looking up GUID&apos;s.


<pre><code>struct ID has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creation_num: u64</code>
</dt>
<dd>
 If creation_num is <code>i</code>, this is the <code>i&#43;1</code>th GUID created by <code>addr</code>
</dd>
<dt>
<code>addr: address</code>
</dt>
<dd>
 Address that created the GUID
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_guid_EGUID_GENERATOR_NOT_PUBLISHED"></a>

GUID generator must be published ahead of first usage of <code>create_with_capability</code> function.


<pre><code>const EGUID_GENERATOR_NOT_PUBLISHED: u64 &#61; 0;<br/></code></pre>



<a id="0x1_guid_create"></a>

## Function `create`

Create and return a new GUID from a trusted module.


<pre><code>public(friend) fun create(addr: address, creation_num_ref: &amp;mut u64): guid::GUID<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create(addr: address, creation_num_ref: &amp;mut u64): GUID &#123;<br/>    let creation_num &#61; &#42;creation_num_ref;<br/>    &#42;creation_num_ref &#61; creation_num &#43; 1;<br/>    GUID &#123;<br/>        id: ID &#123;<br/>            creation_num,<br/>            addr,<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_guid_create_id"></a>

## Function `create_id`

Create a non&#45;privileged id from <code>addr</code> and <code>creation_num</code>


<pre><code>public fun create_id(addr: address, creation_num: u64): guid::ID<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_id(addr: address, creation_num: u64): ID &#123;<br/>    ID &#123; creation_num, addr &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_guid_id"></a>

## Function `id`

Get the non&#45;privileged ID associated with a GUID


<pre><code>public fun id(guid: &amp;guid::GUID): guid::ID<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun id(guid: &amp;GUID): ID &#123;<br/>    guid.id<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_guid_creator_address"></a>

## Function `creator_address`

Return the account address that created the GUID


<pre><code>public fun creator_address(guid: &amp;guid::GUID): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun creator_address(guid: &amp;GUID): address &#123;<br/>    guid.id.addr<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_guid_id_creator_address"></a>

## Function `id_creator_address`

Return the account address that created the guid::ID


<pre><code>public fun id_creator_address(id: &amp;guid::ID): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun id_creator_address(id: &amp;ID): address &#123;<br/>    id.addr<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_guid_creation_num"></a>

## Function `creation_num`

Return the creation number associated with the GUID


<pre><code>public fun creation_num(guid: &amp;guid::GUID): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun creation_num(guid: &amp;GUID): u64 &#123;<br/>    guid.id.creation_num<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_guid_id_creation_num"></a>

## Function `id_creation_num`

Return the creation number associated with the guid::ID


<pre><code>public fun id_creation_num(id: &amp;guid::ID): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun id_creation_num(id: &amp;ID): u64 &#123;<br/>    id.creation_num<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_guid_eq_id"></a>

## Function `eq_id`

Return true if the GUID&apos;s ID is <code>id</code>


<pre><code>public fun eq_id(guid: &amp;guid::GUID, id: &amp;guid::ID): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun eq_id(guid: &amp;GUID, id: &amp;ID): bool &#123;<br/>    &amp;guid.id &#61;&#61; id<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The creation of GUID constructs a unique GUID by combining an address with an incremented creation number.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The create function generates a new GUID by combining an address with an incremented creation number, effectively creating a unique identifier.&lt;/td&gt;<br/>&lt;td&gt;Enforced via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;create&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The operations on GUID and ID, such as construction, field access, and equality comparison, should not abort.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The following functions will never abort: (1) create_id, (2) id, (3) creator_address, (4) id_creator_address, (5) creation_num, (6) id_creation_num, and (7) eq_id.&lt;/td&gt;<br/>&lt;td&gt;Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.1&quot;&gt;create_id&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.2&quot;&gt;id&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.3&quot;&gt;creator_address&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.4&quot;&gt;id_creator_address&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.5&quot;&gt;creation_num&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.6&quot;&gt;id_creation_num&lt;/a&gt;, and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2.7&quot;&gt;eq_id&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The creation number should increment by 1 with each new creation.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;An account can only own up to MAX_U64 resources. Not incrementing the guid_creation_num constantly could lead to shrinking the space for new resources.&lt;/td&gt;<br/>&lt;td&gt;Enforced via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;create&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The creation number and address of an ID / GUID must be immutable.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The addr and creation_num values are meant to be constant and never updated as they are unique and used for identification.&lt;/td&gt;<br/>&lt;td&gt;Audited: This is enforced through missing functionality to update the creation_num or addr.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_create"></a>

### Function `create`


<pre><code>public(friend) fun create(addr: address, creation_num_ref: &amp;mut u64): guid::GUID<br/></code></pre>




<pre><code>aborts_if creation_num_ref &#43; 1 &gt; MAX_U64;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures result.id.creation_num &#61;&#61; old(creation_num_ref);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
ensures creation_num_ref &#61;&#61; old(creation_num_ref) &#43; 1;<br/></code></pre>



<a id="@Specification_1_create_id"></a>

### Function `create_id`


<pre><code>public fun create_id(addr: address, creation_num: u64): guid::ID<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_id"></a>

### Function `id`


<pre><code>public fun id(guid: &amp;guid::GUID): guid::ID<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_creator_address"></a>

### Function `creator_address`


<pre><code>public fun creator_address(guid: &amp;guid::GUID): address<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_id_creator_address"></a>

### Function `id_creator_address`


<pre><code>public fun id_creator_address(id: &amp;guid::ID): address<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_creation_num"></a>

### Function `creation_num`


<pre><code>public fun creation_num(guid: &amp;guid::GUID): u64<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_id_creation_num"></a>

### Function `id_creation_num`


<pre><code>public fun id_creation_num(id: &amp;guid::ID): u64<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.6&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_eq_id"></a>

### Function `eq_id`


<pre><code>public fun eq_id(guid: &amp;guid::GUID, id: &amp;guid::ID): bool<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2.7&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
