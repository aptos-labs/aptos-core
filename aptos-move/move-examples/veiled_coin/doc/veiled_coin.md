
<a name="0x1337_veiled_coin"></a>

# Module `0x1337::veiled_coin`

WARNING: This is an **experimental** module! One should in NO WAY deploy this module without auditing the cryptography
implemented in this module. Doing so will likely lead to lost funds.

This module provides a veiled coin type, denoted <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> that hides the value/denomination of a coin.
Importantly, although veiled transactions hide the amount of coins sent they still leak the sender and recipient.

<a name="@Limitations_0"></a>

## Limitations


WARNING: This module is **experimental**! It is *NOT* production-ready. Specifically:
- it has not been cryptographically-audited
- the current implementation is vulnerable to _front-running attacks_ as described in the Zether paper [BAZB20].
- there is no integration with wallet software which, for veiled accounts, must maintain an additional ElGamal
encryption keypair

Another limitation is veiled coin amounts must be speicified as <code>u32</code>'s rather than <code>u64</code>'s as would be typical for
normal coins in the Aptos framework.

TODO: Describe how this works.


<a name="@Terminology_1"></a>

## Terminology


1. Veiled coin: a coin whose value is secret; i.e., it is encrypted under the owner's public key

2. Veiled amount: any amount that is secret; i.e., encrypted under some public key

3. Veiled transaction: a transaction that hides its amount transferred; i.e., a transaction whose amount is veiled

4. Veiled balance: unlike a normal balance, a veiled balance is secret; i.e., it is encrypted under the account's
public key


<a name="@Implementation_details_2"></a>

## Implementation details

This module leverages a secondary so-called "resource account," which helps us mint a <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> from a
traditional <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;T&gt;</code> by transferring this latter coin into a <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;T&gt;</code> resource stored in the
resource account. Later on, when someone wants to convert their <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> into a traditional <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;T&gt;</code>
the resource account can be used to transfer out said <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;T&gt;</code> from its coin store. This is where the
"resource account" becomes important, since transfering out a coin like this requires a <code><a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a></code> for the resource
account, which this module can obtain via a <code>SignerCapability</code>.


<a name="@References_3"></a>

## References


[BAZB20] Zether: Towards Privacy in a Smart Contract World; by Bunz, Benedikt and Agrawal, Shashank and Zamani,
Mahdi and Boneh, Dan; in Financial Cryptography and Data Security; 2020


