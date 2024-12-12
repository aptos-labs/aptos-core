
<a id="0x1_bcs"></a>

# Module `0x1::bcs`

Utility for converting a Move value to its binary representation in BCS (Binary Canonical
Serialization). BCS is the binary encoding for Move resources and other non-module values
published on-chain. See https://github.com/aptos-labs/bcs#binary-canonical-serialization-bcs for more
details on BCS.


-  [Function `to_bytes`](#0x1_bcs_to_bytes)
-  [Function `serialized_size`](#0x1_bcs_serialized_size)
-  [Function `constant_serialized_size`](#0x1_bcs_constant_serialized_size)
-  [Specification](#@Specification_0)
    -  [Function `serialized_size`](#@Specification_0_serialized_size)


<pre><code><b>use</b> <a href="option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="0x1_bcs_to_bytes"></a>

## Function `to_bytes`

Returns the binary representation of <code>v</code> in BCS (Binary Canonical Serialization) format.
Aborts with <code>0x1c5</code> error code if serialization fails.


<pre><code><b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_to_bytes">to_bytes</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_to_bytes">to_bytes</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_bcs_serialized_size"></a>

## Function `serialized_size`

Returns the size of the binary representation of <code>v</code> in BCS (Binary Canonical Serialization) format.
Aborts with <code>0x1c5</code> error code if there is a failure when calculating serialized size.


<pre><code><b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_serialized_size">serialized_size</a>&lt;MoveValue&gt;(v: &MoveValue): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_serialized_size">serialized_size</a>&lt;MoveValue&gt;(v: &MoveValue): u64;
</code></pre>



</details>

<a id="0x1_bcs_constant_serialized_size"></a>

## Function `constant_serialized_size`

If the type has known constant (always the same, independent of instance) serialized size
in BCS (Binary Canonical Serialization) format, returns it, otherwise returns None.
Aborts with <code>0x1c5</code> error code if there is a failure when calculating serialized size.

Note:
For some types it might not be known they have constant size, and function might return None.
For example, signer appears to have constant size, but it's size might change.
If this function returned Some() for some type before - it is guaranteed to continue returning Some().
On the other hand, if function has returned None for some type,
it might change in the future to return Some() instead, if size becomes "known".


<pre><code><b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_constant_serialized_size">constant_serialized_size</a>&lt;MoveValue&gt;(): <a href="option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_constant_serialized_size">constant_serialized_size</a>&lt;MoveValue&gt;(): Option&lt;u64&gt;;
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



Native function which is defined in the prover's prelude.


<a id="0x1_bcs_serialize"></a>


<pre><code><b>native</b> <b>fun</b> <a href="bcs.md#0x1_bcs_serialize">serialize</a>&lt;MoveValue&gt;(v: &MoveValue): <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a id="@Specification_0_serialized_size"></a>

### Function `serialized_size`


<pre><code><b>public</b> <b>fun</b> <a href="bcs.md#0x1_bcs_serialized_size">serialized_size</a>&lt;MoveValue&gt;(v: &MoveValue): u64
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> result == len(<a href="bcs.md#0x1_bcs_serialize">serialize</a>(v));
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
