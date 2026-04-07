
<a id="0x1_sigma_protocol_transfer"></a>

# Module `0x1::sigma_protocol_transfer`


<a id="@The_transfer_NP_relation_($\mathcal{R}^{-}_\mathsf{txfer}$)_0"></a>

## The transfer NP relation ($\mathcal{R}^{-}_\mathsf{txfer}$)


$\def\old#1{{\color{red}{\dot{#1}}}}\def\new#1{{\color{teal}{\widetilde{#1}}}}\def\opt#1{{\color{orange}{\boldsymbol{[}}} #1 {\color{orange}{\boldsymbol{]}}}}$

A ZKPoK of a correct confidential transfer from sender to recipient. This is a composition of
$\mathcal{R}^\mathsf{veiled}_\mathsf{withdraw}$ (the sender's balance update with SECRET amount $\mathbf{v}$)
and $\mathcal{R}_\mathsf{eq}$ (the transfer amount encrypted identically under all parties' keys).


<a id="@Notation_1"></a>

### Notation


- $\old{x}$ denotes a stale/old ciphertext component; $\new{x}$ denotes a fresh/new one.
- $\opt{\cdot}$ denotes components present only when has\_effective\_auditor is true.
- $\langle \mathbf{x}, \mathbf{y} \rangle = \sum_i x_i \cdot y_i$ denotes the inner product.
- $\mathbf{B} = (B^0, B^1, \ldots)$ where $B = 2^{16}$ is the positional weight vector for chunk encoding.
- $\ell$: number of available balance chunks; $n$: number of transfer (pending balance) chunks.
- $T$: number of voluntary auditors ($T \ge 0$).
- The effective auditor (if present) sees the sender's new balance AND the transfer amount.
Extra auditors see only the transfer amount.


<a id="@The_relation_2"></a>

### The relation


$$
\mathcal{R}^{-}_\mathsf{txfer}\left(\begin{array}{l}
G, H, \mathsf{ek}^\mathsf{sid}, \mathsf{ek}^\mathsf{rid},
\old{\mathbf{P}}, \old{\mathbf{R}}, \new{\mathbf{P}}, \new{\mathbf{R}},
\mathbf{P}, \mathbf{R}^\mathsf{sid}, \mathbf{R}^\mathsf{rid},\\
\opt{\mathsf{ek}^\mathsf{eff}, \new{\mathbf{R}}^\mathsf{eff}, \mathbf{R}^\mathsf{eff}},\;
(\mathsf{ek}^\mathsf{ex}_i, \mathbf{R}^\mathsf{ex}_i)_{i \in [T]}
\textbf{;}\\
\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r}
\end{array}\right) = 1
\Leftrightarrow
\left\{\begin{array}{r@{\,\,}l@{\quad}l}
H &= \mathsf{dk} \cdot \mathsf{ek}^\mathsf{sid}\\
\new{P}_i &= \new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
\new{R}_i &= \new{r}_i \cdot \mathsf{ek}^\mathsf{sid}, &\forall i \in [\ell]\\
\opt{\new{R}^\mathsf{eff}_i} &\opt{= \new{r}_i \cdot \mathsf{ek}^\mathsf{eff},}
&\opt{\forall i \in [\ell]}\\
\langle \mathbf{B}, \old{\mathbf{P}} \rangle &= \mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
+ (\langle \mathbf{B}, \new{\mathbf{a}} \rangle + \langle \mathbf{B}, \mathbf{v} \rangle) \cdot G\\
P_j &= v_j \cdot G + r_j \cdot H, &\forall j \in [n]\\
R^\mathsf{sid}_j &= r_j \cdot \mathsf{ek}^\mathsf{sid}, &\forall j \in [n]\\
R^\mathsf{rid}_j &= r_j \cdot \mathsf{ek}^\mathsf{rid}, &\forall j \in [n]\\
\opt{R^\mathsf{eff}_j} &\opt{= r_j \cdot \mathsf{ek}^\mathsf{eff},}
&\opt{\forall j \in [n]}\\
R^\mathsf{ex}_{i,j} &= r_j \cdot \mathsf{ek}^\mathsf{ex}_i,
&\forall j \in [n],\; \forall i \in [T]\\
\end{array}\right.
$$


<a id="@Homomorphism_3"></a>

### Homomorphism


This can be framed as a homomorphism check $\psi(\mathbf{w}) = f(\mathbf{X})$ where
$\mathbf{w} = (\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r})$
is the witness and $\mathbf{X}$ is the statement.

1. The homomorphism $\psi$ is:

$$
\psi(\mathsf{dk}, \new{\mathbf{a}}, \new{\mathbf{r}}, \mathbf{v}, \mathbf{r}) = \begin{pmatrix}
\mathsf{dk} \cdot \mathsf{ek}^\mathsf{sid}\\
\new{a}_i \cdot G + \new{r}_i \cdot H, &\forall i \in [\ell]\\
\new{r}_i \cdot \mathsf{ek}^\mathsf{sid}, &\forall i \in [\ell]\\
\opt{\new{r}_i \cdot \mathsf{ek}^\mathsf{eff}, \;\forall i \in [\ell]}\\
\mathsf{dk} \cdot \langle \mathbf{B}, \old{\mathbf{R}} \rangle
+ (\langle \mathbf{B}, \new{\mathbf{a}} \rangle + \langle \mathbf{B}, \mathbf{v} \rangle) \cdot G\\
v_j \cdot G + r_j \cdot H, &\forall j \in [n]\\
r_j \cdot \mathsf{ek}^\mathsf{sid}, &\forall j \in [n]\\
r_j \cdot \mathsf{ek}^\mathsf{rid}, &\forall j \in [n]\\
\opt{r_j \cdot \mathsf{ek}^\mathsf{eff}, \;\forall j \in [n]}\\
r_j \cdot \mathsf{ek}^\mathsf{ex}_i, &\forall j \in [n],\; \forall i \in [T]\\
\end{pmatrix}
$$

2. The transformation function $f$ is:

$$
f(\mathbf{X}) = \begin{pmatrix}
H\\
\new{P}_i, &\forall i \in [\ell]\\
\new{R}_i, &\forall i \in [\ell]\\
\opt{\new{R}^\mathsf{eff}_i, \;\forall i \in [\ell]}\\
\langle \mathbf{B}, \old{\mathbf{P}} \rangle\\
P_j, &\forall j \in [n]\\
R^\mathsf{sid}_j, &\forall j \in [n]\\
R^\mathsf{rid}_j, &\forall j \in [n]\\
\opt{R^\mathsf{eff}_j, \;\forall j \in [n]}\\
R^\mathsf{ex}_{i,j}, &\forall j \in [n],\; \forall i \in [T]\\
\end{pmatrix}
$$


-  [The transfer NP relation ($\mathcal{R}^{-}_\mathsf{txfer}$)](#@The_transfer_NP_relation_($\mathcal{R}^{-}_\mathsf{txfer}$)_0)
    -  [Notation](#@Notation_1)
    -  [The relation](#@The_relation_2)
    -  [Homomorphism](#@Homomorphism_3)
-  [Struct `Transfer`](#0x1_sigma_protocol_transfer_Transfer)
-  [Struct `TransferSession`](#0x1_sigma_protocol_transfer_TransferSession)
-  [Constants](#@Constants_4)
-  [Function `get_ell`](#0x1_sigma_protocol_transfer_get_ell)
-  [Function `get_n`](#0x1_sigma_protocol_transfer_get_n)
-  [Function `assert_transfer_statement_is_well_formed`](#0x1_sigma_protocol_transfer_assert_transfer_statement_is_well_formed)
-  [Function `new_session`](#0x1_sigma_protocol_transfer_new_session)
-  [Function `new_transfer_statement`](#0x1_sigma_protocol_transfer_new_transfer_statement)
-  [Function `psi`](#0x1_sigma_protocol_transfer_psi)
-  [Function `expected_output_len`](#0x1_sigma_protocol_transfer_expected_output_len)
-  [Function `f`](#0x1_sigma_protocol_transfer_f)
-  [Function `assert_verifies`](#0x1_sigma_protocol_transfer_assert_verifies)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="confidential_amount.md#0x1_confidential_amount">0x1::confidential_amount</a>;
<b>use</b> <a href="confidential_balance.md#0x1_confidential_balance">0x1::confidential_balance</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
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



<a id="0x1_sigma_protocol_transfer_Transfer"></a>

## Struct `Transfer`

Phantom marker type for transfer statements.


<pre><code><b>struct</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">Transfer</a> <b>has</b> drop
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

<a id="0x1_sigma_protocol_transfer_TransferSession"></a>

## Struct `TransferSession`

Used for domain separation in the Fiat-Shamir transform.


<pre><code><b>struct</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_TransferSession">TransferSession</a> <b>has</b> drop
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
<code>recipient: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_avail_chunks: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>num_transfer_chunks: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>has_effective_auditor: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>num_volun_auditors: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_4"></a>

## Constants


<a id="0x1_sigma_protocol_transfer_E_AUDITOR_COUNT_MISMATCH"></a>

The number of auditor R components does not match the expected auditor count.


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>: u64 = 6;
</code></pre>



<a id="0x1_sigma_protocol_transfer_E_STATEMENT_BUILDER_INCONSISTENCY"></a>

The homomorphism or transformation function implementation is not inserting points at the expected positions.


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_STATEMENT_BUILDER_INCONSISTENCY">E_STATEMENT_BUILDER_INCONSISTENCY</a>: u64 = 7;
</code></pre>



<a id="0x1_sigma_protocol_transfer_IDX_DK"></a>

Index of dk (sender's decryption key).


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_DK">IDX_DK</a>: u64 = 0;
</code></pre>



<a id="0x1_sigma_protocol_transfer_IDX_G"></a>

Index of $G$ (the Ristretto255 basepoint).


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_G">IDX_G</a>: u64 = 0;
</code></pre>



<a id="0x1_sigma_protocol_transfer_IDX_H"></a>

Index of $H$ (the encryption key basepoint).


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_H">IDX_H</a>: u64 = 1;
</code></pre>



<a id="0x1_sigma_protocol_transfer_START_IDX_OLD_P"></a>

old_P starts at index 4.
Layout: old_P[1..ℓ], old_R[1..ℓ], new_P[1..ℓ], new_R[1..ℓ], amount_P[1..n], amount_R_sender[1..n], amount_R_recip[1..n]
With effective auditor: ..., ek_eff_aud, new_R_eff_aud[1..ℓ], amount_R_eff_aud[1..n]
For each voluntary auditor t: ..., ek_volun_auds[t], amount_R_volun_auds[t][1..n]


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a>: u64 = 4;
</code></pre>



<a id="0x1_sigma_protocol_transfer_E_INVALID_TRANSFER_PROOF"></a>

new_a[0..ℓ-1] at 1..ℓ. new_r[0..ℓ-1] at 1+ℓ..2ℓ.
v[0..n-1] at 1+2ℓ..1+2ℓ+n-1. r[0..n-1] at 1+2ℓ+n..1+2ℓ+2n-1.
The transfer proof was invalid.


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_INVALID_TRANSFER_PROOF">E_INVALID_TRANSFER_PROOF</a>: u64 = 5;
</code></pre>



<a id="0x1_sigma_protocol_transfer_IDX_EK_RECIP"></a>

Index of $\mathsf{ek}^\mathsf{rid}$ (the recipient's encryption key).


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_RECIP">IDX_EK_RECIP</a>: u64 = 3;
</code></pre>



<a id="0x1_sigma_protocol_transfer_IDX_EK_SENDER"></a>

Index of $\mathsf{ek}^\mathsf{sid}$ (the sender's encryption key).


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_SENDER">IDX_EK_SENDER</a>: u64 = 2;
</code></pre>



<a id="0x1_sigma_protocol_transfer_PROTOCOL_ID"></a>

Protocol ID used for domain separation


<pre><code><b>const</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_PROTOCOL_ID">PROTOCOL_ID</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 84, 114, 97, 110, 115, 102, 101, 114, 86, 49];
</code></pre>



<a id="0x1_sigma_protocol_transfer_get_ell"></a>

## Function `get_ell`

Returns the fixed number of available balance chunks ℓ.


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_ell">get_ell</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_ell">get_ell</a>(): u64 { get_num_available_chunks() }
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_get_n"></a>

## Function `get_n`

Returns the fixed number of transfer (pending) balance chunks n.


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_n">get_n</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_n">get_n</a>(): u64 { get_num_pending_chunks() }
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_assert_transfer_statement_is_well_formed"></a>

## Function `assert_transfer_statement_is_well_formed`

Validates the statement structure.

Expected point count: 4 + 4ℓ + 3n + (has_eff ? 1+ℓ+n : 0) + num_volun*(1+n)


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_assert_transfer_statement_is_well_formed">assert_transfer_statement_is_well_formed</a>(stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">sigma_protocol_transfer::Transfer</a>&gt;, has_effective_auditor: bool, num_volun_auditors: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_assert_transfer_statement_is_well_formed">assert_transfer_statement_is_well_formed</a>(
    stmt: &Statement&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">Transfer</a>&gt;, has_effective_auditor: bool, num_volun_auditors: u64,
) {
    <b>let</b> ell = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_ell">get_ell</a>();
    <b>let</b> n = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_n">get_n</a>();
    <b>let</b> num_points = stmt.get_points().length();

    <b>let</b> expected_num_points = 4 + 4 * ell + 3 * n
        + <b>if</b> (has_effective_auditor) { 1 + ell + n } <b>else</b> { 0 }
        + num_volun_auditors * (1 + n);
    <b>assert</b>!(num_points == expected_num_points, e_wrong_num_points());
    <b>assert</b>!(stmt.get_scalars().length() == 0, e_wrong_num_scalars());
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_new_session"></a>

## Function `new_session`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_new_session">new_session</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, has_effective_auditor: bool, num_volun_auditors: u64): <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_TransferSession">sigma_protocol_transfer::TransferSession</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_new_session">new_session</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <b>address</b>,
    asset_type: Object&lt;Metadata&gt;,
    has_effective_auditor: bool,
    num_volun_auditors: u64,
): <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_TransferSession">TransferSession</a> {
    <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_TransferSession">TransferSession</a> {
        sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        recipient,
        asset_type,
        num_avail_chunks: get_num_available_chunks(),
        num_transfer_chunks: get_num_pending_chunks(),
        has_effective_auditor,
        num_volun_auditors,
    }
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_new_transfer_statement"></a>

## Function `new_transfer_statement`

Creates a transfer statement, optionally including effective and voluntary auditor components.

Points (base): [G, H, ek_sender, ek_recip, old_P[1..ℓ], old_R[1..ℓ], new_P[1..ℓ], new_R[1..ℓ], amount_P[1..n], amount_R_sender[1..n], amount_R_recip[1..n]]
If effective: + [ek_eff_aud, new_R_eff_aud[1..ℓ], amount_R_eff_aud[1..n]]
For each voluntary auditor t: + [ek_volun_auds[t], amount_R_volun_auds[t][1..n]]

For no effective auditor, pass <code><a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()</code> for <code>compressed_ek_eff_aud</code>
and ensure <code>amount</code> / <code>new_balance</code> have empty effective-auditor R components.
For no voluntary auditors, pass an empty vector for <code>compressed_ek_volun_auds</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_new_transfer_statement">new_transfer_statement</a>(compressed_ek_sender: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, compressed_ek_recip: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, compressed_old_balance: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, compressed_new_balance: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, compressed_amount: &<a href="confidential_amount.md#0x1_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>, compressed_ek_eff_aud: &<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, compressed_ek_volun_auds: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;): (<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">sigma_protocol_transfer::Transfer</a>&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_new_transfer_statement">new_transfer_statement</a>(
    compressed_ek_sender: CompressedRistretto,
    compressed_ek_recip: CompressedRistretto,
    compressed_old_balance: &CompressedBalance&lt;Available&gt;,
    compressed_new_balance: &CompressedBalance&lt;Available&gt;,
    compressed_amount: &CompressedAmount,
    compressed_ek_eff_aud: &Option&lt;CompressedRistretto&gt;,
    compressed_ek_volun_auds: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
): (Statement&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">Transfer</a>&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;, Balance&lt;Pending&gt;) {
    <b>let</b> has_eff = compressed_ek_eff_aud.is_some();
    <b>let</b> num_volun = compressed_ek_volun_auds.length();

    // Validate auditor counts before expensive statement construction
    <b>assert</b>!(
        compressed_amount.num_volun_auditors_compressed() == num_volun,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>)
    );
    <b>assert</b>!(
        compressed_amount.has_effective_auditor_compressed() == has_eff,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>)
    );
    <b>assert</b>!(
        compressed_new_balance.get_compressed_R_aud().length() == <b>if</b> (has_eff) { get_num_available_chunks() } <b>else</b> { 0 },
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>)
    );
    <b>assert</b>!(
        compressed_amount.get_compressed_R_volun_auds().all(|r| r.length() == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_n">get_n</a>()),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>)
    );

    <b>let</b> ell = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_ell">get_ell</a>();
    <b>let</b> n = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_n">get_n</a>();
    <b>let</b> e = <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_STATEMENT_BUILDER_INCONSISTENCY">E_STATEMENT_BUILDER_INCONSISTENCY</a>);

    <b>let</b> b = new_builder();
    <b>assert</b>!(b.add_point(<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>()) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_G">IDX_G</a>, e);                                            // G
    <b>assert</b>!(b.add_point(<a href="confidential_balance.md#0x1_confidential_balance_get_encryption_key_basepoint_compressed">confidential_balance::get_encryption_key_basepoint_compressed</a>()) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_H">IDX_H</a>, e);         // H
    <b>assert</b>!(b.add_point(compressed_ek_sender) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_SENDER">IDX_EK_SENDER</a>, e);                                                       // ek_sender
    <b>assert</b>!(b.add_point(compressed_ek_recip) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_RECIP">IDX_EK_RECIP</a>, e);                                                         // ek_recip
    <b>assert</b>!(b.add_points(compressed_old_balance.get_compressed_P()) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a>, e);                            // old_P
    <b>assert</b>!(b.add_points(compressed_old_balance.get_compressed_R()) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + ell, e);                      // old_R
    <b>let</b> (idx, new_balance_P) = b.add_points_cloned(compressed_new_balance.get_compressed_P()); // new_P
    <b>assert</b>!(idx == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 2 * ell, e);
    <b>assert</b>!(b.add_points(compressed_new_balance.get_compressed_R()) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 3 * ell, e);                  // new_R
    <b>let</b> (idx, amount_P) = b.add_points_cloned(compressed_amount.get_compressed_P());           // amount_P
    <b>assert</b>!(idx == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell, e);
    <b>assert</b>!(b.add_points(compressed_amount.get_compressed_R_sender()) == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + n, e);            // amount_R_sender
    <b>let</b> (idx, recip_R) = b.add_points_cloned(compressed_amount.get_compressed_R_recip());      // amount_R_recip
    <b>assert</b>!(idx == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 2 * n, e);

    // Effective auditor: ek, new_R[1..ℓ], amount_R[1..n]
    <b>let</b> idx_eff_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 3 * n;
    <b>if</b> (has_eff) {
        <b>let</b> ek_eff = *compressed_ek_eff_aud.borrow();
        <b>assert</b>!(b.add_point(ek_eff) == idx_eff_start, e);                                                     // ek_eff_aud
        <b>assert</b>!(b.add_points(compressed_new_balance.get_compressed_R_aud()) == idx_eff_start + 1, e);      // new_R_eff_aud
        <b>assert</b>!(b.add_points(compressed_amount.get_compressed_R_eff_aud()) == idx_eff_start + 1 + ell, e); // amount_R_eff_aud
    };

    // Voluntary auditors: for each, append [ek_volun_aud, amount_R_volun_aud[1..n]]
    <b>let</b> idx_volun_start = idx_eff_start + <b>if</b> (has_eff) { 1 + ell + n } <b>else</b> { 0 };
    <b>let</b> compressed_R_volun_auds = compressed_amount.get_compressed_R_volun_auds();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_volun).for_each(|i| {
        <b>let</b> expected_idx = idx_volun_start + i * (1 + n);
        <b>let</b> ek_volun = compressed_ek_volun_auds[i];
        <b>assert</b>!(b.add_point(ek_volun) == expected_idx, e);                             // ek_volun_aud
        <b>assert</b>!(b.add_points(&compressed_R_volun_auds[i]) == expected_idx + 1, e);  // amount_R_volun_aud
    });

    <b>let</b> stmt = b.build();
    <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_assert_transfer_statement_is_well_formed">assert_transfer_statement_is_well_formed</a>(&stmt, has_eff, num_volun);
    <b>let</b> recip_pending = new_pending_from_p_and_r(amount_P, recip_R);
    (stmt, new_balance_P, recip_pending)
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_psi"></a>

## Function `psi`

The combined homomorphism $\psi$ for the transfer relation (see module-level doc for full definition).


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_psi">psi</a>(stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">sigma_protocol_transfer::Transfer</a>&gt;, w: &<a href="sigma_protocol_witness.md#0x1_sigma_protocol_witness_Witness">sigma_protocol_witness::Witness</a>, has_effective_auditor: bool, num_volun_auditors: u64): <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_psi">psi</a>(
    stmt: &Statement&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">Transfer</a>&gt;, w: &Witness,
    has_effective_auditor: bool, num_volun_auditors: u64,
): RepresentationVec {
    // WARNING: Crucial for security
    <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_assert_transfer_statement_is_well_formed">assert_transfer_statement_is_well_formed</a>(stmt, has_effective_auditor, num_volun_auditors);

    <b>let</b> ell = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_ell">get_ell</a>();
    <b>let</b> n = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_n">get_n</a>();

    // WARNING: Crucial for security
    <b>let</b> expected_witness_len = 1 + 2 * ell + 2 * n;
    <b>assert</b>!(w.length() == expected_witness_len, e_wrong_witness_len());

    <b>let</b> b_powers_ell = get_b_powers(ell);
    <b>let</b> b_powers_n = get_b_powers(n);

    <b>let</b> dk = *w.get(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_DK">IDX_DK</a>);

    <b>let</b> reprs = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // === R^veiled_withdraw part ===

    // 1. dk · ek_sender
    reprs.push_back(repr_scaled(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_SENDER">IDX_EK_SENDER</a>, dk));

    // 2. new_a[i] · G + new_r[i] · H
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        <b>let</b> new_a_i = *w.get(1 + i);
        <b>let</b> new_r_i = *w.get(1 + ell + i);
        reprs.push_back(new_representation(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_G">IDX_G</a>, <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_H">IDX_H</a>], <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[new_a_i, new_r_i]));
    });

    // 3. new_r[i] · ek_sender
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        <b>let</b> new_r_i = *w.get(1 + ell + i);
        reprs.push_back(repr_scaled(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_SENDER">IDX_EK_SENDER</a>, new_r_i));
    });

    // 3b. (effective auditor only) new_r[i] · ek_eff_aud
    <b>if</b> (has_effective_auditor) {
        <b>let</b> idx_ek_eff_aud = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 3 * n;
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
            <b>let</b> new_r_i = *w.get(1 + ell + i);
            reprs.push_back(repr_scaled(idx_ek_eff_aud, new_r_i));
        });
    };

    // 4. Balance equation: dk · ⟨B, old_R⟩ + (⟨B, new_a⟩ + ⟨B, v⟩) · G
    <b>let</b> idx_old_R_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + ell;
    <b>let</b> point_idxs = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> scalars = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // dk · B^i · old_R[i]
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        point_idxs.push_back(idx_old_R_start + i);
        scalars.push_back(dk.scalar_mul(&b_powers_ell[i]));
    });

    // new_a[i] · B^i · G
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        <b>let</b> new_a_i = *w.get(1 + i);
        point_idxs.push_back(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_G">IDX_G</a>);
        scalars.push_back(new_a_i.scalar_mul(&b_powers_ell[i]));
    });

    // v[j] · B^j · G (the secret transfer amount)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
        <b>let</b> v_j = *w.get(1 + 2 * ell + j);
        point_idxs.push_back(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_G">IDX_G</a>);
        scalars.push_back(v_j.scalar_mul(&b_powers_n[j]));
    });

    reprs.push_back(new_representation(point_idxs, scalars));

    // === R_eq part ===

    <b>let</b> idx_v_start = 1 + 2 * ell;
    <b>let</b> idx_r_start = 1 + 2 * ell + n;

    // 5. v[j] · G + r[j] · H
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
        <b>let</b> v_j = *w.get(idx_v_start + j);
        <b>let</b> r_j = *w.get(idx_r_start + j);
        reprs.push_back(new_representation(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_G">IDX_G</a>, <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_H">IDX_H</a>], <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[v_j, r_j]));
    });

    // 6. r[j] · ek_sender
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
        <b>let</b> r_j = *w.get(idx_r_start + j);
        reprs.push_back(repr_scaled(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_SENDER">IDX_EK_SENDER</a>, r_j));
    });

    // 7. r[j] · ek_recip
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
        <b>let</b> r_j = *w.get(idx_r_start + j);
        reprs.push_back(repr_scaled(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_EK_RECIP">IDX_EK_RECIP</a>, r_j));
    });

    // 7b. (effective auditor only) r[j] · ek_eff_aud
    <b>if</b> (has_effective_auditor) {
        <b>let</b> idx_ek_eff_aud = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 3 * n;
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
            <b>let</b> r_j = *w.get(idx_r_start + j);
            reprs.push_back(repr_scaled(idx_ek_eff_aud, r_j));
        });
    };

    // 7c. (voluntary auditors) r[j] · ek_volun_aud_t, for each voluntary auditor t
    <b>let</b> idx_volun_auds_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 3 * n
        + <b>if</b> (has_effective_auditor) { 1 + ell + n } <b>else</b> { 0 };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_volun_auditors).for_each(|i| {
        <b>let</b> idx_ek_volun_aud = idx_volun_auds_start + i * (1 + n);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
            <b>let</b> r_j = *w.get(idx_r_start + j);
            reprs.push_back(repr_scaled(idx_ek_volun_aud, r_j));
        });
    });

    // WARNING: Crucial for security
    <b>assert</b>!(reprs.length() == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_expected_output_len">expected_output_len</a>(ell, n, has_effective_auditor, num_volun_auditors), e_wrong_output_len());
    new_representation_vec(reprs)
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_expected_output_len"></a>

## Function `expected_output_len`



<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_expected_output_len">expected_output_len</a>(ell: u64, n: u64, has_effective_auditor: bool, num_volun_auditors: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_expected_output_len">expected_output_len</a>(ell: u64, n: u64, has_effective_auditor: bool, num_volun_auditors: u64): u64 {
    2 + 2 * ell + 3 * n + (<b>if</b> (has_effective_auditor) { ell + n } <b>else</b> { 0 }) + num_volun_auditors * n
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_f"></a>

## Function `f`

The transformation function $f$ for the transfer relation (see module-level doc for full definition).


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_f">f</a>(_stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">sigma_protocol_transfer::Transfer</a>&gt;, has_effective_auditor: bool, num_volun_auditors: u64): <a href="sigma_protocol_representation_vec.md#0x1_sigma_protocol_representation_vec_RepresentationVec">sigma_protocol_representation_vec::RepresentationVec</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_f">f</a>(
    _stmt: &Statement&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">Transfer</a>&gt;,
    has_effective_auditor: bool, num_volun_auditors: u64,
): RepresentationVec {
    <b>let</b> ell = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_ell">get_ell</a>();
    <b>let</b> n = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_get_n">get_n</a>();
    <b>let</b> b_powers_ell = get_b_powers(ell);

    <b>let</b> idx_new_P_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 2 * ell;
    <b>let</b> idx_new_R_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 3 * ell;
    <b>let</b> idx_amount_P_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell;
    <b>let</b> idx_amount_R_sender_start = idx_amount_P_start + n;
    <b>let</b> idx_amount_R_recip_start = idx_amount_R_sender_start + n;

    <b>let</b> reprs = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // === R^veiled_withdraw part ===

    // 1. H
    reprs.push_back(repr_point(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_IDX_H">IDX_H</a>));

    // 2. new_P[i]
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        reprs.push_back(repr_point(idx_new_P_start + i));
    });

    // 3. new_R[i]
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        reprs.push_back(repr_point(idx_new_R_start + i));
    });

    // 3b. (effective auditor only) new_R_eff_aud[i]
    <b>if</b> (has_effective_auditor) {
        <b>let</b> idx_ek_eff_aud = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 3 * n;
        <b>let</b> idx_new_R_eff_aud_start = idx_ek_eff_aud + 1;
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
            reprs.push_back(repr_point(idx_new_R_eff_aud_start + i));
        });
    };

    // 4. ⟨B, old_P⟩ (no -v·G because v is secret)
    <b>let</b> point_idxs = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> scalars = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, ell).for_each(|i| {
        point_idxs.push_back(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + i);
        scalars.push_back(b_powers_ell[i]);
    });
    reprs.push_back(new_representation(point_idxs, scalars));

    // === R_eq part ===

    // 5. amount_P[j]
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
        reprs.push_back(repr_point(idx_amount_P_start + j));
    });

    // 6. amount_R_sender[j]
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
        reprs.push_back(repr_point(idx_amount_R_sender_start + j));
    });

    // 7. amount_R_recip[j]
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
        reprs.push_back(repr_point(idx_amount_R_recip_start + j));
    });

    // 7b. (effective auditor only) amount_R_eff_aud[j]
    <b>if</b> (has_effective_auditor) {
        <b>let</b> idx_amount_R_eff_aud_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 3 * n + 1 + ell;
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
            reprs.push_back(repr_point(idx_amount_R_eff_aud_start + j));
        });
    };

    // 7c. (voluntary auditors) amount_R_volun_aud_t[j], for each voluntary auditor t
    <b>let</b> idx_volun_auds_start = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_START_IDX_OLD_P">START_IDX_OLD_P</a> + 4 * ell + 3 * n
        + <b>if</b> (has_effective_auditor) { 1 + ell + n } <b>else</b> { 0 };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, num_volun_auditors).for_each(|i| {
        <b>let</b> idx_amount_R_volun_aud_start = idx_volun_auds_start + i * (1 + n) + 1;
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, n).for_each(|j| {
            reprs.push_back(repr_point(idx_amount_R_volun_aud_start + j));
        });
    });

    // Note: Not needed for security, since a mismatched <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_f">f</a>(X) length will be caught in the verifier. But good practice
    // for catching mistakes *early* when implementing your <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_f">f</a>(X).
    <b>assert</b>!(reprs.length() == <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_expected_output_len">expected_output_len</a>(ell, n, has_effective_auditor, num_volun_auditors), e_wrong_output_len());
    new_representation_vec(reprs)
}
</code></pre>



