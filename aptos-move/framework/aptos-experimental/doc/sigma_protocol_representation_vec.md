
<a id="0x7_sigma_protocol_representation_vec"></a>

# Module `0x7::sigma_protocol_representation_vec`



-  [Struct `RepresentationVec`](#0x7_sigma_protocol_representation_vec_RepresentationVec)
-  [Function `new_representation_vec`](#0x7_sigma_protocol_representation_vec_new_representation_vec)
-  [Function `get_representations`](#0x7_sigma_protocol_representation_vec_get_representations)
-  [Function `length`](#0x7_sigma_protocol_representation_vec_length)
-  [Function `for_each_ref`](#0x7_sigma_protocol_representation_vec_for_each_ref)
-  [Function `scale_all`](#0x7_sigma_protocol_representation_vec_scale_all)
-  [Function `scale_each`](#0x7_sigma_protocol_representation_vec_scale_each)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation">0x7::sigma_protocol_representation</a>;
</code></pre>



<a id="0x7_sigma_protocol_representation_vec_RepresentationVec"></a>

## Struct `RepresentationVec`

A vector of <code>Representations</code>.
Used to represent the output of the transformation function $f$ and the homomorphism $\psi$
(i.e., a vector in $\mathbb{G}^m$).


<pre><code><b>struct</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>v: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_sigma_protocol_representation_vec_new_representation_vec"></a>

## Function `new_representation_vec`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_new_representation_vec">new_representation_vec</a>(v: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>&gt;): <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_new_representation_vec">new_representation_vec</a>(v: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Representation&gt;): <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a> {
    <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a> {
        v
    }
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_vec_get_representations"></a>

## Function `get_representations`

Returns all the underlying <code>Representation</code>'s stored in this vector
(Public due to forced inlining for functions that take lambda arguments.)


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_get_representations">get_representations</a>(self: &<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_get_representations">get_representations</a>(self: &<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Representation&gt; {
    &self.v
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_vec_length"></a>

## Function `length`

Returns the number of representations in the vector.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_length">length</a>(self: &<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_length">length</a>(self: &<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a>): u64 {
    self.v.<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_length">length</a>()
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_vec_for_each_ref"></a>

## Function `for_each_ref`

Iterates through every representation in the vector.
(Forced inlining for functions that take lambda arguments.)


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_for_each_ref">for_each_ref</a>(self: &<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>, lambda: |&<a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation_Representation">sigma_protocol_representation::Representation</a>|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_for_each_ref">for_each_ref</a>(self: &<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a>, lambda: |&Representation|) {
    self.<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_get_representations">get_representations</a>().<a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_for_each_ref">for_each_ref</a>(|repr| lambda(repr))
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_vec_scale_all"></a>

## Function `scale_all`

Multiply all representations by $e$ (i.e., multiply each <code>self.v[i].scalars</code> by $e$).


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_scale_all">scale_all</a>(self: &<b>mut</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>, e: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_scale_all">scale_all</a>(self: &<b>mut</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a>, e: &Scalar) {
    self.v.for_each_mut(|repr| {
        repr.scale(e)
    });
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_representation_vec_scale_each"></a>

## Function `scale_each`

For all $i$, multiply the $i$th representation by <code>b[i]</code> (i.e., multiply <code>self.v[i].scalars</code> by <code>b[i]</code>)


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_scale_each">scale_each</a>(self: &<b>mut</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_scale_each">scale_each</a>(self: &<b>mut</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">RepresentationVec</a>, b: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;) {
    self.v.enumerate_mut(|i, repr| {
        repr.scale(&b[i])
    });
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
