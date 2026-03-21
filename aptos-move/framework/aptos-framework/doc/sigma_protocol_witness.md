
<a id="0x1_sigma_protocol_witness"></a>

# Module `0x1::sigma_protocol_witness`



-  [Struct `Witness`](#0x1_sigma_protocol_witness_Witness)
-  [Function `new_secret_witness`](#0x1_sigma_protocol_witness_new_secret_witness)
-  [Function `length`](#0x1_sigma_protocol_witness_length)
-  [Function `get`](#0x1_sigma_protocol_witness_get)
-  [Function `get_scalars`](#0x1_sigma_protocol_witness_get_scalars)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
</code></pre>



<a id="0x1_sigma_protocol_witness_Witness"></a>

## Struct `Witness`

A *secret witness* consists of a vector $w$ of $k$ scalars


<pre><code><b>struct</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">Witness</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>w: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_sigma_protocol_witness_new_secret_witness"></a>

## Function `new_secret_witness`

Creates a new secret witness from a vector of scalars.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_new_secret_witness">new_secret_witness</a>(w: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_new_secret_witness">new_secret_witness</a>(w: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">Witness</a> { <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">Witness</a> { w } }
</code></pre>



</details>

<a id="0x1_sigma_protocol_witness_length"></a>

## Function `length`

Returns the length of the witness: i.e., the number of scalars in it.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_length">length</a>(self: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_length">length</a>(self: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">Witness</a>): u64 {
    self.w.<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_length">length</a>()
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_witness_get"></a>

## Function `get`

Returns the <code>i</code>th scalar in the witness.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_get">get</a>(self: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>, i: u64): &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_get">get</a>(self: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">Witness</a>, i: u64): &Scalar {
    &self.w[i]
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_witness_get_scalars"></a>

## Function `get_scalars`

Returns the underling vector of witness scalars.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_get_scalars">get_scalars</a>(self: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_get_scalars">get_scalars</a>(self: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">Witness</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt; {
    &self.w
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
