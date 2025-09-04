
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

A globally unique identifier derived from the sender's address and a counter


<pre><code><b>struct</b> <a href="guid.md#0x1_guid_GUID">GUID</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="guid.md#0x1_guid_ID">guid::ID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_guid_ID"></a>

## Struct `ID`

A non-privileged identifier that can be freely created by anyone. Useful for looking up GUID's.


<pre><code><b>struct</b> <a href="guid.md#0x1_guid_ID">ID</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creation_num: u64</code>
</dt>
<dd>
 If creation_num is <code>i</code>, this is the <code>i+1</code>th GUID created by <code>addr</code>
</dd>
<dt>
<code>addr: <b>address</b></code>
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


<pre><code><b>const</b> <a href="guid.md#0x1_guid_EGUID_GENERATOR_NOT_PUBLISHED">EGUID_GENERATOR_NOT_PUBLISHED</a>: u64 = 0;
</code></pre>



<a id="0x1_guid_create"></a>

## Function `create`

Create and return a new GUID from a trusted module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="guid.md#0x1_guid_create">create</a>(addr: <b>address</b>, creation_num_ref: &<b>mut</b> u64): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="guid.md#0x1_guid_create">create</a>(addr: <b>address</b>, creation_num_ref: &<b>mut</b> u64): <a href="guid.md#0x1_guid_GUID">GUID</a> {
    <b>let</b> creation_num = *creation_num_ref;
    *creation_num_ref = creation_num + 1;
    <a href="guid.md#0x1_guid_GUID">GUID</a> {
        id: <a href="guid.md#0x1_guid_ID">ID</a> {
            creation_num,
            addr,
        }
    }
}
</code></pre>



</details>

<a id="0x1_guid_create_id"></a>

## Function `create_id`

Create a non-privileged id from <code>addr</code> and <code>creation_num</code>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_create_id">create_id</a>(addr: <b>address</b>, creation_num: u64): <a href="guid.md#0x1_guid_ID">guid::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_create_id">create_id</a>(addr: <b>address</b>, creation_num: u64): <a href="guid.md#0x1_guid_ID">ID</a> {
    <a href="guid.md#0x1_guid_ID">ID</a> { creation_num, addr }
}
</code></pre>



</details>

<a id="0x1_guid_id"></a>

## Function `id`

Get the non-privileged ID associated with a GUID


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id">id</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="guid.md#0x1_guid_ID">guid::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id">id</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">GUID</a>): <a href="guid.md#0x1_guid_ID">ID</a> {
    <a href="guid.md#0x1_guid">guid</a>.id
}
</code></pre>



</details>

<a id="0x1_guid_creator_address"></a>

## Function `creator_address`

Return the account address that created the GUID


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_creator_address">creator_address</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_creator_address">creator_address</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">GUID</a>): <b>address</b> {
    <a href="guid.md#0x1_guid">guid</a>.id.addr
}
</code></pre>



</details>

<a id="0x1_guid_id_creator_address"></a>

## Function `id_creator_address`

Return the account address that created the guid::ID


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id_creator_address">id_creator_address</a>(id: &<a href="guid.md#0x1_guid_ID">guid::ID</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id_creator_address">id_creator_address</a>(id: &<a href="guid.md#0x1_guid_ID">ID</a>): <b>address</b> {
    id.addr
}
</code></pre>



</details>

<a id="0x1_guid_creation_num"></a>

## Function `creation_num`

Return the creation number associated with the GUID


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_creation_num">creation_num</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_creation_num">creation_num</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">GUID</a>): u64 {
    <a href="guid.md#0x1_guid">guid</a>.id.creation_num
}
</code></pre>



</details>

<a id="0x1_guid_id_creation_num"></a>

## Function `id_creation_num`

Return the creation number associated with the guid::ID


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id_creation_num">id_creation_num</a>(id: &<a href="guid.md#0x1_guid_ID">guid::ID</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id_creation_num">id_creation_num</a>(id: &<a href="guid.md#0x1_guid_ID">ID</a>): u64 {
    id.creation_num
}
</code></pre>



