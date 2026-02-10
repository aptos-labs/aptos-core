
<a id="0x7_sigma_protocol_key_rotation"></a>

# Module `0x7::sigma_protocol_key_rotation`


<a id="@The_key_rotation_NP_relation_0"></a>

## The key rotation NP relation


A ZKPoK of having rotated an encryption key to a new one and re-encrypted (part of) a Twisted ElGamal ciphertext.

The NP relation, parameterized by the # of available balance chunks $m$, is:

\begin{align}
\mathcal{R}_\mathsf{keyrot}^\ell(
H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}};
\mathsf{dk}, \delta, \delta_\mathsf{inv}
) = 1 \Leftrightarrow
\begin{cases}
H &= \mathsf{dk} \cdot \mathsf{ek}\\
\widetilde{\mathsf{ek}} &= \delta \cdot \mathsf{ek}\\
\mathsf{ek} &= \delta_\mathsf{inv} \cdot \widetilde{\mathsf{ek}}\\
\widetilde{D}_i &= \delta \cdot \dot{D}_i, \forall i \in [\ell];
\end{cases}
\end{align}

This can be framed as a homomorphism check:

\begin{align}
\psi(\mathsf{dk}, \delta, \delta_\mathsf{inv})  =  f(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}})
\end{align}

where:

1. The homomorphism $\psi$ is:

\begin{align}
\psi(\mathsf{dk}, \delta, \delta_\mathsf{inv}) = \begin{pmatrix}
\mathsf{dk} \cdot \mathsf{ek}\\
\delta \cdot \mathsf{ek}\\
\delta_\mathsf{inv} \cdot \widetilde{\mathsf{ek}}\\
\delta \cdot \dot{D}_i, \forall i \in [\ell]\\
\end{pmatrix}
\end{align}

2. The transformation function $f$ is:

\begin{align}
f(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}}) = \begin{pmatrix}
H, \widetilde{\mathsf{ek}}, \mathsf{ek}, \widetilde{\mathbf{D}}
\end{pmatrix}
\end{align}


