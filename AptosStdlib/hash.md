
<a name="0x1_aptos_hash"></a>

# Module `0x1::aptos_hash`

Cryptographic hashes:
- Keccak-256: see https://keccak.team/keccak.html

In addition, SHA2-256 and SHA3-256 are available in <code>std::hash</code>. Note that SHA3-256 is a variant of Keccak: it is
NOT the same as Keccak-256.

Non-cryptograhic hashes:
- SipHash: an add-rotate-xor (ARX) based family of pseudorandom functions created by Jean-Philippe Aumasson and Daniel J. Bernstein in 2012


-  [Function `sip_hash`](#0x1_aptos_hash_sip_hash)
-  [Function `sip_hash_from_value`](#0x1_aptos_hash_sip_hash_from_value)
-  [Function `keccak256`](#0x1_aptos_hash_keccak256)


<pre><code><b>use</b> <a href="">0x1::bcs</a>;
</code></pre>



<a name="0x1_aptos_hash_sip_hash"></a>

## Function `sip_hash`



<pre><code><b>public</b> <b>fun</b> <a href="hash.md#0x1_aptos_hash_sip_hash">sip_hash</a>(bytes: <a href="">vector</a>&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="hash.md#0x1_aptos_hash_sip_hash">sip_hash</a>(bytes: <a href="">vector</a>&lt;u8&gt;): u64;
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
</code></pre>



</details>

<a name="0x1_aptos_hash_sip_hash_from_value"></a>

## Function `sip_hash_from_value`



<pre><code><b>public</b> <b>fun</b> <a href="hash.md#0x1_aptos_hash_sip_hash_from_value">sip_hash_from_value</a>&lt;MoveValue&gt;(v: &MoveValue): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="hash.md#0x1_aptos_hash_sip_hash_from_value">sip_hash_from_value</a>&lt;MoveValue&gt;(v: &MoveValue): u64 {
    <b>let</b> bytes = <a href="_to_bytes">bcs::to_bytes</a>(v);

    <a href="hash.md#0x1_aptos_hash_sip_hash">sip_hash</a>(bytes)
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
</code></pre>



</details>

<a name="0x1_aptos_hash_keccak256"></a>

## Function `keccak256`



<pre><code><b>public</b> <b>fun</b> <a href="hash.md#0x1_aptos_hash_keccak256">keccak256</a>(bytes: <a href="">vector</a>&lt;u8&gt;): <a href="">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="hash.md#0x1_aptos_hash_keccak256">keccak256</a>(bytes: <a href="">vector</a>&lt;u8&gt;): <a href="">vector</a>&lt;u8&gt;;
</code></pre>



</details>
