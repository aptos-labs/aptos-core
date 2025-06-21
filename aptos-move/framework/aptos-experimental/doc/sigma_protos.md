
<a id="0x7_sigma_protos"></a>

# Module `0x7::sigma_protos`

Package for creating, verifying, serializing & deserializing the $\Sigma$-protocol proofs used in veiled coins.


<a id="@Preliminaries_0"></a>

### Preliminaries


Recall that a $\Sigma$-protocol proof argues knowledge of a *secret* witness $w$ such that an arithmetic relation
$R(x; w) = 1$ is satisfied over group and field elements stored in $x$ and $w$.

Here, $x$ is a public statement known to the verifier (i.e., known to the validators). Importantly, the
$\Sigma$-protocol's zero-knowledge property ensures the witness $w$ remains secret.


<a id="@WithdrawalSubproof:_ElGamal-Pedersen_equality_1"></a>

### WithdrawalSubproof: ElGamal-Pedersen equality


This proof is used to provably convert an ElGamal ciphertext to a Pedersen commitment over which a ZK range proof
can be securely computed. Otherwise, knowledge of the ElGamal SK breaks the binding of the 2nd component of the
ElGamal ciphertext, making any ZK range proof over it useless.
Because the sender cannot, after receiving a fully veiled transaction, compute their balance randomness, their
updated balance ciphertext is computed in the relation, which is then linked to the Pedersen commitment of $b$.

The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
- $b$, sender's new balance, after the withdrawal from their veiled balance
- $r$, randomness used to commit to $b$
- $sk$, the sender's secret ElGamal encryption key