-  [The key rotation NP relation](#@The_key_rotation_NP_relation_0)
-  [Struct `KeyRotationSession`](#0x7_sigma_protocol_key_rotation_KeyRotationSession)
-  [Constants](#@Constants_1)
-  [Function `get_num_chunks`](#0x7_sigma_protocol_key_rotation_get_num_chunks)
-  [Function `get_start_idx_for_new_D`](#0x7_sigma_protocol_key_rotation_get_start_idx_for_new_D)
-  [Function `assert_key_rotation_statement_is_well_formed`](#0x7_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed)
-  [Function `new_session`](#0x7_sigma_protocol_key_rotation_new_session)
-  [Function `new_key_rotation_statement`](#0x7_sigma_protocol_key_rotation_new_key_rotation_statement)
-  [Function `new_key_rotation_witness`](#0x7_sigma_protocol_key_rotation_new_key_rotation_witness)
-  [Function `psi`](#0x7_sigma_protocol_key_rotation_psi)
-  [Function `f`](#0x7_sigma_protocol_key_rotation_f)
-  [Function `assert_verifies`](#0x7_sigma_protocol_key_rotation_assert_verifies)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir">0x7::sigma_protocol_fiat_shamir</a>;
<b>use</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof">0x7::sigma_protocol_proof</a>;
<b>use</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation">0x7::sigma_protocol_representation</a>;
<b>use</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec">0x7::sigma_protocol_representation_vec</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement">0x7::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness">0x7::sigma_protocol_witness</a>;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_KeyRotationSession"></a>

## Struct `KeyRotationSession`

Used for domain separation
TODO(Security): It'd be nice to add more here (like some sort of account TXN counter). I suspect that the
ciphertext randomness in the public statement would act as enough of a "session ID", but I would prefer
to avoid reasoning about that.


<pre><code><b>struct</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a> <b>has</b> drop
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
<code>token_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
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

<a id="@Constants_1"></a>

## Constants


<a id="0x7_sigma_protocol_key_rotation_E_INVALID_KEY_ROTATION_PROOF"></a>

The key rotation proof was invalid


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_INVALID_KEY_ROTATION_PROOF">E_INVALID_KEY_ROTATION_PROOF</a>: u64 = 5;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS"></a>

The expected number of points in a key rotation statement is 3 + 2 * num_chunks, with num_chunks > 0.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_E_WRONG_NUM_SCALARS"></a>

The expected number of scalars in a key rotation statement is 0.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_SCALARS">E_WRONG_NUM_SCALARS</a>: u64 = 2;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_E_WRONG_OUTPUT_LEN"></a>

The expected number of points in the homomorphism & transformation function output is 3 + num_chunks, with num_chunks > 0.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_OUTPUT_LEN">E_WRONG_OUTPUT_LEN</a>: u64 = 4;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_E_WRONG_WITNESS_LEN"></a>

The expected number of scalars in a key rotation witness is 3.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_WITNESS_LEN">E_WRONG_WITNESS_LEN</a>: u64 = 3;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_IDX_DELTA"></a>

Index of $\delta$ in the witness's scalars vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_DELTA">IDX_DELTA</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_IDX_DELTA_INV"></a>

Index of $\delta_\mathsf{inv}$ in the witness's scalars vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_DELTA_INV">IDX_DELTA_INV</a>: u64 = 2;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_IDX_DK"></a>

Index of $\mathsf{dk}$ (old decryption key) in the witness's scalars vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_DK">IDX_DK</a>: u64 = 0;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_IDX_EK"></a>

Index of $\mathsf{ek}$ (old encryption key) in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_IDX_EK_NEW"></a>

Index of $\widetilde{\mathsf{ek}}$ (new encryption key) in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_EK_NEW">IDX_EK_NEW</a>: u64 = 2;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_IDX_H"></a>

Index of $H$ in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_H">IDX_H</a>: u64 = 0;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_PROTOCOL_ID"></a>

Protocol ID used for domain separation


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_PROTOCOL_ID">PROTOCOL_ID</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 75, 101, 121, 82, 111, 116, 97, 116, 105, 111, 110, 86, 49];
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_START_IDX_OLD_D"></a>

The old D values ($\dot{D}_i$ ) occupy indices 3 to 3 + (num_chunks - 1), inclusive.

Note: The new D values ($\widetilde{D}_i$) occupy indices 3 + num_chunks to 3 + (2*num_chunks - 1), inclusive.
A <code><a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_start_idx_for_new_D">get_start_idx_for_new_D</a>(num_chunks)</code> function can be used to fetch the 3 + num_chunks starting index.


<pre><code><b>const</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_START_IDX_OLD_D">START_IDX_OLD_D</a>: u64 = 3;
</code></pre>



<a id="0x7_sigma_protocol_key_rotation_get_num_chunks"></a>

## Function `get_num_chunks`

Returns the number of chunks encoded in the statement.
The statement has 3 + 2*num_chunks points, so num_chunks = (num_points - 3) / 2.


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_num_chunks">get_num_chunks</a>(stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_num_chunks">get_num_chunks</a>(stmt: &Statement): u64 {
    <b>let</b> num_points = stmt.get_points().length();

    (num_points - 3) / 2
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_get_start_idx_for_new_D"></a>

## Function `get_start_idx_for_new_D`

Returns the starting index of new_D values given the number of chunks.


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_start_idx_for_new_D">get_start_idx_for_new_D</a>(num_chunks: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_start_idx_for_new_D">get_start_idx_for_new_D</a>(num_chunks: u64): u64 {
    <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_START_IDX_OLD_D">START_IDX_OLD_D</a> + num_chunks
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed"></a>

## Function `assert_key_rotation_statement_is_well_formed`

Ensures the statement is of the form:
$\left(
H, \mathsf{ek}, \widetilde{\mathsf{ek}},
(\dot{D}_i)_{i \in [\ell]}),
(\widetilde{D}_i)_{i \in [\ell]}
\right)$


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(
    stmt: &Statement,
) {
    <b>let</b> num_points = stmt.get_points().length();

    // The number of points must be odd and greater than 3
    <b>assert</b>!(num_points &gt; 3, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>));
    <b>assert</b>!((num_points - 3) % 2 == 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>));

    // There should be no scalars
    <b>assert</b>!(stmt.get_scalars().length() == 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_SCALARS">E_WRONG_NUM_SCALARS</a>));
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_new_session"></a>

## Function `new_session`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_session">new_session</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, num_chunks: u64): <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_KeyRotationSession">sigma_protocol_key_rotation::KeyRotationSession</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_session">new_session</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_type: Object&lt;Metadata&gt;, num_chunks: u64): <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a> {
    <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a> {
        sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        token_type,
        num_chunks,
    }
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_new_key_rotation_statement"></a>

## Function `new_key_rotation_statement`

Creates a new key rotation statement.
The order matches the NP relation: $(H, \mathsf{ek}, \widetilde{\mathsf{ek}}, \dot{\mathbf{D}}, \widetilde{\mathbf{D}})$.
Note that the # of chunks is inferred from the sizes of the old and new balance ciphertexts.

@param compressed_H: Compressed form of h
@param _H: The hash-to-point base (= dk * ek)

@param compressed_ek: Compressed form of ek
@param ek: The old encryption key

@param compressed_new_ek: Compressed form of new_ek
@param new_ek: The new encryption key

@param compressed_old_D: Compressed forms of old_D
@param old_D: The old D values from the ciphertext (num_chunks elements)

@param compressed_new_D: Compressed forms of d_new
@param new_D: The new D values after re-encryption (num_chunks elements, must match old_D length)


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_key_rotation_statement">new_key_rotation_statement</a>(compressed_H: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, _H: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, compressed_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, compressed_new_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, new_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, compressed_old_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, old_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, compressed_new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, num_chunks: u64): <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_key_rotation_statement">new_key_rotation_statement</a>(
    compressed_H: CompressedRistretto, _H: RistrettoPoint,
    compressed_ek: CompressedRistretto, ek: RistrettoPoint,
    compressed_new_ek: CompressedRistretto, new_ek: RistrettoPoint,
    compressed_old_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, old_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    compressed_new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;, new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    num_chunks: u64,
): Statement {
    <b>assert</b>!(num_chunks &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>));
    // <b>assert</b> all the D-component vectors are of equal size, matching num_chunks &gt; 0
    <b>assert</b>!(compressed_old_D.length() == old_D.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>));
    <b>assert</b>!(compressed_new_D.length() == new_D.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>));
    <b>assert</b>!(old_D.length() == new_D.length(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>));
    <b>assert</b>!(old_D.length() == num_chunks, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_NUM_POINTS">E_WRONG_NUM_POINTS</a>));

    <b>let</b> points = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[_H, ek, new_ek];
    points.append(old_D);
    points.append(new_D);

    <b>let</b> compressed_points = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[compressed_H, compressed_ek, compressed_new_ek];
    compressed_points.append(compressed_old_D);
    compressed_points.append(compressed_new_D);

    <b>let</b> stmt = new_statement(points, compressed_points, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
    <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(&stmt);
    stmt
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_new_key_rotation_witness"></a>

## Function `new_key_rotation_witness`

Creates a new key rotation witness.

@param dk: The old decryption key
@param delta: The ratio new_dk / old_dk (i.e., new_dk * old_dk^{-1})
@param delta_inv: The inverse of delta


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_key_rotation_witness">new_key_rotation_witness</a>(dk: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, delta: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, delta_inv: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_key_rotation_witness">new_key_rotation_witness</a>(dk: Scalar, delta: Scalar, delta_inv: Scalar): Witness {
    new_secret_witness(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[dk, delta, delta_inv])
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_psi"></a>

## Function `psi`

The homomorphism $\psi$ for the key rotation relation.

Given witness $(dk, \delta, \delta_{inv})$, outputs:
```
[
dk * ek,           // should equal H
delta * ek,        // should equal new_ek
delta_inv * new_ek, // should equal ek
delta * old_D_i,   // should equal new_D_i, for i in [num_chunks]
]
```


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_psi">psi</a>(_stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>, w: &<a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>): <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_psi">psi</a>(_stmt: &Statement, w: &Witness): RepresentationVec {
    // WARNING: Crucial for security
    <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_key_rotation_statement_is_well_formed">assert_key_rotation_statement_is_well_formed</a>(_stmt);
    // WARNING: Crucial for security
    <b>assert</b>!(w.length() == 3, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_WITNESS_LEN">E_WRONG_WITNESS_LEN</a>));

    <b>let</b> dk = *w.get(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_DK">IDX_DK</a>);
    <b>let</b> delta = *w.get(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_DELTA">IDX_DELTA</a>);
    <b>let</b> delta_inv = *w.get(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_DELTA_INV">IDX_DELTA_INV</a>);

    // Build the representation <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>
    <b>let</b> reprs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        // dk * ek
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[dk]),
        // delta * ek
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[delta]),
        // delta_inv * new_ek
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_EK_NEW">IDX_EK_NEW</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[delta_inv]),
    ];

    // delta * old_D_i for each chunk
    <b>let</b> num_chunks = <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_num_chunks">get_num_chunks</a>(_stmt);
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).for_each(|i| {
        reprs.push_back(
            new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_START_IDX_OLD_D">START_IDX_OLD_D</a> + i], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[delta])
        );
    });

    <b>let</b> repr_vec = new_representation_vec(reprs);
    <b>let</b> expected_output_len = 3 + num_chunks;

    // WARNING: Crucial for security
    <b>assert</b>!(repr_vec.length() == expected_output_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_OUTPUT_LEN">E_WRONG_OUTPUT_LEN</a>));

    repr_vec
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_f"></a>

## Function `f`

The transformation function $f$ for the key rotation relation.

Given the statement, outputs:
```
[
H,
new_ek,
ek,
new_D_i for i in [num_chunks]
]
```


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_f">f</a>(_stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>): <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_f">f</a>(_stmt: &Statement): RepresentationVec {
    // WARNING: We do not re-<b>assert</b> the stmt is well-formed anymore here, since wherever the transformation function
    // is called, so is the homomorphism, so the check will be done.

    <b>let</b> num_chunks = <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_num_chunks">get_num_chunks</a>(_stmt);
    <b>let</b> idx_d_new_start = <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_start_idx_for_new_D">get_start_idx_for_new_D</a>(num_chunks);

    <b>let</b> reprs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        // H
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_H">IDX_H</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_one">ristretto255::scalar_one</a>()]),
        // new_ek
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_EK_NEW">IDX_EK_NEW</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_one">ristretto255::scalar_one</a>()]),
        // ek
        new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_IDX_EK">IDX_EK</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_one">ristretto255::scalar_one</a>()]),
    ];

    // new_D_i for each chunk
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_chunks).for_each(|i| {
        reprs.push_back(
            new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[idx_d_new_start + i], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_one">ristretto255::scalar_one</a>()])
        );
    });

    <b>let</b> repr_vec = new_representation_vec(reprs);
    <b>let</b> expected_output_len = 3 + num_chunks;

    // WARNING: Crucial for security
    <b>assert</b>!(repr_vec.length() == expected_output_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_WRONG_OUTPUT_LEN">E_WRONG_OUTPUT_LEN</a>));

    repr_vec
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_key_rotation_assert_verifies"></a>

## Function `assert_verifies`

Asserts that a key rotation proof verifies


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_verifies">assert_verifies</a>(session: &<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_KeyRotationSession">sigma_protocol_key_rotation::KeyRotationSession</a>, stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>, proof: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>, num_chunks: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_verifies">assert_verifies</a>(session: &<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_KeyRotationSession">KeyRotationSession</a>, stmt: &Statement, proof: &Proof, num_chunks: u64) {
    // TODO: Ideally, we should do this via a type-safe KeyRotationStatement?
    <b>assert</b>!(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_get_num_chunks">get_num_chunks</a>(stmt) == num_chunks, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_INVALID_KEY_ROTATION_PROOF">E_INVALID_KEY_ROTATION_PROOF</a>));

    <b>let</b> success = <a href="sigma_protocol.md#0x7_sigma_protocol_verify">sigma_protocol::verify</a>(
        new_domain_separator(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_PROTOCOL_ID">PROTOCOL_ID</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(session)),
        // TODO(Ugly): Change `|_X, w| <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_psi">psi</a>(_X, w)` <b>to</b> `psi` and `|_X| <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_f">f</a>(_X)` <b>to</b> `f` when <b>public</b> structs ship and allow this.
        |_X, w| <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_psi">psi</a>(_X, w),
        |_X| <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_f">f</a>(_X),
        stmt,
        proof
    );

    <b>assert</b>!(success, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_E_INVALID_KEY_ROTATION_PROOF">E_INVALID_KEY_ROTATION_PROOF</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
