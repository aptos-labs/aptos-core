
<a id="0x7_veiled_coin"></a>

# Module `0x7::veiled_coin`

**WARNING:** This is an **experimental, proof-of-concept** module! It is *NOT* production-ready and it will likely
lead to loss of funds if used (or misused).

This module provides a veiled coin type, denoted <code><a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> that hides the value/denomination of a coin.
Importantly, although veiled transactions hide the amount of coins sent they still leak the sender and recipient.


<a id="@How_to_use_veiled_coins_0"></a>

### How to use veiled coins


This module allows users to "register" a veiled account for any pre-existing <code>velor_framework::Coin</code> type <code>T</code> via
the <code>register</code> entry function. For this, an encryption public key will need to be given as input, under which
the registered user's veiled balance will be encrypted.

Once Alice registers a veiled account for <code>T</code>, she can call <code>veil</code> with any public amount <code>a</code> of <code>T</code> coins
and add them to her veiled balance. Note that these coins will not be properly veiled yet, since they were withdrawn
from a public balance, which leaks their value.

(Alternatively, another user can initialize Alice's veiled balance by calling <code>veil_to</code>.)

Suppose Bob also registers and veils <code>b</code> of his own coins of type <code>T</code>.

Now Alice can use <code>fully_veiled_transfer</code> to send to Bob a secret amount <code>v</code> of coins from her veiled balance.
This will, for the first time, properly hide both Alice's and Bob's veiled balance.
The only information that an attacker (e.g., an Velor validator) learns, is that Alice transferred an unknown amount
<code>v</code> to Bob (including $v=0$), and as a result Alice's veiled balance is in a range [a-v, a] and Bob's veiled balance
is in [b, b+v]<code>.

As more veiled transfers occur between more veiled accounts, the uncertainity on the balance of each <a href="../../velor-framework/doc/account.md#0x1_account">account</a> becomes
larger and larger.

Lastly, users can easily withdraw veiled coins back into their <b>public</b> balance via </code>unveil<code>. Or, they can withdraw
publicly into someone <b>else</b>'s <b>public</b> balance via </code>unveil_to<code>.


<a id="@Terminology_1"></a>

### Terminology


1. *Veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>*: a <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> whose value is secret; i.e., it is encrypted under the owner's <b>public</b> key.

2. *Veiled amount*: <a href="../../velor-framework/../velor-stdlib/doc/any.md#0x1_any">any</a> amount that is secret because it was encrypted under some <b>public</b> key.
3. *Committed amount*: <a href="../../velor-framework/../velor-stdlib/doc/any.md#0x1_any">any</a> amount that is secret because it was committed <b>to</b> (rather than encrypted).

4. *Veiled transaction*: a transaction that hides its amount transferred; i.e., a transaction whose amount is veiled.

5. *Veiled balance*: unlike a normal balance, a veiled balance is secret; i.e., it is encrypted under the <a href="../../velor-framework/doc/account.md#0x1_account">account</a>'s
<b>public</b> key.

6. *ZKRP*: zero-knowledge range proofs; one of the key cryptographic ingredient in veiled coins which <b>ensures</b> users
can withdraw secretely from their veiled balance without over-withdrawing.


<a id="@Limitations_2"></a>

### Limitations


**WARNING:** This <b>module</b> is **experimental**! It is *NOT* production-ready. Specifically:

1. Deploying this <b>module</b> will likely lead <b>to</b> lost funds.
2. This <b>module</b> <b>has</b> not been cryptographically-audited.
3. The current implementation is vulnerable <b>to</b> _front-running attacks_ <b>as</b> described in the Zether paper [BAZB20].
4. There is no integration <b>with</b> wallet software which, for veiled accounts, must maintain an additional ElGamal
encryption keypair.
5. There is no support for rotating the ElGamal encryption <b>public</b> key of a veiled <a href="../../velor-framework/doc/account.md#0x1_account">account</a>.


<a id="@Veiled_<a_href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>_amounts_<b>as</b>_truncated_</code>u32<code>'s_3"></a>

### Veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> amounts <b>as</b> truncated </code>u32<code>'s


Veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> amounts must be specified <b>as</b> </code>u32<code>'s rather than </code>u64<code>'s <b>as</b> would be typical for normal coins in the
Velor framework. This is because <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> amounts must be encrypted <b>with</b> an *efficient*, additively-homomorphic encryption
scheme. Currently, our best candidate is ElGamal encryption in the exponent, which can only decrypt values around
32 bits or slightly larger.

Specifically, veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> amounts are restricted <b>to</b> be 32 bits and can be cast <b>to</b> a normal 64-bit <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> value by
setting the leftmost and rightmost 16 bits <b>to</b> zero and the "middle" 32 bits <b>to</b> be the veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> bits.

This gives veiled amounts ~10 bits for specifying ~3 decimals and ~22 bits for specifying whole amounts, which
limits veiled balances and veiled transfers <b>to</b> around 4 million coins. (See </code>coin.move<code> for how a normal 64-bit <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>
value gets interpreted <b>as</b> a decimal number.)

In order <b>to</b> convert a </code>u32<code> veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> amount <b>to</b> a normal </code>u64<code> <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> amount, we have <b>to</b> shift it left by 16 bits.

</code>```
u64 normal coin amount format:
[ left    || middle  || right ]
[ 63 - 32 || 31 - 16 || 15 - 0]

u32 veiled coin amount format; we take the middle 32 bits from the u64 format above and store them in a u32:
[ middle ]
[ 31 - 0 ]
```

Recall that: A coin has a *decimal precision* $d$ (e.g., for <code>VelorCoin</code>, $d = 8$; see <code>initialize</code> in
<code><a href="../../velor-framework/doc/velor_coin.md#0x1_velor_coin">velor_coin</a>.<b>move</b></code>). This precision $d$ is used when displaying a <code>u64</code> amount, by dividing the amount by $10^d$.
For example, if the precision $d = 2$, then a <code>u64</code> amount of 505 coins displays as 5.05 coins.

For veiled coins, we can easily display a <code>u32</code> <code>Coin&lt;T&gt;</code> amount $v$ by:
1. Casting $v$ as a u64 and shifting this left by 16 bits, obtaining a 64-bit $v'$
2. Displaying $v'$ normally, by dividing it by $d$, which is the precision in <code>CoinInfo&lt;T&gt;</code>.


<a id="@Implementation_details_4"></a>

### Implementation details


This module leverages a so-called "resource account," which helps us mint a <code><a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> from a
normal <code><a href="../../velor-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;T&gt;</code> by transferring this latter coin into a <code><a href="../../velor-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;T&gt;</code> stored in the
resource account.

Later on, when someone wants to convert their <code><a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> into a normal <code><a href="../../velor-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;T&gt;</code>,
the resource account can be used to transfer out the normal from its coin store. Transferring out a coin like this
requires a <code><a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a></code> for the resource account, which the <code><a href="veiled_coin.md#0x7_veiled_coin">veiled_coin</a></code> module can obtain via a <code>SignerCapability</code>.


<a id="@References_5"></a>

### References


[BAZB20] Zether: Towards Privacy in a Smart Contract World; by Bunz, Benedikt and Agrawal, Shashank and Zamani,
Mahdi and Boneh, Dan; in Financial Cryptography and Data Security; 2020


    -  [How to use veiled coins](#@How_to_use_veiled_coins_0)
    -  [Terminology](#@Terminology_1)
    -  [Limitations](#@Limitations_2)
    -  [Veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> amounts <b>as</b> truncated </code>u32<code>'s](#@Veiled_<a_href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>_amounts_<b>as</b>_truncated_</code>u32<code>'s_3)
    -  [Implementation details](#@Implementation_details_4)
    -  [References](#@References_5)
-  [Resource `VeiledCoinStore`](#0x7_veiled_coin_VeiledCoinStore)
-  [Struct `Deposit`](#0x7_veiled_coin_Deposit)
-  [Struct `Withdraw`](#0x7_veiled_coin_Withdraw)
-  [Resource `VeiledCoinMinter`](#0x7_veiled_coin_VeiledCoinMinter)
-  [Struct `VeiledCoin`](#0x7_veiled_coin_VeiledCoin)
-  [Struct `TransferProof`](#0x7_veiled_coin_TransferProof)
-  [Struct `WithdrawalProof`](#0x7_veiled_coin_WithdrawalProof)
-  [Constants](#@Constants_6)
-  [Function `init_module`](#0x7_veiled_coin_init_module)
-  [Function `register`](#0x7_veiled_coin_register)
-  [Function `veil_to`](#0x7_veiled_coin_veil_to)
-  [Function `veil`](#0x7_veiled_coin_veil)
-  [Function `unveil_to`](#0x7_veiled_coin_unveil_to)
-  [Function `unveil`](#0x7_veiled_coin_unveil)
-  [Function `fully_veiled_transfer`](#0x7_veiled_coin_fully_veiled_transfer)
-  [Function `clamp_u64_to_u32_amount`](#0x7_veiled_coin_clamp_u64_to_u32_amount)
-  [Function `cast_u32_to_u64_amount`](#0x7_veiled_coin_cast_u32_to_u64_amount)
-  [Function `has_veiled_coin_store`](#0x7_veiled_coin_has_veiled_coin_store)
-  [Function `veiled_amount`](#0x7_veiled_coin_veiled_amount)
-  [Function `veiled_balance`](#0x7_veiled_coin_veiled_balance)
-  [Function `encryption_public_key`](#0x7_veiled_coin_encryption_public_key)
-  [Function `total_veiled_coins`](#0x7_veiled_coin_total_veiled_coins)
-  [Function `get_veiled_coin_bulletproofs_dst`](#0x7_veiled_coin_get_veiled_coin_bulletproofs_dst)
-  [Function `get_max_bits_in_veiled_coin_value`](#0x7_veiled_coin_get_max_bits_in_veiled_coin_value)
-  [Function `register_internal`](#0x7_veiled_coin_register_internal)
-  [Function `veiled_deposit`](#0x7_veiled_coin_veiled_deposit)
-  [Function `unveil_to_internal`](#0x7_veiled_coin_unveil_to_internal)
-  [Function `fully_veiled_transfer_internal`](#0x7_veiled_coin_fully_veiled_transfer_internal)
-  [Function `verify_range_proofs`](#0x7_veiled_coin_verify_range_proofs)
-  [Function `get_resource_account_signer`](#0x7_veiled_coin_get_resource_account_signer)
-  [Function `veiled_mint_from_coin`](#0x7_veiled_coin_veiled_mint_from_coin)


<pre><code><b>use</b> <a href="../../velor-framework/doc/account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../velor-framework/doc/coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs">0x1::ristretto255_bulletproofs</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal">0x1::ristretto255_elgamal</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen">0x1::ristretto255_pedersen</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="helpers.md#0x7_helpers">0x7::helpers</a>;
<b>use</b> <a href="sigma_protos.md#0x7_sigma_protos">0x7::sigma_protos</a>;
</code></pre>



<a id="0x7_veiled_coin_VeiledCoinStore"></a>

## Resource `VeiledCoinStore`

A holder of a specific coin type and its associated event handles.
These are kept in a single resource to ensure locality of data.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>veiled_balance: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a></code>
</dt>
<dd>
 A ElGamal ciphertext of a value $v \in [0, 2^{32})$, an invariant that is enforced throughout the code.
</dd>
<dt>
<code>pk: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_veiled_coin_Deposit"></a>

## Struct `Deposit`

Event emitted when some amount of veiled coins were deposited into an account.


<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="veiled_coin.md#0x7_veiled_coin_Deposit">Deposit</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>user: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_veiled_coin_Withdraw"></a>

## Struct `Withdraw`

Event emitted when some amount of veiled coins were withdrawn from an account.


<pre><code>#[<a href="../../velor-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="veiled_coin.md#0x7_veiled_coin_Withdraw">Withdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>user: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_veiled_coin_VeiledCoinMinter"></a>

## Resource `VeiledCoinMinter`

Holds an <code><a href="../../velor-framework/doc/account.md#0x1_account_SignerCapability">account::SignerCapability</a></code> for the resource account created when initializing this module. This
resource account houses a <code><a href="../../velor-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;T&gt;</code> for every type of coin <code>T</code> that is veiled.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>signer_cap: <a href="../../velor-framework/doc/account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_veiled_coin_VeiledCoin"></a>

## Struct `VeiledCoin`

Main structure representing a coin in an account's custody.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>veiled_amount: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a></code>
</dt>
<dd>
 ElGamal ciphertext which encrypts the number of coins $v \in [0, 2^{32})$. This $[0, 2^{32})$ range invariant
 is enforced throughout the code via Bulletproof-based ZK range proofs.
</dd>
</dl>


</details>

<a id="0x7_veiled_coin_TransferProof"></a>

## Struct `TransferProof`

A cryptographic proof that ensures correctness of a veiled-to-veiled coin transfer.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x7_veiled_coin_TransferProof">TransferProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma_proof: <a href="sigma_protos.md#0x7_sigma_protos_TransferSubproof">sigma_protos::TransferSubproof</a></code>
</dt>
<dd>

</dd>
<dt>
<code>zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>

</dd>
<dt>
<code>zkrp_amount: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_veiled_coin_WithdrawalProof"></a>

## Struct `WithdrawalProof`

A cryptographic proof that ensures correctness of a veiled-to-*unveiled* coin transfer.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x7_veiled_coin_WithdrawalProof">WithdrawalProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma_proof: <a href="sigma_protos.md#0x7_sigma_protos_WithdrawalSubproof">sigma_protos::WithdrawalSubproof</a></code>
</dt>
<dd>

</dd>
<dt>
<code>zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_6"></a>

## Constants


<a id="0x7_veiled_coin_EINSUFFICIENT_BALANCE"></a>

Not enough coins to complete transaction.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 5;
</code></pre>



<a id="0x7_veiled_coin_EINTERNAL_ERROR"></a>

Non-specific internal error (see source code)


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_EINTERNAL_ERROR">EINTERNAL_ERROR</a>: u64 = 9;
</code></pre>



<a id="0x7_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED"></a>

A range proof failed to verify.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>: u64 = 2;
</code></pre>



<a id="0x7_veiled_coin_ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE"></a>

The range proof system does not support proofs for any number \in [0, 2^{32})


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>: u64 = 1;
</code></pre>



<a id="0x7_veiled_coin_EBYTES_WRONG_LENGTH"></a>

Byte vector given for deserialization was the wrong length.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_EBYTES_WRONG_LENGTH">EBYTES_WRONG_LENGTH</a>: u64 = 7;
</code></pre>



<a id="0x7_veiled_coin_EDESERIALIZATION_FAILED"></a>

Failed deserializing bytes into either ElGamal ciphertext or $\Sigma$-protocol proof.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>: u64 = 6;
</code></pre>



<a id="0x7_veiled_coin_EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT"></a>

The <code><a href="veiled_coin.md#0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code> and <code><a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a></code> constants need to sum to 32 (bits).


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT">EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT</a>: u64 = 8;
</code></pre>



<a id="0x7_veiled_coin_EVEILED_COIN_STORE_ALREADY_PUBLISHED"></a>

Account already has <code><a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;</code> registered.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_EVEILED_COIN_STORE_ALREADY_PUBLISHED">EVEILED_COIN_STORE_ALREADY_PUBLISHED</a>: u64 = 3;
</code></pre>



<a id="0x7_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED"></a>

Account hasn't registered <code><a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;</code>.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED">EVEILED_COIN_STORE_NOT_PUBLISHED</a>: u64 = 4;
</code></pre>



<a id="0x7_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE"></a>

The maximum number of bits used to represent a coin's value.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE">MAX_BITS_IN_VEILED_COIN_VALUE</a>: u64 = 32;
</code></pre>



<a id="0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED"></a>

When converting a <code>u64</code> normal (public) amount to a <code>u32</code> veiled amount, we keep the middle 32 bits and
remove the <code><a href="veiled_coin.md#0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code> least significant bits and the <code><a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a></code>
most significant bits (see comments in the beginning of this file).

When converting a <code>u32</code> veiled amount to a <code>u64</code> normal (public) amount, we simply cast it to <code>u64</code> and shift it
left by <code><a href="veiled_coin.md#0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code>.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a>: u8 = 16;
</code></pre>



<a id="0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED"></a>

See <code><a href="veiled_coin.md#0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code> comments.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>: u8 = 16;
</code></pre>



<a id="0x7_veiled_coin_VEILED_COIN_BULLETPROOFS_DST"></a>

The domain separation tag (DST) used for the Bulletproofs prover.


<pre><code><b>const</b> <a href="veiled_coin.md#0x7_veiled_coin_VEILED_COIN_BULLETPROOFS_DST">VEILED_COIN_BULLETPROOFS_DST</a>: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 86, 101, 105, 108, 101, 100, 67, 111, 105, 110, 47, 66, 117, 108, 108, 101, 116, 112, 114, 111, 111, 102, 82, 97, 110, 103, 101, 80, 114, 111, 111, 102];
</code></pre>



<a id="0x7_veiled_coin_init_module"></a>

## Function `init_module`

Initializes a so-called "resource" account which will maintain a <code><a href="../../velor-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;T&gt;</code> resource for all <code>Coin&lt;T&gt;</code>'s
that have been converted into a <code><a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code>.


<pre><code><b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_init_module">init_module</a>(deployer: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_init_module">init_module</a>(deployer: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        bulletproofs::get_max_range_bits() &gt;= <a href="veiled_coin.md#0x7_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE">MAX_BITS_IN_VEILED_COIN_VALUE</a>,
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="veiled_coin.md#0x7_veiled_coin_ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>)
    );

    <b>assert</b>!(
        <a href="veiled_coin.md#0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a> + <a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>
            == 32,
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="veiled_coin.md#0x7_veiled_coin_EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT">EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT</a>)
    );

    // Create the resource <a href="../../velor-framework/doc/account.md#0x1_account">account</a>. This will allow this <b>module</b> <b>to</b> later obtain a `<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>` for this <a href="../../velor-framework/doc/account.md#0x1_account">account</a> and
    // transfer `Coin&lt;T&gt;`'s into its `CoinStore&lt;T&gt;` before minting a `<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;`.
    <b>let</b> (_resource, signer_cap) =
        <a href="../../velor-framework/doc/account.md#0x1_account_create_resource_account">account::create_resource_account</a>(deployer, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());

    <b>move_to</b>(deployer, <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> { signer_cap })
}
</code></pre>



</details>

<a id="0x7_veiled_coin_register"></a>

## Function `register`

Initializes a veiled account for the specified <code>user</code> such that their balance is encrypted under public key <code>pk</code>.
Importantly, the user's wallet must retain their corresponding secret key.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_register">register</a>&lt;CoinType&gt;(user: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_register">register</a>&lt;CoinType&gt;(user: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>let</b> pk = elgamal::new_pubkey_from_bytes(pk);
    <a href="veiled_coin.md#0x7_veiled_coin_register_internal">register_internal</a>&lt;CoinType&gt;(user, pk.extract());
}
</code></pre>



</details>

<a id="0x7_veiled_coin_veil_to"></a>

## Function `veil_to`

Sends a *public* <code>amount</code> of normal coins from <code>sender</code> to the <code>recipient</code>'s veiled balance.

**WARNING:** This function *leaks* the transferred <code>amount</code>, since it is given as a public input.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veil_to">veil_to</a>&lt;CoinType&gt;(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veil_to">veil_to</a>&lt;CoinType&gt;(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a>, <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a> {
    <b>let</b> c = <a href="../../velor-framework/doc/coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;CoinType&gt;(sender, <a href="veiled_coin.md#0x7_veiled_coin_cast_u32_to_u64_amount">cast_u32_to_u64_amount</a>(amount));

    <b>let</b> vc = <a href="veiled_coin.md#0x7_veiled_coin_veiled_mint_from_coin">veiled_mint_from_coin</a>(c);

    <a href="veiled_coin.md#0x7_veiled_coin_veiled_deposit">veiled_deposit</a>&lt;CoinType&gt;(recipient, vc)
}
</code></pre>



</details>

<a id="0x7_veiled_coin_veil"></a>

## Function `veil`

Like <code>veil_to</code>, except <code>owner</code> is both the sender and the recipient.

This function can be used by the <code>owner</code> to initialize his veiled balance to a *public* value.

**WARNING:** The initialized balance is *leaked*, since its initialized <code>amount</code> is public here.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veil">veil</a>&lt;CoinType&gt;(owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veil">veil</a>&lt;CoinType&gt;(
    owner: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u32
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a>, <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a> {
    <a href="veiled_coin.md#0x7_veiled_coin_veil_to">veil_to</a>&lt;CoinType&gt;(owner, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), amount)
}
</code></pre>



</details>

<a id="0x7_veiled_coin_unveil_to"></a>

## Function `unveil_to`

Takes a *public* <code>amount</code> of <code><a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;</code> coins from <code>sender</code>, unwraps them to a <code><a href="../../velor-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;</code>,
and sends them to <code>recipient</code>. Maintains secrecy of <code>sender</code>'s new balance.

Requires a ZK range proof on the new balance of the sender, to ensure the sender has enough money to send.
No ZK range proof is necessary for the <code>amount</code>, which is given as a public <code>u32</code> value.

**WARNING:** This *leaks* the transferred <code>amount</code>, since it is a public <code>u32</code> argument.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_unveil_to">unveil_to</a>&lt;CoinType&gt;(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32, comm_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, withdraw_subproof: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_unveil_to">unveil_to</a>&lt;CoinType&gt;(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <b>address</b>,
    amount: u32,
    comm_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    withdraw_subproof: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>, <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> {
    // Deserialize all the proofs into their proper Move structs
    <b>let</b> comm_new_balance = pedersen::new_commitment_from_bytes(comm_new_balance);
    <b>assert</b>!(
        comm_new_balance.is_some(),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>)
    );

    <b>let</b> sigma_proof =
        <a href="sigma_protos.md#0x7_sigma_protos_deserialize_withdrawal_subproof">sigma_protos::deserialize_withdrawal_subproof</a>(withdraw_subproof);
    <b>assert</b>!(
        std::option::is_some(&sigma_proof),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>)
    );

    <b>let</b> comm_new_balance = comm_new_balance.extract();
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);

    <b>let</b> withdrawal_proof = <a href="veiled_coin.md#0x7_veiled_coin_WithdrawalProof">WithdrawalProof</a> {
        sigma_proof: std::option::extract(&<b>mut</b> sigma_proof),
        zkrp_new_balance
    };

    // Do the actual work
    <a href="veiled_coin.md#0x7_veiled_coin_unveil_to_internal">unveil_to_internal</a>&lt;CoinType&gt;(
        sender,
        recipient,
        amount,
        comm_new_balance,
        withdrawal_proof
    );
}
</code></pre>



</details>

<a id="0x7_veiled_coin_unveil"></a>

## Function `unveil`

Like <code>unveil_to</code>, except the <code>sender</code> is also the recipient.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_unveil">unveil</a>&lt;CoinType&gt;(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u32, comm_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, withdraw_subproof: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_unveil">unveil</a>&lt;CoinType&gt;(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    amount: u32,
    comm_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    withdraw_subproof: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>, <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> {
    <a href="veiled_coin.md#0x7_veiled_coin_unveil_to">unveil_to</a>&lt;CoinType&gt;(
        sender,
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        amount,
        comm_new_balance,
        zkrp_new_balance,
        withdraw_subproof
    )
}
</code></pre>



</details>

<a id="0x7_veiled_coin_fully_veiled_transfer"></a>

## Function `fully_veiled_transfer`

Sends a *veiled* amount from <code>sender</code> to <code>recipient</code>. After this call, the veiled balances of both the <code>sender</code>
and the <code>recipient</code> remain (or become) secret.

The sent amount always remains secret; It is encrypted both under the sender's PK (in <code>withdraw_ct</code>) & under the
recipient's PK (in <code>deposit_ct</code>) using the *same* ElGamal randomness, so as to allow for efficiently updating both
the sender's & recipient's veiled balances. It is also committed under <code>comm_amount</code>, so as to allow for a ZK
range proof.

Requires a <code><a href="veiled_coin.md#0x7_veiled_coin_TransferProof">TransferProof</a></code>; i.e.:
1. A range proof <code>zkrp_new_balance</code> on the new balance of the sender, to ensure the sender has enough money to
send.
2. A range proof <code>zkrp_amount</code> on the transferred amount in <code>comm_amount</code>, to ensure the sender won't create
coins out of thin air.
3. A $\Sigma$-protocol proof <code>transfer_subproof</code> which proves that 'withdraw_ct' encrypts the same veiled amount
as in 'deposit_ct' (with the same randomness) and as in <code>comm_amount</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_fully_veiled_transfer">fully_veiled_transfer</a>&lt;CoinType&gt;(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, withdraw_ct: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, deposit_ct: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, comm_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, comm_amount: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_amount: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, transfer_subproof: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_fully_veiled_transfer">fully_veiled_transfer</a>&lt;CoinType&gt;(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <b>address</b>,
    withdraw_ct: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    deposit_ct: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    comm_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    comm_amount: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_amount: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    transfer_subproof: <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a> {
    // Deserialize everything into their proper Move structs
    <b>let</b> veiled_withdraw_amount = elgamal::new_ciphertext_from_bytes(withdraw_ct);
    <b>assert</b>!(
        veiled_withdraw_amount.is_some(),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>)
    );

    <b>let</b> veiled_deposit_amount = elgamal::new_ciphertext_from_bytes(deposit_ct);
    <b>assert</b>!(
        veiled_deposit_amount.is_some(),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>)
    );

    <b>let</b> comm_new_balance = pedersen::new_commitment_from_bytes(comm_new_balance);
    <b>assert</b>!(
        comm_new_balance.is_some(),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>)
    );

    <b>let</b> comm_amount = pedersen::new_commitment_from_bytes(comm_amount);
    <b>assert</b>!(
        comm_amount.is_some(), <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>)
    );

    <b>let</b> transfer_subproof =
        <a href="sigma_protos.md#0x7_sigma_protos_deserialize_transfer_subproof">sigma_protos::deserialize_transfer_subproof</a>(transfer_subproof);
    <b>assert</b>!(
        std::option::is_some(&transfer_subproof),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="veiled_coin.md#0x7_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>)
    );

    <b>let</b> transfer_proof = <a href="veiled_coin.md#0x7_veiled_coin_TransferProof">TransferProof</a> {
        zkrp_new_balance: bulletproofs::range_proof_from_bytes(zkrp_new_balance),
        zkrp_amount: bulletproofs::range_proof_from_bytes(zkrp_amount),
        sigma_proof: std::option::extract(&<b>mut</b> transfer_subproof)
    };

    // Do the actual work
    <a href="veiled_coin.md#0x7_veiled_coin_fully_veiled_transfer_internal">fully_veiled_transfer_internal</a>&lt;CoinType&gt;(
        sender,
        recipient,
        veiled_withdraw_amount.extract(),
        veiled_deposit_amount.extract(),
        comm_new_balance.extract(),
        comm_amount.extract(),
        &transfer_proof
    )
}
</code></pre>



</details>

<a id="0x7_veiled_coin_clamp_u64_to_u32_amount"></a>

## Function `clamp_u64_to_u32_amount`

Clamps a <code>u64</code> normal public amount to a <code>u32</code> to-be-veiled amount.

WARNING: Precision is lost here (see "Veiled coin amounts as truncated <code>u32</code>'s" in the top-level comments)


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_clamp_u64_to_u32_amount">clamp_u64_to_u32_amount</a>(amount: u64): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_clamp_u64_to_u32_amount">clamp_u64_to_u32_amount</a>(amount: u64): u32 {
    // Removes the `<a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>` most significant bits.
    amount &lt;&lt; <a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>;
    amount &gt;&gt; <a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>;

    // Removes the other `32 - <a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>` least significant bits.
    amount = amount &gt;&gt; <a href="veiled_coin.md#0x7_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a>;

    // We are now left <b>with</b> a 32-bit value
    (amount <b>as</b> u32)
}
</code></pre>



</details>

<a id="0x7_veiled_coin_cast_u32_to_u64_amount"></a>

## Function `cast_u32_to_u64_amount`

Casts a <code>u32</code> to-be-veiled amount to a <code>u64</code> normal public amount. No precision is lost here.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_cast_u32_to_u64_amount">cast_u32_to_u64_amount</a>(amount: u32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_cast_u32_to_u64_amount">cast_u32_to_u64_amount</a>(amount: u32): u64 {
    (amount <b>as</b> u64) &lt;&lt; <a href="veiled_coin.md#0x7_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>
}
</code></pre>



</details>

<a id="0x7_veiled_coin_has_veiled_coin_store"></a>

## Function `has_veiled_coin_store`

Returns <code><b>true</b></code> if <code>addr</code> is registered to receive veiled coins of <code>CoinType</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;&gt;(addr)
}
</code></pre>



</details>

<a id="0x7_veiled_coin_veiled_amount"></a>

## Function `veiled_amount`

Returns the ElGamal encryption of the value of <code><a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_amount">veiled_amount</a>&lt;CoinType&gt;(<a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>: &<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;): &<a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_amount">veiled_amount</a>&lt;CoinType&gt;(<a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>: &<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;): &elgamal::Ciphertext {
    &<a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>.veiled_amount
}
</code></pre>



</details>

<a id="0x7_veiled_coin_veiled_balance"></a>

## Function `veiled_balance`

Returns the ElGamal encryption of the veiled balance of <code>owner</code> for the provided <code>CoinType</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_balance">veiled_balance</a>&lt;CoinType&gt;(owner: <b>address</b>): <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_balance">veiled_balance</a>&lt;CoinType&gt;(
    owner: <b>address</b>
): elgamal::CompressedCiphertext <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a> {
    <b>assert</b>!(
        <a href="veiled_coin.md#0x7_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(owner),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="veiled_coin.md#0x7_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED">EVEILED_COIN_STORE_NOT_PUBLISHED</a>)
    );

    <b>borrow_global</b>&lt;<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;&gt;(owner).veiled_balance
}
</code></pre>



</details>

<a id="0x7_veiled_coin_encryption_public_key"></a>

## Function `encryption_public_key`

Given an address <code>addr</code>, returns the ElGamal encryption public key associated with that address


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_encryption_public_key">encryption_public_key</a>&lt;CoinType&gt;(addr: <b>address</b>): <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_encryption_public_key">encryption_public_key</a>&lt;CoinType&gt;(
    addr: <b>address</b>
): elgamal::CompressedPubkey <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a> {
    <b>assert</b>!(
        <a href="veiled_coin.md#0x7_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(addr),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="veiled_coin.md#0x7_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED">EVEILED_COIN_STORE_NOT_PUBLISHED</a>)
    );

    <b>borrow_global_mut</b>&lt;<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;&gt;(addr).pk
}
</code></pre>



</details>

<a id="0x7_veiled_coin_total_veiled_coins"></a>

## Function `total_veiled_coins`

Returns the total supply of veiled coins


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_total_veiled_coins">total_veiled_coins</a>&lt;CoinType&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_total_veiled_coins">total_veiled_coins</a>&lt;CoinType&gt;(): u64 <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> {
    <b>let</b> rsrc_acc_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="veiled_coin.md#0x7_veiled_coin_get_resource_account_signer">get_resource_account_signer</a>());
    <b>assert</b>!(<a href="../../velor-framework/doc/coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(rsrc_acc_addr), <a href="veiled_coin.md#0x7_veiled_coin_EINTERNAL_ERROR">EINTERNAL_ERROR</a>);

    <a href="../../velor-framework/doc/coin.md#0x1_coin_balance">coin::balance</a>&lt;CoinType&gt;(rsrc_acc_addr)
}
</code></pre>



</details>

<a id="0x7_veiled_coin_get_veiled_coin_bulletproofs_dst"></a>

## Function `get_veiled_coin_bulletproofs_dst`

Returns the domain separation tag (DST) for constructing Bulletproof-based range proofs in this module.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_get_veiled_coin_bulletproofs_dst">get_veiled_coin_bulletproofs_dst</a>(): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_get_veiled_coin_bulletproofs_dst">get_veiled_coin_bulletproofs_dst</a>(): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="veiled_coin.md#0x7_veiled_coin_VEILED_COIN_BULLETPROOFS_DST">VEILED_COIN_BULLETPROOFS_DST</a>
}
</code></pre>



</details>

<a id="0x7_veiled_coin_get_max_bits_in_veiled_coin_value"></a>

## Function `get_max_bits_in_veiled_coin_value`

Returns the maximum # of bits used to represent a veiled coin amount. Might differ than the 64 bits used to
represent normal <code>velor_framework::coin::Coin</code> values.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_get_max_bits_in_veiled_coin_value">get_max_bits_in_veiled_coin_value</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_get_max_bits_in_veiled_coin_value">get_max_bits_in_veiled_coin_value</a>(): u64 {
    <a href="veiled_coin.md#0x7_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE">MAX_BITS_IN_VEILED_COIN_VALUE</a>
}
</code></pre>



</details>

<a id="0x7_veiled_coin_register_internal"></a>

## Function `register_internal`

Like <code>register</code>, but the public key has been parsed in a type-safe struct.
TODO: Do we want to require a PoK of the SK here?


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_register_internal">register_internal</a>&lt;CoinType&gt;(user: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_register_internal">register_internal</a>&lt;CoinType&gt;(
    user: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: elgamal::CompressedPubkey
) {
    <b>let</b> account_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(user);
    <b>assert</b>!(
        !<a href="veiled_coin.md#0x7_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(account_addr),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="veiled_coin.md#0x7_veiled_coin_EVEILED_COIN_STORE_ALREADY_PUBLISHED">EVEILED_COIN_STORE_ALREADY_PUBLISHED</a>)
    );

    // Note: There is no way <b>to</b> find an ElGamal SK such that the `(0_G, 0_G)` ciphertext below decrypts <b>to</b> a non-zero
    // value. We'd need <b>to</b> have `(r * G, v * G + r * pk) = (0_G, 0_G)`, which implies `r = 0` for <a href="../../velor-framework/../velor-stdlib/doc/any.md#0x1_any">any</a> choice of PK/SK.
    // Thus, we must have `v * G = 0_G`, which implies `v = 0`.

    <b>let</b> coin_store = <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt; {
        veiled_balance: <a href="helpers.md#0x7_helpers_get_veiled_balance_zero_ciphertext">helpers::get_veiled_balance_zero_ciphertext</a>(),
        pk
    };
    <b>move_to</b>(user, coin_store);
}
</code></pre>



</details>

<a id="0x7_veiled_coin_veiled_deposit"></a>

## Function `veiled_deposit`

Deposits a veiled <code><a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a></code> at address <code>to_addr</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_deposit">veiled_deposit</a>&lt;CoinType&gt;(to_addr: <b>address</b>, <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>: <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_deposit">veiled_deposit</a>&lt;CoinType&gt;(
    to_addr: <b>address</b>, <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>: <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a> {
    <b>assert</b>!(
        <a href="veiled_coin.md#0x7_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(to_addr),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="veiled_coin.md#0x7_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED">EVEILED_COIN_STORE_NOT_PUBLISHED</a>)
    );

    <b>let</b> veiled_coin_store = <b>borrow_global_mut</b>&lt;<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;&gt;(to_addr);

    // Fetch the veiled balance
    <b>let</b> veiled_balance =
        elgamal::decompress_ciphertext(&veiled_coin_store.veiled_balance);

    // Add the veiled amount <b>to</b> the veiled balance (leverages the homomorphism of the encryption scheme)
    elgamal::ciphertext_add_assign(&<b>mut</b> veiled_balance, &<a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>.veiled_amount);

    // Update the veiled balance
    veiled_coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

    // Make sure the veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> is dropped so it cannot be double spent
    <b>let</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt; { veiled_amount: _ } = <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>;

    // Once successful, emit an <a href="../../velor-framework/doc/event.md#0x1_event">event</a> that a veiled deposit occurred.
    <a href="../../velor-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="veiled_coin.md#0x7_veiled_coin_Deposit">Deposit</a> { user: to_addr });
}
</code></pre>



</details>

<a id="0x7_veiled_coin_unveil_to_internal"></a>

## Function `unveil_to_internal`

Like <code>unveil_to</code>, except the proofs have been deserialized into type-safe structs.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_unveil_to_internal">unveil_to_internal</a>&lt;CoinType&gt;(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32, comm_new_balance: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, withdrawal_proof: <a href="veiled_coin.md#0x7_veiled_coin_WithdrawalProof">veiled_coin::WithdrawalProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_unveil_to_internal">unveil_to_internal</a>&lt;CoinType&gt;(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <b>address</b>,
    amount: u32,
    comm_new_balance: pedersen::Commitment,
    withdrawal_proof: <a href="veiled_coin.md#0x7_veiled_coin_WithdrawalProof">WithdrawalProof</a>
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>, <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> {
    <b>let</b> addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>assert</b>!(
        <a href="veiled_coin.md#0x7_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(addr),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="veiled_coin.md#0x7_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED">EVEILED_COIN_STORE_NOT_PUBLISHED</a>)
    );

    // Fetch the sender's ElGamal encryption <b>public</b> key
    <b>let</b> sender_pk = <a href="veiled_coin.md#0x7_veiled_coin_encryption_public_key">encryption_public_key</a>&lt;CoinType&gt;(addr);

    // Fetch the sender's veiled balance
    <b>let</b> veiled_coin_store = <b>borrow_global_mut</b>&lt;<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;&gt;(addr);
    <b>let</b> veiled_balance =
        elgamal::decompress_ciphertext(&veiled_coin_store.veiled_balance);

    // Create a (not-yet-secure) encryption of `amount`, since `amount` is a <b>public</b> argument here.
    <b>let</b> scalar_amount = <a href="../../velor-framework/../velor-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u32">ristretto255::new_scalar_from_u32</a>(amount);

    // Verify that `comm_new_balance` is a commitment <b>to</b> the remaing balance after withdrawing `amount`.
    <a href="sigma_protos.md#0x7_sigma_protos_verify_withdrawal_subproof">sigma_protos::verify_withdrawal_subproof</a>(
        &sender_pk,
        &veiled_balance,
        &comm_new_balance,
        &scalar_amount,
        &withdrawal_proof.sigma_proof
    );

    // Verify a ZK range proof on `comm_new_balance` (and thus on the remaining `veiled_balance`)
    <a href="veiled_coin.md#0x7_veiled_coin_verify_range_proofs">verify_range_proofs</a>(
        &comm_new_balance,
        &withdrawal_proof.zkrp_new_balance,
        &std::option::none(),
        &std::option::none()
    );

    <b>let</b> veiled_amount = elgamal::new_ciphertext_no_randomness(&scalar_amount);

    // <a href="veiled_coin.md#0x7_veiled_coin_Withdraw">Withdraw</a> `amount` from the veiled balance (leverages the homomorphism of the encryption scheme.)
    elgamal::ciphertext_sub_assign(&<b>mut</b> veiled_balance, &veiled_amount);

    // Update the veiled balance <b>to</b> reflect the veiled withdrawal
    veiled_coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

    // Emit <a href="../../velor-framework/doc/event.md#0x1_event">event</a> <b>to</b> indicate a veiled withdrawal occurred
    <a href="../../velor-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="veiled_coin.md#0x7_veiled_coin_Withdraw">Withdraw</a> { user: addr });

    // <a href="veiled_coin.md#0x7_veiled_coin_Withdraw">Withdraw</a> normal `Coin`'s from the resource <a href="../../velor-framework/doc/account.md#0x1_account">account</a> and deposit them in the recipient's
    <b>let</b> c =
        <a href="../../velor-framework/doc/coin.md#0x1_coin_withdraw">coin::withdraw</a>(
            &<a href="veiled_coin.md#0x7_veiled_coin_get_resource_account_signer">get_resource_account_signer</a>(), <a href="veiled_coin.md#0x7_veiled_coin_cast_u32_to_u64_amount">cast_u32_to_u64_amount</a>(amount)
        );

    <a href="../../velor-framework/doc/coin.md#0x1_coin_deposit">coin::deposit</a>&lt;CoinType&gt;(recipient, c);
}
</code></pre>



</details>

<a id="0x7_veiled_coin_fully_veiled_transfer_internal"></a>

## Function `fully_veiled_transfer_internal`

Like <code>fully_veiled_transfer</code>, except the ciphertext and proofs have been deserialized into type-safe structs.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_fully_veiled_transfer_internal">fully_veiled_transfer_internal</a>&lt;CoinType&gt;(sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient_addr: <b>address</b>, veiled_withdraw_amount: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, veiled_deposit_amount: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>, comm_new_balance: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, comm_amount: <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, transfer_proof: &<a href="veiled_coin.md#0x7_veiled_coin_TransferProof">veiled_coin::TransferProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_fully_veiled_transfer_internal">fully_veiled_transfer_internal</a>&lt;CoinType&gt;(
    sender: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient_addr: <b>address</b>,
    veiled_withdraw_amount: elgamal::Ciphertext,
    veiled_deposit_amount: elgamal::Ciphertext,
    comm_new_balance: pedersen::Commitment,
    comm_amount: pedersen::Commitment,
    transfer_proof: &<a href="veiled_coin.md#0x7_veiled_coin_TransferProof">TransferProof</a>
) <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a> {
    <b>let</b> sender_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    <b>let</b> sender_pk = <a href="veiled_coin.md#0x7_veiled_coin_encryption_public_key">encryption_public_key</a>&lt;CoinType&gt;(sender_addr);
    <b>let</b> recipient_pk = <a href="veiled_coin.md#0x7_veiled_coin_encryption_public_key">encryption_public_key</a>&lt;CoinType&gt;(recipient_addr);

    // Note: The `encryption_public_key` call from above already asserts that `sender_addr` <b>has</b> a <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> store.
    <b>let</b> sender_veiled_coin_store =
        <b>borrow_global_mut</b>&lt;<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;&gt;(sender_addr);

    // Fetch the veiled balance of the veiled <a href="../../velor-framework/doc/account.md#0x1_account">account</a>
    <b>let</b> veiled_balance =
        elgamal::decompress_ciphertext(&sender_veiled_coin_store.veiled_balance);

    // Checks that `veiled_withdraw_amount` and `veiled_deposit_amount` encrypt the same amount of coins, under the
    // sender and recipient's PKs. Also checks this amount is committed inside `comm_amount`. Also, checks that the
    // new balance encrypted in `veiled_balance` is committed in `comm_new_balance`.
    <a href="sigma_protos.md#0x7_sigma_protos_verify_transfer_subproof">sigma_protos::verify_transfer_subproof</a>(
        &sender_pk,
        &recipient_pk,
        &veiled_withdraw_amount,
        &veiled_deposit_amount,
        &comm_amount,
        &comm_new_balance,
        &veiled_balance,
        &transfer_proof.sigma_proof
    );

    // Update the <a href="../../velor-framework/doc/account.md#0x1_account">account</a>'s veiled balance by homomorphically subtracting the veiled amount from the veiled balance.
    elgamal::ciphertext_sub_assign(&<b>mut</b> veiled_balance, &veiled_withdraw_amount);

    // Verifies range proofs on the transferred amount and the remaining balance
    <a href="veiled_coin.md#0x7_veiled_coin_verify_range_proofs">verify_range_proofs</a>(
        &comm_new_balance,
        &transfer_proof.zkrp_new_balance,
        &std::option::some(comm_amount),
        &std::option::some(transfer_proof.zkrp_amount)
    );

    // Update the veiled balance <b>to</b> reflect the veiled withdrawal
    sender_veiled_coin_store.veiled_balance = elgamal::compress_ciphertext(
        &veiled_balance
    );

    // Once everything succeeds, emit an <a href="../../velor-framework/doc/event.md#0x1_event">event</a> <b>to</b> indicate a veiled withdrawal occurred
    <a href="../../velor-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="veiled_coin.md#0x7_veiled_coin_Withdraw">Withdraw</a> { user: sender_addr });

    // Create a new veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> for the recipient.
    <b>let</b> vc = <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt; { veiled_amount: veiled_deposit_amount };

    // Deposits `veiled_deposit_amount` into the recipient's <a href="../../velor-framework/doc/account.md#0x1_account">account</a>
    // (Note, <b>if</b> this aborts, the whole transaction aborts, so we do not need <b>to</b> worry about atomicity.)
    <a href="veiled_coin.md#0x7_veiled_coin_veiled_deposit">veiled_deposit</a>(recipient_addr, vc);
}
</code></pre>



</details>

<a id="0x7_veiled_coin_verify_range_proofs"></a>

## Function `verify_range_proofs`

Verifies range proofs on the remaining balance of an account committed in <code>comm_new_balance</code> and, optionally, on
the transferred amount committed inside <code>comm_amount</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_verify_range_proofs">verify_range_proofs</a>(comm_new_balance: &<a href="../../velor-framework/../velor-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, zkrp_new_balance: &<a href="../../velor-framework/../velor-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>, comm_amount: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../velor-framework/../velor-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>&gt;, zkrp_amount: &<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../velor-framework/../velor-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_verify_range_proofs">verify_range_proofs</a>(
    comm_new_balance: &pedersen::Commitment,
    zkrp_new_balance: &RangeProof,
    comm_amount: &Option&lt;pedersen::Commitment&gt;,
    zkrp_amount: &Option&lt;RangeProof&gt;
) {
    // Let `amount` denote the amount committed in `comm_amount` and `new_bal` the balance committed in `comm_new_balance`.
    //
    // This function checks <b>if</b> it is possible <b>to</b> withdraw a veiled `amount` from a veiled `bal`, obtaining a new
    // veiled balance `new_bal = bal - amount`. This function is used <b>to</b> maintains a key safety <b>invariant</b> throughout
    // the veild <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> <a href="../../velor-framework/doc/code.md#0x1_code">code</a>: i.e., that every <a href="../../velor-framework/doc/account.md#0x1_account">account</a> <b>has</b> `new_bal \in [0, 2^{32})`.
    //
    // This <b>invariant</b> is enforced <b>as</b> follows:
    //
    //  1. We <b>assume</b> (by the <b>invariant</b>) that `bal \in [0, 2^{32})`.
    //
    //  2. We verify a ZK range proof that `amount \in [0, 2^{32})`. Otherwise, a sender could set `amount = p-1`
    //     <b>where</b> `p` is the order of the scalar field, which would give `new_bal = bal - (p-1) mod p = bal + 1`.
    //     Therefore, a malicious spender could create coins out of thin air for themselves.
    //
    //  3. We verify a ZK range proof that `new_bal \in [0, 2^{32})`. Otherwise, a sender could set `amount = bal + 1`,
    //     which would satisfy condition (2) from above but would give `new_bal = bal - (bal + 1) = -1`. Therefore,
    //     a malicious spender could spend more coins than they have.
    //
    // Altogether, these checks ensure that `bal - amount &gt;= 0` (<b>as</b> integers) and therefore that `bal &gt;= amount`
    // (again, <b>as</b> integers).
    //
    // When the caller of this function created the `comm_amount` from a <b>public</b> `u32` value, it is guaranteed that
    // condition (2) from above holds, so no range proof is necessary. This happens when withdrawing a <b>public</b>
    // amount from a veiled balance via `unveil_to` or `unveil`.

    // Checks that the remaining balance is &gt;= 0; i.e., range condition (3)
    <b>assert</b>!(
        bulletproofs::verify_range_proof_pedersen(
            comm_new_balance,
            zkrp_new_balance,
            <a href="veiled_coin.md#0x7_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE">MAX_BITS_IN_VEILED_COIN_VALUE</a>,
            <a href="veiled_coin.md#0x7_veiled_coin_VEILED_COIN_BULLETPROOFS_DST">VEILED_COIN_BULLETPROOFS_DST</a>
        ),
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="veiled_coin.md#0x7_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>)
    );

    // Checks that the transferred amount is in range (when this amount did not originate from a <b>public</b> amount); i.e., range condition (2)
    <b>if</b> (zkrp_amount.is_some()) {
        <b>assert</b>!(
            bulletproofs::verify_range_proof_pedersen(
                comm_amount.borrow(),
                zkrp_amount.borrow(),
                <a href="veiled_coin.md#0x7_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE">MAX_BITS_IN_VEILED_COIN_VALUE</a>,
                <a href="veiled_coin.md#0x7_veiled_coin_VEILED_COIN_BULLETPROOFS_DST">VEILED_COIN_BULLETPROOFS_DST</a>
            ),
            <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="veiled_coin.md#0x7_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>)
        );
    };
}
</code></pre>



</details>

<a id="0x7_veiled_coin_get_resource_account_signer"></a>

## Function `get_resource_account_signer`

Returns a signer for the resource account storing all the normal coins that have been veiled.


<pre><code><b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_get_resource_account_signer">get_resource_account_signer</a>(): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_get_resource_account_signer">get_resource_account_signer</a>(): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> {
    <a href="../../velor-framework/doc/account.md#0x1_account_create_signer_with_capability">account::create_signer_with_capability</a>(
        &<b>borrow_global</b>&lt;<a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a>&gt;(@velor_experimental).signer_cap
    )
}
</code></pre>



</details>

<a id="0x7_veiled_coin_veiled_mint_from_coin"></a>

## Function `veiled_mint_from_coin`

Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.

**WARNING:** Fundamentally, there is no way to hide the value of the coin being minted here.


<pre><code><b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_mint_from_coin">veiled_mint_from_coin</a>&lt;CoinType&gt;(c: <a href="../../velor-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;): <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="veiled_coin.md#0x7_veiled_coin_veiled_mint_from_coin">veiled_mint_from_coin</a>&lt;CoinType&gt;(
    c: Coin&lt;CoinType&gt;
): <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt; <b>acquires</b> <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> {
    // If there is no `<a href="../../velor-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;CoinType&gt;` in the resource <a href="../../velor-framework/doc/account.md#0x1_account">account</a>, create one.
    <b>let</b> rsrc_acc_signer = <a href="veiled_coin.md#0x7_veiled_coin_get_resource_account_signer">get_resource_account_signer</a>();
    <b>let</b> rsrc_acc_addr = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&rsrc_acc_signer);
    <b>if</b> (!<a href="../../velor-framework/doc/coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;CoinType&gt;(rsrc_acc_addr)) {
        <a href="../../velor-framework/doc/coin.md#0x1_coin_register">coin::register</a>&lt;CoinType&gt;(&rsrc_acc_signer);
    };

    // Move the normal <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> into the <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> store, so we can mint a veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>.
    // (There is no other way <b>to</b> drop a normal <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>, for safety reasons, so moving it into a <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> store is
    //  the only <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/option.md#0x1_option">option</a>.)
    <b>let</b> value_u64 = <a href="../../velor-framework/doc/coin.md#0x1_coin_value">coin::value</a>(&c);
    <b>let</b> value_u32 = <a href="veiled_coin.md#0x7_veiled_coin_clamp_u64_to_u32_amount">clamp_u64_to_u32_amount</a>(value_u64);

    // Paranoid check: <b>assert</b> that the u64 <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> value had only its middle 32 bits set (should be the case
    // because the caller should have withdrawn a u32 amount, but enforcing this here anyway).
    <b>assert</b>!(
        <a href="veiled_coin.md#0x7_veiled_coin_cast_u32_to_u64_amount">cast_u32_to_u64_amount</a>(value_u32) == value_u64,
        <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="veiled_coin.md#0x7_veiled_coin_EINTERNAL_ERROR">EINTERNAL_ERROR</a>)
    );

    // <a href="veiled_coin.md#0x7_veiled_coin_Deposit">Deposit</a> a normal <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a> into the resource <a href="../../velor-framework/doc/account.md#0x1_account">account</a>...
    <a href="../../velor-framework/doc/coin.md#0x1_coin_deposit">coin::deposit</a>(rsrc_acc_addr, c);

    // ...and mint a veiled <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>, which is backed by the normal <a href="../../velor-framework/doc/coin.md#0x1_coin">coin</a>
    <a href="veiled_coin.md#0x7_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt; {
        veiled_amount: <a href="helpers.md#0x7_helpers_public_amount_to_veiled_balance">helpers::public_amount_to_veiled_balance</a>(value_u32)
    }
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
