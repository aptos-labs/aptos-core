
<a id="0x7_ristretto255_twisted_elgamal"></a>

# Module `0x7::ristretto255_twisted_elgamal`

Twisted ElGamal encryption over Ristretto255.
EK = DK^(-1) * H. Ciphertext: C = v*G + r*H, D = r*EK. Decrypt: v*G = C - DK*D.


-  [Function `get_encryption_key_basepoint_compressed`](#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed)
-  [Function `get_encryption_key_basepoint`](#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
</code></pre>



<a id="0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed"></a>

## Function `get_encryption_key_basepoint_compressed`

Returns the compressed generator H used to derive the encryption key as EK = DK^(-1) * H.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">get_encryption_key_basepoint_compressed</a>(): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">get_encryption_key_basepoint_compressed</a>(): CompressedRistretto {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_H_compressed">ristretto255::basepoint_H_compressed</a>()
}
</code></pre>



</details>

<a id="0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint"></a>

## Function `get_encryption_key_basepoint`

Returns the decompressed generator H used to derive the encryption key as EK = DK^(-1) * H.


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint">get_encryption_key_basepoint</a>(): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint">get_encryption_key_basepoint</a>(): RistrettoPoint {
    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_hash_to_point_base">ristretto255::hash_to_point_base</a>()
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