(Note that the $\Sigma$-protocol's zero-knowledge property ensures the witness is not revealed.)

The public statement $x$ in this relation consists of:
- $G$, basepoint of a given elliptic curve
- $H$, basepoint used for randomness in the Pedersen commitments
- $(C_1, C_2)$, ElGamal encryption of the sender's current balance
- $c$, Pedersen commitment to $b$ with randomness $r$
- $v$, the amount the sender is withdrawing
- $Y$, the sender's ElGamal encryption public key

The relation being proved is as follows:

```
R(
x = [ (C_1, C_2), c, G, H, Y, v]
w = [ b, r, sk ]
) = {
C_1 - v G = b G + sk C_2
c = b G + r H
Y = sk G
}
```


<a id="@TransferSubproof:_ElGamal-Pedersen_equality_and_ElGamal-ElGamal_equality_2"></a>

### TransferSubproof: ElGamal-Pedersen equality and ElGamal-ElGamal equality


This protocol argues two things. First, that the same amount is ElGamal-encrypted for both the sender and recipient.
This is needed to correctly withdraw & deposit the same amount during a transfer. Second, that this same amount is
committed via Pedersen. Third, that a Pedersen-committed balance is correctly ElGamal encrypted. ZK range proofs
are computed over these last two Pedersen commitments, to prevent overflowing attacks on the balance.

The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
- $v$, amount being transferred
- $r$, randomness used to ElGamal-encrypt $v$
- $b$, sender's new balance after the transfer occurs
- $r_b$, randomness used to Pedersen commit $b$
- $sk$, the sender's secret ElGamal encryption key

The public statement $x$ in this relation consists of:
- Public parameters
+ $G$, basepoint of a given elliptic curve
+ $H$, basepoint used for randomness in the Pedersen commitments
- PKs
+ $Y$, sender's PK
+ $Y'$, recipient's PK
- Amount encryption & commitment
+ $(C, D)$, ElGamal encryption of $v$, under the sender's PK, using randomness $r$
+ $(C', D)$, ElGamal encryption of $v$, under the recipient's PK, using randomness $r$
+ $c$, Pedersen commitment to $v$ using randomness $r$
- New balance encryption & commitment
+ $(C_1, C_2)$, ElGamal encryption of the sender's *current* balance, under the sender's PK. This is used to
compute the sender's updated balance in the relation, as the sender cannot know their balance randomness.
+ $c'$, Pedersen commitment to $b$ using randomness $r_b$

The relation being proved is:
```
R(
x = [ Y, Y', (C, C', D), c, (C_1, C_2), c', G, H ]
w = [ v, r, b, r_b, sk ]
) = {
C  = v G + r Y
C' = v G + r Y'
D  = r G
C_1 - C  = b G + sk (C_2 - D)
c  = v G + r H
c' = b G + r_b H
Y  = sk G
}
```

A relation similar to this is also described on page 14 of the Zether paper [BAZB20] (just replace  $G$ -> $g$,
$C'$ -> $\bar{C}$, $Y$ -> $y$, $Y'$ -> $\bar{y}$, $v$ -> $b^*$). Note that their relation does not include the
ElGamal-to-Pedersen conversion parts, as they can do ZK range proofs directly over ElGamal ciphertexts using their
$\Sigma$-bullets modification of Bulletproofs.


    -  [Preliminaries](#@Preliminaries_0)
    -  [WithdrawalSubproof: ElGamal-Pedersen equality](#@WithdrawalSubproof:_ElGamal-Pedersen_equality_1)
    -  [TransferSubproof: ElGamal-Pedersen equality and ElGamal-ElGamal equality](#@TransferSubproof:_ElGamal-Pedersen_equality_and_ElGamal-ElGamal_equality_2)
-  [Struct `WithdrawalSubproof`](#0x7_sigma_protos_WithdrawalSubproof)
-  [Struct `TransferSubproof`](#0x7_sigma_protos_TransferSubproof)
-  [Constants](#@Constants_3)
-  [Function `verify_transfer_subproof`](#0x7_sigma_protos_verify_transfer_subproof)
-  [Function `verify_withdrawal_subproof`](#0x7_sigma_protos_verify_withdrawal_subproof)
-  [Function `deserialize_withdrawal_subproof`](#0x7_sigma_protos_deserialize_withdrawal_subproof)
-  [Function `deserialize_transfer_subproof`](#0x7_sigma_protos_deserialize_transfer_subproof)
-  [Function `fiat_shamir_withdrawal_subproof_challenge`](#0x7_sigma_protos_fiat_shamir_withdrawal_subproof_challenge)
-  [Function `fiat_shamir_transfer_subproof_challenge`](#0x7_sigma_protos_fiat_shamir_transfer_subproof_challenge)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal">0x1::ristretto255_elgamal</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen">0x1::ristretto255_pedersen</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="helpers.md#0x7_helpers">0x7::helpers</a>;
</code></pre>



<a id="0x7_sigma_protos_WithdrawalSubproof"></a>

## Struct `WithdrawalSubproof`

A $\Sigma$-protocol used during an unveiled withdrawal (for proving the correct ElGamal encryption of a
Pedersen-committed balance).


<pre><code><b>struct</b> <a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_sigma_protos_TransferSubproof"></a>

## Struct `TransferSubproof`

A $\Sigma$-protocol proof used during a veiled transfer. This proof encompasses the $\Sigma$-protocol from
<code><a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a></code>.


<pre><code><b>struct</b> <a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x4: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x5: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x6: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x7: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha4: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>alpha5: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_3"></a>

## Constants


<a id="0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED"></a>

The $\Sigma$-protocol proof for withdrawals did not verify.


<pre><code><b>const</b> <a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protos_FIAT_SHAMIR_SIGMA_DST"></a>

The domain separation tag (DST) used in the Fiat-Shamir transform of our $\Sigma$-protocol.


<pre><code><b>const</b> <a href="sigma_protos.md#0x7_sigma_protos_FIAT_SHAMIR_SIGMA_DST">FIAT_SHAMIR_SIGMA_DST</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 86, 101, 105, 108, 101, 100, 67, 111, 105, 110, 47, 87, 105, 116, 104, 100, 114, 97, 119, 97, 108, 83, 117, 98, 112, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a id="0x7_sigma_protos_verify_transfer_subproof"></a>

## Function `verify_transfer_subproof`

Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled transfer.

Specifically, the proof argues that the same amount $v$ is Pedersen-committed in <code>comm_amount</code> and ElGamal-
encrypted in <code>withdraw_ct</code> (under <code>sender_pk</code>) and in <code>deposit_ct</code> (under <code>recipient_pk</code>), all three using the
same randomness $r$.

In addition, it argues that the sender's new balance $b$ committed to by sender_new_balance_comm is the same
as the value encrypted by the ciphertext obtained by subtracting withdraw_ct from sender_curr_balance_ct


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_verify_transfer_subproof">verify_transfer_subproof</a>(sender_pk: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, recipient_pk: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, withdraw_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, deposit_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, comm_amount: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, sender_new_balance_comm: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, sender_curr_balance_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, proof: &<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">sigma_protos::TransferSubproof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_verify_transfer_subproof">verify_transfer_subproof</a>(
    sender_pk: &elgamal::CompressedPubkey,
    recipient_pk: &elgamal::CompressedPubkey,
    withdraw_ct: &elgamal::Ciphertext,
    deposit_ct: &elgamal::Ciphertext,
    comm_amount: &pedersen::Commitment,
    sender_new_balance_comm: &pedersen::Commitment,
    sender_curr_balance_ct: &elgamal::Ciphertext,
    proof: &<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>)
{
    <b>let</b> h = pedersen::randomness_base_for_bulletproof();
    <b>let</b> sender_pk_point = elgamal::pubkey_to_point(sender_pk);
    <b>let</b> recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
    <b>let</b> (big_c, big_d) = elgamal::ciphertext_as_points(withdraw_ct);
    <b>let</b> (bar_big_c, _) = elgamal::ciphertext_as_points(deposit_ct);
    <b>let</b> c = pedersen::commitment_as_point(comm_amount);
    <b>let</b> (c1, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
    <b>let</b> bar_c = pedersen::commitment_as_point(sender_new_balance_comm);

    // TODO: Can be optimized so we don't re-serialize the proof for Fiat-Shamir
    <b>let</b> rho = <a href="sigma_protos.md#0x7_sigma_protos_fiat_shamir_transfer_subproof_challenge">fiat_shamir_transfer_subproof_challenge</a>(
        sender_pk, recipient_pk,
        withdraw_ct, deposit_ct, comm_amount,
        sender_curr_balance_ct, sender_new_balance_comm,
        &proof.x1, &proof.x2, &proof.x3, &proof.x4,
        &proof.x5, &proof.x6, &proof.x7);

    <b>let</b> g_alpha2 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(&proof.alpha2);
    // \rho * D + X1 =? \alpha_2 * g
    <b>let</b> d_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(big_d, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> d_acc, &proof.x1);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&d_acc, &g_alpha2), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    <b>let</b> g_alpha1 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(&proof.alpha1);
    // \rho * C + X2 =? \alpha_1 * g + \alpha_2 * y
    <b>let</b> big_c_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(big_c, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> big_c_acc, &proof.x2);
    <b>let</b> y_alpha2 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(&sender_pk_point, &proof.alpha2);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> y_alpha2, &g_alpha1);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&big_c_acc, &y_alpha2), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    // \rho * \bar{C} + X3 =? \alpha_1 * g + \alpha_2 * \bar{y}
    <b>let</b> big_bar_c_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(bar_big_c, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> big_bar_c_acc, &proof.x3);
    <b>let</b> y_bar_alpha2 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(&recipient_pk_point, &proof.alpha2);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> y_bar_alpha2, &g_alpha1);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&big_bar_c_acc, &y_bar_alpha2), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    <b>let</b> g_alpha3 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(&proof.alpha3);
    // \rho * (C_1 - C) + X_4 =? \alpha_3 * g + \alpha_5 * (C_2 - D)
    <b>let</b> big_c1_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(c1, big_c);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul_assign">ristretto255::point_mul_assign</a>(&<b>mut</b> big_c1_acc, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> big_c1_acc, &proof.x4);

    <b>let</b> big_c2_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(c2, big_d);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul_assign">ristretto255::point_mul_assign</a>(&<b>mut</b> big_c2_acc, &proof.alpha5);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> big_c2_acc, &g_alpha3);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&big_c1_acc, &big_c2_acc), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    // \rho * c + X_5 =? \alpha_1 * g + \alpha_2 * h
    <b>let</b> c_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(c, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> c_acc, &proof.x5);

    <b>let</b> h_alpha2_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(&h, &proof.alpha2);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> h_alpha2_acc, &g_alpha1);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&c_acc, &h_alpha2_acc), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    // \rho * \bar{c} + X_6 =? \alpha_3 * g + \alpha_4 * h
    <b>let</b> bar_c_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(bar_c, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> bar_c_acc, &proof.x6);

    <b>let</b> h_alpha4_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(&h, &proof.alpha4);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> h_alpha4_acc, &g_alpha3);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&bar_c_acc, &h_alpha4_acc), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    // \rho * Y + X_7 =? \alpha_5 * G
    <b>let</b> y_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(&sender_pk_point, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> y_acc, &proof.x7);

    <b>let</b> g_alpha5 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(&proof.alpha5);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&y_acc, &g_alpha5), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));
}
</code></pre>



</details>

<a id="0x7_sigma_protos_verify_withdrawal_subproof"></a>

## Function `verify_withdrawal_subproof`

Verifies the $\Sigma$-protocol proof necessary to ensure correctness of a veiled-to-unveiled transfer.

Specifically, the proof argues that the same amount $v$ is Pedersen-committed in <code>sender_new_balance_comm</code> and
ElGamal-encrypted in the ciphertext obtained by subtracting the ciphertext (vG, 0G) from sender_curr_balance_ct


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_verify_withdrawal_subproof">verify_withdrawal_subproof</a>(sender_pk: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, sender_curr_balance_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, sender_new_balance_comm: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, amount: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, proof: &<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">sigma_protos::WithdrawalSubproof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_verify_withdrawal_subproof">verify_withdrawal_subproof</a>(
    sender_pk: &elgamal::CompressedPubkey,
    sender_curr_balance_ct: &elgamal::Ciphertext,
    sender_new_balance_comm: &pedersen::Commitment,
    amount: &Scalar,
    proof: &<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>)
{
    <b>let</b> h = pedersen::randomness_base_for_bulletproof();
    <b>let</b> (big_c1, big_c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
    <b>let</b> c = pedersen::commitment_as_point(sender_new_balance_comm);
    <b>let</b> sender_pk_point = elgamal::pubkey_to_point(sender_pk);

    <b>let</b> rho = <a href="sigma_protos.md#0x7_sigma_protos_fiat_shamir_withdrawal_subproof_challenge">fiat_shamir_withdrawal_subproof_challenge</a>(
        sender_pk,
        sender_curr_balance_ct,
        sender_new_balance_comm,
        amount,
        &proof.x1,
        &proof.x2,
        &proof.x3);

    <b>let</b> g_alpha1 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(&proof.alpha1);
    // \rho * (C_1 - v * g) + X_1 =? \alpha_1 * g + \alpha_3 * C_2
    <b>let</b> gv = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(amount);
    <b>let</b> big_c1_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_sub">ristretto255::point_sub</a>(big_c1, &gv);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul_assign">ristretto255::point_mul_assign</a>(&<b>mut</b> big_c1_acc, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> big_c1_acc, &proof.x1);

    <b>let</b> big_c2_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(big_c2, &proof.alpha3);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> big_c2_acc, &g_alpha1);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&big_c1_acc, &big_c2_acc), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    // \rho * c + X_2 =? \alpha_1 * g + \alpha_2 * h
    <b>let</b> c_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(c, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> c_acc, &proof.x2);

    <b>let</b> h_alpha2_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(&h, &proof.alpha2);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> h_alpha2_acc, &g_alpha1);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&c_acc, &h_alpha2_acc), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));

    // \rho * Y + X_3 =? \alpha_3 * g
    <b>let</b> y_acc = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_mul">ristretto255::point_mul</a>(&sender_pk_point, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_add_assign">ristretto255::point_add_assign</a>(&<b>mut</b> y_acc, &proof.x3);

    <b>let</b> g_alpha3 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_mul">ristretto255::basepoint_mul</a>(&proof.alpha3);
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&y_acc, &g_alpha3), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protos.md#0x7_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>));
}
</code></pre>



