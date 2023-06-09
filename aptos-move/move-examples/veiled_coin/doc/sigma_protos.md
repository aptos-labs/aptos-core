
<a name="0x1337_sigma_protos"></a>

# Module `0x1337::sigma_protos`

Package for creating, verifying, serializing & deserializing the $\Sigma$-protocol proofs used in veiled coins.


<a name="@Preliminaries_0"></a>

## Preliminaries


Recall that a $\Sigma$-protocol proof argues knowledge of a *secret* witness $w$ such that an arithmetic relation
$R(x; w) = 1$ is satisfied over group and field elements stored in $x$ and $w$.

Here, $x$ is a public statement known to the verifier (i.e., known to the validators). Importantly, the
$\Sigma$-protocol's zero-knowledge property ensures the witness $w$ remains secret.


<a name="@The_"full"_sigma_protocol_for_a_veiled_transfer_1"></a>

## The "full" sigma protocol for a veiled transfer


This protocol argues two things. First, that the same amount is ElGamal-encrypted for both the sender and recipient.
This is needed to correctly withdraw & deposit the same amount during a transfer. Second, that this same amount is
committed via Pedersen. Third, that a Pedersen-committed balance is correctly ElGamal encrypted. These last two
Pedersen commitments are needed to prevent overflowing attacks on the balance.

The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
- $v$, amount being transferred
- $r$, randomness used to ElGamal-encrypt $v$
- $b$, sender's new balance after the transfer occurs
- $r_b$, randomness used to ElGamal-encrypt $b$

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
+ $(c_1, c_2)$, ElGamal encryption of $b$, under the sender's PK, using randomness $r_b$
+ $c'$, Pedersen commitment to $b$ using randomness $r_b$

The relation being proved is:
```
R(
x = [ Y, Y', (C, C', D), c, (c_1, c_2), c', G, H]
w = [ v, r, b, r_b ]
) = {
C  = v G + r Y
C' = v G + r Y'
D = r G
c_1 =   b G + r_b Y
c_2 = r_b G
c  = b G + r_b H
c' = v G +   r H
}
```

A relation similar to this is also described on page 14 of the Zether paper [BAZB20] (just replace  $G$ -> $g$,
$C'$ -> $\bar{C}$, $Y$ -> $y$, $Y'$ -> $\bar{y}$, $v$ -> $b^*$). Note that their relation does not include the
ElGamal-to-Pedersen conversion parts, as they can do ZK range proofs directly over ElGamal ciphertexts using their
$\Sigma$-bullets modification of Bulletproofs.

Note also that the equations $C_L - C = b' G + sk (C_R - D)$ and $Y = sk G$ in the Zether paper are enforced
programmatically by this smart contract and so are not needed in our $\Sigma$-protocol.


<a name="@_2"></a>

##



