
<a id="0x1_sigma_protocol_statement_builder"></a>

# Module `0x1::sigma_protocol_statement_builder`

A builder for <code>Statement&lt;P&gt;</code> that eliminates manual parallel-vector construction.

Instead of manually maintaining two parallel vectors (<code>points</code> and <code>compressed_points</code>) that must
stay in sync, callers add points via builder methods that handle both vectors internally.


<a id="@CRITICAL:_Builder_order_must_match_index_constants_0"></a>

### CRITICAL: Builder order must match index constants


Points must be added in exactly the order the index constants define:
- <code>IDX_H = 0</code> → first <code>add_point</code> call adds H
- <code>IDX_EK = 1</code> → second <code>add_point</code> call adds ek
- etc.

The <code>assert_*_statement_is_well_formed()</code> check catches size mismatches but NOT ordering mistakes.
The builder does NOT change the index layout.


    -  [CRITICAL: Builder order must match index constants](#@CRITICAL:_Builder_order_must_match_index_constants_0)
-  [Struct `StatementBuilder`](#0x1_sigma_protocol_statement_builder_StatementBuilder)
-  [Function `new_builder`](#0x1_sigma_protocol_statement_builder_new_builder)
-  [Function `add_point`](#0x1_sigma_protocol_statement_builder_add_point)
-  [Function `add_points`](#0x1_sigma_protocol_statement_builder_add_points)
-  [Function `add_points_cloned`](#0x1_sigma_protocol_statement_builder_add_points_cloned)
-  [Function `add_scalar`](#0x1_sigma_protocol_statement_builder_add_scalar)
-  [Function `build`](#0x1_sigma_protocol_statement_builder_build)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement">0x1::sigma_protocol_statement</a>;
</code></pre>



<a id="0x1_sigma_protocol_statement_builder_StatementBuilder"></a>

## Struct `StatementBuilder`



<pre><code><b>struct</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a>&lt;P&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>scalars: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_sigma_protocol_statement_builder_new_builder"></a>

## Function `new_builder`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_new_builder">new_builder</a>&lt;P&gt;(): <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">sigma_protocol_statement_builder::StatementBuilder</a>&lt;P&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_new_builder">new_builder</a>&lt;P&gt;(): <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a>&lt;P&gt; {
    <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a> {
        points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        compressed_points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        scalars: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    }
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_builder_add_point"></a>

## Function `add_point`

Add a compressed point; decompresses internally. Returns the index.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_point">add_point</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">sigma_protocol_statement_builder::StatementBuilder</a>&lt;P&gt;, p: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_point">add_point</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a>&lt;P&gt;, p: CompressedRistretto): u64 {
    <b>let</b> idx = self.points.length();
    self.points.push_back(p.point_decompress());
    self.compressed_points.push_back(p);
    idx
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_builder_add_points"></a>

## Function `add_points`

Add a vector of compressed points; decompresses all internally. Returns the starting index.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_points">add_points</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">sigma_protocol_statement_builder::StatementBuilder</a>&lt;P&gt;, v: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_points">add_points</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a>&lt;P&gt;, v: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;): u64 {
    <b>let</b> start = self.points.length();
    v.for_each_ref(|p| {
        <b>let</b> p_val = *p;
        self.points.push_back(p_val.point_decompress());
        self.compressed_points.push_back(p_val);
    });
    start
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_builder_add_points_cloned"></a>

## Function `add_points_cloned`

Like <code>add_points</code>, but also returns clones of the decompressed points.
Useful when the caller needs the decompressed points for other purposes (e.g., range proofs).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_points_cloned">add_points_cloned</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">sigma_protocol_statement_builder::StatementBuilder</a>&lt;P&gt;, v: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): (u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_points_cloned">add_points_cloned</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a>&lt;P&gt;, v: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;): (u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;) {
    <b>let</b> start = self.points.length();
    <b>let</b> cloned = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    v.for_each_ref(|p| {
        <b>let</b> p_val = *p;
        <b>let</b> decompressed = p_val.point_decompress();
        cloned.push_back(decompressed.point_clone());
        self.points.push_back(decompressed);
        self.compressed_points.push_back(p_val);
    });
    (start, cloned)
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_builder_add_scalar"></a>

## Function `add_scalar`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_scalar">add_scalar</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">sigma_protocol_statement_builder::StatementBuilder</a>&lt;P&gt;, s: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_add_scalar">add_scalar</a>&lt;P&gt;(self: &<b>mut</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a>&lt;P&gt;, s: Scalar): u64 {
    <b>let</b> idx = self.scalars.length();
    self.scalars.push_back(s);
    idx
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_statement_builder_build"></a>

## Function `build`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_build">build</a>&lt;P&gt;(self: <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">sigma_protocol_statement_builder::StatementBuilder</a>&lt;P&gt;): <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;P&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_build">build</a>&lt;P&gt;(self: <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a>&lt;P&gt;): Statement&lt;P&gt; {
    <b>let</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder_StatementBuilder">StatementBuilder</a> { points, compressed_points, scalars } = self;
    <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_new_statement">sigma_protocol_statement::new_statement</a>(points, compressed_points, scalars)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