</details>

<a id="0x1_guid_eq_id"></a>

## Function `eq_id`

Return true if the GUID's ID is <code>id</code>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_eq_id">eq_id</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>, id: &<a href="guid.md#0x1_guid_ID">guid::ID</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_eq_id">eq_id</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">GUID</a>, id: &<a href="guid.md#0x1_guid_ID">ID</a>): bool {
    &<a href="guid.md#0x1_guid">guid</a>.id == id
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The creation of GUID constructs a unique GUID by combining an address with an incremented creation number.</td>
<td>Low</td>
<td>The create function generates a new GUID by combining an address with an incremented creation number, effectively creating a unique identifier.</td>
<td>Enforced via <a href="#high-level-req-1">create</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The operations on GUID and ID, such as construction, field access, and equality comparison, should not abort.</td>
<td>Low</td>
<td>The following functions will never abort: (1) create_id, (2) id, (3) creator_address, (4) id_creator_address, (5) creation_num, (6) id_creation_num, and (7) eq_id.</td>
<td>Verified via <a href="#high-level-req-2.1">create_id</a>, <a href="#high-level-req-2.2">id</a>, <a href="#high-level-req-2.3">creator_address</a>, <a href="#high-level-req-2.4">id_creator_address</a>, <a href="#high-level-req-2.5">creation_num</a>, <a href="#high-level-req-2.6">id_creation_num</a>, and <a href="#high-level-req-2.7">eq_id</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The creation number should increment by 1 with each new creation.</td>
<td>Low</td>
<td>An account can only own up to MAX_U64 resources. Not incrementing the guid_creation_num constantly could lead to shrinking the space for new resources.</td>
<td>Enforced via <a href="#high-level-req-3">create</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The creation number and address of an ID / GUID must be immutable.</td>
<td>Medium</td>
<td>The addr and creation_num values are meant to be constant and never updated as they are unique and used for identification.</td>
<td>Audited: This is enforced through missing functionality to update the creation_num or addr.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_create"></a>

### Function `create`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="guid.md#0x1_guid_create">create</a>(addr: <b>address</b>, creation_num_ref: &<b>mut</b> u64): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>




<pre><code><b>aborts_if</b> creation_num_ref + 1 &gt; MAX_U64;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> result.id.creation_num == <b>old</b>(creation_num_ref);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>ensures</b> creation_num_ref == <b>old</b>(creation_num_ref) + 1;
</code></pre>



<a id="@Specification_1_create_id"></a>

### Function `create_id`


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_create_id">create_id</a>(addr: <b>address</b>, creation_num: u64): <a href="guid.md#0x1_guid_ID">guid::ID</a>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.1" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_id"></a>

### Function `id`


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id">id</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <a href="guid.md#0x1_guid_ID">guid::ID</a>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_creator_address"></a>

### Function `creator_address`


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_creator_address">creator_address</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>): <b>address</b>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.3" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_id_creator_address"></a>

### Function `id_creator_address`


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id_creator_address">id_creator_address</a>(id: &<a href="guid.md#0x1_guid_ID">guid::ID</a>): <b>address</b>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.4" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_creation_num"></a>

### Function `creation_num`


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_creation_num">creation_num</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>): u64
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.5" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_id_creation_num"></a>

### Function `id_creation_num`


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_id_creation_num">id_creation_num</a>(id: &<a href="guid.md#0x1_guid_ID">guid::ID</a>): u64
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.6" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>



<a id="@Specification_1_eq_id"></a>

### Function `eq_id`


<pre><code><b>public</b> <b>fun</b> <a href="guid.md#0x1_guid_eq_id">eq_id</a>(<a href="guid.md#0x1_guid">guid</a>: &<a href="guid.md#0x1_guid_GUID">guid::GUID</a>, id: &<a href="guid.md#0x1_guid_ID">guid::ID</a>): bool
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2.7" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