-  [Preliminaries](#@Preliminaries_0)
-  [The "full" sigma protocol for a veiled transfer](#@The_"full"_sigma_protocol_for_a_veiled_transfer_1)
-  [](#@_2)
-  [Struct `ElGamalPedEqProof`](#0x1337_sigma_protos_ElGamalPedEqProof)
-  [Struct `FullSigmaProof`](#0x1337_sigma_protos_FullSigmaProof)
-  [Constants](#@Constants_3)
-  [Function `verify_full_sigma_proof`](#0x1337_sigma_protos_verify_full_sigma_proof)
-  [Function `verify_elgamalpedeq_proof`](#0x1337_sigma_protos_verify_elgamalpedeq_proof)
    -  [Cryptographic details](#@Cryptographic_details_4)
-  [Function `deserialize_elgamalpedeq_proof`](#0x1337_sigma_protos_deserialize_elgamalpedeq_proof)
-  [Function `deserialize_full_sigma_proof`](#0x1337_sigma_protos_deserialize_full_sigma_proof)


<pre><code><b>use</b> <a href="helpers.md#0x1337_helpers">0x1337::helpers</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal">0x1::elgamal</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/pedersen.md#0x1_pedersen">0x1::pedersen</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1337_sigma_protos_ElGamalPedEqProof"></a>

## Struct `ElGamalPedEqProof`

A $\Sigma$-protocol for proving the correct ElGamal encryption of a Pedersen-committed balance.


<pre><code><b>struct</b> <a href="sigma_protos.md#0x1337_sigma_protos_ElGamalPedEqProof">ElGamalPedEqProof</a>&lt;CoinType&gt; <b>has</b> drop
</code></pre>



<a name="0x1337_sigma_protos_FullSigmaProof"></a>

## Struct `FullSigmaProof`

A $\Sigma$-protocol proof used as part of a <code>VeiledTransferProof</code>.
This proof encompasses the $\Sigma$-protocol from <code><a href="sigma_protos.md#0x1337_sigma_protos_ElGamalPedEqProof">ElGamalPedEqProof</a></code>.
(A more detailed description can be found in <code>verify_withdrawal_sigma_protocol</code>.)
TODO: rename more clearly


<pre><code><b>struct</b> <a href="sigma_protos.md#0x1337_sigma_protos_FullSigmaProof">FullSigmaProof</a>&lt;CoinType&gt; <b>has</b> drop
</code></pre>



<a name="@Constants_3"></a>

## Constants


<a name="0x1337_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED"></a>

The $\Sigma$-protocol proof for withdrawals did not verify.


<pre><code><b>const</b> <a href="sigma_protos.md#0x1337_sigma_protos_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>: u64 = 1;
</code></pre>



<a name="0x1337_sigma_protos_FIAT_SHAMIR_SIGMA_DST"></a>

The domain separation tag (DST) used in the Fiat-Shamir transform of our $\Sigma$-protocol.


<pre><code><b>const</b> <a href="sigma_protos.md#0x1337_sigma_protos_FIAT_SHAMIR_SIGMA_DST">FIAT_SHAMIR_SIGMA_DST</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 86, 101, 105, 108, 101, 100, 67, 111, 105, 110, 47, 87, 105, 116, 104, 100, 114, 97, 119, 97, 108, 80, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a name="0x1337_sigma_protos_verify_full_sigma_proof"></a>

## Function `verify_full_sigma_proof`

Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled transfer.

Specifically, the proof argues that the same amount $v$ is Pedersen-committed in <code>transfer_value</code> and ElGamal-
encrypted in <code>withdraw_ct</code> (under <code>sender_pk</code>) and in <code>deposit_ct</code> (under <code>recipient_pk</code>), all three using the
same randomness $r$.

In addition, it argues that the same balance $b$ is ElGamal-encrypted in <code>sender_updated_balance_ct</code> under
<code>sender_pk</code> and Pedersen-committed in <code>sender_updated_balance_comm</code>, both with the same randomness $r_b$.

The Pedersen commitments are used as a simple mechanism to convert ElGamal ciphertexts to a
computationally-binding commitment, over which a ZK range proof can be securely argued. Otherwise, knowledge of
the ElGamal SK breaks the binding of the ElGamal commitment making any ZK range proof over it useless.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_verify_full_sigma_proof">verify_full_sigma_proof</a>&lt;CoinType&gt;(sender_pk: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_CompressedPubkey">elgamal::CompressedPubkey</a>, recipient_pk: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_CompressedPubkey">elgamal::CompressedPubkey</a>, withdraw_ct: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, deposit_ct: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, sender_updated_balance_ct: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, sender_updated_balance_comm: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, transfer_value: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, proof: &<a href="sigma_protos.md#0x1337_sigma_protos_FullSigmaProof">sigma_protos::FullSigmaProof</a>&lt;CoinType&gt;)
</code></pre>



<a name="0x1337_sigma_protos_verify_elgamalpedeq_proof"></a>

## Function `verify_elgamalpedeq_proof`

Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled-to-unveiled transfer.
Specifically, this proof proves that <code>sender_updated_balance_ct</code> and <code>sender_updated_balance_comm</code> encode the
same amount $v$ using the same randomness $r$, with <code>sender_pk</code> being used in <code>sender_updated_balance_ct</code>.

This is necessary to prevent the forgery of range proofs, as computing a range proof over the left half of an
ElGamal ciphertext allows a user with their secret key to create range proofs over false values.


<a name="@Cryptographic_details_4"></a>

### Cryptographic details


The proof argues knowledge of a witness $w$ such that a specific relation $R(x; w)$ is satisfied, for a public
statement $x$ known to the verifier (i.e., known to the validators). We describe this relation below.

The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
- $b$, the new veiled balance of the sender after their transaction goes through
- $r$, ElGamal encryption randomness of the sender's new balance

(Note that the $\Sigma$-protocol's zero-knowledge property ensures the witness is not revealed.)

The public statement $x$ in this relation consists of:
- $G$, the basepoint of a given elliptic curve
- $Y$, the sender's PK
- $(c1, c2)$, the ElGamal ecnryption of the sender's updated balance $b$ with updated randomness $r$ after
their transaction is sent
- $c$, the Pedersen commitment to $b$ with randomness $r$, using fixed randomness base $H$

The relation being proved is as follows:

```
R(
x = [ Y, (c1, c2), c, G, H]
w = [ b, r ]
) = {
c1 = r * G
c2 = b * G + r * Y
c = b * G + r * H
}
```


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_verify_elgamalpedeq_proof">verify_elgamalpedeq_proof</a>&lt;CoinType&gt;(sender_pk: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_CompressedPubkey">elgamal::CompressedPubkey</a>, sender_updated_balance_ct: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, sender_updated_balance_comm: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/pedersen.md#0x1_pedersen_Commitment">pedersen::Commitment</a>, proof: &<a href="sigma_protos.md#0x1337_sigma_protos_ElGamalPedEqProof">sigma_protos::ElGamalPedEqProof</a>&lt;CoinType&gt;)
</code></pre>



<a name="0x1337_sigma_protos_deserialize_elgamalpedeq_proof"></a>

## Function `deserialize_elgamalpedeq_proof`

Deserializes and returns an <code><a href="sigma_protos.md#0x1337_sigma_protos_ElGamalPedEqProof">ElGamalPedEqProof</a></code> given its byte representation (see protocol description in
<code>verify_elgamalpedeq_proof</code>)

Elements at the end of the <code><a href="sigma_protos.md#0x1337_sigma_protos_ElGamalPedEqProof">ElGamalPedEqProof</a></code> struct are expected to be at the start of the byte vector, and
serialized using the serialization formats in the <code><a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">ristretto255</a></code> module.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_deserialize_elgamalpedeq_proof">deserialize_elgamalpedeq_proof</a>&lt;CoinType&gt;(proof_bytes: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="sigma_protos.md#0x1337_sigma_protos_ElGamalPedEqProof">sigma_protos::ElGamalPedEqProof</a>&lt;CoinType&gt;&gt;
</code></pre>



<a name="0x1337_sigma_protos_deserialize_full_sigma_proof"></a>

## Function `deserialize_full_sigma_proof`

Deserializes and returns a <code><a href="sigma_protos.md#0x1337_sigma_protos_FullSigmaProof">FullSigmaProof</a></code> given its byte representation (see protocol description in
<code>verify_full_sigma_protocol</code>)
TODO: update all other occurences

Elements at the end of the <code><a href="sigma_protos.md#0x1337_sigma_protos_FullSigmaProof">FullSigmaProof</a></code> struct are expected to be at the start  of the byte vector, and
serialized using the serialization formats in the <code><a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">ristretto255</a></code> module.


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protos.md#0x1337_sigma_protos_deserialize_full_sigma_proof">deserialize_full_sigma_proof</a>&lt;CoinType&gt;(proof_bytes: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="sigma_protos.md#0x1337_sigma_protos_FullSigmaProof">sigma_protos::FullSigmaProof</a>&lt;CoinType&gt;&gt;
</code></pre>
