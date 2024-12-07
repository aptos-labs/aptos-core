
<a id="0x1_verified_external_value"></a>

# Module `0x1::verified_external_value`

Module for ability to take move values, and store them externally
(and save on the cost difference between onchain and offchain state),
with ability to safely retrieve them later: to safely transorm them back
to move value, by providing the externally stored bytes.

We do so by keeping the hash of the value in the onchain state, used to verify validity
of external bytes, guaranteeing validity of deserialized value.


-  [Struct `ExternalValue`](#0x1_verified_external_value_ExternalValue)
-  [Struct `Dummy`](#0x1_verified_external_value_Dummy)
-  [Struct `ExternalValuesSet`](#0x1_verified_external_value_ExternalValuesSet)
-  [Struct `MovedToExternalStorage`](#0x1_verified_external_value_MovedToExternalStorage)
-  [Constants](#@Constants_0)
-  [Function `move_to_external_storage`](#0x1_verified_external_value_move_to_external_storage)
-  [Function `get_hash`](#0x1_verified_external_value_get_hash)
-  [Function `into_value`](#0x1_verified_external_value_into_value)
-  [Function `get_value_copy`](#0x1_verified_external_value_get_value_copy)
-  [Function `new_set`](#0x1_verified_external_value_new_set)
-  [Function `contains`](#0x1_verified_external_value_contains)
-  [Function `add`](#0x1_verified_external_value_add)
-  [Function `remove`](#0x1_verified_external_value_remove)
-  [Function `get_copy`](#0x1_verified_external_value_get_copy)
-  [Function `bytes_to_hash`](#0x1_verified_external_value_bytes_to_hash)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="util.md#0x1_util">0x1::util</a>;
</code></pre>



<a id="0x1_verified_external_value_ExternalValue"></a>

## Struct `ExternalValue`

Externally storing any move value, while keeping the hash inside ExternalValue.

Currently we require value to have drop - as value dissapears from chain


<pre><code><b>struct</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T: drop, store&gt; <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_verified_external_value_Dummy"></a>

## Struct `Dummy`



<pre><code><b>struct</b> <a href="verified_external_value.md#0x1_verified_external_value_Dummy">Dummy</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_verified_external_value_ExternalValuesSet"></a>

## Struct `ExternalValuesSet`

Set of <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a></code>s, where only their hashes are stored onchain.
Set is without duplicates - i.e. same value cannot be stored twice.


<pre><code><b>struct</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a>&lt;T: drop, store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hashes: <a href="../../aptos-stdlib/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;u256, <a href="verified_external_value.md#0x1_verified_external_value_Dummy">verified_external_value::Dummy</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_verified_external_value_MovedToExternalStorage"></a>

## Struct `MovedToExternalStorage`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="verified_external_value.md#0x1_verified_external_value_MovedToExternalStorage">MovedToExternalStorage</a>&lt;T: drop, store&gt; <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256</code>
</dt>
<dd>

</dd>
<dt>
<code>bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_verified_external_value_EHASH_DOESNT_EXIST"></a>



<pre><code><b>const</b> <a href="verified_external_value.md#0x1_verified_external_value_EHASH_DOESNT_EXIST">EHASH_DOESNT_EXIST</a>: u64 = 1;
</code></pre>



<a id="0x1_verified_external_value_EHASH_DOESNT_MATCH"></a>



<pre><code><b>const</b> <a href="verified_external_value.md#0x1_verified_external_value_EHASH_DOESNT_MATCH">EHASH_DOESNT_MATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_verified_external_value_move_to_external_storage"></a>

## Function `move_to_external_storage`

Takes a value, emits it as an event, and creates ExternalValue representing it
(which stores it's hash inside)


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_move_to_external_storage">move_to_external_storage</a>&lt;T: drop, store&gt;(value: T): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">verified_external_value::ExternalValue</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_move_to_external_storage">move_to_external_storage</a>&lt;T: drop + store&gt;(value: T): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T&gt; {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&value);
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = <a href="verified_external_value.md#0x1_verified_external_value_bytes_to_hash">bytes_to_hash</a>(bytes);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="verified_external_value.md#0x1_verified_external_value_MovedToExternalStorage">MovedToExternalStorage</a>&lt;T&gt; {
        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
        bytes,
    });
    <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
    }
}
</code></pre>



</details>

<a id="0x1_verified_external_value_get_hash"></a>

## Function `get_hash`

Retrieves the hash of the value ExternalValue represents.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_get_hash">get_hash</a>&lt;T: drop, store&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">verified_external_value::ExternalValue</a>&lt;T&gt;): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_get_hash">get_hash</a>&lt;T: drop + store&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T&gt;): u256 {
    self.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
}
</code></pre>



</details>

<a id="0x1_verified_external_value_into_value"></a>

## Function `into_value`

Converts <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a></code> into it's original move representation, by providing it's bytes.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_into_value">into_value</a>&lt;T: drop, store&gt;(self: <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">verified_external_value::ExternalValue</a>&lt;T&gt;, external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_into_value">into_value</a>&lt;T: drop + store&gt;(self: <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T&gt;, external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T {
    <b>let</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a> { <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> } = self;
    <b>let</b> data_hash = <a href="verified_external_value.md#0x1_verified_external_value_bytes_to_hash">bytes_to_hash</a>(external_bytes);
    <b>assert</b>!(data_hash == <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="verified_external_value.md#0x1_verified_external_value_EHASH_DOESNT_MATCH">EHASH_DOESNT_MATCH</a>));

    // maybe emit consumed <a href="event.md#0x1_event">event</a>, so indexer can remove storing it?

    <a href="util.md#0x1_util_from_bytes">util::from_bytes</a>&lt;T&gt;(external_bytes)
}
</code></pre>



</details>

<a id="0x1_verified_external_value_get_value_copy"></a>

## Function `get_value_copy`

For a type that has <code><b>copy</b></code>, return original move representation of <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a></code>, without consuming it.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_get_value_copy">get_value_copy</a>&lt;T: <b>copy</b>, drop, store&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">verified_external_value::ExternalValue</a>&lt;T&gt;, external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_get_value_copy">get_value_copy</a>&lt;T: drop + store + <b>copy</b>&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T&gt;, external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): T {
    <b>let</b> data_hash = <a href="verified_external_value.md#0x1_verified_external_value_bytes_to_hash">bytes_to_hash</a>(external_bytes);
    <b>assert</b>!(data_hash == self.<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="verified_external_value.md#0x1_verified_external_value_EHASH_DOESNT_MATCH">EHASH_DOESNT_MATCH</a>));
    <a href="util.md#0x1_util_from_bytes">util::from_bytes</a>&lt;T&gt;(external_bytes)
}
</code></pre>



</details>

<a id="0x1_verified_external_value_new_set"></a>

## Function `new_set`

Creates new <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a></code>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_new_set">new_set</a>&lt;T: drop, store&gt;(): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">verified_external_value::ExternalValuesSet</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_new_set">new_set</a>&lt;T: drop + store&gt;(): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a>&lt;T&gt; {
    <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a> { hashes: <a href="../../aptos-stdlib/doc/big_ordered_map.md#0x1_big_ordered_map_new">big_ordered_map::new</a>() }
}
</code></pre>



</details>

<a id="0x1_verified_external_value_contains"></a>

## Function `contains`

Checks whether <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a></code> contains <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a></code> with corresponding hash.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_contains">contains</a>&lt;T: drop, store&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">verified_external_value::ExternalValuesSet</a>&lt;T&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_contains">contains</a>&lt;T: drop + store&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a>&lt;T&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256): bool {
    self.hashes.<a href="verified_external_value.md#0x1_verified_external_value_contains">contains</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>)
}
</code></pre>



</details>

<a id="0x1_verified_external_value_add"></a>

## Function `add`

Adds <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a></code> to <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a></code>
Abort if <code>value</code> already exists.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_add">add</a>&lt;T: drop, store&gt;(self: &<b>mut</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">verified_external_value::ExternalValuesSet</a>&lt;T&gt;, value: <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">verified_external_value::ExternalValue</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_add">add</a>&lt;T: drop + store&gt;(self: &<b>mut</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a>&lt;T&gt;, value: <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T&gt;) {
    <b>let</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a> { <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> } = value;
    self.hashes.<a href="verified_external_value.md#0x1_verified_external_value_add">add</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, <a href="verified_external_value.md#0x1_verified_external_value_Dummy">Dummy</a> {});
}
</code></pre>



</details>

<a id="0x1_verified_external_value_remove"></a>

## Function `remove`

Removes and returns <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a></code> with given <code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a></code> from <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a></code>.
Aborts if there is no entry for <code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_remove">remove</a>&lt;T: drop, store&gt;(self: &<b>mut</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">verified_external_value::ExternalValuesSet</a>&lt;T&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">verified_external_value::ExternalValue</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_remove">remove</a>&lt;T: drop + store&gt;(self: &<b>mut</b> <a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a>&lt;T&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T&gt; {
    self.hashes.<a href="verified_external_value.md#0x1_verified_external_value_remove">remove</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>);
    <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a> { <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> }
}
</code></pre>



</details>

<a id="0x1_verified_external_value_get_copy"></a>

## Function `get_copy`

For a type that has <code><b>copy</b></code>, returns <code><a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a></code> with the given hash.
Aborts if there is no entry for <code><a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_get_copy">get_copy</a>&lt;T: <b>copy</b>, drop, store&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">verified_external_value::ExternalValuesSet</a>&lt;T&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">verified_external_value::ExternalValue</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_get_copy">get_copy</a>&lt;T: drop + store + <b>copy</b>&gt;(self: &<a href="verified_external_value.md#0x1_verified_external_value_ExternalValuesSet">ExternalValuesSet</a>&lt;T&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>: u256): <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a>&lt;T&gt; {
    <b>assert</b>!(self.hashes.<a href="verified_external_value.md#0x1_verified_external_value_contains">contains</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="verified_external_value.md#0x1_verified_external_value_EHASH_DOESNT_EXIST">EHASH_DOESNT_EXIST</a>));
    <a href="verified_external_value.md#0x1_verified_external_value_ExternalValue">ExternalValue</a> { <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> }
}
</code></pre>



</details>

<a id="0x1_verified_external_value_bytes_to_hash"></a>

## Function `bytes_to_hash`

Computes a hash of a given bytes.
Value is first serialized using <code><a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a></code>, and then the hash is computed using this function.


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_bytes_to_hash">bytes_to_hash</a>(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="verified_external_value.md#0x1_verified_external_value_bytes_to_hash">bytes_to_hash</a>(external_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u256 {
    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_u256">from_bcs::to_u256</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(external_bytes))
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
