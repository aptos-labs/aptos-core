
<a id="0x7_confidential_proof"></a>

# Module `0x7::confidential_proof`

The <code><a href="confidential_proof.md#0x7_confidential_proof">confidential_proof</a></code> module provides the infrastructure for verifying zero-knowledge proofs used in the Confidential Asset protocol.
These proofs ensure correctness for operations such as <code>confidential_transfer</code>, <code>withdraw</code>, <code>rotate_encryption_key</code>, and <code>normalize</code>.


-  [Struct `WithdrawalProof`](#0x7_confidential_proof_WithdrawalProof)
-  [Struct `TransferProof`](#0x7_confidential_proof_TransferProof)
-  [Struct `NormalizationProof`](#0x7_confidential_proof_NormalizationProof)
-  [Struct `RotationProof`](#0x7_confidential_proof_RotationProof)
-  [Struct `WithdrawalSigmaProofXs`](#0x7_confidential_proof_WithdrawalSigmaProofXs)
-  [Struct `WithdrawalSigmaProofAlphas`](#0x7_confidential_proof_WithdrawalSigmaProofAlphas)
-  [Struct `WithdrawalSigmaProofGammas`](#0x7_confidential_proof_WithdrawalSigmaProofGammas)
-  [Struct `WithdrawalSigmaProof`](#0x7_confidential_proof_WithdrawalSigmaProof)
-  [Struct `TransferSigmaProofXs`](#0x7_confidential_proof_TransferSigmaProofXs)
-  [Struct `TransferSigmaProofAlphas`](#0x7_confidential_proof_TransferSigmaProofAlphas)
-  [Struct `TransferSigmaProofGammas`](#0x7_confidential_proof_TransferSigmaProofGammas)
-  [Struct `TransferSigmaProof`](#0x7_confidential_proof_TransferSigmaProof)
-  [Struct `NormalizationSigmaProofXs`](#0x7_confidential_proof_NormalizationSigmaProofXs)
-  [Struct `NormalizationSigmaProofAlphas`](#0x7_confidential_proof_NormalizationSigmaProofAlphas)
-  [Struct `NormalizationSigmaProofGammas`](#0x7_confidential_proof_NormalizationSigmaProofGammas)
-  [Struct `NormalizationSigmaProof`](#0x7_confidential_proof_NormalizationSigmaProof)
-  [Struct `RotationSigmaProofXs`](#0x7_confidential_proof_RotationSigmaProofXs)
-  [Struct `RotationSigmaProofAlphas`](#0x7_confidential_proof_RotationSigmaProofAlphas)
-  [Struct `RotationSigmaProofGammas`](#0x7_confidential_proof_RotationSigmaProofGammas)
-  [Struct `RotationSigmaProof`](#0x7_confidential_proof_RotationSigmaProof)
-  [Constants](#@Constants_0)
-  [Function `verify_withdrawal_proof`](#0x7_confidential_proof_verify_withdrawal_proof)
-  [Function `verify_transfer_proof`](#0x7_confidential_proof_verify_transfer_proof)
-  [Function `verify_normalization_proof`](#0x7_confidential_proof_verify_normalization_proof)
-  [Function `verify_rotation_proof`](#0x7_confidential_proof_verify_rotation_proof)
-  [Function `verify_withdrawal_sigma_proof`](#0x7_confidential_proof_verify_withdrawal_sigma_proof)
-  [Function `verify_transfer_sigma_proof`](#0x7_confidential_proof_verify_transfer_sigma_proof)
-  [Function `verify_normalization_sigma_proof`](#0x7_confidential_proof_verify_normalization_sigma_proof)
-  [Function `verify_rotation_sigma_proof`](#0x7_confidential_proof_verify_rotation_sigma_proof)
-  [Function `verify_new_balance_range_proof`](#0x7_confidential_proof_verify_new_balance_range_proof)
-  [Function `verify_transfer_amount_range_proof`](#0x7_confidential_proof_verify_transfer_amount_range_proof)
-  [Function `auditors_count_in_transfer_proof`](#0x7_confidential_proof_auditors_count_in_transfer_proof)
-  [Function `deserialize_withdrawal_proof`](#0x7_confidential_proof_deserialize_withdrawal_proof)
-  [Function `deserialize_transfer_proof`](#0x7_confidential_proof_deserialize_transfer_proof)
-  [Function `deserialize_normalization_proof`](#0x7_confidential_proof_deserialize_normalization_proof)
-  [Function `deserialize_rotation_proof`](#0x7_confidential_proof_deserialize_rotation_proof)
-  [Function `deserialize_withdrawal_sigma_proof`](#0x7_confidential_proof_deserialize_withdrawal_sigma_proof)
-  [Function `deserialize_transfer_sigma_proof`](#0x7_confidential_proof_deserialize_transfer_sigma_proof)
-  [Function `deserialize_normalization_sigma_proof`](#0x7_confidential_proof_deserialize_normalization_sigma_proof)
-  [Function `deserialize_rotation_sigma_proof`](#0x7_confidential_proof_deserialize_rotation_sigma_proof)
-  [Function `get_fiat_shamir_withdrawal_sigma_dst`](#0x7_confidential_proof_get_fiat_shamir_withdrawal_sigma_dst)
-  [Function `get_fiat_shamir_transfer_sigma_dst`](#0x7_confidential_proof_get_fiat_shamir_transfer_sigma_dst)
-  [Function `get_fiat_shamir_normalization_sigma_dst`](#0x7_confidential_proof_get_fiat_shamir_normalization_sigma_dst)
-  [Function `get_fiat_shamir_rotation_sigma_dst`](#0x7_confidential_proof_get_fiat_shamir_rotation_sigma_dst)
-  [Function `get_bulletproofs_dst`](#0x7_confidential_proof_get_bulletproofs_dst)
-  [Function `get_bulletproofs_num_bits`](#0x7_confidential_proof_get_bulletproofs_num_bits)
-  [Function `fiat_shamir_withdrawal_sigma_proof_challenge`](#0x7_confidential_proof_fiat_shamir_withdrawal_sigma_proof_challenge)
-  [Function `fiat_shamir_transfer_sigma_proof_challenge`](#0x7_confidential_proof_fiat_shamir_transfer_sigma_proof_challenge)
-  [Function `fiat_shamir_normalization_sigma_proof_challenge`](#0x7_confidential_proof_fiat_shamir_normalization_sigma_proof_challenge)
-  [Function `fiat_shamir_rotation_sigma_proof_challenge`](#0x7_confidential_proof_fiat_shamir_rotation_sigma_proof_challenge)
-  [Function `msm_withdrawal_gammas`](#0x7_confidential_proof_msm_withdrawal_gammas)
-  [Function `msm_transfer_gammas`](#0x7_confidential_proof_msm_transfer_gammas)
-  [Function `msm_normalization_gammas`](#0x7_confidential_proof_msm_normalization_gammas)
-  [Function `msm_rotation_gammas`](#0x7_confidential_proof_msm_rotation_gammas)
-  [Function `msm_gamma_1`](#0x7_confidential_proof_msm_gamma_1)
-  [Function `msm_gamma_2`](#0x7_confidential_proof_msm_gamma_2)
-  [Function `scalar_mul_3`](#0x7_confidential_proof_scalar_mul_3)
-  [Function `scalar_linear_combination`](#0x7_confidential_proof_scalar_linear_combination)
-  [Function `new_scalar_from_pow2`](#0x7_confidential_proof_new_scalar_from_pow2)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs">0x1::ristretto255_bulletproofs</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="confidential_balance.md#0x7_confidential_balance">0x7::confidential_balance</a>;
<b>use</b> <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal">0x7::ristretto255_twisted_elgamal</a>;
</code></pre>



<a id="0x7_confidential_proof_WithdrawalProof"></a>

## Struct `WithdrawalProof`

Represents the proof structure for validating a withdrawal operation.


<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma_proof: <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">confidential_proof::WithdrawalSigmaProof</a></code>
</dt>
<dd>
 Sigma proof ensuring that the withdrawal operation maintains balance integrity.
</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
</dd>
</dl>


</details>

<a id="0x7_confidential_proof_TransferProof"></a>

## Struct `TransferProof`

Represents the proof structure for validating a transfer operation.


<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma_proof: <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">confidential_proof::TransferSigmaProof</a></code>
</dt>
<dd>
 Sigma proof ensuring that the transfer operation maintains balance integrity and correctness.
</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the resulting balance chunks for the sender are normalized (i.e., within the 16-bit limit).
</dd>
<dt>
<code>zkrp_transfer_amount: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the transferred amount chunks are normalized (i.e., within the 16-bit limit).
</dd>
</dl>


</details>

<a id="0x7_confidential_proof_NormalizationProof"></a>

## Struct `NormalizationProof`

Represents the proof structure for validating a normalization operation.


<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">NormalizationProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma_proof: <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">confidential_proof::NormalizationSigmaProof</a></code>
</dt>
<dd>
 Sigma proof ensuring that the normalization operation maintains balance integrity.
</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
</dd>
</dl>


</details>

<a id="0x7_confidential_proof_RotationProof"></a>

## Struct `RotationProof`

Represents the proof structure for validating a key rotation operation.


<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_RotationProof">RotationProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma_proof: <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">confidential_proof::RotationSigmaProof</a></code>
</dt>
<dd>
 Sigma proof ensuring that the key rotation operation preserves balance integrity.
</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the resulting balance chunks after key rotation are normalized (i.e., within the 16-bit limit).
</dd>
</dl>


</details>

<a id="0x7_confidential_proof_WithdrawalSigmaProofXs"></a>

## Struct `WithdrawalSigmaProofXs`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofXs">WithdrawalSigmaProofXs</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>x4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_WithdrawalSigmaProofAlphas"></a>

## Struct `WithdrawalSigmaProofAlphas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofAlphas">WithdrawalSigmaProofAlphas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>a1s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>a2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_WithdrawalSigmaProofGammas"></a>

## Struct `WithdrawalSigmaProofGammas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofGammas">WithdrawalSigmaProofGammas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_WithdrawalSigmaProof"></a>

## Struct `WithdrawalSigmaProof`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alphas: <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofAlphas">confidential_proof::WithdrawalSigmaProofAlphas</a></code>
</dt>
<dd>

</dd>
<dt>
<code>xs: <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofXs">confidential_proof::WithdrawalSigmaProofXs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_TransferSigmaProofXs"></a>

## Struct `TransferSigmaProofXs`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofXs">TransferSigmaProofXs</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x2s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>x3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>x4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>x5: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x6s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>x7s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_TransferSigmaProofAlphas"></a>

## Struct `TransferSigmaProofAlphas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofAlphas">TransferSigmaProofAlphas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>a1s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>a2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>a4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>a5: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_TransferSigmaProofGammas"></a>

## Struct `TransferSigmaProofGammas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofGammas">TransferSigmaProofGammas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g2s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>g3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>g5: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g6s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>g7s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_TransferSigmaProof"></a>

## Struct `TransferSigmaProof`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alphas: <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofAlphas">confidential_proof::TransferSigmaProofAlphas</a></code>
</dt>
<dd>

</dd>
<dt>
<code>xs: <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofXs">confidential_proof::TransferSigmaProofXs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_NormalizationSigmaProofXs"></a>

## Struct `NormalizationSigmaProofXs`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofXs">NormalizationSigmaProofXs</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>x4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_NormalizationSigmaProofAlphas"></a>

## Struct `NormalizationSigmaProofAlphas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofAlphas">NormalizationSigmaProofAlphas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>a1s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>a2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_NormalizationSigmaProofGammas"></a>

## Struct `NormalizationSigmaProofGammas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofGammas">NormalizationSigmaProofGammas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_NormalizationSigmaProof"></a>

## Struct `NormalizationSigmaProof`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alphas: <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofAlphas">confidential_proof::NormalizationSigmaProofAlphas</a></code>
</dt>
<dd>

</dd>
<dt>
<code>xs: <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofXs">confidential_proof::NormalizationSigmaProofXs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_RotationSigmaProofXs"></a>

## Struct `RotationSigmaProofXs`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofXs">RotationSigmaProofXs</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>x4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>x5s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_RotationSigmaProofAlphas"></a>

## Struct `RotationSigmaProofAlphas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofAlphas">RotationSigmaProofAlphas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>a1s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>a2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a4: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>a5s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_RotationSigmaProofGammas"></a>

## Struct `RotationSigmaProofGammas`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofGammas">RotationSigmaProofGammas</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a></code>
</dt>
<dd>

</dd>
<dt>
<code>g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>g5s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_proof_RotationSigmaProof"></a>

## Struct `RotationSigmaProof`



<pre><code><b>struct</b> <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alphas: <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofAlphas">confidential_proof::RotationSigmaProofAlphas</a></code>
</dt>
<dd>

</dd>
<dt>
<code>xs: <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofXs">confidential_proof::RotationSigmaProofXs</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_proof_BULLETPROOFS_DST"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_DST">BULLETPROOFS_DST</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 66, 117, 108, 108, 101, 116, 112, 114, 111, 111, 102, 82, 97, 110, 103, 101, 80, 114, 111, 111, 102];
</code></pre>



<a id="0x7_confidential_proof_BULLETPROOFS_NUM_BITS"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_NUM_BITS">BULLETPROOFS_NUM_BITS</a>: u64 = 16;
</code></pre>



<a id="0x7_confidential_proof_ERANGE_PROOF_VERIFICATION_FAILED"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>: u64 = 2;
</code></pre>



<a id="0x7_confidential_proof_ESIGMA_PROTOCOL_VERIFY_FAILED"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>: u64 = 1;
</code></pre>



<a id="0x7_confidential_proof_FIAT_SHAMIR_NORMALIZATION_SIGMA_DST"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_NORMALIZATION_SIGMA_DST">FIAT_SHAMIR_NORMALIZATION_SIGMA_DST</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 78, 111, 114, 109, 97, 108, 105, 122, 97, 116, 105, 111, 110, 80, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a id="0x7_confidential_proof_FIAT_SHAMIR_ROTATION_SIGMA_DST"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_ROTATION_SIGMA_DST">FIAT_SHAMIR_ROTATION_SIGMA_DST</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 82, 111, 116, 97, 116, 105, 111, 110, 80, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a id="0x7_confidential_proof_FIAT_SHAMIR_TRANSFER_SIGMA_DST"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_TRANSFER_SIGMA_DST">FIAT_SHAMIR_TRANSFER_SIGMA_DST</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 84, 114, 97, 110, 115, 102, 101, 114, 80, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a id="0x7_confidential_proof_FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST"></a>



<pre><code><b>const</b> <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST">FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 67, 111, 110, 102, 105, 100, 101, 110, 116, 105, 97, 108, 65, 115, 115, 101, 116, 47, 87, 105, 116, 104, 100, 114, 97, 119, 97, 108, 80, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a id="0x7_confidential_proof_verify_withdrawal_proof"></a>

## Function `verify_withdrawal_proof`

Verifies the validity of the <code>withdraw</code> operation.

This function ensures that the provided proof (<code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a></code>) meets the following conditions:
1. The current balance (<code>current_balance</code>) and new balance (<code>new_balance</code>) encrypt the corresponding values
under the same encryption key (<code>ek</code>) before and after the withdrawal of the specified amount (<code>amount</code>), respectively.
2. The relationship <code>new_balance = current_balance - amount</code> holds, verifying that the withdrawal amount is deducted correctly.
3. The new balance (<code>new_balance</code>) is normalized, with each chunk adhering to the range [0, 2^16).

If all conditions are satisfied, the proof validates the withdrawal; otherwise, the function causes an error.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_withdrawal_proof">verify_withdrawal_proof</a>(ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, amount: u64, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: &<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">confidential_proof::WithdrawalProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_withdrawal_proof">verify_withdrawal_proof</a>(
    ek: &twisted_elgamal::CompressedPubkey,
    amount: u64,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a>)
{
    <a href="confidential_proof.md#0x7_confidential_proof_verify_withdrawal_sigma_proof">verify_withdrawal_sigma_proof</a>(ek, amount, current_balance, new_balance, &proof.sigma_proof);
    <a href="confidential_proof.md#0x7_confidential_proof_verify_new_balance_range_proof">verify_new_balance_range_proof</a>(new_balance, &proof.zkrp_new_balance);
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_transfer_proof"></a>

## Function `verify_transfer_proof`

Verifies the validity of the <code>confidential_transfer</code> operation.

This function ensures that the provided proof (<code><a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a></code>) meets the following conditions:
1. The transferred amount (<code>transfer_amount</code>) and the auditor's balances (<code>auditor_amounts</code>), if provided,
encrypt the same transfer value under the recipient's encryption key (<code>recipient_ek</code>) and the auditor's
encryption keys (<code>auditor_eks</code>), respectively.
2. The sender's current balance (<code>current_balance</code>) and new balance (<code>new_balance</code>) encrypt the corresponding values
under the sender's encryption key (<code>sender_ek</code>) before and after the transfer, respectively.
3. The relationship <code>new_balance = current_balance - transfer_amount</code> is maintained, ensuring balance integrity.
4. The transferred value is properly normalized, with each chunk in both <code>transfer_amount</code> and the <code>auditor_amounts</code>
balance adhering to the range [0, 2^16).
5. The sender's new balance is normalized, with each chunk in <code>new_balance</code> also adhering to the range [0, 2^16).

If all conditions are satisfied, the proof validates the transfer; otherwise, the function causes an error.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_proof">verify_transfer_proof</a>(sender_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, recipient_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>&gt;, auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;, proof: &<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">confidential_proof::TransferProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_proof">verify_transfer_proof</a>(
    sender_ek: &twisted_elgamal::CompressedPubkey,
    recipient_ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;twisted_elgamal::CompressedPubkey&gt;,
    auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a>)
{
    <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_sigma_proof">verify_transfer_sigma_proof</a>(
        sender_ek,
        recipient_ek,
        current_balance,
        new_balance,
        transfer_amount,
        auditor_eks,
        auditor_amounts,
        &proof.sigma_proof
    );
    <a href="confidential_proof.md#0x7_confidential_proof_verify_new_balance_range_proof">verify_new_balance_range_proof</a>(new_balance, &proof.zkrp_new_balance);
    <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_amount_range_proof">verify_transfer_amount_range_proof</a>(transfer_amount, &proof.zkrp_transfer_amount);
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_normalization_proof"></a>

## Function `verify_normalization_proof`

Verifies the validity of the <code>normalize</code> operation.

This function ensures that the provided proof (<code><a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">NormalizationProof</a></code>) meets the following conditions:
1. The current balance (<code>current_balance</code>) and new balance (<code>new_balance</code>) encrypt the same value
under the same provided encryption key (<code>ek</code>), verifying that the normalization process preserves the balance value.
2. The new balance (<code>new_balance</code>) is properly normalized, with each chunk adhering to the range [0, 2^16),
as verified through the range proof in the normalization process.

If all conditions are satisfied, the proof validates the normalization; otherwise, the function causes an error.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_normalization_proof">verify_normalization_proof</a>(ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: &<a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">confidential_proof::NormalizationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_normalization_proof">verify_normalization_proof</a>(
    ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">NormalizationProof</a>)
{
    <a href="confidential_proof.md#0x7_confidential_proof_verify_normalization_sigma_proof">verify_normalization_sigma_proof</a>(ek, current_balance, new_balance, &proof.sigma_proof);
    <a href="confidential_proof.md#0x7_confidential_proof_verify_new_balance_range_proof">verify_new_balance_range_proof</a>(new_balance, &proof.zkrp_new_balance);
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_rotation_proof"></a>

## Function `verify_rotation_proof`

Verifies the validity of the <code>rotate_encryption_key</code> operation.

This function ensures that the provided proof (<code><a href="confidential_proof.md#0x7_confidential_proof_RotationProof">RotationProof</a></code>) meets the following conditions:
1. The current balance (<code>current_balance</code>) and new balance (<code>new_balance</code>) encrypt the same value under the
current encryption key (<code>current_ek</code>) and the new encryption key (<code>new_ek</code>), respectively, verifying
that the key rotation preserves the balance value.
2. The new balance (<code>new_balance</code>) is properly normalized, with each chunk adhering to the range [0, 2^16),
ensuring balance integrity after the key rotation.

If all conditions are satisfied, the proof validates the key rotation; otherwise, the function causes an error.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_rotation_proof">verify_rotation_proof</a>(current_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, new_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: &<a href="confidential_proof.md#0x7_confidential_proof_RotationProof">confidential_proof::RotationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_rotation_proof">verify_rotation_proof</a>(
    current_ek: &twisted_elgamal::CompressedPubkey,
    new_ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_RotationProof">RotationProof</a>)
{
    <a href="confidential_proof.md#0x7_confidential_proof_verify_rotation_sigma_proof">verify_rotation_sigma_proof</a>(current_ek, new_ek, current_balance, new_balance, &proof.sigma_proof);
    <a href="confidential_proof.md#0x7_confidential_proof_verify_new_balance_range_proof">verify_new_balance_range_proof</a>(new_balance, &proof.zkrp_new_balance);
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_withdrawal_sigma_proof"></a>

## Function `verify_withdrawal_sigma_proof`

Verifies the validity of the <code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_withdrawal_sigma_proof">verify_withdrawal_sigma_proof</a>(ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, amount: u64, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: &<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">confidential_proof::WithdrawalSigmaProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_withdrawal_sigma_proof">verify_withdrawal_sigma_proof</a>(
    ek: &twisted_elgamal::CompressedPubkey,
    amount: u64,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a>)
{
    <b>let</b> amount_chunks = <a href="confidential_balance.md#0x7_confidential_balance_split_into_chunks_u64">confidential_balance::split_into_chunks_u64</a>(amount);
    <b>let</b> amount = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u64">ristretto255::new_scalar_from_u64</a>(amount);

    <b>let</b> rho = <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_withdrawal_sigma_proof_challenge">fiat_shamir_withdrawal_sigma_proof_challenge</a>(ek, &amount_chunks, current_balance, &proof.xs);

    <b>let</b> gammas = <a href="confidential_proof.md#0x7_confidential_proof_msm_withdrawal_gammas">msm_withdrawal_gammas</a>(&rho);

    <b>let</b> scalars_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[gammas.g1, gammas.g2];
    scalars_lhs.append(gammas.g3s);
    scalars_lhs.append(gammas.g4s);

    <b>let</b> points_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x1),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x2)
    ];
    points_lhs.append(proof.xs.x3s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    points_lhs.append(proof.xs.x4s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));

    <b>let</b> scalar_g = <a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(
        &proof.alphas.a1s,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| <a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    );
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul_assign">ristretto255::scalar_mul_assign</a>(&<b>mut</b> scalar_g, &gammas.g1);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_g,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g3s, &proof.alphas.a1s)
    );
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_sub_assign">ristretto255::scalar_sub_assign</a>(&<b>mut</b> scalar_g, &<a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &rho, &amount));

    <b>let</b> scalar_h = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g2, &proof.alphas.a3);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_h,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g3s, &proof.alphas.a4s)
    );

    <b>let</b> scalar_ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g2, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_ek,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g4s, &proof.alphas.a4s)
    );

    <b>let</b> scalars_current_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &proof.alphas.a2, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_new_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g4s[i], &rho)
    });

    <b>let</b> scalars_current_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &rho, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_new_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g3s[i], &rho)
    });

    <b>let</b> scalars_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_g, scalar_h, scalar_ek];
    scalars_rhs.append(scalars_current_balance_d);
    scalars_rhs.append(scalars_new_balance_d);
    scalars_rhs.append(scalars_current_balance_c);
    scalars_rhs.append(scalars_new_balance_c);

    <b>let</b> points_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
        twisted_elgamal::pubkey_to_point(ek)
    ];
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(current_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(new_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(current_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(new_balance));

    <b>let</b> lhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_lhs, &scalars_lhs);
    <b>let</b> rhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_rhs, &scalars_rhs);

    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs, &rhs),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_proof.md#0x7_confidential_proof_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_transfer_sigma_proof"></a>

## Function `verify_transfer_sigma_proof`

Verifies the validity of the <code><a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_sigma_proof">verify_transfer_sigma_proof</a>(sender_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, recipient_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>&gt;, auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;, proof: &<a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">confidential_proof::TransferSigmaProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_sigma_proof">verify_transfer_sigma_proof</a>(
    sender_ek: &twisted_elgamal::CompressedPubkey,
    recipient_ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;twisted_elgamal::CompressedPubkey&gt;,
    auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a>)
{
    <b>let</b> rho = <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_transfer_sigma_proof_challenge">fiat_shamir_transfer_sigma_proof_challenge</a>(
        sender_ek,
        recipient_ek,
        current_balance,
        new_balance,
        transfer_amount,
        auditor_eks,
        auditor_amounts,
        &proof.xs
    );

    <b>let</b> gammas = <a href="confidential_proof.md#0x7_confidential_proof_msm_transfer_gammas">msm_transfer_gammas</a>(&rho, proof.xs.x7s.length());

    <b>let</b> scalars_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[gammas.g1];
    scalars_lhs.append(gammas.g2s);
    scalars_lhs.append(gammas.g3s);
    scalars_lhs.append(gammas.g4s);
    scalars_lhs.push_back(gammas.g5);
    scalars_lhs.append(gammas.g6s);
    gammas.g7s.for_each(|gamma| scalars_lhs.append(gamma));

    <b>let</b> points_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x1),
    ];
    points_lhs.append(proof.xs.x2s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    points_lhs.append(proof.xs.x3s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    points_lhs.append(proof.xs.x4s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    points_lhs.push_back(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x5));
    points_lhs.append(proof.xs.x6s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    proof.xs.x7s.for_each_ref(|xs| {
        points_lhs.append(xs.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    });

    <b>let</b> scalar_g = <a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(
        &proof.alphas.a1s,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| <a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    );
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul_assign">ristretto255::scalar_mul_assign</a>(&<b>mut</b> scalar_g, &gammas.g1);
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).for_each(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
            &<b>mut</b> scalar_g,
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g4s[i], &proof.alphas.a4s[i])
        );
    });
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_g,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g6s, &proof.alphas.a1s)
    );

    <b>let</b> scalar_h = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g5, &proof.alphas.a5);
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).for_each(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
            &<b>mut</b> scalar_h,
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g4s[i], &proof.alphas.a3s[i])
        );
    });
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_h,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g6s, &proof.alphas.a3s)
    );
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(4, 8).for_each(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
            &<b>mut</b> scalar_h,
            &<a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &proof.alphas.a3s[i], &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
        );
    });

    <b>let</b> scalar_sender_ek = <a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g2s, &proof.alphas.a3s);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(&<b>mut</b> scalar_sender_ek, &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g5, &rho));

    <b>let</b> scalar_recipient_ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_zero">ristretto255::scalar_zero</a>();
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).for_each(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
            &<b>mut</b> scalar_recipient_ek,
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g3s[i], &proof.alphas.a3s[i])
        );
    });

    <b>let</b> scalar_ek_auditors = gammas.g7s.map_ref(|gamma: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;| {
        <b>let</b> scalar_auditor_ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_zero">ristretto255::scalar_zero</a>();
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).for_each(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
                &<b>mut</b> scalar_auditor_ek,
                &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gamma[i], &proof.alphas.a3s[i])
            );
        });
        scalar_auditor_ek
    });

    <b>let</b> scalars_new_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <b>let</b> scalar = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g2s[i], &rho);
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_sub_assign">ristretto255::scalar_sub_assign</a>(
            &<b>mut</b> scalar,
            &<a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &proof.alphas.a2, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
        );
        scalar
    });

    <b>let</b> scalars_transfer_amount_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g3s[i], &rho)
    });

    <b>let</b> scalars_current_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &proof.alphas.a2, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_auditor_amount_d = gammas.g7s.map_ref(|gamma| {
        gamma.map_ref(|gamma| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(gamma, &rho))
    });

    <b>let</b> scalars_current_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &rho, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_transfer_amount_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).map(|i| {
        <b>let</b> scalar = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g4s[i], &rho);
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_sub_assign">ristretto255::scalar_sub_assign</a>(
            &<b>mut</b> scalar,
            &<a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &rho, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
        );
        scalar
    });

    <b>let</b> scalars_new_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g6s[i], &rho)
    });

    <b>let</b> scalars_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_g, scalar_h, scalar_sender_ek, scalar_recipient_ek];
    scalars_rhs.append(scalar_ek_auditors);
    scalars_rhs.append(scalars_new_balance_d);
    scalars_rhs.append(scalars_transfer_amount_d);
    scalars_rhs.append(scalars_current_balance_d);
    scalars_auditor_amount_d.for_each(|scalars| scalars_rhs.append(scalars));
    scalars_rhs.append(scalars_current_balance_c);
    scalars_rhs.append(scalars_transfer_amount_c);
    scalars_rhs.append(scalars_new_balance_c);

    <b>let</b> points_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
        twisted_elgamal::pubkey_to_point(sender_ek),
        twisted_elgamal::pubkey_to_point(recipient_ek)
    ];
    points_rhs.append(auditor_eks.map_ref(|ek| twisted_elgamal::pubkey_to_point(ek)));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(new_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(transfer_amount));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(current_balance));
    auditor_amounts.for_each_ref(|balance| {
        points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(balance));
    });
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(current_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(transfer_amount));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(new_balance));

    <b>let</b> lhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_lhs, &scalars_lhs);
    <b>let</b> rhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_rhs, &scalars_rhs);

    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs, &rhs),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_proof.md#0x7_confidential_proof_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_normalization_sigma_proof"></a>

