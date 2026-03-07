
<a id="0x7_sigma_protocol_withdraw"></a>

# Module `0x7::sigma_protocol_withdraw`


<a id="@The_withdrawal_NP_relation_($\mathcal{R}^{-}_\mathsf{withdraw}$)_0"></a>

## The withdrawal NP relation ($\mathcal{R}^{-}_\mathsf{withdraw}$)


$\def\old#1{{\color{red}{\dot{#1}}}}\def\new#1{{\color{teal}{\widetilde{#1}}}}\def\opt#1{{\color{orange}{\boldsymbol{[}}} #1 {\color{orange}{\boldsymbol{]}}}}$

A ZKPoK of a correct balance update when publicly withdrawing amount $v$ from an old available balance.
Also used for normalization (where $v = 0$).


<a id="@Notation_1"></a>

### Notation


- $\old{x}$ denotes a stale/old ciphertext component; $\new{x}$ denotes a fresh/new one.
- $\opt{\cdot}$ denotes components present only when an auditor is set.
Auditor components are placed at the **end** of the statement so that the common prefix is
identical in both cases. psi/f receive auditor presence via an explicit <code>has_auditor</code> flag.
- $\langle \mathbf{x}, \mathbf{y} \rangle = \sum_i x_i \cdot y_i$ denotes the inner product.
- $\mathbf{B} = (B^0, B^1, \ldots)$ where $B = 2^{16}$ is the positional weight vector for chunk encoding.
- $\ell$: number of available balance chunks.


<a id="@The_relation_2"></a>

### The relation


$$
\mathcal{R}^{-}_\mathsf{withdraw}\left(\begin{array}{l}
G, H, \mathsf{ek},
\old{\mathbf{P}}, \old{\mathbf{R}}, \new{\mathbf{P}}, \new{\mathbf{R}},
\opt{\mathsf{ek}^\mathsf{eff}, \new{\mathbf{R}}^\mathsf{eff}}
\textbf{;}\\
\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}
\textbf{;}\; v
\end{array}\right) = 1
\Leftrightarrow
\left\{\begin{array}{r@{\,\,}l@{\quad}l}
H &= \mathsf{dk} \cdot \mathsf{ek}\\
\new{P}_i &= \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
\new{R}_i &= \new{r}_i \cdot \mathsf{ek}, &\forall i \in [\ell]\\
\opt{\new{R}^\mathsf{eff}_i} &\opt{= \new{r}_i \cdot \mathsf{ek}^\mathsf{eff},}
&\opt{\forall i \in [\ell]}\\
\langle \mathbf{B}, \old{\mathbf{P}} \rangle - v \cdot G
&= \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
+ \langle \mathbf{B}, \new{\mathbf{a}} \rangle \cdot G\\
\end{array}\right.
$$

Note: $v$ is a **public** scalar in the statement (not in the witness). It appears in $f$ but not in $\psi$.


<a id="@Homomorphism_3"></a>

### Homomorphism


This can be framed as a homomorphism check $\psi(\mathbf{w}) = f(\mathbf{X})$ where
$\mathbf{w} = (\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}})$ is the witness
and $\mathbf{X}$ is the statement (including public scalar $v$).

1. The homomorphism $\psi$ is:

$$
\psi(\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}) = \begin{pmatrix}
\mathsf{dk} \cdot \mathsf{ek}\\
\new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
\new{r}_i \cdot \mathsf{ek}, &\forall i \in [\ell]\\
\opt{\new{r}_i \cdot \mathsf{ek}^\mathsf{eff}, \;\forall i \in [\ell]}\\
\mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
+ \langle \mathbf{B}, \new{\mathbf{a}} \rangle \cdot G\\
\end{pmatrix}
$$

2. The transformation function $f$ is:

$$
f(\mathbf{X}) = \begin{pmatrix}
H\\
\new{P}_i, &\forall i \in [\ell]\\
\new{R}_i, &\forall i \in [\ell]\\
\opt{\new{R}^\mathsf{eff}_i, \;\forall i \in [\ell]}\\
\langle \mathbf{B}, \old{\mathbf{P}} \rangle - v \cdot G\\
\end{pmatrix}
$$