</details>

<a id="0x7_sigma_protos_deserialize_withdrawal_subproof"></a>

## Function `deserialize_withdrawal_subproof`

Deserializes and returns an <code><a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a></code> given its byte representation.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_deserialize_withdrawal_subproof">deserialize_withdrawal_subproof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">sigma_protos::WithdrawalSubproof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_deserialize_withdrawal_subproof">deserialize_withdrawal_subproof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt; {
    <b>if</b> (proof_bytes.length() != 192) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt;()
    };

    <b>let</b> x1_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x1 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x1_bytes);
    <b>if</b> (!x1.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt;()
    };
    <b>let</b> x1 = x1.extract();

    <b>let</b> x2_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x2 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x2_bytes);
    <b>if</b> (!x2.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt;()
    };
    <b>let</b> x2 = x2.extract();

    <b>let</b> x3_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x3 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x3_bytes);
    <b>if</b> (!x3.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt;()
    };
    <b>let</b> x3 = x3.extract();

    <b>let</b> alpha1_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha1 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha1_bytes);
    <b>if</b> (!alpha1.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt;()
    };
    <b>let</b> alpha1 = alpha1.extract();

    <b>let</b> alpha2_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha2 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha2_bytes);
    <b>if</b> (!alpha2.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt;()
    };
    <b>let</b> alpha2 = alpha2.extract();

    <b>let</b> alpha3_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha3 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha3_bytes);
    <b>if</b> (!alpha3.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a>&gt;()
    };
    <b>let</b> alpha3 = alpha3.extract();

    std::option::some(<a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a> {
        x1, x2, x3, alpha1, alpha2, alpha3
    })
}
</code></pre>



