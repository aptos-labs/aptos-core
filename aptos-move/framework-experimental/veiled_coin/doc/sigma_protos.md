
<a name="0x1337_sigma_protos"></a>

# Module `0x1337::sigma_protos`

Package for creating, verifying, serializing & deserializing the $\Sigma$-protocol proofs used in veiled coins.


<a name="@Preliminaries_0"></a>

### Preliminaries


Recall that a $\Sigma$-protocol proof argues knowledge of a *secret* witness $w$ such that an arithmetic relation
$R(x; w) = 1$ is satisfied over group and field elements stored in $x$ and $w$.

Here, $x$ is a public statement known to the verifier (i.e., known to the validators). Importantly, the
$\Sigma$-protocol's zero-knowledge property ensures the witness $w$ remains secret.


<a name="@WithdrawalSubproof:_ElGamal-Pedersen_equality_1"></a>

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


<a name="@TransferSubproof:_ElGamal-Pedersen_equality_and_ElGamal-ElGamal_equality_2"></a>

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
-  [Struct `WithdrawalSubproof`](#0x1337_sigma_protos_WithdrawalSubproof)
-  [Struct `TransferSubproof`](#0x1337_sigma_protos_TransferSubproof)
-  [Constants](#@Constants_3)
-  [Function `verify_transfer_subproof`](#0x1337_sigma_protos_verify_transfer_subproof)
-  [Function `verify_withdrawal_subproof`](#0x1337_sigma_protos_verify_withdrawal_subproof)
-  [Function `deserialize_withdrawal_subproof`](#0x1337_sigma_protos_deserialize_withdrawal_subproof)
-  [Function `deserialize_transfer_subproof`](#0x1337_sigma_protos_deserialize_transfer_subproof)


<pre><code><b>use</b> <a href="helpers.md#0x1337_helpers">0x1337::helpers</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="">0x1::ristretto255_elgamal</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen">0x1::ristretto255_pedersen</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1337_sigma_protos_WithdrawalSubproof"></a>

## Struct `WithdrawalSubproof`

A $\Sigma$-protocol used during an unveiled withdrawal (for proving the correct ElGamal encryption of a
Pedersen-committed balance).


<pre><code><b>struct</b> <a href="sigma_protos.md#0x1337_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a> <b>has</b> drop
</code></pre>



<a name="0x1337_sigma_protos_TransferSubproof"></a>

## Struct `TransferSubproof`

A $\Sigma$-protocol proof used during a veiled transfer. This proof encompasses the $\Sigma$-protocol from
<code><a href="sigma_protos.md#0x1337_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a></code>.


<pre><code><b>struct</b> <a href="sigma_protos.md#0x1337_sigma_protos_TransferSubproof">TransferSubproof</a> <b>has</b> drop
</code></pre>



<a name="@Constants_3"></a>

## Constants


<a name="0x1337_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED"></a>

The $\Sigma$-protocol proof for withdrawals did not verify.


<pre><code><b>const</b> <a href="sigma_protos.md#0x1337_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>: u64 = 1;
</code></pre>



<a name="0x1337_sigma_protos_FIAT_SHAMIR_SIGMA_DST"></a>

The domain separation tag (DST) used in the Fiat-Shamir transform of our $\Sigma$-protocol.


<pre><code><b>const</b> <a href="sigma_protos.md#0x1337_sigma_protos_FIAT_SHAMIR_SIGMA_DST">FIAT_SHAMIR_SIGMA_DST</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 86, 101, 105, 108, 101, 100, 67, 111, 105, 110, 47, 87, 105, 116, 104, 100, 114, 97, 119, 97, 108, 83, 117, 98, 112, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a name="0x1337_sigma_protos_verify_transfer_subproof"></a>

## Function `verify_transfer_subproof`

Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled transfer.

Specifically, the proof argues that the same amount $v$ is Pedersen-committed in <code>comm_amount</code> and ElGamal-
encrypted in <code>withdraw_ct</code> (under <code>sender_pk</code>) and in <code>deposit_ct</code> (under <code>recipient_pk</code>), all three using the
same randomness $r$.

In addition, it argues that the sender's new balance $b$ committed to by sender_new_balance_comm is the same
as the value encrypted by the ciphertext obtained by subtracting withdraw_ct from sender_curr_balance_ct


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_verify_transfer_subproof">verify_transfer_subproof</a>(sender_pk: &<a href="_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, recipient_pk: &<a href="_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, withdraw_ct: &<a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>, deposit_ct: &<a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>, comm_amount: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, sender_new_balance_comm: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, sender_curr_balance_ct: &<a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>, proof: &<a href="sigma_protos.md#0x1337_sigma_protos_TransferSubproof">sigma_protos::TransferSubproof</a>)
</code></pre>



<a name="0x1337_sigma_protos_verify_withdrawal_subproof"></a>

## Function `verify_withdrawal_subproof`

Verifies the $\Sigma$-protocol proof necessary to ensure correctness of a veiled-to-unveiled transfer.

Specifically, the proof argues that the same amount $v$ is Pedersen-committed in <code>sender_new_balance_comm</code> and
ElGamal-encrypted in the ciphertext obtained by subtracting the ciphertext (vG, 0G) from sender_curr_balance_ct


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_verify_withdrawal_subproof">verify_withdrawal_subproof</a>(sender_pk: &<a href="_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>, sender_curr_balance_ct: &<a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>, sender_new_balance_comm: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, amount: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, proof: &<a href="sigma_protos.md#0x1337_sigma_protos_WithdrawalSubproof">sigma_protos::WithdrawalSubproof</a>)
</code></pre>



<a name="0x1337_sigma_protos_deserialize_withdrawal_subproof"></a>

## Function `deserialize_withdrawal_subproof`

Deserializes and returns an <code><a href="sigma_protos.md#0x1337_sigma_protos_WithdrawalSubproof">WithdrawalSubproof</a></code> given its byte representation.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_deserialize_withdrawal_subproof">deserialize_withdrawal_subproof</a>(proof_bytes: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="sigma_protos.md#0x1337_sigma_protos_WithdrawalSubproof">sigma_protos::WithdrawalSubproof</a>&gt;
</code></pre>



<a name="0x1337_sigma_protos_deserialize_transfer_subproof"></a>

## Function `deserialize_transfer_subproof`

Deserializes and returns a <code><a href="sigma_protos.md#0x1337_sigma_protos_TransferSubproof">TransferSubproof</a></code> given its byte representation.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_deserialize_transfer_subproof">deserialize_transfer_subproof</a>(proof_bytes: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="sigma_protos.md#0x1337_sigma_protos_TransferSubproof">sigma_protos::TransferSubproof</a>&gt;
</code></pre>