-  [Limitations](#@Limitations_0)
-  [Terminology](#@Terminology_1)
-  [Implementation details](#@Implementation_details_2)
-  [References](#@References_3)
-  [Struct `VeiledCoin`](#0x1337_veiled_coin_VeiledCoin)
-  [Resource `VeiledCoinStore`](#0x1337_veiled_coin_VeiledCoinStore)
-  [Resource `VeiledCoinMinter`](#0x1337_veiled_coin_VeiledCoinMinter)
-  [Struct `VeiledTransferProof`](#0x1337_veiled_coin_VeiledTransferProof)
-  [Struct `SigmaProof`](#0x1337_veiled_coin_SigmaProof)
-  [Struct `DepositEvent`](#0x1337_veiled_coin_DepositEvent)
-  [Struct `WithdrawEvent`](#0x1337_veiled_coin_WithdrawEvent)
-  [Constants](#@Constants_4)
-  [Function `register`](#0x1337_veiled_coin_register)
-  [Function `veil_to`](#0x1337_veiled_coin_veil_to)
-  [Function `veil`](#0x1337_veiled_coin_veil)
-  [Function `unveil_to`](#0x1337_veiled_coin_unveil_to)
-  [Function `unveil`](#0x1337_veiled_coin_unveil)
-  [Function `fully_veiled_transfer`](#0x1337_veiled_coin_fully_veiled_transfer)
-  [Function `has_veiled_coin_store`](#0x1337_veiled_coin_has_veiled_coin_store)
-  [Function `veiled_amount`](#0x1337_veiled_coin_veiled_amount)
-  [Function `veiled_balance`](#0x1337_veiled_coin_veiled_balance)
-  [Function `encryption_public_key`](#0x1337_veiled_coin_encryption_public_key)
-  [Function `register_internal`](#0x1337_veiled_coin_register_internal)
-  [Function `unveiled_to_veiled_coin`](#0x1337_veiled_coin_unveiled_to_veiled_coin)
-  [Function `veiled_to_unveiled_coin`](#0x1337_veiled_coin_veiled_to_unveiled_coin)
-  [Function `fully_veiled_transfer_internal`](#0x1337_veiled_coin_fully_veiled_transfer_internal)
-  [Function `deposit`](#0x1337_veiled_coin_deposit)
-  [Function `withdraw`](#0x1337_veiled_coin_withdraw)


<pre><code><b>use</b> <a href="../../../framework/aptos-framework/doc/account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/bulletproofs.md#0x1_bulletproofs">0x1::bulletproofs</a>;
<b>use</b> <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal">0x1::elgamal</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../../framework/aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1337_veiled_coin_VeiledCoin"></a>

## Struct `VeiledCoin`

Main structure representing a coin in an account's custody.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;
</code></pre>



<a name="0x1337_veiled_coin_VeiledCoinStore"></a>

## Resource `VeiledCoinStore`

A holder of a specific coin type and its associated event handles.
These are kept in a single resource to ensure locality of data.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt; <b>has</b> key
</code></pre>



<a name="0x1337_veiled_coin_VeiledCoinMinter"></a>

## Resource `VeiledCoinMinter`

Holds a signer capability for the resource account created when initializing this module. This account houses a
<code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;T&gt;</code> for every type of coin <code>T</code> that is veiled.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> <b>has</b> store, key
</code></pre>



<a name="0x1337_veiled_coin_VeiledTransferProof"></a>

## Struct `VeiledTransferProof`

Represents a cryptographic proof necessary to authorize a veiled coin transfer. This module will verify this
proof w.r.t. encryptions of the transferred amount, both under the sender's PK and under the recipient's PK.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledTransferProof">VeiledTransferProof</a>&lt;CoinType&gt; <b>has</b> drop
</code></pre>



<a name="0x1337_veiled_coin_SigmaProof"></a>

## Struct `SigmaProof`

Represents the Sigma protocol proof used as part of a <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledTransferProof">VeiledTransferProof</a></code>.
A more detailed description can be found in <code>verify_withdrawal_sigma_protocol</code>


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_SigmaProof">SigmaProof</a>&lt;CoinType&gt; <b>has</b> drop
</code></pre>



<a name="0x1337_veiled_coin_DepositEvent"></a>

## Struct `DepositEvent`

Event emitted when some amount of a coin is deposited into an account.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_DepositEvent">DepositEvent</a> <b>has</b> drop, store
</code></pre>



<a name="0x1337_veiled_coin_WithdrawEvent"></a>

## Struct `WithdrawEvent`

Event emitted when some amount of a coin is withdrawn from an account.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store
</code></pre>



<a name="@Constants_4"></a>

## Constants


<a name="0x1337_veiled_coin_EINSUFFICIENT_BALANCE"></a>

Not enough coins to complete transaction.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 5;
</code></pre>



<a name="0x1337_veiled_coin_EBYTES_WRONG_LENGTH"></a>

Byte vector given for deserialization was the wrong length.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EBYTES_WRONG_LENGTH">EBYTES_WRONG_LENGTH</a>: u64 = 7;
</code></pre>



<a name="0x1337_veiled_coin_EDESERIALIZATION_FAILED"></a>

Failed deserializing bytes into either ElGamal ciphertext or Sigma protocol proof.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>: u64 = 6;
</code></pre>



<a name="0x1337_veiled_coin_ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE"></a>

The range proof system does not support proofs for any number \in [0, 2^{32})


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>: u64 = 1;
</code></pre>



<a name="0x1337_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED"></a>

A range proof failed to verify.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>: u64 = 2;
</code></pre>



<a name="0x1337_veiled_coin_ESIGMA_PROTOCOL_VERIFY_FAILED"></a>

Sigma protocol proof for withdrawals did not verify.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_ESIGMA_PROTOCOL_VERIFY_FAILED">ESIGMA_PROTOCOL_VERIFY_FAILED</a>: u64 = 8;
</code></pre>



<a name="0x1337_veiled_coin_EVECTOR_CUT_TOO_LARGE"></a>

Tried cutting out more elements than are in the vector via <code>cut_vector</code>.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EVECTOR_CUT_TOO_LARGE">EVECTOR_CUT_TOO_LARGE</a>: u64 = 9;
</code></pre>



<a name="0x1337_veiled_coin_EVEILED_COIN_STORE_ALREADY_PUBLISHED"></a>

Account already has <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;</code> registered.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EVEILED_COIN_STORE_ALREADY_PUBLISHED">EVEILED_COIN_STORE_ALREADY_PUBLISHED</a>: u64 = 3;
</code></pre>



<a name="0x1337_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED"></a>

Account hasn't registered <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;</code>.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED">EVEILED_COIN_STORE_NOT_PUBLISHED</a>: u64 = 4;
</code></pre>



<a name="0x1337_veiled_coin_FIAT_SHAMIR_SIGMA_DST"></a>

The domain separation tag (DST) used in the Fiat-Shamir transform of our Sigma protocol.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_FIAT_SHAMIR_SIGMA_DST">FIAT_SHAMIR_SIGMA_DST</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 86, 101, 105, 108, 101, 100, 67, 111, 105, 110, 47, 87, 105, 116, 104, 100, 114, 97, 119, 97, 108, 80, 114, 111, 111, 102, 70, 105, 97, 116, 83, 104, 97, 109, 105, 114];
</code></pre>



<a name="0x1337_veiled_coin_MAX_BITS_IN_VALUE"></a>

The maximum number of bits used to represent a coin's value.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_MAX_BITS_IN_VALUE">MAX_BITS_IN_VALUE</a>: u64 = 32;
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_1"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_1">SOME_RANDOMNESS_1</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [167, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_2"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_2">SOME_RANDOMNESS_2</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [183, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_3"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_3">SOME_RANDOMNESS_3</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [199, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_4"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_4">SOME_RANDOMNESS_4</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [215, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_5"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_5">SOME_RANDOMNESS_5</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [162, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_6"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_6">SOME_RANDOMNESS_6</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [178, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_7"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_7">SOME_RANDOMNESS_7</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [194, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_SOME_RANDOMNESS_8"></a>



<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_SOME_RANDOMNESS_8">SOME_RANDOMNESS_8</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [210, 199, 180, 43, 117, 80, 59, 252, 123, 25, 50, 120, 55, 134, 210, 39, 235, 248, 143, 121, 218, 117, 43, 104, 246, 184, 101, 169, 193, 121, 100, 12];
</code></pre>



<a name="0x1337_veiled_coin_VEILED_COIN_DST"></a>

The domain separation tag (DST) used for the Bulletproofs prover.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_VEILED_COIN_DST">VEILED_COIN_DST</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 86, 101, 105, 108, 101, 100, 67, 111, 105, 110, 47, 66, 117, 108, 108, 101, 116, 112, 114, 111, 111, 102, 82, 97, 110, 103, 101, 80, 114, 111, 111, 102];
</code></pre>



<a name="0x1337_veiled_coin_register"></a>

## Function `register`

Initializes a veiled coin store for the specified <code>user</code> account with that user's ElGamal public encryption key.
Importantly, the user's wallet must retain their corresponding secret key.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_register">register</a>&lt;CoinType&gt;(user: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_veil_to"></a>

## Function `veil_to`

Sends a *public* <code>amount</code> of normal coins from <code>sender</code> to the <code>recipient</code>'s veiled balance.

WARNING: This function *leaks* the transferred <code>amount</code>, since it is given as a public input.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veil_to">veil_to</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u64)
</code></pre>



<a name="0x1337_veiled_coin_veil"></a>

## Function `veil`

Like <code>veil_to</code> except the <code>sender</code> is also the recipient.

This function can be used by the <code>sender</code> to initialize his veiled balance to a *public* value.

WARNING: The initialized balance is *leaked*, since its initialized <code>amount</code> is public here.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veil">veil</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>



<a name="0x1337_veiled_coin_unveil_to"></a>

## Function `unveil_to`

Takes a *public* <code>amount</code> of <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;</code> coins from <code>sender</code>, unwraps them to a <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;</code>,
and sends them to <code>recipient</code>. Maintains secrecy of <code>sender</code>'s new balance.

Requires a range proof on the new balance of the sender, to ensure the sender has enough money to send.
No range proof is necessary for the <code>amount</code>, which is given as a public <code>u32</code> value.

WARNING: This *leaks* the transferred <code>amount</code>, since it is a public <code>u32</code> argument.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_unveil_to">unveil_to</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32, range_proof_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_unveil"></a>

## Function `unveil`

Like <code>unveil_to</code>, except the <code>sender</code> is also the recipient.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_unveil">unveil</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u32, range_proof_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_fully_veiled_transfer"></a>

## Function `fully_veiled_transfer`

Sends a *veiled* <code>amount</code> from <code>sender</code> to <code>recipient</code>. After this call, the balance of the <code>sender</code>
and <code>recipient</code> remains (or becomes) secret.

The sent amount remain secret! It is encrypted both under the sender's PK (in <code>withdraw_ct</code>) and under the
recipient's PK (in <code>deposit_ct</code>) using the *same* ElGamal randomness.

Requires a <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledTransferProof">VeiledTransferProof</a></code>; i.e.:
1. A range proof on the new balance of the sender, to ensure the sender has enough money to send (in <code>range_proof_new_balance</code>)
2. A range proof on the transferred amount, to ensure the sender won't create coins out of thin air (in <code>range_proof_veiled_amount</code>).
3. A Sigma protocol to prove that 'veiled_withdraw_amount' encrypts the same veiled amount as
'veiled_deposit_amount' with the same randomness (in <code>sigma_proof_bytes</code>).


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_fully_veiled_transfer">fully_veiled_transfer</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, withdraw_ct: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, deposit_ct: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, range_proof_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, range_proof_veiled_amount: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proof_bytes: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_has_veiled_coin_store"></a>

## Function `has_veiled_coin_store`

Returns <code><b>true</b></code> if <code>addr</code> is registered to receive veiled coins of <code>CoinType</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(addr: <b>address</b>): bool
</code></pre>



<a name="0x1337_veiled_coin_veiled_amount"></a>

## Function `veiled_amount`

Returns the ElGamal encryption of the value of <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veiled_amount">veiled_amount</a>&lt;CoinType&gt;(<a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>: &<a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;): &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>
</code></pre>



<a name="0x1337_veiled_coin_veiled_balance"></a>

## Function `veiled_balance`

Returns the ElGamal encryption of the veiled balance of <code>owner</code> for the provided <code>CoinType</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veiled_balance">veiled_balance</a>&lt;CoinType&gt;(owner: <b>address</b>): <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_CompressedCiphertext">elgamal::CompressedCiphertext</a>
</code></pre>



<a name="0x1337_veiled_coin_encryption_public_key"></a>

## Function `encryption_public_key`

Given an address <code>addr</code>, returns the ElGamal encryption public key associated with that address


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_encryption_public_key">encryption_public_key</a>&lt;CoinType&gt;(addr: <b>address</b>): <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_CompressedPubkey">elgamal::CompressedPubkey</a>
</code></pre>



<a name="0x1337_veiled_coin_register_internal"></a>

## Function `register_internal`

Like <code>register</code>, but the public key is parsed in an <code><a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_CompressedPubkey">elgamal::CompressedPubkey</a></code> struct.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_register_internal">register_internal</a>&lt;CoinType&gt;(user: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_CompressedPubkey">elgamal::CompressedPubkey</a>)
</code></pre>



<a name="0x1337_veiled_coin_unveiled_to_veiled_coin"></a>

## Function `unveiled_to_veiled_coin`

Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.

WARNING: Fundamentally, there is no way to hide the value of the coin being minted here.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_unveiled_to_veiled_coin">unveiled_to_veiled_coin</a>&lt;CoinType&gt;(c: <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;
</code></pre>



<a name="0x1337_veiled_coin_veiled_to_unveiled_coin"></a>

## Function `veiled_to_unveiled_coin`

Removes a *public* <code>amount</code> of veiled coins from <code>sender</code> and returns them as a normal <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a></code>.

Requires a ZK range proof on the new balance of the <code>sender</code>, to ensure the <code>sender</code> has enough money to send.
Since the <code>amount</code> is public, no ZK range proof on it is required.

WARNING: This function *leaks* the public <code>amount</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veiled_to_unveiled_coin">veiled_to_unveiled_coin</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u32, new_balance_proof: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/bulletproofs.md#0x1_bulletproofs_RangeProof">bulletproofs::RangeProof</a>): <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;
</code></pre>



<a name="0x1337_veiled_coin_fully_veiled_transfer_internal"></a>

## Function `fully_veiled_transfer_internal`

Like <code>fully_veiled_transfer</code>, except the ciphertext and proofs have been deserialized into their respective structs.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_fully_veiled_transfer_internal">fully_veiled_transfer_internal</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient_addr: <b>address</b>, veiled_withdraw_amount: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, veiled_deposit_amount: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, transfer_proof: &<a href="veiled_coin.md#0x1337_veiled_coin_VeiledTransferProof">veiled_coin::VeiledTransferProof</a>&lt;CoinType&gt;)
</code></pre>



<a name="0x1337_veiled_coin_deposit"></a>

## Function `deposit`

Deposits a veiled coin at address <code>to_addr</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_deposit">deposit</a>&lt;CoinType&gt;(to_addr: <b>address</b>, <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>: <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;)
</code></pre>



<a name="0x1337_veiled_coin_withdraw"></a>

## Function `withdraw`

Withdraws a *veiled* <code>amount</code> of coins from the specified coin store. Let <code>balance</code> denote its current
*veiled* balance.

Always requires a ZK range proof <code>new_balance_proof</code> on <code>balance - amount</code>. When the veiled amount was NOT
created from a public value, additionally requires a ZK range proof <code>veiled_amount_proof</code> on <code>amount</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_withdraw">withdraw</a>&lt;CoinType&gt;(veiled_amount: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/elgamal.md#0x1_elgamal_Ciphertext">elgamal::Ciphertext</a>, coin_store: &<b>mut</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinStore">veiled_coin::VeiledCoinStore</a>&lt;CoinType&gt;, new_balance_proof: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/bulletproofs.md#0x1_bulletproofs_RangeProof">bulletproofs::RangeProof</a>, veiled_amount_proof: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/bulletproofs.md#0x1_bulletproofs_RangeProof">bulletproofs::RangeProof</a>&gt;)
</code></pre>
