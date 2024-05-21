
<a id="0x1_aptos_hash"></a>

# Module `0x1::aptos_hash`

Cryptographic hashes:
- Keccak-256: see https://keccak.team/keccak.html

In addition, SHA2-256 and SHA3-256 are available in <code>std::hash</code>. Note that SHA3-256 is a variant of Keccak: it is
NOT the same as Keccak-256.

Non-cryptograhic hashes:
- SipHash: an add-rotate-xor (ARX) based family of pseudorandom functions created by Jean-Philippe Aumasson and Daniel J. Bernstein in 2012


-  [Constants](#@Constants_0)
-  [Function `sip_hash`](#0x1_aptos_hash_sip_hash)
-  [Function `sip_hash_from_value`](#0x1_aptos_hash_sip_hash_from_value)
-  [Function `keccak256`](#0x1_aptos_hash_keccak256)
-  [Function `sha2_512`](#0x1_aptos_hash_sha2_512)
-  [Function `sha3_512`](#0x1_aptos_hash_sha3_512)
-  [Function `ripemd160`](#0x1_aptos_hash_ripemd160)
-  [Function `blake2b_256`](#0x1_aptos_hash_blake2b_256)
-  [Function `sha2_512_internal`](#0x1_aptos_hash_sha2_512_internal)
-  [Function `sha3_512_internal`](#0x1_aptos_hash_sha3_512_internal)
-  [Function `ripemd160_internal`](#0x1_aptos_hash_ripemd160_internal)
-  [Function `blake2b_256_internal`](#0x1_aptos_hash_blake2b_256_internal)
-  [Specification](#@Specification_1)
    -  [Function `sip_hash`](#@Specification_1_sip_hash)
    -  [Function `sip_hash_from_value`](#@Specification_1_sip_hash_from_value)
    -  [Function `keccak256`](#@Specification_1_keccak256)
    -  [Function `sha2_512`](#@Specification_1_sha2_512)
    -  [Function `sha3_512`](#@Specification_1_sha3_512)
    -  [Function `ripemd160`](#@Specification_1_ripemd160)
    -  [Function `blake2b_256`](#@Specification_1_blake2b_256)
    -  [Function `sha2_512_internal`](#@Specification_1_sha2_512_internal)
    -  [Function `sha3_512_internal`](#@Specification_1_sha3_512_internal)
    -  [Function `ripemd160_internal`](#@Specification_1_ripemd160_internal)
    -  [Function `blake2b_256_internal`](#@Specification_1_blake2b_256_internal)


<pre><code>use 0x1::bcs;
use 0x1::error;
use 0x1::features;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_aptos_hash_E_NATIVE_FUN_NOT_AVAILABLE"></a>

A newly-added native function is not yet enabled.


<pre><code>const E_NATIVE_FUN_NOT_AVAILABLE: u64 &#61; 1;
</code></pre>



<a id="0x1_aptos_hash_sip_hash"></a>

## Function `sip_hash`

Returns the (non-cryptographic) SipHash of <code>bytes</code>. See https://en.wikipedia.org/wiki/SipHash


<pre><code>public fun sip_hash(bytes: vector&lt;u8&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun sip_hash(bytes: vector&lt;u8&gt;): u64;
</code></pre>



</details>

<a id="0x1_aptos_hash_sip_hash_from_value"></a>

## Function `sip_hash_from_value`

Returns the (non-cryptographic) SipHash of the BCS serialization of <code>v</code>. See https://en.wikipedia.org/wiki/SipHash


<pre><code>public fun sip_hash_from_value&lt;MoveValue&gt;(v: &amp;MoveValue): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sip_hash_from_value&lt;MoveValue&gt;(v: &amp;MoveValue): u64 &#123;
    let bytes &#61; bcs::to_bytes(v);

    sip_hash(bytes)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_hash_keccak256"></a>

## Function `keccak256`

Returns the Keccak-256 hash of <code>bytes</code>.


<pre><code>public fun keccak256(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun keccak256(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_aptos_hash_sha2_512"></a>

## Function `sha2_512`

Returns the SHA2-512 hash of <code>bytes</code>.


<pre><code>public fun sha2_512(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sha2_512(bytes: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;
    if(!features::sha_512_and_ripemd_160_enabled()) &#123;
        abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    &#125;;

    sha2_512_internal(bytes)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_hash_sha3_512"></a>

## Function `sha3_512`

Returns the SHA3-512 hash of <code>bytes</code>.


<pre><code>public fun sha3_512(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sha3_512(bytes: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;
    if(!features::sha_512_and_ripemd_160_enabled()) &#123;
        abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    &#125;;

    sha3_512_internal(bytes)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_hash_ripemd160"></a>

## Function `ripemd160`

Returns the RIPEMD-160 hash of <code>bytes</code>.

WARNING: Only 80-bit security is provided by this function. This means an adversary who can compute roughly 2^80
hashes will, with high probability, find a collision x_1 != x_2 such that RIPEMD-160(x_1) = RIPEMD-160(x_2).


<pre><code>public fun ripemd160(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ripemd160(bytes: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;
    if(!features::sha_512_and_ripemd_160_enabled()) &#123;
        abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    &#125;;

    ripemd160_internal(bytes)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_hash_blake2b_256"></a>

## Function `blake2b_256`

Returns the BLAKE2B-256 hash of <code>bytes</code>.


<pre><code>public fun blake2b_256(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun blake2b_256(bytes: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;
    if(!features::blake2b_256_enabled()) &#123;
        abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
    &#125;;

    blake2b_256_internal(bytes)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_hash_sha2_512_internal"></a>

## Function `sha2_512_internal`

Returns the SHA2-512 hash of <code>bytes</code>.


<pre><code>fun sha2_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun sha2_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_aptos_hash_sha3_512_internal"></a>

## Function `sha3_512_internal`

Returns the SHA3-512 hash of <code>bytes</code>.


<pre><code>fun sha3_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun sha3_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_aptos_hash_ripemd160_internal"></a>

## Function `ripemd160_internal`

Returns the RIPEMD-160 hash of <code>bytes</code>.

WARNING: Only 80-bit security is provided by this function. This means an adversary who can compute roughly 2^80
hashes will, with high probability, find a collision x_1 != x_2 such that RIPEMD-160(x_1) = RIPEMD-160(x_2).


<pre><code>fun ripemd160_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun ripemd160_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_aptos_hash_blake2b_256_internal"></a>

## Function `blake2b_256_internal`

Returns the BLAKE2B-256 hash of <code>bytes</code>.


<pre><code>fun blake2b_256_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun blake2b_256_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<code>spec_sip_hash</code> is not assumed to be injective.


<a id="0x1_aptos_hash_spec_sip_hash"></a>


<pre><code>fun spec_sip_hash(bytes: vector&lt;u8&gt;): u64;
</code></pre>


<code>spec_keccak256</code> is an injective function.


<a id="0x1_aptos_hash_spec_keccak256"></a>


<pre><code>fun spec_keccak256(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
axiom forall b1: vector&lt;u8&gt;, b2: vector&lt;u8&gt;:
    (spec_keccak256(b1) &#61;&#61; spec_keccak256(b2) &#61;&#61;&gt; b1 &#61;&#61; b2);
</code></pre>


<code>spec_sha2_512_internal</code> is an injective function.


<a id="0x1_aptos_hash_spec_sha2_512_internal"></a>


<pre><code>fun spec_sha2_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
axiom forall b1: vector&lt;u8&gt;, b2: vector&lt;u8&gt;:
    (spec_sha2_512_internal(b1) &#61;&#61; spec_sha2_512_internal(b2) &#61;&#61;&gt; b1 &#61;&#61; b2);
</code></pre>


<code>spec_sha3_512_internal</code> is an injective function.


<a id="0x1_aptos_hash_spec_sha3_512_internal"></a>


<pre><code>fun spec_sha3_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
axiom forall b1: vector&lt;u8&gt;, b2: vector&lt;u8&gt;:
    (spec_sha3_512_internal(b1) &#61;&#61; spec_sha3_512_internal(b2) &#61;&#61;&gt; b1 &#61;&#61; b2);
</code></pre>


<code>spec_ripemd160_internal</code> is an injective function.


<a id="0x1_aptos_hash_spec_ripemd160_internal"></a>


<pre><code>fun spec_ripemd160_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
axiom forall b1: vector&lt;u8&gt;, b2: vector&lt;u8&gt;:
    (spec_ripemd160_internal(b1) &#61;&#61; spec_ripemd160_internal(b2) &#61;&#61;&gt; b1 &#61;&#61; b2);
</code></pre>


<code>spec_blake2b_256_internal</code> is an injective function.


<a id="0x1_aptos_hash_spec_blake2b_256_internal"></a>


<pre><code>fun spec_blake2b_256_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;;
axiom forall b1: vector&lt;u8&gt;, b2: vector&lt;u8&gt;:
    (spec_blake2b_256_internal(b1) &#61;&#61; spec_blake2b_256_internal(b2) &#61;&#61;&gt; b1 &#61;&#61; b2);
</code></pre>



<a id="@Specification_1_sip_hash"></a>

### Function `sip_hash`


<pre><code>public fun sip_hash(bytes: vector&lt;u8&gt;): u64
</code></pre>




<pre><code>pragma opaque;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_sip_hash(bytes);
</code></pre>



<a id="@Specification_1_sip_hash_from_value"></a>

### Function `sip_hash_from_value`


<pre><code>public fun sip_hash_from_value&lt;MoveValue&gt;(v: &amp;MoveValue): u64
</code></pre>




<pre><code>pragma opaque;
ensures result &#61;&#61; spec_sip_hash(bcs::serialize(v));
</code></pre>



<a id="@Specification_1_keccak256"></a>

### Function `keccak256`


<pre><code>public fun keccak256(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_keccak256(bytes);
</code></pre>



<a id="@Specification_1_sha2_512"></a>

### Function `sha2_512`


<pre><code>public fun sha2_512(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if !features::spec_is_enabled(features::SHA_512_AND_RIPEMD_160_NATIVES);
ensures result &#61;&#61; spec_sha2_512_internal(bytes);
</code></pre>



<a id="@Specification_1_sha3_512"></a>

### Function `sha3_512`


<pre><code>public fun sha3_512(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if !features::spec_is_enabled(features::SHA_512_AND_RIPEMD_160_NATIVES);
ensures result &#61;&#61; spec_sha3_512_internal(bytes);
</code></pre>



<a id="@Specification_1_ripemd160"></a>

### Function `ripemd160`


<pre><code>public fun ripemd160(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if !features::spec_is_enabled(features::SHA_512_AND_RIPEMD_160_NATIVES);
ensures result &#61;&#61; spec_ripemd160_internal(bytes);
</code></pre>



<a id="@Specification_1_blake2b_256"></a>

### Function `blake2b_256`


<pre><code>public fun blake2b_256(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if !features::spec_is_enabled(features::BLAKE2B_256_NATIVE);
ensures result &#61;&#61; spec_blake2b_256_internal(bytes);
</code></pre>



<a id="@Specification_1_sha2_512_internal"></a>

### Function `sha2_512_internal`


<pre><code>fun sha2_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_sha2_512_internal(bytes);
</code></pre>



<a id="@Specification_1_sha3_512_internal"></a>

### Function `sha3_512_internal`


<pre><code>fun sha3_512_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_sha3_512_internal(bytes);
</code></pre>



<a id="@Specification_1_ripemd160_internal"></a>

### Function `ripemd160_internal`


<pre><code>fun ripemd160_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_ripemd160_internal(bytes);
</code></pre>



<a id="@Specification_1_blake2b_256_internal"></a>

### Function `blake2b_256_internal`


<pre><code>fun blake2b_256_internal(bytes: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; spec_blake2b_256_internal(bytes);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
