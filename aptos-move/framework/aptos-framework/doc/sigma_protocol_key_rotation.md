
<a id="0x1_sigma_protocol_key_rotation"></a>

# Module `0x1::sigma_protocol_key_rotation`


<a id="@The_key_rotation_NP_relation_($\mathcal{R}_\mathsf{keyrot}$)_0"></a>

## The key rotation NP relation ($\mathcal{R}_\mathsf{keyrot}$)


$\def\old#1{{\color{red}{\dot{#1}}}}\def\new#1{{\color{teal}{\widetilde{#1}}}}$

A ZKPoK of having rotated an encryption key to a new one and re-encrypted (part of) a Twisted ElGamal ciphertext.


<a id="@Notation_1"></a>

### Notation


- $\old{x}$ denotes a stale/old ciphertext component; $\new{x}$ denotes a fresh/new one.
- $\ell$: number of available balance chunks.


<a id="@The_relation_2"></a>

### The relation


$$
\mathcal{R}_\mathsf{keyrot}^\ell\left(\begin{array}{l}
H, \mathsf{ek}, \new{\mathsf{ek}},
\old{\mathbf{R}}, \new{\mathbf{R}}
\textbf{;}\\
\mathsf{dk}, \delta, \delta_\mathsf{inv}
\end{array}\right) = 1
\Leftrightarrow
\left\{\begin{array}{r@{\,\,}l@{\quad}l}
H &= \mathsf{dk} \cdot \mathsf{ek}\\
\new{\mathsf{ek}} &= \delta \cdot \mathsf{ek}\\
\mathsf{ek} &= \delta_\mathsf{inv} \cdot \new{\mathsf{ek}}\\
\new{R}_i &= \delta \cdot \old{R}_i, &\forall i \in [\ell]\\
\end{array}\right.
$$


<a id="@Homomorphism_3"></a>

### Homomorphism


This can be framed as a homomorphism check $\psi(\mathbf{w}) = f(\mathbf{X})$ where
$\mathbf{w} = (\mathsf{dk}, \delta, \delta_\mathsf{inv})$ is the witness
and $\mathbf{X} = (H, \mathsf{ek}, \new{\mathsf{ek}}, \old{\mathbf{R}}, \new{\mathbf{R}})$ is the statement.

1. The homomorphism $\psi$ is:

$$
\psi(\mathsf{dk}, \delta, \delta_\mathsf{inv}) = \begin{pmatrix}
\mathsf{dk} \cdot \mathsf{ek}\\
\delta \cdot \mathsf{ek}\\
\delta_\mathsf{inv} \cdot \new{\mathsf{ek}}\\
\delta \cdot \old{R}_i, &\forall i \in [\ell]\\
\end{pmatrix}
$$

2. The transformation function $f$ is:

$$
f(\mathbf{X}) = \begin{pmatrix}
H\\
\new{\mathsf{ek}}\\
\mathsf{ek}\\
\new{R}_i, &\forall i \in [\ell]\\
\end{pmatrix}
$$


-  [The key rotation NP relation ($\mathcal{R}_\mathsf{keyrot}$)](#@The_key_rotation_NP_relation_($\mathcal{R}_\mathsf{keyrot}$)_0)
    -  [Notation](#@Notation_1)
    -  [The relation](#@The_relation_2)
    -  [Homomorphism](#@Homomorphism_3)
-  [Struct `KeyRotation`](#0x1_sigma_protocol_key_rotation_KeyRotation)
-  [Struct `KeyRotationSession`](#0x1_sigma_protocol_key_rotation_KeyRotationSession)
-  [Constants](#@Constants_4)
-  [Function `get_start_idx_for_new_R`](#0x1_sigma_protocol_key_rotation_get_start_idx_for_new_R)
-  [Function `assert_key_rotation_statement_is_well_formed`](#0x1_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed)
-  [Function `new_session`](#0x1_sigma_protocol_key_rotation_new_session)
-  [Function `new_key_rotation_statement`](#0x1_sigma_protocol_key_rotation_new_key_rotation_statement)
-  [Function `psi`](#0x1_sigma_protocol_key_rotation_psi)
-  [Function `f`](#0x1_sigma_protocol_key_rotation_f)
-  [Function `assert_verifies`](#0x1_sigma_protocol_key_rotation_assert_verifies)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="sigma_protocol_fiat_shamir.md#0x1_sigma_protocol_fiat_shamir">0x1::sigma_protocol_fiat_shamir</a>;
<b>use</b> <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof">0x1::sigma_protocol_proof</a>;
<b>use</b> <a href="sigma_protocol_representation.md#0x1_sigma_protocol_representation">0x1::sigma_protocol_representation</a>;
<b>use</b> <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec">0x1::sigma_protocol_representation_vec</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement">0x1::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder">0x1::sigma_protocol_statement_builder</a>;
<b>use</b> <a href="sigma_protocol_utils.md#0x1_sigma_protocol_utils">0x1::sigma_protocol_utils</a>;
<b>use</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness">0x1::sigma_protocol_witness</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_KeyRotation"></a>

## Struct `KeyRotation`

Phantom marker type for key rotation statements.


<pre><code><b>struct</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">KeyRotation</a> <b>has</b> drop
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

<a id="0x1_sigma_protocol_key_rotation_KeyRotationSession"></a>

## Struct `KeyRotationSession`

Used for domain separation


<pre><code><b>struct</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sender: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>token_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_chunks: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_4"></a>

## Constants


<a id="0x1_sigma_protocol_key_rotation_E_STATEMENT_BUILDER_INCONSISTENCY"></a>

The homomorphism or transformation function implementation is not inserting points at the expected positions.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_E_STATEMENT_BUILDER_INCONSISTENCY">E_STATEMENT_BUILDER_INCONSISTENCY</a>: u64 = 6;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_IDX_DK"></a>

Index of $\mathsf{dk}$ (old decryption key) in the witness's scalars vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_DK">IDX_DK</a>: u64 = 0;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_IDX_EK"></a>

Index of $\mathsf{ek}$ (old encryption key) in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>: u64 = 1;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_IDX_H"></a>

Index of $H$ in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_H">IDX_H</a>: u64 = 0;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_PROTOCOL_ID"></a>

Protocol ID used for domain separation


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_PROTOCOL_ID">PROTOCOL_ID</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 75, 101, 121, 82, 111, 116, 97, 116, 105, 111, 110, 86, 49];
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_E_INVALID_KEY_ROTATION_PROOF"></a>

The key rotation proof was invalid


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_E_INVALID_KEY_ROTATION_PROOF">E_INVALID_KEY_ROTATION_PROOF</a>: u64 = 5;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_IDX_DELTA"></a>

Index of $\delta$ in the witness's scalars vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_DELTA">IDX_DELTA</a>: u64 = 1;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_IDX_DELTA_INV"></a>

Index of $\delta_\mathsf{inv}$ in the witness's scalars vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_DELTA_INV">IDX_DELTA_INV</a>: u64 = 2;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_IDX_EK_NEW"></a>

Index of $\widetilde{\mathsf{ek}}$ (new encryption key) in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK_NEW">IDX_EK_NEW</a>: u64 = 2;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_START_IDX_OLD_R"></a>

The old R values ($\dot{R}_i$ ) occupy indices 3 to 3 + (num_chunks - 1), inclusive.

Note: The new R values ($\widetilde{R}_i$) occupy indices 3 + num_chunks to 3 + (2*num_chunks - 1), inclusive.
A <code><a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_get_start_idx_for_new_R">get_start_idx_for_new_R</a>(num_chunks)</code> function can be used to fetch the 3 + num_chunks starting index.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_START_IDX_OLD_R">START_IDX_OLD_R</a>: u64 = 3;
</code></pre>



<a id="0x1_sigma_protocol_key_rotation_get_start_idx_for_new_R"></a>

## Function `get_start_idx_for_new_R`

Returns the starting index of new_R values.


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_get_start_idx_for_new_R">get_start_idx_for_new_R</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_get_start_idx_for_new_R">get_start_idx_for_new_R</a>(): u64 {
    <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_START_IDX_OLD_R">START_IDX_OLD_R</a> + get_num_available_chunks()
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed"></a>

## Function `assert_key_rotation_statement_is_well_formed`

Ensures the statement is of the form:
$\left(
H, \mathsf{ek}, \widetilde{\mathsf{ek}},
(\dot{R}_i)_{i \in [\ell]}),
(\widetilde{R}_i)_{i \in [\ell]}
\right)$


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">sigma_protocol_key_rotation::KeyRotation</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(
    stmt: &Statement&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">KeyRotation</a>&gt;,
) {
    <b>assert</b>!(stmt.get_points().length() == 3 + 2 * get_num_available_chunks(), e_wrong_num_points());
    <b>assert</b>!(stmt.get_scalars().length() == 0, e_wrong_num_scalars());
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_key_rotation_new_session"></a>

## Function `new_session`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_new_session">new_session</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotationSession">sigma_protocol_key_rotation::KeyRotationSession</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_new_session">new_session</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_type: Object&lt;Metadata&gt;): <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a> {
    <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a> {
        sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        token_type,
        num_chunks: get_num_available_chunks(),
    }
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_key_rotation_new_key_rotation_statement"></a>

## Function `new_key_rotation_statement`

Creates a new key rotation statement.
The order matches the NP relation: $(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{R}}, \widetilde{\mathbf{R}})$.
Note that the # of chunks is inferred from the sizes of the old and new balance ciphertexts.

All points are decompressed internally from their compressed forms by the <code>StatementBuilder</code>.

@param compressed_ek: Compressed form of the old encryption key
@param compressed_new_ek: Compressed form of the new encryption key
@param compressed_old_R: Compressed forms of old_R (by reference; num_chunks elements)
@param compressed_new_R: Compressed forms of new_R (by reference; num_chunks elements)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_new_key_rotation_statement">new_key_rotation_statement</a>(compressed_ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, compressed_new_ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, compressed_old_R: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, compressed_new_R: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">sigma_protocol_key_rotation::KeyRotation</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_new_key_rotation_statement">new_key_rotation_statement</a>(
    compressed_ek: CompressedRistretto,
    compressed_new_ek: CompressedRistretto,
    compressed_old_R: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    compressed_new_R: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
): Statement&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">KeyRotation</a>&gt; {
    <b>let</b> err = <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_E_STATEMENT_BUILDER_INCONSISTENCY">E_STATEMENT_BUILDER_INCONSISTENCY</a>);
    <b>let</b> b = new_builder();
    <b>assert</b>!(b.add_point(get_encryption_key_basepoint_compressed()) == <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_H">IDX_H</a>, err);                  // H
    <b>assert</b>!(b.add_point(compressed_ek) == <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>, err);                                                // ek
    <b>assert</b>!(b.add_point(compressed_new_ek) == <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK_NEW">IDX_EK_NEW</a>, err);                                        // new_ek
    <b>assert</b>!(b.add_points(compressed_old_R) == <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_START_IDX_OLD_R">START_IDX_OLD_R</a>, err);                                   // old_R
    <b>assert</b>!(b.add_points(compressed_new_R) == <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_START_IDX_OLD_R">START_IDX_OLD_R</a> + get_num_available_chunks(), err);      // new_R
    <b>let</b> stmt = b.build();
    <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(&stmt);
    stmt
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_key_rotation_psi"></a>

## Function `psi`

The homomorphism $\psi$ for the key rotation relation.

Given witness $(dk, \delta, \delta_{inv})$, outputs:
```
[
dk * ek,           // should equal H
delta * ek,        // should equal new_ek
delta_inv * new_ek, // should equal ek
delta * old_R_i,   // should equal new_R_i, for i in [1..num_chunks]
]
```


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_psi">psi</a>(_stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">sigma_protocol_key_rotation::KeyRotation</a>&gt;, w: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>): <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_psi">psi</a>(_stmt: &Statement&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">KeyRotation</a>&gt;, w: &Witness): RepresentationVec {
    // WARNING: Crucial for security
    <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(_stmt);
    // WARNING: Crucial for security
    <b>assert</b>!(w.length() == 3, e_wrong_witness_len());

    <b>let</b> dk = *w.get(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_DK">IDX_DK</a>);
    <b>let</b> delta = *w.get(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_DELTA">IDX_DELTA</a>);
    <b>let</b> delta_inv = *w.get(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_DELTA_INV">IDX_DELTA_INV</a>);

    // Build the representation <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>
    <b>let</b> reprs = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        // dk * ek
        repr_scaled(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>, dk),
        // delta * ek
        repr_scaled(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>, delta),
        // delta_inv * new_ek
        repr_scaled(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK_NEW">IDX_EK_NEW</a>, delta_inv),
    ];

    // delta * old_R_i for each chunk
    <b>let</b> ell = get_num_available_chunks();
    reprs.append(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).map(|i|
        repr_scaled(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_START_IDX_OLD_R">START_IDX_OLD_R</a> + i, delta)
    ));

    // WARNING: Crucial for security
    <b>assert</b>!(reprs.length() == 3 + ell, e_wrong_output_len());
    new_representation_vec(reprs)
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_key_rotation_f"></a>

## Function `f`

The transformation function $f$ for the key rotation relation.

Given the statement, outputs:
```
[
H,
new_ek,
ek,
new_R_i for i in [1..num_chunks]
]
```


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_f">f</a>(_stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">sigma_protocol_key_rotation::KeyRotation</a>&gt;): <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_f">f</a>(_stmt: &Statement&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">KeyRotation</a>&gt;): RepresentationVec {
    // WARNING: We do not re-<b>assert</b> the stmt is well-formed anymore here, since wherever the transformation function
    // is called, so is the homomorphism, so the check will be done.

    <b>let</b> ell = get_num_available_chunks();
    <b>let</b> idx_r_new_start = <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_get_start_idx_for_new_R">get_start_idx_for_new_R</a>();

    <b>let</b> reprs = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        // H
        repr_point(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_H">IDX_H</a>),
        // new_ek
        repr_point(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK_NEW">IDX_EK_NEW</a>),
        // ek
        repr_point(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>),
    ];

    // new_R_i for each chunk
    reprs.append(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).map(|i|
        repr_point(idx_r_new_start + i)
    ));

    // Note: Not needed for security, since a mismatched <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_f">f</a>(X) length will be caught in the verifier. But good practice
    // for catching mistakes *early* when implementing your <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_f">f</a>(X).
    <b>assert</b>!(reprs.length() == 3 + ell, e_wrong_output_len());
    new_representation_vec(reprs)
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_key_rotation_assert_verifies"></a>

## Function `assert_verifies`

Asserts that a key rotation proof verifies


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_assert_verifies">assert_verifies</a>(self: &<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotationSession">sigma_protocol_key_rotation::KeyRotationSession</a>, stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">sigma_protocol_key_rotation::KeyRotation</a>&gt;, proof: &<a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_assert_verifies">assert_verifies</a>(self: &<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a>, stmt: &Statement&lt;<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_KeyRotation">KeyRotation</a>&gt;, proof: &Proof) {
    <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(stmt);

    <b>let</b> success = <a href="sigma_protocol.md#0x1_sigma_protocol_verify">sigma_protocol::verify</a>(
        new_domain_separator(@aptos_framework, <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>(), <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_PROTOCOL_ID">PROTOCOL_ID</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(self)),
        |_X, w| <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_psi">psi</a>(_X, w),
        |_X| <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_f">f</a>(_X),
        stmt,
        proof
    );

    <b>assert</b>!(success, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_E_INVALID_KEY_ROTATION_PROOF">E_INVALID_KEY_ROTATION_PROOF</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
