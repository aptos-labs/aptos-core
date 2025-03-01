
<a name="0x1337_helpers"></a>

# Module `0x1337::helpers`



-  [Constants](#@Constants_0)
-  [Function `cut_vector`](#0x1337_helpers_cut_vector)
-  [Function `get_veiled_balance_zero_ciphertext`](#0x1337_helpers_get_veiled_balance_zero_ciphertext)
-  [Function `public_amount_to_veiled_balance`](#0x1337_helpers_public_amount_to_veiled_balance)


<pre><code><b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="">0x1::ristretto255_elgamal</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1337_helpers_EVECTOR_CUT_TOO_LARGE"></a>

Tried cutting out more elements than are in the vector via <code>cut_vector</code>.


<pre><code><b>const</b> <a href="helpers.md#0x1337_helpers_EVECTOR_CUT_TOO_LARGE">EVECTOR_CUT_TOO_LARGE</a>: u64 = 1;
</code></pre>



<a name="0x1337_helpers_cut_vector"></a>

## Function `cut_vector`

Given a vector <code>vec</code>, removes the last <code>cut_len</code> elements of <code>vec</code> and returns them in order. (This function
exists because we did not like the interface of <code>std::vector::trim</code>.)


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x1337_helpers_cut_vector">cut_vector</a>&lt;T&gt;(vec: &<b>mut</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, cut_len: u64): <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<a name="0x1337_helpers_get_veiled_balance_zero_ciphertext"></a>

## Function `get_veiled_balance_zero_ciphertext`

Returns an encryption of zero, without any randomness (i.e., $r=0$), under any ElGamal PK.


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x1337_helpers_get_veiled_balance_zero_ciphertext">get_veiled_balance_zero_ciphertext</a>(): <a href="_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a>
</code></pre>



<a name="0x1337_helpers_public_amount_to_veiled_balance"></a>

## Function `public_amount_to_veiled_balance`

Returns an encryption of <code>amount</code>, without any randomness (i.e., $r=0$), under any ElGamal PK.
WARNING: This is not a proper ciphertext: the value <code>amount</code> can be easily bruteforced.


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x1337_helpers_public_amount_to_veiled_balance">public_amount_to_veiled_balance</a>(amount: u32): <a href="_Ciphertext">ristretto255_elgamal::Ciphertext</a>
</code></pre>