</details>

<a id="0x1_sigma_protocol_transfer_assert_verifies"></a>

## Function `assert_verifies`

Asserts that a transfer proof verifies.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_assert_verifies">assert_verifies</a>(self: &<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_TransferSession">sigma_protocol_transfer::TransferSession</a>, stmt: &<a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">sigma_protocol_transfer::Transfer</a>&gt;, proof: &<a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_assert_verifies">assert_verifies</a>(
    self: &<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_TransferSession">TransferSession</a>, stmt: &Statement&lt;<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_Transfer">Transfer</a>&gt;, proof: &Proof,
) {
    <b>let</b> has_eff = self.has_effective_auditor;
    <b>let</b> num_volun = self.num_volun_auditors;
    <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_assert_transfer_statement_is_well_formed">assert_transfer_statement_is_well_formed</a>(stmt, has_eff, num_volun);

    <b>let</b> success = <a href="sigma_protocol.md#0x1_sigma_protocol_verify">sigma_protocol::verify</a>(
        new_domain_separator(@aptos_framework, <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>(), <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_PROTOCOL_ID">PROTOCOL_ID</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(self)),
        |_X, w| <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_psi">psi</a>(_X, w, has_eff, num_volun),
        |_X| <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_f">f</a>(_X, has_eff, num_volun),
        stmt,
        proof
    );

    <b>assert</b>!(success, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_E_INVALID_TRANSFER_PROOF">E_INVALID_TRANSFER_PROOF</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
