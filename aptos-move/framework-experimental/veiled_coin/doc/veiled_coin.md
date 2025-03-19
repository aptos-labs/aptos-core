
<a name="0x1337_veiled_coin"></a>

# Module `0x1337::veiled_coin`

**WARNING:** This is an **experimental, proof-of-concept** module! It is *NOT* production-ready and it will likely
lead to loss of funds if used (or misused).

This module provides a veiled coin type, denoted <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> that hides the value/denomination of a coin.
Importantly, although veiled transactions hide the amount of coins sent they still leak the sender and recipient.


<a name="@How_to_use_veiled_coins_0"></a>

### How to use veiled coins


This module allows users to "register" a veiled account for any pre-existing <code>aptos_framework::Coin</code> type <code>T</code> via
the <code>register</code> entry function. For this, an encryption public key will need to be given as input, under which
the registered user's veiled balance will be encrypted.

Once Alice registers a veiled account for <code>T</code>, she can call <code>veil</code> with any public amount <code>a</code> of <code>T</code> coins
and add them to her veiled balance. Note that these coins will not be properly veiled yet, since they were withdrawn
from a public balance, which leaks their value.

(Alternatively, another user can initialize Alice's veiled balance by calling <code>veil_to</code>.)

Suppose Bob also registers and veils <code>b</code> of his own coins of type <code>T</code>.

Now Alice can use <code>fully_veiled_transfer</code> to send to Bob a secret amount <code>v</code> of coins from her veiled balance.
This will, for the first time, properly hide both Alice's and Bob's veiled balance.
The only information that an attacker (e.g., an Aptos validator) learns, is that Alice transferred an unknown amount
<code>v</code> to Bob (including $v=0$), and as a result Alice's veiled balance is in a range [a-v, a] and Bob's veiled balance
is in [b, b+v]<code>.

As more veiled transfers occur between more veiled accounts, the uncertainity on the balance of each <a href="../../../framework/aptos-framework/doc/account.md#0x1_account">account</a> becomes
larger and larger.

Lastly, users can easily withdraw veiled coins back into their <b>public</b> balance via </code>unveil<code>. Or, they can withdraw
publicly into someone <b>else</b>'s <b>public</b> balance via </code>unveil_to<code>.


<a name="@Terminology_1"></a>

### Terminology


1. *Veiled <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>*: a <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> whose value is secret; i.e., it is encrypted under the owner's <b>public</b> key.

2. *Veiled amount*: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a> amount that is secret because it was encrypted under some <b>public</b> key.
3. *Committed amount*: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a> amount that is secret because it was committed <b>to</b> (rather than encrypted).

4. *Veiled transaction*: a transaction that hides its amount transferred; i.e., a transaction whose amount is veiled.

5. *Veiled balance*: unlike a normal balance, a veiled balance is secret; i.e., it is encrypted under the <a href="../../../framework/aptos-framework/doc/account.md#0x1_account">account</a>'s
<b>public</b> key.

6. *ZKRP*: zero-knowledge range proofs; one of the key cryptographic ingredient in veiled coins which <b>ensures</b> users
can withdraw secretely from their veiled balance without over-withdrawing.


<a name="@Limitations_2"></a>

### Limitations


**WARNING:** This <b>module</b> is **experimental**! It is *NOT* production-ready. Specifically:

1. Deploying this <b>module</b> will likely lead <b>to</b> lost funds.
2. This <b>module</b> <b>has</b> not been cryptographically-audited.
3. The current implementation is vulnerable <b>to</b> _front-running attacks_ <b>as</b> described in the Zether paper [BAZB20].
4. There is no integration <b>with</b> wallet software which, for veiled accounts, must maintain an additional ElGamal
encryption keypair.
5. There is no support for rotating the ElGamal encryption <b>public</b> key of a veiled <a href="../../../framework/aptos-framework/doc/account.md#0x1_account">account</a>.


<a name="@Veiled_<a_href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>_amounts_<b>as</b>_truncated_</code>u32<code>'s_3"></a>

### Veiled <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> amounts <b>as</b> truncated </code>u32<code>'s


Veiled <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> amounts must be specified <b>as</b> </code>u32<code>'s rather than </code>u64<code>'s <b>as</b> would be typical for normal coins in the
Aptos framework. This is because <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> amounts must be encrypted <b>with</b> an *efficient*, additively-homomorphic encryption
scheme. Currently, our best candidate is ElGamal encryption in the exponent, which can only decrypt values around
32 bits or slightly larger.

Specifically, veiled <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> amounts are restricted <b>to</b> be 32 bits and can be cast <b>to</b> a normal 64-bit <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> value by
setting the leftmost and rightmost 16 bits <b>to</b> zero and the "middle" 32 bits <b>to</b> be the veiled <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> bits.

This gives veiled amounts ~10 bits for specifying ~3 decimals and ~22 bits for specifying whole amounts, which
limits veiled balances and veiled transfers <b>to</b> around 4 million coins. (See </code>coin.move<code> for how a normal 64-bit <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>
value gets interpreted <b>as</b> a decimal number.)

In order <b>to</b> convert a </code>u32<code> veiled <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> amount <b>to</b> a normal </code>u64<code> <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> amount, we have <b>to</b> shift it left by 16 bits.

</code>```
u64 normal coin amount format:
[ left    || middle  || right ]
[ 63 - 32 || 31 - 16 || 15 - 0]

u32 veiled coin amount format; we take the middle 32 bits from the u64 format above and store them in a u32:
[ middle ]
[ 31 - 0 ]
```

Recall that: A coin has a *decimal precision* $d$ (e.g., for <code>AptosCoin</code>, $d = 8$; see <code>initialize</code> in
<code><a href="../../../framework/aptos-framework/doc/aptos_coin.md#0x1_aptos_coin">aptos_coin</a>.<b>move</b></code>). This precision $d$ is used when displaying a <code>u64</code> amount, by dividing the amount by $10^d$.
For example, if the precision $d = 2$, then a <code>u64</code> amount of 505 coins displays as 5.05 coins.

For veiled coins, we can easily display a <code>u32</code> <code>Coin&lt;T&gt;</code> amount $v$ by:
1. Casting $v$ as a u64 and shifting this left by 16 bits, obtaining a 64-bit $v'$
2. Displaying $v'$ normally, by dividing it by $d$, which is the precision in <code>CoinInfo&lt;T&gt;</code>.


<a name="@Implementation_details_4"></a>

### Implementation details


This module leverages a so-called "resource account," which helps us mint a <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> from a
normal <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;T&gt;</code> by transferring this latter coin into a <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;T&gt;</code> stored in the
resource account.

Later on, when someone wants to convert their <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;T&gt;</code> into a normal <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;T&gt;</code>,
the resource account can be used to transfer out the normal from its coin store. Transferring out a coin like this
requires a <code><a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a></code> for the resource account, which the <code><a href="veiled_coin.md#0x1337_veiled_coin">veiled_coin</a></code> module can obtain via a <code>SignerCapability</code>.


<a name="@References_5"></a>

### References


[BAZB20] Zether: Towards Privacy in a Smart Contract World; by Bunz, Benedikt and Agrawal, Shashank and Zamani,
Mahdi and Boneh, Dan; in Financial Cryptography and Data Security; 2020


    -  [How to use veiled coins](#@How_to_use_veiled_coins_0)
    -  [Terminology](#@Terminology_1)
    -  [Limitations](#@Limitations_2)
    -  [Veiled <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a> amounts <b>as</b> truncated </code>u32<code>'s](#@Veiled_<a_href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>_amounts_<b>as</b>_truncated_</code>u32<code>'s_3)
    -  [Implementation details](#@Implementation_details_4)
    -  [References](#@References_5)
-  [Resource `VeiledCoinStore`](#0x1337_veiled_coin_VeiledCoinStore)
-  [Struct `DepositEvent`](#0x1337_veiled_coin_DepositEvent)
-  [Struct `WithdrawEvent`](#0x1337_veiled_coin_WithdrawEvent)
-  [Resource `VeiledCoinMinter`](#0x1337_veiled_coin_VeiledCoinMinter)
-  [Struct `VeiledCoin`](#0x1337_veiled_coin_VeiledCoin)
-  [Struct `TransferProof`](#0x1337_veiled_coin_TransferProof)
-  [Struct `WithdrawalProof`](#0x1337_veiled_coin_WithdrawalProof)
-  [Constants](#@Constants_6)
-  [Function `register`](#0x1337_veiled_coin_register)
-  [Function `veil_to`](#0x1337_veiled_coin_veil_to)
-  [Function `veil`](#0x1337_veiled_coin_veil)
-  [Function `unveil_to`](#0x1337_veiled_coin_unveil_to)
-  [Function `unveil`](#0x1337_veiled_coin_unveil)
-  [Function `fully_veiled_transfer`](#0x1337_veiled_coin_fully_veiled_transfer)
-  [Function `clamp_u64_to_u32_amount`](#0x1337_veiled_coin_clamp_u64_to_u32_amount)
-  [Function `cast_u32_to_u64_amount`](#0x1337_veiled_coin_cast_u32_to_u64_amount)
-  [Function `has_veiled_coin_store`](#0x1337_veiled_coin_has_veiled_coin_store)
-  [Function `veiled_amount`](#0x1337_veiled_coin_veiled_amount)
-  [Function `veiled_balance`](#0x1337_veiled_coin_veiled_balance)
-  [Function `encryption_public_key`](#0x1337_veiled_coin_encryption_public_key)
-  [Function `total_veiled_coins`](#0x1337_veiled_coin_total_veiled_coins)
-  [Function `get_veiled_coin_bulletproofs_dst`](#0x1337_veiled_coin_get_veiled_coin_bulletproofs_dst)
-  [Function `get_max_bits_in_veiled_coin_value`](#0x1337_veiled_coin_get_max_bits_in_veiled_coin_value)
-  [Function `register_internal`](#0x1337_veiled_coin_register_internal)
-  [Function `veiled_deposit`](#0x1337_veiled_coin_veiled_deposit)
-  [Function `unveil_to_internal`](#0x1337_veiled_coin_unveil_to_internal)
-  [Function `fully_veiled_transfer_internal`](#0x1337_veiled_coin_fully_veiled_transfer_internal)
-  [Function `verify_range_proofs`](#0x1337_veiled_coin_verify_range_proofs)


<pre><code><b>use</b> <a href="helpers.md#0x1337_helpers">0x1337::helpers</a>;
<b>use</b> <a href="sigma_protos.md#0x1337_sigma_protos">0x1337::sigma_protos</a>;
<b>use</b> <a href="../../../framework/aptos-framework/doc/account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../../framework/aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="">0x1::ristretto255_bulletproofs</a>;
<b>use</b> <a href="">0x1::ristretto255_elgamal</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen">0x1::ristretto255_pedersen</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a name="0x1337_veiled_coin_VeiledCoinStore"></a>

## Resource `VeiledCoinStore`

A holder of a specific coin type and its associated event handles.
These are kept in a single resource to ensure locality of data.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt; <b>has</b> key
</code></pre>



<a name="0x1337_veiled_coin_DepositEvent"></a>

## Struct `DepositEvent`

Event emitted when some amount of veiled coins were deposited into an account.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_DepositEvent">DepositEvent</a> <b>has</b> drop, store
</code></pre>



<a name="0x1337_veiled_coin_WithdrawEvent"></a>

## Struct `WithdrawEvent`

Event emitted when some amount of veiled coins were withdrawn from an account.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store
</code></pre>



<a name="0x1337_veiled_coin_VeiledCoinMinter"></a>

## Resource `VeiledCoinMinter`

Holds an <code><a href="../../../framework/aptos-framework/doc/account.md#0x1_account_SignerCapability">account::SignerCapability</a></code> for the resource account created when initializing this module. This
resource account houses a <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;T&gt;</code> for every type of coin <code>T</code> that is veiled.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinMinter">VeiledCoinMinter</a> <b>has</b> store, key
</code></pre>



<a name="0x1337_veiled_coin_VeiledCoin"></a>

## Struct `VeiledCoin`

Main structure representing a coin in an account's custody.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;
</code></pre>



<a name="0x1337_veiled_coin_TransferProof"></a>

## Struct `TransferProof`

A cryptographic proof that ensures correctness of a veiled-to-veiled coin transfer.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_TransferProof">TransferProof</a> <b>has</b> drop
</code></pre>



<a name="0x1337_veiled_coin_WithdrawalProof"></a>

## Struct `WithdrawalProof`

A cryptographic proof that ensures correctness of a veiled-to-*unveiled* coin transfer.


<pre><code><b>struct</b> <a href="veiled_coin.md#0x1337_veiled_coin_WithdrawalProof">WithdrawalProof</a> <b>has</b> drop
</code></pre>



<a name="@Constants_6"></a>

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

Failed deserializing bytes into either ElGamal ciphertext or $\Sigma$-protocol proof.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EDESERIALIZATION_FAILED">EDESERIALIZATION_FAILED</a>: u64 = 6;
</code></pre>



<a name="0x1337_veiled_coin_EINTERNAL_ERROR"></a>

Non-specific internal error (see source code)


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EINTERNAL_ERROR">EINTERNAL_ERROR</a>: u64 = 9;
</code></pre>



<a name="0x1337_veiled_coin_ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE"></a>

The range proof system does not support proofs for any number \in [0, 2^{32})


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>: u64 = 1;
</code></pre>



<a name="0x1337_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED"></a>

A range proof failed to verify.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_ERANGE_PROOF_VERIFICATION_FAILED">ERANGE_PROOF_VERIFICATION_FAILED</a>: u64 = 2;
</code></pre>



<a name="0x1337_veiled_coin_EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT"></a>

The <code><a href="veiled_coin.md#0x1337_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code> and <code><a href="veiled_coin.md#0x1337_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a></code> constants need to sum to 32 (bits).


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT">EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT</a>: u64 = 8;
</code></pre>



<a name="0x1337_veiled_coin_EVEILED_COIN_STORE_ALREADY_PUBLISHED"></a>

Account already has <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;</code> registered.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EVEILED_COIN_STORE_ALREADY_PUBLISHED">EVEILED_COIN_STORE_ALREADY_PUBLISHED</a>: u64 = 3;
</code></pre>



<a name="0x1337_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED"></a>

Account hasn't registered <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoinStore">VeiledCoinStore</a>&lt;CoinType&gt;</code>.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_EVEILED_COIN_STORE_NOT_PUBLISHED">EVEILED_COIN_STORE_NOT_PUBLISHED</a>: u64 = 4;
</code></pre>



<a name="0x1337_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE"></a>

The maximum number of bits used to represent a coin's value.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_MAX_BITS_IN_VEILED_COIN_VALUE">MAX_BITS_IN_VEILED_COIN_VALUE</a>: u64 = 32;
</code></pre>



<a name="0x1337_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED"></a>

When converting a <code>u64</code> normal (public) amount to a <code>u32</code> veiled amount, we keep the middle 32 bits and
remove the <code><a href="veiled_coin.md#0x1337_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code> least significant bits and the <code><a href="veiled_coin.md#0x1337_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a></code>
most significant bits (see comments in the beginning of this file).

When converting a <code>u32</code> veiled amount to a <code>u64</code> normal (public) amount, we simply cast it to <code>u64</code> and shift it
left by <code><a href="veiled_coin.md#0x1337_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code>.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a>: u8 = 16;
</code></pre>



<a name="0x1337_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED"></a>

See <code><a href="veiled_coin.md#0x1337_veiled_coin_NUM_LEAST_SIGNIFICANT_BITS_REMOVED">NUM_LEAST_SIGNIFICANT_BITS_REMOVED</a></code> comments.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_NUM_MOST_SIGNIFICANT_BITS_REMOVED">NUM_MOST_SIGNIFICANT_BITS_REMOVED</a>: u8 = 16;
</code></pre>



<a name="0x1337_veiled_coin_VEILED_COIN_BULLETPROOFS_DST"></a>

The domain separation tag (DST) used for the Bulletproofs prover.


<pre><code><b>const</b> <a href="veiled_coin.md#0x1337_veiled_coin_VEILED_COIN_BULLETPROOFS_DST">VEILED_COIN_BULLETPROOFS_DST</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 112, 116, 111, 115, 86, 101, 105, 108, 101, 100, 67, 111, 105, 110, 47, 66, 117, 108, 108, 101, 116, 112, 114, 111, 111, 102, 82, 97, 110, 103, 101, 80, 114, 111, 111, 102];
</code></pre>



<a name="0x1337_veiled_coin_register"></a>

## Function `register`

Initializes a veiled account for the specified <code>user</code> such that their balance is encrypted under public key <code>pk</code>.
Importantly, the user's wallet must retain their corresponding secret key.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_register">register</a>&lt;CoinType&gt;(user: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_veil_to"></a>

## Function `veil_to`

Sends a *public* <code>amount</code> of normal coins from <code>sender</code> to the <code>recipient</code>'s veiled balance.

**WARNING:** This function *leaks* the transferred <code>amount</code>, since it is given as a public input.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veil_to">veil_to</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32)
</code></pre>



<a name="0x1337_veiled_coin_veil"></a>

## Function `veil`

Like <code>veil_to</code>, except <code>owner</code> is both the sender and the recipient.

This function can be used by the <code>owner</code> to initialize his veiled balance to a *public* value.

**WARNING:** The initialized balance is *leaked*, since its initialized <code>amount</code> is public here.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veil">veil</a>&lt;CoinType&gt;(owner: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u32)
</code></pre>



<a name="0x1337_veiled_coin_unveil_to"></a>

## Function `unveil_to`

Takes a *public* <code>amount</code> of <code><a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">VeiledCoin</a>&lt;CoinType&gt;</code> coins from <code>sender</code>, unwraps them to a <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;</code>,
and sends them to <code>recipient</code>. Maintains secrecy of <code>sender</code>'s new balance.

Requires a ZK range proof on the new balance of the sender, to ensure the sender has enough money to send.
No ZK range proof is necessary for the <code>amount</code>, which is given as a public <code>u32</code> value.

**WARNING:** This *leaks* the transferred <code>amount</code>, since it is a public <code>u32</code> argument.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_unveil_to">unveil_to</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32, comm_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, withdraw_subproof: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_unveil"></a>

## Function `unveil`

Like <code>unveil_to</code>, except the <code>sender</code> is also the recipient.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_unveil">unveil</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u32, comm_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, withdraw_subproof: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_fully_veiled_transfer"></a>

## Function `fully_veiled_transfer`

Sends a *veiled* amount from <code>sender</code> to <code>recipient</code>. After this call, the veiled balances of both the <code>sender</code>
and the <code>recipient</code> remain (or become) secret.

The sent amount always remains secret; It is encrypted both under the sender's PK (in <code>withdraw_ct</code>) & under the
recipient's PK (in <code>deposit_ct</code>) using the *same* ElGamal randomness, so as to allow for efficiently updating both
the sender's & recipient's veiled balances. It is also committed under <code>comm_amount</code>, so as to allow for a ZK
range proof.

Requires a <code><a href="veiled_coin.md#0x1337_veiled_coin_TransferProof">TransferProof</a></code>; i.e.:
1. A range proof <code>zkrp_new_balance</code> on the new balance of the sender, to ensure the sender has enough money to
send.
2. A range proof <code>zkrp_amount</code> on the transferred amount in <code>comm_amount</code>, to ensure the sender won't create
coins out of thin air.
3. A $\Sigma$-protocol proof <code>transfer_subproof</code> which proves that 'withdraw_ct' encrypts the same veiled amount
as in 'deposit_ct' (with the same randomness) and as in <code>comm_amount</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_fully_veiled_transfer">fully_veiled_transfer</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, withdraw_ct: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, deposit_ct: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, comm_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, comm_amount: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_amount: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, transfer_subproof: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<a name="0x1337_veiled_coin_clamp_u64_to_u32_amount"></a>

## Function `clamp_u64_to_u32_amount`

Clamps a <code>u64</code> normal public amount to a <code>u32</code> to-be-veiled amount.

WARNING: Precision is lost here (see "Veiled coin amounts as truncated <code>u32</code>'s" in the top-level comments)


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_clamp_u64_to_u32_amount">clamp_u64_to_u32_amount</a>(amount: u64): u32
</code></pre>



<a name="0x1337_veiled_coin_cast_u32_to_u64_amount"></a>

## Function `cast_u32_to_u64_amount`

Casts a <code>u32</code> to-be-veiled amount to a <code>u64</code> normal public amount. No precision is lost here.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_cast_u32_to_u64_amount">cast_u32_to_u64_amount</a>(amount: u32): u64
</code></pre>



<a name="0x1337_veiled_coin_has_veiled_coin_store"></a>

## Function `has_veiled_coin_store`

Returns <code><b>true</b></code> if <code>addr</code> is registered to receive veiled coins of <code>CoinType</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_has_veiled_coin_store">has_veiled_coin_store</a>&lt;CoinType&gt;(addr: <b>address</b>): bool
</code></pre>



<a name="0x1337_veiled_coin_veiled_amount"></a>

## Function `veiled_amount`

Returns the ElGamal encryption of the value of <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veiled_amount">veiled_amount</a>&lt;CoinType&gt;(<a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>: &<a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;): &<a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>
</code></pre>



<a name="0x1337_veiled_coin_veiled_balance"></a>

## Function `veiled_balance`

Returns the ElGamal encryption of the veiled balance of <code>owner</code> for the provided <code>CoinType</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veiled_balance">veiled_balance</a>&lt;CoinType&gt;(owner: <b>address</b>): <a href="_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a>
</code></pre>



<a name="0x1337_veiled_coin_encryption_public_key"></a>

## Function `encryption_public_key`

Given an address <code>addr</code>, returns the ElGamal encryption public key associated with that address


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_encryption_public_key">encryption_public_key</a>&lt;CoinType&gt;(addr: <b>address</b>): <a href="_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>
</code></pre>



<a name="0x1337_veiled_coin_total_veiled_coins"></a>

## Function `total_veiled_coins`

Returns the total supply of veiled coins


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_total_veiled_coins">total_veiled_coins</a>&lt;CoinType&gt;(): u64
</code></pre>



<a name="0x1337_veiled_coin_get_veiled_coin_bulletproofs_dst"></a>

## Function `get_veiled_coin_bulletproofs_dst`

Returns the domain separation tag (DST) for constructing Bulletproof-based range proofs in this module.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_get_veiled_coin_bulletproofs_dst">get_veiled_coin_bulletproofs_dst</a>(): <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<a name="0x1337_veiled_coin_get_max_bits_in_veiled_coin_value"></a>

## Function `get_max_bits_in_veiled_coin_value`

Returns the maximum # of bits used to represent a veiled coin amount. Might differ than the 64 bits used to
represent normal <code>aptos_framework::coin::Coin</code> values.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_get_max_bits_in_veiled_coin_value">get_max_bits_in_veiled_coin_value</a>(): u64
</code></pre>



<a name="0x1337_veiled_coin_register_internal"></a>

## Function `register_internal`

Like <code>register</code>, but the public key has been parsed in a type-safe struct.
TODO: Do we want to require a PoK of the SK here?


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_register_internal">register_internal</a>&lt;CoinType&gt;(user: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="_CompressedPubkey">ristretto255_elgamal::CompressedPubkey</a>)
</code></pre>



<a name="0x1337_veiled_coin_veiled_deposit"></a>

## Function `veiled_deposit`

Deposits a veiled <code><a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a></code> at address <code>to_addr</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_veiled_deposit">veiled_deposit</a>&lt;CoinType&gt;(to_addr: <b>address</b>, <a href="../../../framework/aptos-framework/doc/coin.md#0x1_coin">coin</a>: <a href="veiled_coin.md#0x1337_veiled_coin_VeiledCoin">veiled_coin::VeiledCoin</a>&lt;CoinType&gt;)
</code></pre>



<a name="0x1337_veiled_coin_unveil_to_internal"></a>

## Function `unveil_to_internal`

Like <code>unveil_to</code>, except the proofs have been deserialized into type-safe structs.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_unveil_to_internal">unveil_to_internal</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u32, comm_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, withdrawal_proof: <a href="veiled_coin.md#0x1337_veiled_coin_WithdrawalProof">veiled_coin::WithdrawalProof</a>)
</code></pre>



<a name="0x1337_veiled_coin_fully_veiled_transfer_internal"></a>

## Function `fully_veiled_transfer_internal`

Like <code>fully_veiled_transfer</code>, except the ciphertext and proofs have been deserialized into type-safe structs.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_fully_veiled_transfer_internal">fully_veiled_transfer_internal</a>&lt;CoinType&gt;(sender: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient_addr: <b>address</b>, veiled_withdraw_amount: <a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>, veiled_deposit_amount: <a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>, comm_new_balance: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, comm_amount: <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, transfer_proof: &<a href="veiled_coin.md#0x1337_veiled_coin_TransferProof">veiled_coin::TransferProof</a>)
</code></pre>



<a name="0x1337_veiled_coin_verify_range_proofs"></a>

## Function `verify_range_proofs`

Verifies range proofs on the remaining balance of an account committed in <code>comm_new_balance</code> and, optionally, on
the transferred amount committed inside <code>comm_amount</code>.


<pre><code><b>public</b> <b>fun</b> <a href="veiled_coin.md#0x1337_veiled_coin_verify_range_proofs">verify_range_proofs</a>(comm_new_balance: &<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>, zkrp_new_balance: &<a href="_RangeProof">ristretto255_bulletproofs::RangeProof</a>, comm_amount: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255_pedersen.md#0x1_ristretto255_pedersen_Commitment">ristretto255_pedersen::Commitment</a>&gt;, zkrp_amount: &<a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="_RangeProof">ristretto255_bulletproofs::RangeProof</a>&gt;)
</code></pre>
