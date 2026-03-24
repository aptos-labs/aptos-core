
<a id="0x1_sigma_protocol_registration"></a>

# Module `0x1::sigma_protocol_registration`


<a id="@The_registration_NP_relation_($\mathcal{R}_\mathsf{dl}$)_0"></a>

## The registration NP relation ($\mathcal{R}_\mathsf{dl}$)


A ZKPoK that the user knows the decryption key corresponding to their encryption key.

\begin{align}
\mathcal{R}_\mathsf{dl}\left(\mathsf{ek}; \mathsf{dk}\right) = 1
\Leftrightarrow H = \mathsf{dk} \cdot \mathsf{ek}
\end{align}

This is a Schnorr-like proof framed as a homomorphism check:

\begin{align}
\underbrace{H}_{\mathsf{f}_\mathsf{dl}(\mathsf{ek})}
=
\underbrace{\mathsf{dk} \cdot \mathsf{ek}}_{\psi_\mathsf{dl}(\mathsf{dk} \mid \mathsf{ek})}
\end{align}


-  [The registration NP relation ($\mathcal{R}_\mathsf{dl}$)](#@The_registration_NP_relation_($\mathcal{R}_\mathsf{dl}$)_0)
-  [Struct `Registration`](#0x1_sigma_protocol_registration_Registration)
-  [Struct `RegistrationSession`](#0x1_sigma_protocol_registration_RegistrationSession)
-  [Constants](#@Constants_1)
-  [Function `assert_registration_statement_is_well_formed`](#0x1_sigma_protocol_registration_assert_registration_statement_is_well_formed)
-  [Function `new_session`](#0x1_sigma_protocol_registration_new_session)
-  [Function `new_registration_statement`](#0x1_sigma_protocol_registration_new_registration_statement)
-  [Function `psi`](#0x1_sigma_protocol_registration_psi)
-  [Function `f`](#0x1_sigma_protocol_registration_f)
-  [Function `assert_verifies`](#0x1_sigma_protocol_registration_assert_verifies)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="sigma_protocol_fiat_shamir.md#0x1_sigma_protocol_fiat_shamir">0x1::sigma_protocol_fiat_shamir</a>;
<b>use</b> <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof">0x1::sigma_protocol_proof</a>;
<b>use</b> <a href="sigma_protocol_representation.md#0x1_sigma_protocol_representation">0x1::sigma_protocol_representation</a>;
<b>use</b> <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec">0x1::sigma_protocol_representation_vec</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement">0x1::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_statement_builder.md#0x1_sigma_protocol_statement_builder">0x1::sigma_protocol_statement_builder</a>;
<b>use</b> <a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness">0x1::sigma_protocol_witness</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_sigma_protocol_registration_Registration"></a>

## Struct `Registration`

Phantom marker type for registration statements.


<pre><code><b>struct</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">Registration</a> <b>has</b> drop
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

<a id="0x1_sigma_protocol_registration_RegistrationSession"></a>

## Struct `RegistrationSession`

Used for domain separation in the Fiat-Shamir transform.


<pre><code><b>struct</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_RegistrationSession">RegistrationSession</a> <b>has</b> drop
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
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_1"></a>

## Constants


<a id="0x1_sigma_protocol_registration_K"></a>

The number of scalars $k$ in a $\mathcal{R}_\mathsf{dl}$ secret witness.
WARNING: Crucial for security.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_K">K</a>: u64 = 1;
</code></pre>



<a id="0x1_sigma_protocol_registration_E_STATEMENT_BUILDER_INCONSISTENCY"></a>

The homomorphism or transformation function implementation is not inserting points at the expected positions.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_E_STATEMENT_BUILDER_INCONSISTENCY">E_STATEMENT_BUILDER_INCONSISTENCY</a>: u64 = 6;
</code></pre>



<a id="0x1_sigma_protocol_registration_IDX_DK"></a>

Index of $\mathsf{dk}$ (the user's decryption key) in the witness's scalars vector.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_DK">IDX_DK</a>: u64 = 0;
</code></pre>



<a id="0x1_sigma_protocol_registration_IDX_EK"></a>

Index of $\mathsf{ek}$ (the user's encryption key) in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_EK">IDX_EK</a>: u64 = 1;
</code></pre>



<a id="0x1_sigma_protocol_registration_IDX_H"></a>

Index of $H$ (the encryption key basepoint) in the statement's points vector.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_H">IDX_H</a>: u64 = 0;
</code></pre>



<a id="0x1_sigma_protocol_registration_PROTOCOL_ID"></a>

Protocol ID used for domain separation


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_PROTOCOL_ID">PROTOCOL_ID</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 82, 101, 103, 105, 115, 116, 114, 97, 116, 105, 111, 110, 86, 49];
</code></pre>



<a id="0x1_sigma_protocol_registration_E_INVALID_REGISTRATION_PROOF"></a>

The registration proof was invalid


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_E_INVALID_REGISTRATION_PROOF">E_INVALID_REGISTRATION_PROOF</a>: u64 = 5;
</code></pre>



<a id="0x1_sigma_protocol_registration_M"></a>

The number of points $m$ in the image of the $\mathcal{R}_\mathsf{dl}$ homomorphism and transformation function.
WARNING: Crucial for security.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_M">M</a>: u64 = 1;
</code></pre>



<a id="0x1_sigma_protocol_registration_N_1"></a>

The number of points $n_1$ in a $\mathcal{R}_\mathsf{dl}$ public statement.
WARNING: Crucial for security.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_N_1">N_1</a>: u64 = 2;
</code></pre>



<a id="0x1_sigma_protocol_registration_N_2"></a>

The number of scalars $n_2$ in a $\mathcal{R}_\mathsf{dl}$ public statement.
WARNING: Crucial for security.


<pre><code><b>const</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_N_2">N_2</a>: u64 = 0;
</code></pre>



<a id="0x1_sigma_protocol_registration_assert_registration_statement_is_well_formed"></a>

## Function `assert_registration_statement_is_well_formed`

Ensures the statement has <code><a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_N_1">N_1</a></code> points and <code><a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_N_2">N_2</a></code> scalars.


<pre><code><b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_assert_registration_statement_is_well_formed">assert_registration_statement_is_well_formed</a>(stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">sigma_protocol_registration::Registration</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_assert_registration_statement_is_well_formed">assert_registration_statement_is_well_formed</a>(stmt: &Statement&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">Registration</a>&gt;) {
    <b>assert</b>!(stmt.get_points().length() == <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_N_1">N_1</a>, e_wrong_num_points());
    <b>assert</b>!(stmt.get_scalars().length() == <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_N_2">N_2</a>, e_wrong_num_scalars());
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_registration_new_session"></a>

## Function `new_session`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_new_session">new_session</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_RegistrationSession">sigma_protocol_registration::RegistrationSession</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_new_session">new_session</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;Metadata&gt;): <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_RegistrationSession">RegistrationSession</a> {
    <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_RegistrationSession">RegistrationSession</a> {
        sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        asset_type,
    }
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_registration_new_registration_statement"></a>

## Function `new_registration_statement`

Creates a new registration statement: $(H, \mathsf{ek})$.

H is computed internally via <code>get_encryption_key_basepoint_compressed()</code>.
ek is decompressed internally from <code>compressed_ek</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_new_registration_statement">new_registration_statement</a>(compressed_ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>): <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">sigma_protocol_registration::Registration</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_new_registration_statement">new_registration_statement</a>(
    compressed_ek: CompressedRistretto,
): Statement&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">Registration</a>&gt; {
    <b>let</b> b = new_builder();
    <b>assert</b>!(b.add_point(get_encryption_key_basepoint_compressed()) == <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_H">IDX_H</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_E_STATEMENT_BUILDER_INCONSISTENCY">E_STATEMENT_BUILDER_INCONSISTENCY</a>)); // H
    <b>assert</b>!(b.add_point(compressed_ek) == <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_EK">IDX_EK</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_E_STATEMENT_BUILDER_INCONSISTENCY">E_STATEMENT_BUILDER_INCONSISTENCY</a>)); // ek
    <b>let</b> stmt = b.build();
    <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_assert_registration_statement_is_well_formed">assert_registration_statement_is_well_formed</a>(&stmt);
    stmt
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_registration_psi"></a>

## Function `psi`

The homomorphism $\psi_\mathsf{dl}(\mathsf{dk} \mid \mathsf{ek}) = \mathsf{dk} \cdot \mathsf{ek}$.


<pre><code><b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_psi">psi</a>(stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">sigma_protocol_registration::Registration</a>&gt;, w: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>): <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_psi">psi</a>(stmt: &Statement&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">Registration</a>&gt;, w: &Witness): RepresentationVec {
    // WARNING: Crucial for security
    <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_assert_registration_statement_is_well_formed">assert_registration_statement_is_well_formed</a>(stmt);
    // WARNING: Crucial for security
    <b>assert</b>!(w.length() == <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_K">K</a>, e_wrong_witness_len());

    <b>let</b> dk = *w.get(<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_DK">IDX_DK</a>);

    <b>let</b> reprs = (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        // dk * ek
        repr_scaled(<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_EK">IDX_EK</a>, dk),
    ]);

    // WARNING: Crucial for security
    <b>assert</b>!(reprs.length() == <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_M">M</a>, e_wrong_output_len());

    new_representation_vec(reprs)
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_registration_f"></a>

## Function `f`

The transformation function $\mathsf{f}_\mathsf{dl}(\mathsf{ek}) = H$.


<pre><code><b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_f">f</a>(_stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">sigma_protocol_registration::Registration</a>&gt;): <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_f">f</a>(_stmt: &Statement&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">Registration</a>&gt;): RepresentationVec {
    // We do not re-<b>assert</b> well-formedness since wherever f is called, psi is also called.
    new_representation_vec(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        // H
        repr_point(<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_IDX_H">IDX_H</a>),
    ])
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_registration_assert_verifies"></a>

## Function `assert_verifies`

Asserts that a registration proof verifies.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_assert_verifies">assert_verifies</a>(self: &<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_RegistrationSession">sigma_protocol_registration::RegistrationSession</a>, stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">sigma_protocol_registration::Registration</a>&gt;, proof: &<a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_assert_verifies">assert_verifies</a>(self: &<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_RegistrationSession">RegistrationSession</a>, stmt: &Statement&lt;<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_Registration">Registration</a>&gt;, proof: &Proof) {
    <b>let</b> success = <a href="sigma_protocol.md#0x1_sigma_protocol_verify">sigma_protocol::verify</a>(
        new_domain_separator(@aptos_framework, <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>(), <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_PROTOCOL_ID">PROTOCOL_ID</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(self)),
        |_X, w| <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_psi">psi</a>(_X, w),
        |_X| <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_f">f</a>(_X),
        stmt,
        proof
    );

    <b>assert</b>!(success, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_E_INVALID_REGISTRATION_PROOF">E_INVALID_REGISTRATION_PROOF</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
