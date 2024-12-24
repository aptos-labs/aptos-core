
<a id="0x1_dkg_rounding"></a>

# Module `0x1::dkg_rounding`



-  [Struct `RoundingResult`](#0x1_dkg_rounding_RoundingResult)
-  [Resource `CurEpochRounding`](#0x1_dkg_rounding_CurEpochRounding)
-  [Resource `NextEpochRounding`](#0x1_dkg_rounding_NextEpochRounding)
-  [Function `on_reconfig_start`](#0x1_dkg_rounding_on_reconfig_start)
-  [Function `on_new_epoch`](#0x1_dkg_rounding_on_new_epoch)
-  [Function `rounding`](#0x1_dkg_rounding_rounding)
-  [Function `rounding_internal`](#0x1_dkg_rounding_rounding_internal)
-  [Function `get_total_weight`](#0x1_dkg_rounding_get_total_weight)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_dkg_rounding_RoundingResult"></a>

## Struct `RoundingResult`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ideal_total_weight: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reconstruct_threshold_default_path: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>reconstruct_threshold_fast_path: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_rounding_CurEpochRounding"></a>

## Resource `CurEpochRounding`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_CurEpochRounding">CurEpochRounding</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>rounding: <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">dkg_rounding::RoundingResult</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_rounding_NextEpochRounding"></a>

## Resource `NextEpochRounding`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>rounding: <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">dkg_rounding::RoundingResult</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_rounding_on_reconfig_start"></a>

## Function `on_reconfig_start`

When an async reconfig starts,
compute weights + threshold for the next validator set and store it on chain.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_on_reconfig_start">on_reconfig_start</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, reconstruct_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, fast_secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_on_reconfig_start">on_reconfig_start</a>(
    framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    secrecy_threshold_in_stake_ratio: FixedPoint64,
    reconstruct_threshold_in_stake_ratio: FixedPoint64,
    fast_secrecy_threshold_in_stake_ratio: Option&lt;FixedPoint64&gt;,
) <b>acquires</b> <a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a> {
    <b>let</b> rounding = <a href="dkg_rounding.md#0x1_dkg_rounding_rounding">rounding</a>(stakes, secrecy_threshold_in_stake_ratio, reconstruct_threshold_in_stake_ratio, fast_secrecy_threshold_in_stake_ratio);
    <b>if</b> (<b>exists</b>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a>&gt;(@aptos_framework)) {
        <b>move_from</b>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a>&gt;(@aptos_framework);
    };
    <b>move_to</b>(framework, <a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a> { rounding });
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_on_new_epoch"></a>

## Function `on_new_epoch`

Invoked when an async reconfig finishes.
Discard the rounding for the current epoch, mark the rounding for the next epoch as "current".


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="dkg_rounding.md#0x1_dkg_rounding_CurEpochRounding">CurEpochRounding</a>, <a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_CurEpochRounding">CurEpochRounding</a>&gt;(@aptos_framework)) {
        <b>move_from</b>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_CurEpochRounding">CurEpochRounding</a>&gt;(@aptos_framework);
    };
    <b>if</b> (<b>exists</b>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a>&gt;(@aptos_framework)) {
        <b>let</b> <a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a> { rounding} = <b>move_from</b>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_NextEpochRounding">NextEpochRounding</a>&gt;(@aptos_framework);
        <b>move_to</b>(framework, <a href="dkg_rounding.md#0x1_dkg_rounding_CurEpochRounding">CurEpochRounding</a> { rounding })
    }
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_rounding"></a>

## Function `rounding`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding">rounding</a>(stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, reconstruct_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, fast_secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>&gt;): <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">dkg_rounding::RoundingResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding">rounding</a>(
    stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    secrecy_threshold_in_stake_ratio: FixedPoint64,
    reconstruct_threshold_in_stake_ratio: FixedPoint64,
    fast_secrecy_threshold_in_stake_ratio: Option&lt;FixedPoint64&gt;,
): <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a> {
    <b>let</b> fast_secrecy_thresh_raw = <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&fast_secrecy_threshold_in_stake_ratio)) {
        <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> fast_secrecy_threshold_in_stake_ratio))
    } <b>else</b> {
        0
    };
    <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_internal">rounding_internal</a>(
        stakes,
        <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(secrecy_threshold_in_stake_ratio),
        <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(reconstruct_threshold_in_stake_ratio),
        fast_secrecy_thresh_raw,
    )
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_rounding_internal"></a>

## Function `rounding_internal`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_internal">rounding_internal</a>(stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, secrecy_thresh_raw: u128, recon_thresh_raw: u128, fast_secrecy_thresh_raw: u128): <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">dkg_rounding::RoundingResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_internal">rounding_internal</a>(
    stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    secrecy_thresh_raw: u128,
    recon_thresh_raw: u128,
    fast_secrecy_thresh_raw: u128,
): <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a>;
</code></pre>



</details>

<a id="0x1_dkg_rounding_get_total_weight"></a>

## Function `get_total_weight`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_get_total_weight">get_total_weight</a>(result: &<a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">dkg_rounding::RoundingResult</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_get_total_weight">get_total_weight</a>(result: &<a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a>): u128 {
    <b>let</b> ret = 0;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(result.weights, |weight|{
        ret = ret + (weight <b>as</b> u128);
    });
    ret
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
