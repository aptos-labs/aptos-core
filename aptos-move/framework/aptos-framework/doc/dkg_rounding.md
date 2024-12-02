
<a id="0x1_dkg_rounding"></a>

# Module `0x1::dkg_rounding`



-  [Struct `WeightConfig`](#0x1_dkg_rounding_WeightConfig)
-  [Struct `RoundingResult`](#0x1_dkg_rounding_RoundingResult)
-  [Resource `CurEpochRounding`](#0x1_dkg_rounding_CurEpochRounding)
-  [Resource `NextEpochRounding`](#0x1_dkg_rounding_NextEpochRounding)
-  [Struct `ReconstructThresholdInfo`](#0x1_dkg_rounding_ReconstructThresholdInfo)
-  [Struct `Profile`](#0x1_dkg_rounding_Profile)
-  [Struct `Obj`](#0x1_dkg_rounding_Obj)
-  [Constants](#@Constants_0)
-  [Function `default_threshold_info`](#0x1_dkg_rounding_default_threshold_info)
-  [Function `default_profile`](#0x1_dkg_rounding_default_profile)
-  [Function `on_reconfig_start`](#0x1_dkg_rounding_on_reconfig_start)
-  [Function `on_new_epoch`](#0x1_dkg_rounding_on_new_epoch)
-  [Function `rounding`](#0x1_dkg_rounding_rounding)
-  [Function `compute_profile`](#0x1_dkg_rounding_compute_profile)
-  [Function `compute_threshold`](#0x1_dkg_rounding_compute_threshold)
-  [Function `rounding_v0`](#0x1_dkg_rounding_rounding_v0)
-  [Function `rounding_v0_internal`](#0x1_dkg_rounding_rounding_v0_internal)
-  [Function `get_total_weight`](#0x1_dkg_rounding_get_total_weight)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision">0x1::arbitrary_precision</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_dkg_rounding_WeightConfig"></a>

## Struct `WeightConfig`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_WeightConfig">WeightConfig</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reconsutruct_threshold: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_rounding_RoundingResult"></a>

## Struct `RoundingResult`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ideal_total_weight: u64</code>
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

<a id="0x1_dkg_rounding_ReconstructThresholdInfo"></a>

## Struct `ReconstructThresholdInfo`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>in_weights: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>in_stakes: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_rounding_Profile"></a>

## Struct `Profile`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">Profile</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ideal_total_weight: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>validator_weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>
 weight is a u64 because we assume <code>weight_per_stake &lt;= 1</code> and validator stake is a u64.
</dd>
<dt>
<code>threshold_default_path: <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">dkg_rounding::ReconstructThresholdInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>threshold_fast_path: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">dkg_rounding::ReconstructThresholdInfo</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_rounding_Obj"></a>

## Struct `Obj`



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_Obj">Obj</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>vid: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="stake.md#0x1_stake">stake</a>: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>weight_0: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>weight_1: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_dkg_rounding_BINARY_SEARCH_ERR_1"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_1">BINARY_SEARCH_ERR_1</a>: u64 = 0;
</code></pre>



<a id="0x1_dkg_rounding_BINARY_SEARCH_ERR_2"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_2">BINARY_SEARCH_ERR_2</a>: u64 = 0;
</code></pre>



<a id="0x1_dkg_rounding_BINARY_SEARCH_ERR_3"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_3">BINARY_SEARCH_ERR_3</a>: u64 = 0;
</code></pre>



<a id="0x1_dkg_rounding_BINARY_SEARCH_ERR_4"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_4">BINARY_SEARCH_ERR_4</a>: u64 = 0;
</code></pre>



<a id="0x1_dkg_rounding_BINARY_SEARCH_ERR_5"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_5">BINARY_SEARCH_ERR_5</a>: u64 = 0;
</code></pre>



<a id="0x1_dkg_rounding_E_FATAL"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_E_FATAL">E_FATAL</a>: u64 = 9999;
</code></pre>



<a id="0x1_dkg_rounding_ROUNDING_METHOD_BINARY_SEARCH"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_ROUNDING_METHOD_BINARY_SEARCH">ROUNDING_METHOD_BINARY_SEARCH</a>: u64 = 1;
</code></pre>



<a id="0x1_dkg_rounding_ROUNDING_METHOD_INFALLIBLE"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_ROUNDING_METHOD_INFALLIBLE">ROUNDING_METHOD_INFALLIBLE</a>: u64 = 2;
</code></pre>



<a id="0x1_dkg_rounding_default_threshold_info"></a>

## Function `default_threshold_info`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_default_threshold_info">default_threshold_info</a>(): <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">dkg_rounding::ReconstructThresholdInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_default_threshold_info">default_threshold_info</a>(): <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> {
    <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> {
        in_weights: 0,
        in_stakes: 0,
    }
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_default_profile"></a>

## Function `default_profile`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_default_profile">default_profile</a>(): <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">dkg_rounding::Profile</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_default_profile">default_profile</a>(): <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">Profile</a> {
    <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">Profile</a> {
        ideal_total_weight: 0,
        validator_weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        threshold_default_path: <a href="dkg_rounding.md#0x1_dkg_rounding_default_threshold_info">default_threshold_info</a>(),
        threshold_fast_path: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_on_reconfig_start"></a>

## Function `on_reconfig_start`

Invoked when an async reconfig starts.
Compute rounding for the next epoch and store it on chain.


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
Update discard the rounding for the current epoch, mark the rounding for the next epoch as "current".


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

Given a stake distribution, compute a weight distribution.


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
    <b>let</b> epsilon = <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>(1);
    <b>let</b> n = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&stakes);

    // Ensure reconstruct_threshold &gt; secrecy_threshold
    reconstruct_threshold_in_stake_ratio = <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_max">fixed_point64::max</a>(
        reconstruct_threshold_in_stake_ratio,
        <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_add">fixed_point64::add</a>(secrecy_threshold_in_stake_ratio, epsilon)
    );

    <b>let</b> secrecy_threshold_in_stake_ratio = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_fixed_point64">arbitrary_precision::from_fixed_point64</a>(secrecy_threshold_in_stake_ratio);
    <b>let</b> reconstruct_threshold_in_stake_ratio = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_fixed_point64">arbitrary_precision::from_fixed_point64</a>(reconstruct_threshold_in_stake_ratio);

    <b>let</b> total_weight_max = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_div_ceil">arbitrary_precision::div_ceil</a>(
        <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_sum">arbitrary_precision::sum</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u64">arbitrary_precision::from_u64</a>(n), <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u64">arbitrary_precision::from_u64</a>(4)]),
        <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_product">arbitrary_precision::product</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
            <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_sub">arbitrary_precision::sub</a>(reconstruct_threshold_in_stake_ratio, secrecy_threshold_in_stake_ratio),
            <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u64">arbitrary_precision::from_u64</a>(2),
        ]),
    );
    <b>let</b> stakes_total = 0;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(stakes, |<a href="stake.md#0x1_stake">stake</a>|{
        stakes_total = stakes_total + (<a href="stake.md#0x1_stake">stake</a> <b>as</b> u128);
    });
    <b>let</b> stakes_total = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u128">arbitrary_precision::from_u128</a>(stakes_total);

    <b>let</b> bar = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_as_u128">arbitrary_precision::as_u128</a>(
        <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_ceil">arbitrary_precision::ceil</a>(
            <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_product">arbitrary_precision::product</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[stakes_total, reconstruct_threshold_in_stake_ratio])));
    <b>let</b> fast_secrecy_threshold_in_stake_ratio = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_map">option::map</a>(fast_secrecy_threshold_in_stake_ratio, |v|<a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_fixed_point64">arbitrary_precision::from_fixed_point64</a>(v));

    <b>let</b> profile = <a href="dkg_rounding.md#0x1_dkg_rounding_default_profile">default_profile</a>();
    <b>let</b> lo = 0;
    <b>let</b> hi = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_as_u128">arbitrary_precision::as_u128</a>(total_weight_max) * 2;
    // <b>while</b> (lo + 1 &lt; hi) {
    <b>while</b> (<b>true</b>) {
        <b>let</b> md = lo + 1;
        <b>let</b> weight_per_stake = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_shift_down_by_bit">arbitrary_precision::shift_down_by_bit</a>(
            <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_div_ceil">arbitrary_precision::div_ceil</a>(
                <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_shift_up_by_bit">arbitrary_precision::shift_up_by_bit</a>(<a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u128">arbitrary_precision::from_u128</a>(md), 64),
                stakes_total,
            ),
            64,
        );
        <b>let</b> cur_profile = <a href="dkg_rounding.md#0x1_dkg_rounding_compute_profile">compute_profile</a>(secrecy_threshold_in_stake_ratio, fast_secrecy_threshold_in_stake_ratio, stakes, (md <b>as</b> u64), weight_per_stake);

        <b>if</b> (cur_profile.threshold_default_path.in_stakes &lt;= bar) {
            // hi = md;
            profile = cur_profile;
            <b>break</b>;
        } <b>else</b> {
            lo = md;
        };
    };

    <b>let</b> <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">Profile</a> {
        ideal_total_weight,
        validator_weights,
        threshold_default_path,
        threshold_fast_path,
    } = profile;

    <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a> {
        ideal_total_weight,
        weights: validator_weights,
        reconstruct_threshold_default_path: threshold_default_path.in_weights,
        reconstruct_threshold_fast_path: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_map">option::map</a>(threshold_fast_path, |t|{<b>let</b> t: <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> = t; t.in_weights}),
    }
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_compute_profile"></a>

## Function `compute_profile`


Now, a validator subset of stake ratio <code>r</code> has <code>weight_sub_total</code> in range <code>[stake_total * r * weight_per_stake - delta_down, stake_total * r * weight_per_stake + delta_up]</code>.
Therefore,
- the threshold in weight has to be set to <code>1 + floor(secrecy_threshold_in_stake_ratio * stake_total * weight_per_stake + delta_up)</code>;
- the stake ratio required for liveness is <code>secrecy_threshold_in_stake_ratio + (1 + delta_down + delta_up) / (take_total * weight_per_stake)</code>.
Note that as <code>weight_per_stake</code> increases, the <code>stake_ratio_required_for_liveness</code> decreases.
Further, when <code>weight_per_stake &gt;= (n + 2) / (2 * stake_total * (reconstruct_threshold_in_stake_ratio - secrecy_threshold_in_stake_ratio))</code>,
it is guaranteed that <code>stake_ratio_required_for_liveness &lt;= reconstruct_threshold_in_stake_ratio</code>.


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_profile">compute_profile</a>(secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, secrecy_threshold_in_stake_ratio_fast_path: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>&gt;, stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, ideal_total_weight: u64, weight_per_stake: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">dkg_rounding::Profile</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_profile">compute_profile</a>(
    secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>,
    secrecy_threshold_in_stake_ratio_fast_path: Option&lt;<a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>&gt;,
    stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    ideal_total_weight: u64,
    weight_per_stake: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>,
): <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">Profile</a> {
    <b>let</b> one = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u64">arbitrary_precision::from_u64</a>(1);
    <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_min_assign">arbitrary_precision::min_assign</a>(&<b>mut</b> weight_per_stake, one);

    // Initialize accumulators.
    <b>let</b> validator_weights = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> delta_down = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u64">arbitrary_precision::from_u64</a>(0);
    <b>let</b> delta_up = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u64">arbitrary_precision::from_u64</a>(0);
    <b>let</b> weight_total = 0;
    <b>let</b> stake_total = 0;

    // Assign weights.
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(stakes, |<a href="stake.md#0x1_stake">stake</a>|{
        <b>let</b> <a href="stake.md#0x1_stake">stake</a>: u64 = <a href="stake.md#0x1_stake">stake</a>;
        stake_total = stake_total + (<a href="stake.md#0x1_stake">stake</a> <b>as</b> u128);
        <b>let</b> ideal_weight = weight_per_stake;
        <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_mul_u64_assign">arbitrary_precision::mul_u64_assign</a>(&<b>mut</b> ideal_weight, <a href="stake.md#0x1_stake">stake</a>);
        <b>let</b> rounded_weight = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_round">arbitrary_precision::round</a>(ideal_weight, one);
        <b>let</b> rounded_weight_u64 = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_as_u64">arbitrary_precision::as_u64</a>(rounded_weight);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> validator_weights, rounded_weight_u64);
        weight_total = weight_total + (rounded_weight_u64 <b>as</b> u128);
        <b>if</b> (<a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_greater_than">arbitrary_precision::greater_than</a>(&ideal_weight, &rounded_weight)) {
            <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_add_assign">arbitrary_precision::add_assign</a>(&<b>mut</b> delta_down, <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_sub">arbitrary_precision::sub</a>(ideal_weight, rounded_weight));
        } <b>else</b> {
            <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_add_assign">arbitrary_precision::add_assign</a>(&<b>mut</b> delta_up, <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_sub">arbitrary_precision::sub</a>(rounded_weight, ideal_weight));
        };
    });

    // Compute default path thresholds.
    <b>let</b> threshold_default_path = <a href="dkg_rounding.md#0x1_dkg_rounding_compute_threshold">compute_threshold</a>(
        secrecy_threshold_in_stake_ratio,
        weight_per_stake,
        stake_total,
        weight_total,
        delta_up,
        delta_down,
    );

    <b>let</b> threshold_fast_path = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_map">option::map</a>(secrecy_threshold_in_stake_ratio_fast_path, |t|{
        <b>let</b> t: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a> = t;
        <a href="dkg_rounding.md#0x1_dkg_rounding_compute_threshold">compute_threshold</a>(
            t,
            weight_per_stake,
            stake_total,
            weight_total,
            delta_up,
            delta_down,
        )
    });

    <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">Profile</a> {
        ideal_total_weight,
        validator_weights,
        threshold_default_path,
        threshold_fast_path,
    }
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_compute_threshold"></a>

## Function `compute_threshold`

Once a **weight assignment** with <code>weight_per_stake</code> is done and <code>(weight_total, delta_up, delta_down)</code> are available,
return the minimum reconstruct threshold that satisfies a <code>secrecy_threshold_in_stake_ratio</code>.


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_threshold">compute_threshold</a>(secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, weight_per_stake: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, stake_total: u128, weight_total: u128, delta_up: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>, delta_down: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>): <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">dkg_rounding::ReconstructThresholdInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_threshold">compute_threshold</a>(
    secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>,
    weight_per_stake: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>,
    stake_total: u128,
    weight_total: u128,
    delta_up: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>,
    delta_down: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_Number">arbitrary_precision::Number</a>,
): <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> {
    <b>let</b> reconstruct_threshold_in_weights = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_sum">arbitrary_precision::sum</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_product">arbitrary_precision::product</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
            secrecy_threshold_in_stake_ratio,
            <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u128">arbitrary_precision::from_u128</a>(stake_total),
            weight_per_stake,
        ]),
        delta_up
    ]);
    <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_floor_assign">arbitrary_precision::floor_assign</a>(&<b>mut</b> reconstruct_threshold_in_weights);
    <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_add_assign">arbitrary_precision::add_assign</a>(&<b>mut</b> reconstruct_threshold_in_weights, <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u64">arbitrary_precision::from_u64</a>(1));
    <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_min_assign">arbitrary_precision::min_assign</a>(&<b>mut</b> reconstruct_threshold_in_weights, <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_from_u128">arbitrary_precision::from_u128</a>(weight_total));

    <b>let</b> reconstruct_threshold_in_stakes = <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_div_ceil">arbitrary_precision::div_ceil</a>(
        <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_sum">arbitrary_precision::sum</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[reconstruct_threshold_in_weights, delta_down]),
        weight_per_stake,
    );

    <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> {
        in_stakes: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_as_u128">arbitrary_precision::as_u128</a>(reconstruct_threshold_in_stakes),
        in_weights: <a href="../../aptos-stdlib/doc/arbitrary_precision.md#0x1_arbitrary_precision_as_u128">arbitrary_precision::as_u128</a>(reconstruct_threshold_in_weights),
    }
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_rounding_v0"></a>

## Function `rounding_v0`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_v0">rounding_v0</a>(stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, reconstruct_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>, fast_secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_FixedPoint64">fixed_point64::FixedPoint64</a>&gt;): <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">dkg_rounding::RoundingResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_v0">rounding_v0</a>(
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

    <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_v0_internal">rounding_v0_internal</a>(
        stakes,
        <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(secrecy_threshold_in_stake_ratio),
        <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_get_raw_value">fixed_point64::get_raw_value</a>(reconstruct_threshold_in_stake_ratio),
        fast_secrecy_thresh_raw,
    )
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_rounding_v0_internal"></a>

## Function `rounding_v0_internal`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_v0_internal">rounding_v0_internal</a>(stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, secrecy_thresh_raw: u128, recon_thresh_raw: u128, fast_secrecy_thresh_raw: u128): <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">dkg_rounding::RoundingResult</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_v0_internal">rounding_v0_internal</a>(
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
