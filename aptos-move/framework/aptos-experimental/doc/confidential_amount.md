
<a id="0x7_confidential_amount"></a>

# Module `0x7::confidential_amount`

A transfer amount encrypted under multiple keys, sharing P (commitment) components.

P = v*G + r*H encodes the amount; each R_* = r*ek_* allows decryption under that key.
This bundles the sender, recipient, effective-auditor, and voluntary-auditor R components
together with their shared P components.


-  [Struct `Amount`](#0x7_confidential_amount_Amount)
-  [Struct `CompressedAmount`](#0x7_confidential_amount_CompressedAmount)
-  [Constants](#@Constants_0)
-  [Function `assert_correct_num_chunks`](#0x7_confidential_amount_assert_correct_num_chunks)
-  [Function `new`](#0x7_confidential_amount_new)
-  [Function `new_compressed`](#0x7_confidential_amount_new_compressed)
-  [Function `new_compressed_from_bytes`](#0x7_confidential_amount_new_compressed_from_bytes)
-  [Function `get_compressed_P`](#0x7_confidential_amount_get_compressed_P)
-  [Function `get_compressed_R_sender`](#0x7_confidential_amount_get_compressed_R_sender)
-  [Function `get_compressed_R_recip`](#0x7_confidential_amount_get_compressed_R_recip)
-  [Function `get_compressed_R_eff_aud`](#0x7_confidential_amount_get_compressed_R_eff_aud)
-  [Function `get_compressed_R_volun_auds`](#0x7_confidential_amount_get_compressed_R_volun_auds)
-  [Function `num_volun_auditors_compressed`](#0x7_confidential_amount_num_volun_auditors_compressed)
-  [Function `has_effective_auditor_compressed`](#0x7_confidential_amount_has_effective_auditor_compressed)
-  [Function `compress`](#0x7_confidential_amount_compress)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x7_confidential_amount_Amount"></a>

## Struct `Amount`

Uncompressed transfer amount encrypted under multiple keys.


<pre><code><b>struct</b> <a href="confidential_amount.md#0x7_confidential_amount_Amount">Amount</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R_sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R_recip: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R_eff_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>R_volun_auds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_amount_CompressedAmount"></a>

## Struct `CompressedAmount`

Compressed transfer amount encrypted under multiple keys.


<pre><code><b>struct</b> <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>compressed_P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_R_sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_R_recip: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_R_eff_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_R_volun_auds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_amount_E_WRONG_NUM_CHUNKS"></a>

Expected the P, R_sender, or R_recip components to have exactly n chunks.


<pre><code><b>const</b> <a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>: u64 = 3;
</code></pre>



<a id="0x7_confidential_amount_E_WRONG_NUM_CHUNKS_FOR_EFFECTIVE_AUDITOR"></a>

Expected the effective auditor R-component to be either empty or have n chunks.


<pre><code><b>const</b> <a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS_FOR_EFFECTIVE_AUDITOR">E_WRONG_NUM_CHUNKS_FOR_EFFECTIVE_AUDITOR</a>: u64 = 1;
</code></pre>



<a id="0x7_confidential_amount_E_WRONG_NUM_CHUNKS_FOR_VOLUN_AUDITORS"></a>

Expected either all voluntary auditors' R-components to be empty or all to have n chunks.


<pre><code><b>const</b> <a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS_FOR_VOLUN_AUDITORS">E_WRONG_NUM_CHUNKS_FOR_VOLUN_AUDITORS</a>: u64 = 2;
</code></pre>



<a id="0x7_confidential_amount_assert_correct_num_chunks"></a>

## Function `assert_correct_num_chunks`



<pre><code><b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_assert_correct_num_chunks">assert_correct_num_chunks</a>&lt;T&gt;(p: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_recip: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_eff_aud: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_volun_auds: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_assert_correct_num_chunks">assert_correct_num_chunks</a>&lt;T&gt;(
    p: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_recip: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;,
    r_eff_aud: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;, r_volun_auds: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;T&gt;&gt;
) {
    <b>let</b> n = <a href="confidential_balance.md#0x7_confidential_balance_get_num_pending_chunks">confidential_balance::get_num_pending_chunks</a>();
    <b>assert</b>!(p.length() == n, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>));
    <b>assert</b>!(r_sender.length() == n, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>));
    <b>assert</b>!(r_recip.length() == n, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS">E_WRONG_NUM_CHUNKS</a>));
    <b>assert</b>!(r_eff_aud.is_empty() || r_eff_aud.length() == n, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(
        <a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS_FOR_EFFECTIVE_AUDITOR">E_WRONG_NUM_CHUNKS_FOR_EFFECTIVE_AUDITOR</a>
    ));
    <b>assert</b>!(
        r_volun_auds.all(|r| r.length() == n),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_amount.md#0x7_confidential_amount_E_WRONG_NUM_CHUNKS_FOR_VOLUN_AUDITORS">E_WRONG_NUM_CHUNKS_FOR_VOLUN_AUDITORS</a>)
    );
}
</code></pre>



</details>

<a id="0x7_confidential_amount_new"></a>

## Function `new`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_new">new</a>(_P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, _R_sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, _R_recip: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, _R_eff_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, _R_volun_auds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;&gt;): <a href="confidential_amount.md#0x7_confidential_amount_Amount">confidential_amount::Amount</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_new">new</a>(
    _P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    _R_sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    _R_recip: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    _R_eff_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    _R_volun_auds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;&gt;,
): <a href="confidential_amount.md#0x7_confidential_amount_Amount">Amount</a> {
    <a href="confidential_amount.md#0x7_confidential_amount_assert_correct_num_chunks">assert_correct_num_chunks</a>(&_P, &_R_sender, &_R_recip, &_R_eff_aud, &_R_volun_auds);
    <a href="confidential_amount.md#0x7_confidential_amount_Amount">Amount</a> { P: _P, R_sender: _R_sender, R_recip: _R_recip, R_eff_aud: _R_eff_aud, R_volun_auds: _R_volun_auds }
}
</code></pre>



</details>

<a id="0x7_confidential_amount_new_compressed"></a>

## Function `new_compressed`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_new_compressed">new_compressed</a>(compressed_P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, compressed_R_sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, compressed_R_recip: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, compressed_R_eff_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, compressed_R_volun_auds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;&gt;): <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_new_compressed">new_compressed</a>(
    compressed_P: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    compressed_R_sender: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    compressed_R_recip: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    compressed_R_eff_aud: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    compressed_R_volun_auds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;&gt;,
): <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a> {
    <a href="confidential_amount.md#0x7_confidential_amount_assert_correct_num_chunks">assert_correct_num_chunks</a>(&compressed_P, &compressed_R_sender, &compressed_R_recip, &compressed_R_eff_aud, &compressed_R_volun_auds);

    <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a> {
        compressed_P,
        compressed_R_sender,
        compressed_R_recip,
        compressed_R_eff_aud,
        compressed_R_volun_auds,
    }
}
</code></pre>



</details>

<a id="0x7_confidential_amount_new_compressed_from_bytes"></a>

## Function `new_compressed_from_bytes`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_new_compressed_from_bytes">new_compressed_from_bytes</a>(amount_P_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_sender_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_recip_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_eff_aud_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_volun_auds_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;): <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_new_compressed_from_bytes">new_compressed_from_bytes</a>(
    amount_P_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    amount_R_sender_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    amount_R_recip_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    amount_R_eff_aud_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    amount_R_volun_auds_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
): <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a> {
    <a href="confidential_amount.md#0x7_confidential_amount_new_compressed">new_compressed</a>(
        deserialize_compressed_points(amount_P_bytes),
        deserialize_compressed_points(amount_R_sender_bytes),
        deserialize_compressed_points(amount_R_recip_bytes),
        deserialize_compressed_points(amount_R_eff_aud_bytes),
        amount_R_volun_auds_bytes.map(|bytes| deserialize_compressed_points(bytes)),
    )
}
</code></pre>



</details>

<a id="0x7_confidential_amount_get_compressed_P"></a>

## Function `get_compressed_P`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_P">get_compressed_P</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_P">get_compressed_P</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; { &self.compressed_P }
</code></pre>



</details>

<a id="0x7_confidential_amount_get_compressed_R_sender"></a>

## Function `get_compressed_R_sender`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_sender">get_compressed_R_sender</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_sender">get_compressed_R_sender</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; { &self.compressed_R_sender }
</code></pre>



</details>

<a id="0x7_confidential_amount_get_compressed_R_recip"></a>

## Function `get_compressed_R_recip`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_recip">get_compressed_R_recip</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_recip">get_compressed_R_recip</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; { &self.compressed_R_recip }
</code></pre>



</details>

<a id="0x7_confidential_amount_get_compressed_R_eff_aud"></a>

## Function `get_compressed_R_eff_aud`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_eff_aud">get_compressed_R_eff_aud</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_eff_aud">get_compressed_R_eff_aud</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt; { &self.compressed_R_eff_aud }
</code></pre>



</details>

<a id="0x7_confidential_amount_get_compressed_R_volun_auds"></a>

## Function `get_compressed_R_volun_auds`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_volun_auds">get_compressed_R_volun_auds</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_get_compressed_R_volun_auds">get_compressed_R_volun_auds</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a>): &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;&gt; { &self.compressed_R_volun_auds }
</code></pre>



</details>

<a id="0x7_confidential_amount_num_volun_auditors_compressed"></a>

## Function `num_volun_auditors_compressed`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_num_volun_auditors_compressed">num_volun_auditors_compressed</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_num_volun_auditors_compressed">num_volun_auditors_compressed</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a>): u64 {
    self.compressed_R_volun_auds.length()
}
</code></pre>



</details>

<a id="0x7_confidential_amount_has_effective_auditor_compressed"></a>

## Function `has_effective_auditor_compressed`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_has_effective_auditor_compressed">has_effective_auditor_compressed</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_has_effective_auditor_compressed">has_effective_auditor_compressed</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a>): bool {
    !self.compressed_R_eff_aud.is_empty()
}
</code></pre>



</details>

<a id="0x7_confidential_amount_compress"></a>

## Function `compress`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_compress">compress</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_Amount">confidential_amount::Amount</a>): <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_amount.md#0x7_confidential_amount_compress">compress</a>(self: &<a href="confidential_amount.md#0x7_confidential_amount_Amount">Amount</a>): <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a> {
    <a href="confidential_amount.md#0x7_confidential_amount_CompressedAmount">CompressedAmount</a> {
        compressed_P: self.P.map_ref(|p| p.point_compress()),
        compressed_R_sender: self.R_sender.map_ref(|r| r.point_compress()),
        compressed_R_recip: self.R_recip.map_ref(|r| r.point_compress()),
        compressed_R_eff_aud: self.R_eff_aud.map_ref(|r| r.point_compress()),
        compressed_R_volun_auds: self.R_volun_auds.map_ref(|rs| {
            rs.map_ref(|r: &RistrettoPoint| r.point_compress())
        }),
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