</details>

<a id="0x7_sigma_protos_deserialize_transfer_subproof"></a>

## Function `deserialize_transfer_subproof`

Deserializes and returns a <code><a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a></code> given its byte representation.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_deserialize_transfer_subproof">deserialize_transfer_subproof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">sigma_protos::TransferSubproof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_deserialize_transfer_subproof">deserialize_transfer_subproof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt; {
    <b>if</b> (proof_bytes.length() != 384) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };

    <b>let</b> x1_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x1 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x1_bytes);
    <b>if</b> (!x1.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> x1 = x1.extract();

    <b>let</b> x2_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x2 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x2_bytes);
    <b>if</b> (!x2.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> x2 = x2.extract();

    <b>let</b> x3_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x3 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x3_bytes);
    <b>if</b> (!x3.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> x3 = x3.extract();

    <b>let</b> x4_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x4 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x4_bytes);
    <b>if</b> (!x4.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> x4 = x4.extract();

    <b>let</b> x5_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x5 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x5_bytes);
    <b>if</b> (!x5.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> x5 = x5.extract();

    <b>let</b> x6_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x6 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x6_bytes);
    <b>if</b> (!x6.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> x6 = x6.extract();

    <b>let</b> x7_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> x7 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_from_bytes">ristretto255::new_point_from_bytes</a>(x7_bytes);
    <b>if</b> (!x7.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> x7 = x7.extract();

    <b>let</b> alpha1_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha1 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha1_bytes);
    <b>if</b> (!alpha1.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> alpha1 = alpha1.extract();

    <b>let</b> alpha2_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha2 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha2_bytes);
    <b>if</b> (!alpha2.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> alpha2 = alpha2.extract();

    <b>let</b> alpha3_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha3 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha3_bytes);
    <b>if</b> (!alpha3.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> alpha3 = alpha3.extract();

    <b>let</b> alpha4_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha4 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha4_bytes);
    <b>if</b> (!alpha4.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> alpha4 = alpha4.extract();

    <b>let</b> alpha5_bytes = cut_vector&lt;u8&gt;(&<b>mut</b> proof_bytes, 32);
    <b>let</b> alpha5 = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(alpha5_bytes);
    <b>if</b> (!alpha5.is_some()) {
        <b>return</b> std::option::none&lt;<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a>&gt;()
    };
    <b>let</b> alpha5 = alpha5.extract();

    std::option::some(<a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a> {
        x1, x2, x3, x4, x5, x6, x7, alpha1, alpha2, alpha3, alpha4, alpha5
    })
}
</code></pre>



</details>

<a id="0x7_sigma_protos_fiat_shamir_withdrawal_subproof_challenge"></a>

## Function `fiat_shamir_withdrawal_subproof_challenge`

Computes a Fiat-Shamir challenge <code>rho = H(G, H, Y, C_1, C_2, c, x_1, x_2, x_3)</code> for the <code><a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a></code>
$\Sigma$-protocol.


<pre><code><b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_fiat_shamir_withdrawal_subproof_challenge">fiat_shamir_withdrawal_subproof_challenge</a>(sender_pk: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, sender_curr_balance_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, sender_new_balance_comm: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, amount: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, x1: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x2: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x3: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_fiat_shamir_withdrawal_subproof_challenge">fiat_shamir_withdrawal_subproof_challenge</a>(
    sender_pk: &elgamal::CompressedPubkey,
    sender_curr_balance_ct: &elgamal::Ciphertext,
    sender_new_balance_comm: &pedersen::Commitment,
    amount: &Scalar,
    x1: &RistrettoPoint,
    x2: &RistrettoPoint,
    x3: &RistrettoPoint): Scalar
{
    <b>let</b> (c1, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
    <b>let</b> c = pedersen::commitment_as_point(sender_new_balance_comm);
    <b>let</b> y = elgamal::pubkey_to_compressed_point(sender_pk);

    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();

    bytes.append(<a href="sigma_protos.md#0x7_sigma_protos_FIAT_SHAMIR_SIGMA_DST">FIAT_SHAMIR_SIGMA_DST</a>);
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>()));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(
        &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&pedersen::randomness_base_for_bulletproof())));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&y));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(c1)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(c2)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(c)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_to_bytes">ristretto255::scalar_to_bytes</a>(amount));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x1)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x2)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x3)));

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(bytes)
}
</code></pre>