## Function `verify_normalization_sigma_proof`

Verifies the validity of the <code><a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_normalization_sigma_proof">verify_normalization_sigma_proof</a>(ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: &<a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">confidential_proof::NormalizationSigmaProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_normalization_sigma_proof">verify_normalization_sigma_proof</a>(
    ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a>)
{
    <b>let</b> rho = <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_normalization_sigma_proof_challenge">fiat_shamir_normalization_sigma_proof_challenge</a>(ek, current_balance, new_balance, &proof.xs);
    <b>let</b> gammas = <a href="confidential_proof.md#0x7_confidential_proof_msm_normalization_gammas">msm_normalization_gammas</a>(&rho);

    <b>let</b> scalars_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[gammas.g1, gammas.g2];
    scalars_lhs.append(gammas.g3s);
    scalars_lhs.append(gammas.g4s);

    <b>let</b> points_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x1),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x2)
    ];
    points_lhs.append(proof.xs.x3s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    points_lhs.append(proof.xs.x4s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));

    <b>let</b> scalar_g = <a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(
        &proof.alphas.a1s,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| <a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    );
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul_assign">ristretto255::scalar_mul_assign</a>(&<b>mut</b> scalar_g, &gammas.g1);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_g,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g3s, &proof.alphas.a1s)
    );

    <b>let</b> scalar_h = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g2, &proof.alphas.a3);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_h,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g3s, &proof.alphas.a4s)
    );

    <b>let</b> scalar_ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g2, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_ek,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g4s, &proof.alphas.a4s)
    );

    <b>let</b> scalars_current_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &proof.alphas.a2, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_new_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g4s[i], &rho)
    });

    <b>let</b> scalars_current_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &rho, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_new_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g3s[i], &rho)
    });

    <b>let</b> scalars_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_g, scalar_h, scalar_ek];
    scalars_rhs.append(scalars_current_balance_d);
    scalars_rhs.append(scalars_new_balance_d);
    scalars_rhs.append(scalars_current_balance_c);
    scalars_rhs.append(scalars_new_balance_c);

    <b>let</b> points_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
        twisted_elgamal::pubkey_to_point(ek)
    ];
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(current_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(new_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(current_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(new_balance));

    <b>let</b> lhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_lhs, &scalars_lhs);
    <b>let</b> rhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_rhs, &scalars_rhs);

    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs, &rhs),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_proof.md#0x7_confidential_proof_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_rotation_sigma_proof"></a>

