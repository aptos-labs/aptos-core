
<a id="0x7_helpers"></a>

# Module `0x7::helpers`



-  [Constants](#@Constants_0)
-  [Function `cut_vector`](#0x7_helpers_cut_vector)
-  [Function `get_veiled_balance_zero_ciphertext`](#0x7_helpers_get_veiled_balance_zero_ciphertext)
-  [Function `public_amount_to_veiled_balance`](#0x7_helpers_public_amount_to_veiled_balance)


<pre><code><b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal">0x1::ristretto255_elgamal</a>;
<b>use</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_helpers_EVECTOR_CUT_TOO_LARGE"></a>

Tried cutting out more elements than are in the vector via <code>cut_vector</code>.


<pre><code><b>const</b> <a href="helpers.md#0x7_helpers_EVECTOR_CUT_TOO_LARGE">EVECTOR_CUT_TOO_LARGE</a>: u64 = 1;
</code></pre>



<a id="0x7_helpers_cut_vector"></a>

## Function `cut_vector`

Given a vector <code>vec</code>, removes the last <code>cut_len</code> elements of <code>vec</code> and returns them in order. (This function
exists because we did not like the interface of <code>std::vector::trim</code>.)


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x7_helpers_cut_vector">cut_vector</a>&lt;T&gt;(vec: &<b>mut</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, cut_len: u64): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x7_helpers_cut_vector">cut_vector</a>&lt;T&gt;(vec: &<b>mut</b> <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, cut_len: u64): <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt; {
    <b>let</b> len = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(vec);
    <b>let</b> res = <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>assert</b>!(len &gt;= cut_len, <a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="helpers.md#0x7_helpers_EVECTOR_CUT_TOO_LARGE">EVECTOR_CUT_TOO_LARGE</a>));
    <b>while</b> (cut_len &gt; 0) {
        res.push_back(<a href="../../velor-framework/../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(vec));
        cut_len -= 1;
    };
    res.reverse();
    res
}
</code></pre>



</details>

<a id="0x7_helpers_get_veiled_balance_zero_ciphertext"></a>

## Function `get_veiled_balance_zero_ciphertext`

Returns an encryption of zero, without any randomness (i.e., $r=0$), under any ElGamal PK.


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x7_helpers_get_veiled_balance_zero_ciphertext">get_veiled_balance_zero_ciphertext</a>(): <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_CompressedCiphertext">ristretto255_elgamal::CompressedCiphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x7_helpers_get_veiled_balance_zero_ciphertext">get_veiled_balance_zero_ciphertext</a>(): elgamal::CompressedCiphertext {
    elgamal::ciphertext_from_compressed_points(
        <a href="../../velor-framework/../velor-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>(),
        <a href="../../velor-framework/../velor-stdlib/doc/ristretto255.md#0x1_ristretto255_point_identity_compressed">ristretto255::point_identity_compressed</a>()
    )
}
</code></pre>



</details>

<a id="0x7_helpers_public_amount_to_veiled_balance"></a>

## Function `public_amount_to_veiled_balance`

Returns an encryption of <code>amount</code>, without any randomness (i.e., $r=0$), under any ElGamal PK.
WARNING: This is not a proper ciphertext: the value <code>amount</code> can be easily bruteforced.


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x7_helpers_public_amount_to_veiled_balance">public_amount_to_veiled_balance</a>(amount: u32): <a href="../../velor-framework/../velor-stdlib/doc/ristretto255_elgamal.md#0x1_ristretto255_elgamal_Ciphertext">ristretto255_elgamal::Ciphertext</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="helpers.md#0x7_helpers_public_amount_to_veiled_balance">public_amount_to_veiled_balance</a>(amount: u32): elgamal::Ciphertext {
    <b>let</b> scalar = <a href="../../velor-framework/../velor-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u32">ristretto255::new_scalar_from_u32</a>(amount);

    elgamal::new_ciphertext_no_randomness(&scalar)
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
