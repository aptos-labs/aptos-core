
<a name="0x1_GUID"></a>

# Module `0x1::GUID`

A module for generating globally unique identifiers


-  [Resource `Generator`](#0x1_GUID_Generator)
-  [Struct `GUID`](#0x1_GUID_GUID)
-  [Struct `ID`](#0x1_GUID_ID)
-  [Function `create_id`](#0x1_GUID_create_id)
-  [Function `create`](#0x1_GUID_create)
-  [Function `publish_generator`](#0x1_GUID_publish_generator)
-  [Function `id`](#0x1_GUID_id)
-  [Function `creator_address`](#0x1_GUID_creator_address)
-  [Function `id_creator_address`](#0x1_GUID_id_creator_address)
-  [Function `creation_num`](#0x1_GUID_creation_num)
-  [Function `id_creation_num`](#0x1_GUID_id_creation_num)
-  [Function `eq_id`](#0x1_GUID_eq_id)
-  [Function `get_next_creation_num`](#0x1_GUID_get_next_creation_num)


<pre><code><b>use</b> <a href="Signer.md#0x1_Signer">0x1::Signer</a>;
</code></pre>



<a name="0x1_GUID_Generator"></a>

## Resource `Generator`

A generator for new GUIDs.


<pre><code><b>struct</b> <a href="GUID.md#0x1_GUID_Generator">Generator</a> has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>counter: u64</code>
</dt>
<dd>
 A monotonically increasing counter
</dd>
</dl>


</details>

<a name="0x1_GUID_GUID"></a>

## Struct `GUID`

A globally unique identifier derived from the sender's address and a counter


<pre><code><b>struct</b> <a href="GUID.md#0x1_GUID">GUID</a> has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="GUID.md#0x1_GUID_ID">GUID::ID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_GUID_ID"></a>

## Struct `ID`

A non-privileged identifier that can be freely created by anyone. Useful for looking up GUID's.


<pre><code><b>struct</b> <a href="GUID.md#0x1_GUID_ID">ID</a> has <b>copy</b>, drop, store
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
<code>addr: address</code>
</dt>
<dd>
 Address that created the GUID
</dd>
</dl>


</details>

<a name="0x1_GUID_create_id"></a>

## Function `create_id`

Create a non-privileged id from <code>addr</code> and <code>creation_num</code>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_create_id">create_id</a>(addr: address, creation_num: u64): <a href="GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_create_id">create_id</a>(addr: address, creation_num: u64): <a href="GUID.md#0x1_GUID_ID">ID</a> {
    <a href="GUID.md#0x1_GUID_ID">ID</a> { creation_num, addr }
}
</code></pre>



</details>

<a name="0x1_GUID_create"></a>

## Function `create`

Create and return a new GUID. Creates a <code><a href="GUID.md#0x1_GUID_Generator">Generator</a></code> under <code>account</code>
if it does not already have one


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_create">create</a>(account: &signer): <a href="GUID.md#0x1_GUID_GUID">GUID::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_create">create</a>(account: &signer): <a href="GUID.md#0x1_GUID">GUID</a> <b>acquires</b> <a href="GUID.md#0x1_GUID_Generator">Generator</a> {
    <b>let</b> addr = <a href="Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="GUID.md#0x1_GUID_Generator">Generator</a>&gt;(addr)) {
        move_to(account, <a href="GUID.md#0x1_GUID_Generator">Generator</a> { counter: 0 })
    };

    <b>let</b> generator = borrow_global_mut&lt;<a href="GUID.md#0x1_GUID_Generator">Generator</a>&gt;(addr);
    <b>let</b> creation_num = generator.counter;
    generator.counter = creation_num + 1;
    <a href="GUID.md#0x1_GUID">GUID</a> { id: <a href="GUID.md#0x1_GUID_ID">ID</a> { creation_num, addr } }
}
</code></pre>



</details>

<a name="0x1_GUID_publish_generator"></a>

## Function `publish_generator`

Publish a Generator resource under <code>account</code>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_publish_generator">publish_generator</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_publish_generator">publish_generator</a>(account: &signer) {
    move_to(account, <a href="GUID.md#0x1_GUID_Generator">Generator</a> { counter: 0 })
}
</code></pre>



</details>

<a name="0x1_GUID_id"></a>

## Function `id`

Get the non-privileged ID associated with a GUID


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_id">id</a>(guid: &<a href="GUID.md#0x1_GUID_GUID">GUID::GUID</a>): <a href="GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_id">id</a>(guid: &<a href="GUID.md#0x1_GUID">GUID</a>): <a href="GUID.md#0x1_GUID_ID">ID</a> {
    *&guid.id
}
</code></pre>



</details>

<a name="0x1_GUID_creator_address"></a>

## Function `creator_address`

Return the account address that created the GUID


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_creator_address">creator_address</a>(guid: &<a href="GUID.md#0x1_GUID_GUID">GUID::GUID</a>): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_creator_address">creator_address</a>(guid: &<a href="GUID.md#0x1_GUID">GUID</a>): address {
    guid.id.addr
}
</code></pre>



</details>

<a name="0x1_GUID_id_creator_address"></a>

## Function `id_creator_address`

Return the account address that created the GUID::ID


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_id_creator_address">id_creator_address</a>(id: &<a href="GUID.md#0x1_GUID_ID">GUID::ID</a>): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_id_creator_address">id_creator_address</a>(id: &<a href="GUID.md#0x1_GUID_ID">ID</a>): address {
    id.addr
}
</code></pre>



</details>

<a name="0x1_GUID_creation_num"></a>

## Function `creation_num`

Return the creation number associated with the GUID


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_creation_num">creation_num</a>(guid: &<a href="GUID.md#0x1_GUID_GUID">GUID::GUID</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_creation_num">creation_num</a>(guid: &<a href="GUID.md#0x1_GUID">GUID</a>): u64 {
    guid.id.creation_num
}
</code></pre>



</details>

<a name="0x1_GUID_id_creation_num"></a>

## Function `id_creation_num`

Return the creation number associated with the GUID::ID


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_id_creation_num">id_creation_num</a>(id: &<a href="GUID.md#0x1_GUID_ID">GUID::ID</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_id_creation_num">id_creation_num</a>(id: &<a href="GUID.md#0x1_GUID_ID">ID</a>): u64 {
    id.creation_num
}
</code></pre>



</details>

<a name="0x1_GUID_eq_id"></a>

## Function `eq_id`

Return true if the GUID's ID is <code>id</code>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_eq_id">eq_id</a>(guid: &<a href="GUID.md#0x1_GUID_GUID">GUID::GUID</a>, id: &<a href="GUID.md#0x1_GUID_ID">GUID::ID</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_eq_id">eq_id</a>(guid: &<a href="GUID.md#0x1_GUID">GUID</a>, id: &<a href="GUID.md#0x1_GUID_ID">ID</a>): bool {
    &guid.id == id
}
</code></pre>



</details>

<a name="0x1_GUID_get_next_creation_num"></a>

## Function `get_next_creation_num`

Return the number of the next GUID to be created by <code>addr</code>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_get_next_creation_num">get_next_creation_num</a>(addr: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="GUID.md#0x1_GUID_get_next_creation_num">get_next_creation_num</a>(addr: address): u64 <b>acquires</b> <a href="GUID.md#0x1_GUID_Generator">Generator</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="GUID.md#0x1_GUID_Generator">Generator</a>&gt;(addr)) {
        0
    } <b>else</b> {
        borrow_global&lt;<a href="GUID.md#0x1_GUID_Generator">Generator</a>&gt;(addr).counter
    }
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")