## Function `verify_rotation_sigma_proof`

Verifies the validity of the <code><a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_rotation_sigma_proof">verify_rotation_sigma_proof</a>(current_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, new_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: &<a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">confidential_proof::RotationSigmaProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_rotation_sigma_proof">verify_rotation_sigma_proof</a>(
    current_ek: &twisted_elgamal::CompressedPubkey,
    new_ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof: &<a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a>)
{
    <b>let</b> rho = <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_rotation_sigma_proof_challenge">fiat_shamir_rotation_sigma_proof_challenge</a>(
        current_ek,
        new_ek,
        current_balance,
        new_balance,
        &proof.xs
    );
    <b>let</b> gammas = <a href="confidential_proof.md#0x7_confidential_proof_msm_rotation_gammas">msm_rotation_gammas</a>(&rho);

    <b>let</b> scalars_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[gammas.g1, gammas.g2, gammas.g3];
    scalars_lhs.append(gammas.g4s);
    scalars_lhs.append(gammas.g5s);

    <b>let</b> points_lhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x1),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x2),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&proof.xs.x3)
    ];
    points_lhs.append(proof.xs.x4s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));
    points_lhs.append(proof.xs.x5s.map_ref(|x| <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(x)));

    <b>let</b> scalar_g = <a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(
        &proof.alphas.a1s,
        &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| <a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    );
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul_assign">ristretto255::scalar_mul_assign</a>(&<b>mut</b> scalar_g, &gammas.g1);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_g,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g4s, &proof.alphas.a1s)
    );

    <b>let</b> scalar_h = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g2, &proof.alphas.a3);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(&<b>mut</b> scalar_h, &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g3, &proof.alphas.a4));
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_h,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g4s, &proof.alphas.a5s)
    );

    <b>let</b> scalar_ek_cur = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g2, &rho);

    <b>let</b> scalar_ek_new = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g3, &rho);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(
        &<b>mut</b> scalar_ek_new,
        &<a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(&gammas.g5s, &proof.alphas.a5s)
    );

    <b>let</b> scalars_current_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &proof.alphas.a2, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_new_balance_d = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g5s[i], &rho)
    });

    <b>let</b> scalars_current_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(&gammas.g1, &rho, &<a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(i * 16))
    });

    <b>let</b> scalars_new_balance_c = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(&gammas.g4s[i], &rho)
    });

    <b>let</b> scalars_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[scalar_g, scalar_h, scalar_ek_cur, scalar_ek_new];
    scalars_rhs.append(scalars_current_balance_d);
    scalars_rhs.append(scalars_new_balance_d);
    scalars_rhs.append(scalars_current_balance_c);
    scalars_rhs.append(scalars_new_balance_c);

    <b>let</b> points_rhs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
        twisted_elgamal::pubkey_to_point(current_ek),
        twisted_elgamal::pubkey_to_point(new_ek)
    ];
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(current_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(new_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(current_balance));
    points_rhs.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(new_balance));

    <b>let</b> lhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_lhs, &scalars_lhs);
    <b>let</b> rhs = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_multi_scalar_mul">ristretto255::multi_scalar_mul</a>(&points_rhs, &scalars_rhs);

    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&lhs, &rhs),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_proof.md#0x7_confidential_proof_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_new_balance_range_proof"></a>