</details>

<a id="0x7_sigma_protos_fiat_shamir_transfer_subproof_challenge"></a>

## Function `fiat_shamir_transfer_subproof_challenge`

Computes a Fiat-Shamir challenge <code>rho = H(G, H, Y, Y', C, D, c, c_1, c_2, \bar{c}, {X_i}_{i=1}^7)</code> for the
<code><a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">TransferSubproof</a></code> $\Sigma$-protocol.


<pre><code><b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_fiat_shamir_transfer_subproof_challenge">fiat_shamir_transfer_subproof_challenge</a>(sender_pk: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, recipient_pk: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, withdraw_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, deposit_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, comm_amount: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, sender_curr_balance_ct: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, sender_new_balance_comm: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, x1: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x2: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x3: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x4: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x5: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x6: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, x7: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sigma_protos.md#0x7_sigma_protos_fiat_shamir_transfer_subproof_challenge">fiat_shamir_transfer_subproof_challenge</a>(
    sender_pk: &elgamal::CompressedPubkey,
    recipient_pk: &elgamal::CompressedPubkey,
    withdraw_ct: &elgamal::Ciphertext,
    deposit_ct: &elgamal::Ciphertext,
    comm_amount: &pedersen::Commitment,
    sender_curr_balance_ct: &elgamal::Ciphertext,
    sender_new_balance_comm: &pedersen::Commitment,
    x1: &RistrettoPoint,
    x2: &RistrettoPoint,
    x3: &RistrettoPoint,
    x4: &RistrettoPoint,
    x5: &RistrettoPoint,
    x6: &RistrettoPoint,
    x7: &RistrettoPoint): Scalar
{
    <b>let</b> y = elgamal::pubkey_to_compressed_point(sender_pk);
    <b>let</b> y_prime = elgamal::pubkey_to_compressed_point(recipient_pk);
    <b>let</b> (big_c, big_d) = elgamal::ciphertext_as_points(withdraw_ct);
    <b>let</b> (big_c_prime, _) = elgamal::ciphertext_as_points(deposit_ct);
    <b>let</b> c = pedersen::commitment_as_point(comm_amount);
    <b>let</b> (c1, c2) = elgamal::ciphertext_as_points(sender_curr_balance_ct);
    <b>let</b> bar_c = pedersen::commitment_as_point(sender_new_balance_comm);

    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();

    bytes.append(<a href="sigma_protos.md#0x7_sigma_protos_FIAT_SHAMIR_SIGMA_DST">FIAT_SHAMIR_SIGMA_DST</a>);
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>()));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(
        &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&pedersen::randomness_base_for_bulletproof())));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&y));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&y_prime));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(big_c)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(big_c_prime)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(big_d)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(c)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(c1)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(c2)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(bar_c)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x1)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x2)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x3)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x4)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x5)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x6)));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(x7)));

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(bytes)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