-  [The withdrawal NP relation ($\mathcal{R}^{-}_\mathsf{withdraw}$)](#@The_withdrawal_NP_relation_($\mathcal{R}^{-}_\mathsf{withdraw}$)_0)
    -  [Notation](#@Notation_1)
    -  [The relation](#@The_relation_2)
    -  [Homomorphism](#@Homomorphism_3)
-  [Struct `Withdrawal`](#0x7_sigma_protocol_withdraw_Withdrawal)
-  [Struct `WithdrawSession`](#0x7_sigma_protocol_withdraw_WithdrawSession)
-  [Constants](#@Constants_4)
-  [Function `get_num_chunks`](#0x7_sigma_protocol_withdraw_get_num_chunks)
-  [Function `assert_withdraw_statement_is_well_formed`](#0x7_sigma_protocol_withdraw_assert_withdraw_statement_is_well_formed)
-  [Function `new_session`](#0x7_sigma_protocol_withdraw_new_session)
-  [Function `new_withdrawal_statement`](#0x7_sigma_protocol_withdraw_new_withdrawal_statement)
-  [Function `psi`](#0x7_sigma_protocol_withdraw_psi)
-  [Function `f`](#0x7_sigma_protocol_withdraw_f)
-  [Function `assert_verifies`](#0x7_sigma_protocol_withdraw_assert_verifies)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="confidential_balance.md#0x7_confidential_balance">0x7::confidential_balance</a>;
<b>use</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir">0x7::sigma_protocol_fiat_shamir</a>;
<b>use</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof">0x7::sigma_protocol_proof</a>;
<b>use</b> <a href="sigma_protocol_representation.md#0x7_sigma_protocol_representation">0x7::sigma_protocol_representation</a>;
<b>use</b> <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec">0x7::sigma_protocol_representation_vec</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement">0x7::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_statement_builder.md#0x7_sigma_protocol_statement_builder">0x7::sigma_protocol_statement_builder</a>;
<b>use</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils">0x7::sigma_protocol_utils</a>;
<b>use</b> <a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness">0x7::sigma_protocol_witness</a>;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_Withdrawal"></a>

## Struct `Withdrawal`

Phantom marker type for withdrawal statements.


<pre><code><b>struct</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">Withdrawal</a> <b>has</b> drop
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

<a id="0x7_sigma_protocol_withdraw_WithdrawSession"></a>

## Struct `WithdrawSession`

Used for domain separation in the Fiat-Shamir transform.


<pre><code><b>struct</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WithdrawSession">WithdrawSession</a> <b>has</b> drop
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
<code>asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_chunks: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>has_auditor: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_4"></a>

## Constants


<a id="0x7_sigma_protocol_withdraw_E_AUDITOR_COUNT_MISMATCH"></a>

The number of auditor R components does not match the expected auditor count.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>: u64 = 6;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_E_INVALID_PROOF"></a>

new_a[0..ℓ-1] starts at index 1. new_r[0..ℓ-1] starts at 1 + ℓ.
The withdrawal proof was invalid.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_E_INVALID_PROOF">E_INVALID_PROOF</a>: u64 = 5;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_E_TEST_INTERNAL"></a>

An error occurred in one of our tests.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_E_TEST_INTERNAL">E_TEST_INTERNAL</a>: u64 = 1000;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_IDX_DK"></a>

Index of $\mathsf{dk}$ in the witness.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_DK">IDX_DK</a>: u64 = 0;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_IDX_EK"></a>

Index of $\mathsf{ek}$ (the sender's encryption key) in the statement.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_EK">IDX_EK</a>: u64 = 2;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_IDX_G"></a>

Index of $G$ (the Ristretto255 basepoint) in the statement.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_G">IDX_G</a>: u64 = 0;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_IDX_H"></a>

Index of $H$ (the encryption key basepoint) in the statement.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_H">IDX_H</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_START_IDX_OLD_P"></a>

old_P values start at index 3. old_R starts at 3 + ℓ. new_P at 3 + 2ℓ. new_R at 3 + 3ℓ.
If auditor present: ek_aud at 3 + 4ℓ, then new_R_aud at 3 + 4ℓ + 1.


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_START_IDX_OLD_P">START_IDX_OLD_P</a>: u64 = 3;
</code></pre>



<a id="0x7_sigma_protocol_withdraw_WITHDRAWAL_PROTOCOL_ID"></a>

Protocol ID for withdrawal proofs (also used for normalization, which is withdrawal with v = 0)


<pre><code><b>const</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WITHDRAWAL_PROTOCOL_ID">WITHDRAWAL_PROTOCOL_ID</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 87, 105, 116, 104, 100, 114, 97, 119, 97, 108, 86, 49];
</code></pre>



<a id="0x7_sigma_protocol_withdraw_get_num_chunks"></a>

## Function `get_num_chunks`

Returns the fixed number of balance chunks ℓ (= AVAILABLE_BALANCE_CHUNKS).


<pre><code><b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_get_num_chunks">get_num_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_get_num_chunks">get_num_chunks</a>(): u64 {
    get_num_available_chunks()
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_withdraw_assert_withdraw_statement_is_well_formed"></a>

## Function `assert_withdraw_statement_is_well_formed`

Validates that the statement has the correct structure for the given auditor flag.


<pre><code><b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_withdraw_statement_is_well_formed">assert_withdraw_statement_is_well_formed</a>(stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">sigma_protocol_withdraw::Withdrawal</a>&gt;, has_auditor: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_withdraw_statement_is_well_formed">assert_withdraw_statement_is_well_formed</a>(stmt: &Statement&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">Withdrawal</a>&gt;, has_auditor: bool) {
    <b>let</b> ell = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_get_num_chunks">get_num_chunks</a>();
    <b>let</b> expected = 3 + 4 * ell + <b>if</b> (has_auditor) { 1 + ell } <b>else</b> { 0 };
    <b>assert</b>!(stmt.get_points().length() == expected,e_wrong_num_points());
    // i.e., the transferred amount v
    <b>assert</b>!(stmt.get_scalars().length() == 1, e_wrong_num_scalars());
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_withdraw_new_session"></a>

## Function `new_session`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_session">new_session</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, has_auditor: bool): <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WithdrawSession">sigma_protocol_withdraw::WithdrawSession</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_session">new_session</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;Metadata&gt;, has_auditor: bool): <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WithdrawSession">WithdrawSession</a> {
    <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WithdrawSession">WithdrawSession</a> {
        sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        asset_type,
        num_chunks: <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_get_num_chunks">get_num_chunks</a>(),
        has_auditor,
    }
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_withdraw_new_withdrawal_statement"></a>

## Function `new_withdrawal_statement`

Creates a withdrawal statement, optionally including auditor components.

Points (auditorless): [ G, H, ek, old_P[0..ℓ-1], old_R[0..ℓ-1], new_P[0..ℓ-1], new_R[0..ℓ-1] ]
Points (w/ auditor):  [ ---------------------------- as above ------------------------------, ek_aud, new_R_aud]
Scalars:              [ v ]

For the auditorless case, pass <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()</code> for <code>compressed_ek_aud</code>
and ensure <code>new_balance</code> / <code>compressed_new_balance</code> have empty R_aud.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_withdrawal_statement">new_withdrawal_statement</a>(compressed_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, compressed_old_balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Available">confidential_balance::Available</a>&gt;, compressed_new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_Available">confidential_balance::Available</a>&gt;, compressed_ek_aud: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, v: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): (<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">sigma_protocol_withdraw::Withdrawal</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_withdrawal_statement">new_withdrawal_statement</a>(
    compressed_ek: CompressedRistretto,
    compressed_old_balance: &CompressedBalance&lt;Available&gt;,
    compressed_new_balance: &CompressedBalance&lt;Available&gt;,
    compressed_ek_aud: &Option&lt;CompressedRistretto&gt;,
    v: Scalar,
): (Statement&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">Withdrawal</a>&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;) {
    <b>assert</b>!(
        compressed_new_balance.get_compressed_R_aud().length() == <b>if</b> (compressed_ek_aud.is_some()) { <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_get_num_chunks">get_num_chunks</a>() } <b>else</b> { 0 },
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>)
    );

    <b>let</b> b = new_builder();
    b.add_point(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>());                                                 // G
    b.add_point(get_encryption_key_basepoint_compressed());                                            // H
    b.add_point(compressed_ek);                                                                           // ek
    b.add_points(compressed_old_balance.get_compressed_P());                                           // old_P
    b.add_points(compressed_old_balance.get_compressed_R());                                           // old_R
    <b>let</b> (_, new_P) = b.add_points_cloned(compressed_new_balance.get_compressed_P()); // new_P
    b.add_points(compressed_new_balance.get_compressed_R());                                           // new_R

    <b>if</b> (compressed_ek_aud.is_some()) {
        b.add_point(*compressed_ek_aud.borrow());                                                      // ek_aud
        b.add_points(compressed_new_balance.get_compressed_R_aud());                                   // new_R_aud
    };

    b.add_scalar(v);
    <b>let</b> stmt = b.build();
    <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_withdraw_statement_is_well_formed">assert_withdraw_statement_is_well_formed</a>(&stmt, compressed_ek_aud.is_some());
    (stmt, new_P)
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_withdraw_psi"></a>

## Function `psi`

The homomorphism $\psi$ for the withdrawal relation.

Here, B = (B^0, B^1, …, B^{ℓ-1}) with B = 2^16 is the chunk weight vector (see module doc).

Outputs (auditorless, m = 2 + 2ℓ):
1. dk · ek
2. new_a[i] · G + new_r[i] · H, for i ∈ [1..ℓ]
3. new_r[i] · ek, for i ∈ [1..ℓ]
4. dk · ⟨B, old_R⟩ + ⟨B, new_a⟩ · G

With auditor (m = 2 + 3ℓ), inserts between 3 and 4:
3b. new_r[i] · ek_aud, for i ∈ [1..ℓ]


<pre><code><b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_psi">psi</a>(stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">sigma_protocol_withdraw::Withdrawal</a>&gt;, w: &<a href="sigma_protocol_witness.md#0x7_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>, has_auditor: bool): <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_psi">psi</a>(stmt: &Statement&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">Withdrawal</a>&gt;, w: &Witness, has_auditor: bool): RepresentationVec {
    // WARNING: Crucial for security
    <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_withdraw_statement_is_well_formed">assert_withdraw_statement_is_well_formed</a>(stmt, has_auditor);

    <b>let</b> ell = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_get_num_chunks">get_num_chunks</a>();
    <b>let</b> b_powers = get_b_powers(ell);

    // WARNING: Crucial for security
    <b>let</b> expected_witness_len = 1 + 2 * ell;
    <b>assert</b>!(w.length() == expected_witness_len, e_wrong_witness_len());

    <b>let</b> dk = *w.get(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_DK">IDX_DK</a>);

    <b>let</b> reprs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // 1. dk · ek
    reprs.push_back(repr_scaled(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_EK">IDX_EK</a>, dk));

    // 2. new_a[i] · G + new_r[i] · H, for i ∈ [1..ℓ]
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        <b>let</b> new_a_i = *w.get(1 + i);
        <b>let</b> new_r_i = *w.get(1 + ell + i);
        reprs.push_back(new_representation(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_G">IDX_G</a>, <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_H">IDX_H</a>], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[new_a_i, new_r_i]));
    });

    // 3. new_r[i] · ek, for i ∈ [1..ℓ]
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        <b>let</b> new_r_i = *w.get(1 + ell + i);
        reprs.push_back(repr_scaled(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_EK">IDX_EK</a>, new_r_i));
    });

    // 3b. (auditor only) new_r[i] · ek_aud, for i ∈ [1..ℓ]
    <b>if</b> (has_auditor) {
        <b>let</b> idx_ek_aud = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell;
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
            <b>let</b> new_r_i = *w.get(1 + ell + i);
            reprs.push_back(repr_scaled(idx_ek_aud, new_r_i));
        });
    };

    // 4. Balance equation: dk · ⟨B, old_R⟩ + ⟨B, new_a⟩ · G
    <b>let</b> idx_old_R_start = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_START_IDX_OLD_P">START_IDX_OLD_P</a> + ell;

    <b>let</b> point_idxs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> scalars = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // dk · B^i · old_R[i]
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        point_idxs.push_back(idx_old_R_start + i);
        scalars.push_back(dk.scalar_mul(&b_powers[i]));
    });

    // new_a[i] · B^i · G
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        <b>let</b> new_a_i = *w.get(1 + i);
        point_idxs.push_back(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_G">IDX_G</a>);
        scalars.push_back(new_a_i.scalar_mul(&b_powers[i]));
    });

    reprs.push_back(new_representation(point_idxs, scalars));

    <b>let</b> repr_vec = new_representation_vec(reprs);
    <b>let</b> expected_output_len = <b>if</b> (has_auditor) { 2 + 3 * ell } <b>else</b> { 2 + 2 * ell };

    // WARNING: Crucial for security
    <b>assert</b>!(repr_vec.length() == expected_output_len, e_wrong_output_len());

    repr_vec
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_withdraw_f"></a>

## Function `f`

The transformation function $f$ for the withdrawal relation.

Outputs (auditorless, m = 2 + 2ℓ):
1. H
2. new_P[i], for i ∈ [1..ℓ]
3. new_R[i], for i ∈ [1..ℓ]
4. ⟨B, old_P⟩ − v · G

With auditor (m = 2 + 3ℓ), inserts between 3 and 4:
3b. new_R_aud[i], for i ∈ [1..ℓ]


<pre><code><b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_f">f</a>(stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">sigma_protocol_withdraw::Withdrawal</a>&gt;, has_auditor: bool): <a href="sigma_protocol_representation_vec.md#0x7_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_f">f</a>(stmt: &Statement&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">Withdrawal</a>&gt;, has_auditor: bool): RepresentationVec {
    <b>let</b> ell = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_get_num_chunks">get_num_chunks</a>();
    <b>let</b> b_powers = get_b_powers(ell);
    <b>let</b> v = stmt.get_scalars()[0];

    <b>let</b> idx_new_P_start = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_START_IDX_OLD_P">START_IDX_OLD_P</a> + 2 * ell;
    <b>let</b> idx_new_R_start = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_START_IDX_OLD_P">START_IDX_OLD_P</a> + 3 * ell;

    <b>let</b> reprs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // 1. H
    reprs.push_back(repr_point(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_H">IDX_H</a>));

    // 2. new_P[i]
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        reprs.push_back(repr_point(idx_new_P_start + i));
    });

    // 3. new_R[i]
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        reprs.push_back(repr_point(idx_new_R_start + i));
    });

    // 3b. (auditor only) new_R_aud[i]
    <b>if</b> (has_auditor) {
        <b>let</b> idx_new_R_aud_start = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 1; // +1 for ek_aud
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
            reprs.push_back(repr_point(idx_new_R_aud_start + i));
        });
    };

    // 4. ⟨B, old_P⟩ − v · G
    <b>let</b> point_idxs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> scalars = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        point_idxs.push_back(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_START_IDX_OLD_P">START_IDX_OLD_P</a> + i);
        scalars.push_back(b_powers[i]);
    });

    point_idxs.push_back(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_IDX_G">IDX_G</a>);
    scalars.push_back(v.scalar_neg());

    reprs.push_back(new_representation(point_idxs, scalars));

    new_representation_vec(reprs)
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_withdraw_assert_verifies"></a>

## Function `assert_verifies`

Asserts that a withdrawal proof verifies.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_verifies">assert_verifies</a>(self: &<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WithdrawSession">sigma_protocol_withdraw::WithdrawSession</a>, stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">sigma_protocol_withdraw::Withdrawal</a>&gt;, proof: &<a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_verifies">assert_verifies</a>(self: &<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WithdrawSession">WithdrawSession</a>, stmt: &Statement&lt;<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_Withdrawal">Withdrawal</a>&gt;, proof: &Proof) {
    <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_withdraw_statement_is_well_formed">assert_withdraw_statement_is_well_formed</a>(stmt, self.has_auditor);

    <b>let</b> success = <a href="sigma_protocol.md#0x7_sigma_protocol_verify">sigma_protocol::verify</a>(
        new_domain_separator(@aptos_experimental, <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id_get">chain_id::get</a>(), <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_WITHDRAWAL_PROTOCOL_ID">WITHDRAWAL_PROTOCOL_ID</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(self)),
        |_X, w| <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_psi">psi</a>(_X, w, self.has_auditor),
        |_X| <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_f">f</a>(_X, self.has_auditor),
        stmt,
        proof
    );

    <b>assert</b>!(success, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_E_INVALID_PROOF">E_INVALID_PROOF</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