## Function `verify_new_balance_range_proof`

Verifies the validity of the <code>NewBalanceRangeProof</code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_new_balance_range_proof">verify_new_balance_range_proof</a>(new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, zkrp_new_balance: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_new_balance_range_proof">verify_new_balance_range_proof</a>(
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    zkrp_new_balance: &RangeProof)
{
    <b>let</b> balance_c = <a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(new_balance);

    <b>assert</b>!(
        bulletproofs::verify_batch_range_proof(
            &balance_c,
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
            zkrp_new_balance,
            <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_NUM_BITS">BULLETPROOFS_NUM_BITS</a>,
            <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_DST">BULLETPROOFS_DST</a>
        ),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="confidential_proof.md#0x7_confidential_proof_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_proof_verify_transfer_amount_range_proof"></a>

## Function `verify_transfer_amount_range_proof`

Verifies the validity of the <code>TransferBalanceRangeProof</code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_amount_range_proof">verify_transfer_amount_range_proof</a>(transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, zkrp_transfer_amount: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_amount_range_proof">verify_transfer_amount_range_proof</a>(
    transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    zkrp_transfer_amount: &RangeProof)
{
    <b>let</b> balance_c = <a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_c">confidential_balance::balance_to_points_c</a>(transfer_amount);

    <b>assert</b>!(
        bulletproofs::verify_batch_range_proof(
            &balance_c,
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>(),
            &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>(),
            zkrp_transfer_amount,
            <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_NUM_BITS">BULLETPROOFS_NUM_BITS</a>,
            <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_DST">BULLETPROOFS_DST</a>
        ),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="confidential_proof.md#0x7_confidential_proof_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_proof_auditors_count_in_transfer_proof"></a>

## Function `auditors_count_in_transfer_proof`

Returns the number of range proofs in the provided <code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a></code>.
Used in the <code><a href="confidential_asset.md#0x7_confidential_asset">confidential_asset</a></code> module to validate input parameters of the <code>confidential_transfer</code> function.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_auditors_count_in_transfer_proof">auditors_count_in_transfer_proof</a>(proof: &<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">confidential_proof::TransferProof</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_auditors_count_in_transfer_proof">auditors_count_in_transfer_proof</a>(proof: &<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a>): u64 {
    proof.sigma_proof.xs.x7s.length()
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_withdrawal_proof"></a>

## Function `deserialize_withdrawal_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_withdrawal_proof">deserialize_withdrawal_proof</a>(sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">confidential_proof::WithdrawalProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_withdrawal_proof">deserialize_withdrawal_proof</a>(
    sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a>&gt;
{
    <b>let</b> sigma_proof = <a href="confidential_proof.md#0x7_confidential_proof_deserialize_withdrawal_sigma_proof">deserialize_withdrawal_sigma_proof</a>(sigma_proof_bytes);
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);

    <b>if</b> (sigma_proof.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">WithdrawalProof</a> {
            sigma_proof: sigma_proof.extract(),
            zkrp_new_balance,
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_transfer_proof"></a>

## Function `deserialize_transfer_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_transfer_proof">deserialize_transfer_proof</a>(sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_transfer_amount_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">confidential_proof::TransferProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_transfer_proof">deserialize_transfer_proof</a>(
    sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_transfer_amount_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a>&gt;
{
    <b>let</b> sigma_proof = <a href="confidential_proof.md#0x7_confidential_proof_deserialize_transfer_sigma_proof">deserialize_transfer_sigma_proof</a>(sigma_proof_bytes);
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);
    <b>let</b> zkrp_transfer_amount = bulletproofs::range_proof_from_bytes(zkrp_transfer_amount_bytes);

    <b>if</b> (sigma_proof.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_TransferProof">TransferProof</a> {
            sigma_proof: sigma_proof.extract(),
            zkrp_new_balance,
            zkrp_transfer_amount,
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_normalization_proof"></a>

## Function `deserialize_normalization_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">NormalizationProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">NormalizationProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_normalization_proof">deserialize_normalization_proof</a>(sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">confidential_proof::NormalizationProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_normalization_proof">deserialize_normalization_proof</a>(
    sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">NormalizationProof</a>&gt;
{
    <b>let</b> sigma_proof = <a href="confidential_proof.md#0x7_confidential_proof_deserialize_normalization_sigma_proof">deserialize_normalization_sigma_proof</a>(sigma_proof_bytes);
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);

    <b>if</b> (sigma_proof.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">NormalizationProof</a> {
            sigma_proof: sigma_proof.extract(),
            zkrp_new_balance,
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_rotation_proof"></a>

## Function `deserialize_rotation_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_RotationProof">RotationProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_RotationProof">RotationProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_rotation_proof">deserialize_rotation_proof</a>(sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_RotationProof">confidential_proof::RotationProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_rotation_proof">deserialize_rotation_proof</a>(
    sigma_proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_RotationProof">RotationProof</a>&gt;
{
    <b>let</b> sigma_proof = <a href="confidential_proof.md#0x7_confidential_proof_deserialize_rotation_sigma_proof">deserialize_rotation_sigma_proof</a>(sigma_proof_bytes);
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance_bytes);

    <b>if</b> (sigma_proof.is_none()) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_RotationProof">RotationProof</a> {
            sigma_proof: sigma_proof.extract(),
            zkrp_new_balance,
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_withdrawal_sigma_proof"></a>

## Function `deserialize_withdrawal_sigma_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_withdrawal_sigma_proof">deserialize_withdrawal_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">confidential_proof::WithdrawalSigmaProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_withdrawal_sigma_proof">deserialize_withdrawal_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a>&gt; {
    <b>let</b> alphas_count = 18;
    <b>let</b> xs_count = 18;

    <b>if</b> (proof_bytes.length() != 32 * xs_count + 32 * alphas_count) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> alphas = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, alphas_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });
    <b>let</b> xs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(alphas_count, alphas_count + xs_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });

    <b>if</b> (alphas.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|alpha| alpha.is_none()) || xs.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|x| x.is_none())) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a> {
            alphas: <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofAlphas">WithdrawalSigmaProofAlphas</a> {
                a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                a2: alphas[8].extract(),
                a3: alphas[9].extract(),
                a4s: alphas.slice(10, 18).map(|alpha| alpha.extract()),
            },
            xs: <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofXs">WithdrawalSigmaProofXs</a> {
                x1: xs[0].extract(),
                x2: xs[1].extract(),
                x3s: xs.slice(2, 10).map(|x| x.extract()),
                x4s: xs.slice(10, 18).map(|x| x.extract()),
            },
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_transfer_sigma_proof"></a>

## Function `deserialize_transfer_sigma_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_transfer_sigma_proof">deserialize_transfer_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">confidential_proof::TransferSigmaProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_transfer_sigma_proof">deserialize_transfer_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a>&gt; {
    <b>let</b> alphas_count = 22;
    <b>let</b> xs_count = 26;

    <b>if</b> (proof_bytes.length() &lt; 32 * xs_count + 32 * alphas_count) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    // Transfer proof may contain additional four Xs for each auditor.
    <b>let</b> auditor_xs = proof_bytes.length() - (32 * xs_count + 32 * alphas_count);

    <b>if</b> (auditor_xs % 128 != 0) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    xs_count += auditor_xs / 32;

    <b>let</b> alphas = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, alphas_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });
    <b>let</b> xs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(alphas_count, alphas_count + xs_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });

    <b>if</b> (alphas.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|alpha| alpha.is_none()) || xs.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|x| x.is_none())) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a> {
            alphas: <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofAlphas">TransferSigmaProofAlphas</a> {
                a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                a2: alphas[8].extract(),
                a3s: alphas.slice(9, 17).map(|alpha| alpha.extract()),
                a4s: alphas.slice(17, 21).map(|alpha| alpha.extract()),
                a5: alphas[21].extract(),
            },
            xs: <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofXs">TransferSigmaProofXs</a> {
                x1: xs[0].extract(),
                x2s: xs.slice(1, 9).map(|x| x.extract()),
                x3s: xs.slice(9, 13).map(|x| x.extract()),
                x4s: xs.slice(13, 17).map(|x| x.extract()),
                x5: xs[17].extract(),
                x6s: xs.slice(18, 26).map(|x| x.extract()),
                x7s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range_with_step">vector::range_with_step</a>(26, xs_count, 4).map(|i| {
                    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(i, i + 4).map(|j| xs[j].extract())
                }),
            },
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_normalization_sigma_proof"></a>

## Function `deserialize_normalization_sigma_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_normalization_sigma_proof">deserialize_normalization_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">confidential_proof::NormalizationSigmaProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_normalization_sigma_proof">deserialize_normalization_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a>&gt; {
    <b>let</b> alphas_count = 18;
    <b>let</b> xs_count = 18;

    <b>if</b> (proof_bytes.length() != 32 * xs_count + 32 * alphas_count) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> alphas = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, alphas_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });
    <b>let</b> xs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(alphas_count, alphas_count + xs_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });

    <b>if</b> (alphas.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|alpha| alpha.is_none()) || xs.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|x| x.is_none())) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a> {
            alphas: <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofAlphas">NormalizationSigmaProofAlphas</a> {
                a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                a2: alphas[8].extract(),
                a3: alphas[9].extract(),
                a4s: alphas.slice(10, 18).map(|alpha| alpha.extract()),
            },
            xs: <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofXs">NormalizationSigmaProofXs</a> {
                x1: xs[0].extract(),
                x2: xs[1].extract(),
                x3s: xs.slice(2, 10).map(|x| x.extract()),
                x4s: xs.slice(10, 18).map(|x| x.extract()),
            },
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_deserialize_rotation_sigma_proof"></a>

## Function `deserialize_rotation_sigma_proof`

Deserializes the <code><a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a></code> from the byte array.
Returns <code>Some(<a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a>)</code> if the deserialization is successful; otherwise, returns <code>None</code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_rotation_sigma_proof">deserialize_rotation_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">confidential_proof::RotationSigmaProof</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_deserialize_rotation_sigma_proof">deserialize_rotation_sigma_proof</a>(proof_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a>&gt; {
    <b>let</b> alphas_count = 19;
    <b>let</b> xs_count = 19;

    <b>if</b> (proof_bytes.length() != 32 * xs_count + 32 * alphas_count) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <b>let</b> alphas = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, alphas_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_bytes">ristretto255::new_scalar_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });
    <b>let</b> xs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(alphas_count, alphas_count + xs_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(proof_bytes.slice(i * 32, (i + 1) * 32))
    });

    <b>if</b> (alphas.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|alpha| alpha.is_none()) || xs.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|x| x.is_none())) {
        <b>return</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };

    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
        <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a> {
            alphas: <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofAlphas">RotationSigmaProofAlphas</a> {
                a1s: alphas.slice(0, 8).map(|alpha| alpha.extract()),
                a2: alphas[8].extract(),
                a3: alphas[9].extract(),
                a4: alphas[10].extract(),
                a5s: alphas.slice(11, 19).map(|alpha| alpha.extract()),
            },
            xs: <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofXs">RotationSigmaProofXs</a> {
                x1: xs[0].extract(),
                x2: xs[1].extract(),
                x3: xs[2].extract(),
                x4s: xs.slice(3, 11).map(|x| x.extract()),
                x5s: xs.slice(11, 19).map(|x| x.extract()),
            },
        }
    )
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_fiat_shamir_withdrawal_sigma_dst"></a>

## Function `get_fiat_shamir_withdrawal_sigma_dst`

Returns the Fiat Shamir DST for the <code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a></code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_withdrawal_sigma_dst">get_fiat_shamir_withdrawal_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_withdrawal_sigma_dst">get_fiat_shamir_withdrawal_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST">FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST</a>
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_fiat_shamir_transfer_sigma_dst"></a>

## Function `get_fiat_shamir_transfer_sigma_dst`

Returns the Fiat Shamir DST for the <code><a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a></code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_transfer_sigma_dst">get_fiat_shamir_transfer_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_transfer_sigma_dst">get_fiat_shamir_transfer_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_TRANSFER_SIGMA_DST">FIAT_SHAMIR_TRANSFER_SIGMA_DST</a>
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_fiat_shamir_normalization_sigma_dst"></a>

## Function `get_fiat_shamir_normalization_sigma_dst`

Returns the Fiat Shamir DST for the <code><a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a></code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_normalization_sigma_dst">get_fiat_shamir_normalization_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_normalization_sigma_dst">get_fiat_shamir_normalization_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_NORMALIZATION_SIGMA_DST">FIAT_SHAMIR_NORMALIZATION_SIGMA_DST</a>
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_fiat_shamir_rotation_sigma_dst"></a>

## Function `get_fiat_shamir_rotation_sigma_dst`

Returns the Fiat Shamir DST for the <code><a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a></code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_rotation_sigma_dst">get_fiat_shamir_rotation_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_fiat_shamir_rotation_sigma_dst">get_fiat_shamir_rotation_sigma_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_ROTATION_SIGMA_DST">FIAT_SHAMIR_ROTATION_SIGMA_DST</a>
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_bulletproofs_dst"></a>

## Function `get_bulletproofs_dst`

Returns the DST for the range proofs.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_dst">get_bulletproofs_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_dst">get_bulletproofs_dst</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_DST">BULLETPROOFS_DST</a>
}
</code></pre>



</details>

<a id="0x7_confidential_proof_get_bulletproofs_num_bits"></a>

## Function `get_bulletproofs_num_bits`

Returns the maximum number of bits of the normalized chunk for the range proofs.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_num_bits">get_bulletproofs_num_bits</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_num_bits">get_bulletproofs_num_bits</a>(): u64 {
    <a href="confidential_proof.md#0x7_confidential_proof_BULLETPROOFS_NUM_BITS">BULLETPROOFS_NUM_BITS</a>
}
</code></pre>



</details>

<a id="0x7_confidential_proof_fiat_shamir_withdrawal_sigma_proof_challenge"></a>

## Function `fiat_shamir_withdrawal_sigma_proof_challenge`

Derives the Fiat-Shamir challenge for the <code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_withdrawal_sigma_proof_challenge">fiat_shamir_withdrawal_sigma_proof_challenge</a>(ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, amount_chunks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofXs">confidential_proof::WithdrawalSigmaProofXs</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_withdrawal_sigma_proof_challenge">fiat_shamir_withdrawal_sigma_proof_challenge</a>(
    ek: &twisted_elgamal::CompressedPubkey,
    amount_chunks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofXs">WithdrawalSigmaProofXs</a>): Scalar
{
    // rho = H(DST, v_{1..4}, P, (C_cur, D_cur)_{1..8}, G, H, X_{1..18})
    <b>let</b> bytes = <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST">FIAT_SHAMIR_WITHDRAWAL_SIGMA_DST</a>;

    amount_chunks.for_each_ref(|chunk| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_to_bytes">ristretto255::scalar_to_bytes</a>(chunk));
    });
    bytes.append(twisted_elgamal::pubkey_to_bytes(ek));
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(current_balance));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>()));
    bytes.append(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>()))
    );
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x1));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x2));
    proof_xs.x3s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });
    proof_xs.x4s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(bytes)
}
</code></pre>



</details>

<a id="0x7_confidential_proof_fiat_shamir_transfer_sigma_proof_challenge"></a>

## Function `fiat_shamir_transfer_sigma_proof_challenge`

Derives the Fiat-Shamir challenge for the <code><a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_transfer_sigma_proof_challenge">fiat_shamir_transfer_sigma_proof_challenge</a>(sender_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, recipient_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>&gt;, auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;, proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofXs">confidential_proof::TransferSigmaProofXs</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_transfer_sigma_proof_challenge">fiat_shamir_transfer_sigma_proof_challenge</a>(
    sender_ek: &twisted_elgamal::CompressedPubkey,
    recipient_ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;twisted_elgamal::CompressedPubkey&gt;,
    auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;,
    proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofXs">TransferSigmaProofXs</a>): Scalar
{
    // rho = H(DST, G, H, P_s, P_r, P_a_{1..n}, (C_cur, D_cur)_{1..8}, (C_new, D_new)_{1..8}, (C_v, D_v)_{1..8}, D_a_{1..n}, X_{1..26 + 4n})
    <b>let</b> bytes = <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_TRANSFER_SIGMA_DST">FIAT_SHAMIR_TRANSFER_SIGMA_DST</a>;

    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>()));
    bytes.append(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>()))
    );
    bytes.append(twisted_elgamal::pubkey_to_bytes(sender_ek));
    bytes.append(twisted_elgamal::pubkey_to_bytes(recipient_ek));
    auditor_eks.for_each_ref(|ek| {
        bytes.append(twisted_elgamal::pubkey_to_bytes(ek));
    });
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(current_balance));
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(new_balance));
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(transfer_amount));
    auditor_amounts.for_each_ref(|balance| {
        <a href="confidential_balance.md#0x7_confidential_balance_balance_to_points_d">confidential_balance::balance_to_points_d</a>(balance).for_each_ref(|d| {
            bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(d)));
        });
    });
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x1));
    proof_xs.x2s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });
    proof_xs.x3s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });
    proof_xs.x4s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x5));
    proof_xs.x6s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });
    proof_xs.x7s.for_each_ref(|xs| {
        xs.for_each_ref(|x| {
            bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
        });
    });

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(bytes)
}
</code></pre>



</details>

<a id="0x7_confidential_proof_fiat_shamir_normalization_sigma_proof_challenge"></a>

## Function `fiat_shamir_normalization_sigma_proof_challenge`

Derives the Fiat-Shamir challenge for the <code><a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_normalization_sigma_proof_challenge">fiat_shamir_normalization_sigma_proof_challenge</a>(ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofXs">confidential_proof::NormalizationSigmaProofXs</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_normalization_sigma_proof_challenge">fiat_shamir_normalization_sigma_proof_challenge</a>(
    ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofXs">NormalizationSigmaProofXs</a>): Scalar
{
    // rho = H(DST, G, H, P, (C_cur, D_cur)_{1..8}, (C_new, D_new)_{1..8}, X_{1..18})
    <b>let</b> bytes = <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_NORMALIZATION_SIGMA_DST">FIAT_SHAMIR_NORMALIZATION_SIGMA_DST</a>;

    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>()));
    bytes.append(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>()))
    );
    bytes.append(twisted_elgamal::pubkey_to_bytes(ek));
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(current_balance));
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(new_balance));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x1));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x2));
    proof_xs.x3s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });
    proof_xs.x4s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(bytes)
}
</code></pre>



</details>

<a id="0x7_confidential_proof_fiat_shamir_rotation_sigma_proof_challenge"></a>

## Function `fiat_shamir_rotation_sigma_proof_challenge`

Derives the Fiat-Shamir challenge for the <code><a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_rotation_sigma_proof_challenge">fiat_shamir_rotation_sigma_proof_challenge</a>(current_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, new_ek: &<a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_CompressedPubkey">ristretto255_twisted_elgamal::CompressedPubkey</a>, current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofXs">confidential_proof::RotationSigmaProofXs</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_fiat_shamir_rotation_sigma_proof_challenge">fiat_shamir_rotation_sigma_proof_challenge</a>(
    current_ek: &twisted_elgamal::CompressedPubkey,
    new_ek: &twisted_elgamal::CompressedPubkey,
    current_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    new_balance: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>,
    proof_xs: &<a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofXs">RotationSigmaProofXs</a>): Scalar
{
    // rho = H(DST, G, H, P_cur, P_new, (C_cur, D_cur)_{1..8}, (C_new, D_new)_{1..8}, X_{1..19})
    <b>let</b> bytes = <a href="confidential_proof.md#0x7_confidential_proof_FIAT_SHAMIR_ROTATION_SIGMA_DST">FIAT_SHAMIR_ROTATION_SIGMA_DST</a>;

    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>()));
    bytes.append(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_compressed_point_to_bytes">ristretto255::compressed_point_to_bytes</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_compress">ristretto255::point_compress</a>(&<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>()))
    );
    bytes.append(twisted_elgamal::pubkey_to_bytes(current_ek));
    bytes.append(twisted_elgamal::pubkey_to_bytes(new_ek));
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(current_balance));
    bytes.append(<a href="confidential_balance.md#0x7_confidential_balance_balance_to_bytes">confidential_balance::balance_to_bytes</a>(new_balance));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x1));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x2));
    bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(&proof_xs.x3));
    proof_xs.x4s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });
    proof_xs.x5s.for_each_ref(|x| {
        bytes.append(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_to_bytes">ristretto255::point_to_bytes</a>(x));
    });

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(bytes)
}
</code></pre>



</details>

<a id="0x7_confidential_proof_msm_withdrawal_gammas"></a>

## Function `msm_withdrawal_gammas`

Returns the scalar multipliers for the <code><a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProof">WithdrawalSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_withdrawal_gammas">msm_withdrawal_gammas</a>(rho: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofGammas">confidential_proof::WithdrawalSigmaProofGammas</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_withdrawal_gammas">msm_withdrawal_gammas</a>(rho: &Scalar): <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofGammas">WithdrawalSigmaProofGammas</a> {
    <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalSigmaProofGammas">WithdrawalSigmaProofGammas</a> {
        g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 1)),
        g2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 2)),
        g3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 3, (i <b>as</b> u8)))
        }),
        g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 4, (i <b>as</b> u8)))
        }),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_proof_msm_transfer_gammas"></a>

## Function `msm_transfer_gammas`

Returns the scalar multipliers for the <code><a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProof">TransferSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_transfer_gammas">msm_transfer_gammas</a>(rho: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, auditors_count: u64): <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofGammas">confidential_proof::TransferSigmaProofGammas</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_transfer_gammas">msm_transfer_gammas</a>(rho: &Scalar, auditors_count: u64): <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofGammas">TransferSigmaProofGammas</a> {
    <a href="confidential_proof.md#0x7_confidential_proof_TransferSigmaProofGammas">TransferSigmaProofGammas</a> {
        g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 1)),
        g2s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 2, (i <b>as</b> u8)))
        }),
        g3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 3, (i <b>as</b> u8)))
        }),
        g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 4, (i <b>as</b> u8)))
        }),
        g5: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 5)),
        g6s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 6, (i <b>as</b> u8)))
        }),
        g7s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, auditors_count).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 4).map(|j| {
                <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, (i + 7 <b>as</b> u8), (j <b>as</b> u8)))
            })
        }),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_proof_msm_normalization_gammas"></a>

## Function `msm_normalization_gammas`

Returns the scalar multipliers for the <code><a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProof">NormalizationSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_normalization_gammas">msm_normalization_gammas</a>(rho: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofGammas">confidential_proof::NormalizationSigmaProofGammas</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_normalization_gammas">msm_normalization_gammas</a>(rho: &Scalar): <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofGammas">NormalizationSigmaProofGammas</a> {
    <a href="confidential_proof.md#0x7_confidential_proof_NormalizationSigmaProofGammas">NormalizationSigmaProofGammas</a> {
        g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 1)),
        g2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 2)),
        g3s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 3, (i <b>as</b> u8)))
        }),
        g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 4, (i <b>as</b> u8)))
        }),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_proof_msm_rotation_gammas"></a>

## Function `msm_rotation_gammas`

Returns the scalar multipliers for the <code><a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProof">RotationSigmaProof</a></code>.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_rotation_gammas">msm_rotation_gammas</a>(rho: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofGammas">confidential_proof::RotationSigmaProofGammas</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_rotation_gammas">msm_rotation_gammas</a>(rho: &Scalar): <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofGammas">RotationSigmaProofGammas</a> {
    <a href="confidential_proof.md#0x7_confidential_proof_RotationSigmaProofGammas">RotationSigmaProofGammas</a> {
        g1: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 1)),
        g2: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 2)),
        g3: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho, 3)),
        g4s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 4, (i <b>as</b> u8)))
        }),
        g5s: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, 8).map(|i| {
            <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_sha2_512">ristretto255::new_scalar_from_sha2_512</a>(<a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho, 5, (i <b>as</b> u8)))
        }),
    }
}
</code></pre>



</details>

<a id="0x7_confidential_proof_msm_gamma_1"></a>

## Function `msm_gamma_1`

Returns the scalar multiplier computed as a hash of the provided <code>rho</code> and corresponding <code>gamma</code> index.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, i: u8): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_1">msm_gamma_1</a>(rho: &Scalar, i: u8): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_to_bytes">ristretto255::scalar_to_bytes</a>(rho);
    bytes.push_back(i);
    bytes
}
</code></pre>



</details>

<a id="0x7_confidential_proof_msm_gamma_2"></a>

## Function `msm_gamma_2`

Returns the scalar multiplier computed as a hash of the provided <code>rho</code> and corresponding <code>gamma</code> indices.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, i: u8, j: u8): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_msm_gamma_2">msm_gamma_2</a>(rho: &Scalar, i: u8, j: u8): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_to_bytes">ristretto255::scalar_to_bytes</a>(rho);
    bytes.push_back(i);
    bytes.push_back(j);
    bytes
}
</code></pre>



</details>

<a id="0x7_confidential_proof_scalar_mul_3"></a>

## Function `scalar_mul_3`

Calculates the product of the provided scalars.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(scalar1: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, scalar2: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, scalar3: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_scalar_mul_3">scalar_mul_3</a>(scalar1: &Scalar, scalar2: &Scalar, scalar3: &Scalar): Scalar {
    <b>let</b> result = *scalar1;

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul_assign">ristretto255::scalar_mul_assign</a>(&<b>mut</b> result, scalar2);
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul_assign">ristretto255::scalar_mul_assign</a>(&<b>mut</b> result, scalar3);

    result
}
</code></pre>



</details>

<a id="0x7_confidential_proof_scalar_linear_combination"></a>

## Function `scalar_linear_combination`

Calculates the linear combination of the provided scalars.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(lhs: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;, rhs: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_scalar_linear_combination">scalar_linear_combination</a>(lhs: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;, rhs: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;): Scalar {
    <b>let</b> result = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_zero">ristretto255::scalar_zero</a>();

    lhs.zip_ref(rhs, |l, r| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_add_assign">ristretto255::scalar_add_assign</a>(&<b>mut</b> result, &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_scalar_mul">ristretto255::scalar_mul</a>(l, r));
    });

    result
}
</code></pre>



</details>

<a id="0x7_confidential_proof_new_scalar_from_pow2"></a>

## Function `new_scalar_from_pow2`

Raises 2 to the power of the provided exponent and returns the result as a scalar.


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(exp: u64): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_proof.md#0x7_confidential_proof_new_scalar_from_pow2">new_scalar_from_pow2</a>(exp: u64): Scalar {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u128">ristretto255::new_scalar_from_u128</a>(1 &lt;&lt; (exp <b>as</b> u8))
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
