
<a id="0x1_dkg_rounding"></a>

# Module `0x1::dkg_rounding`



-  [Struct `WeightConfig`](#0x1_dkg_rounding_WeightConfig)
-  [Struct `RoundingResult`](#0x1_dkg_rounding_RoundingResult)
-  [Struct `ReconstructThresholdInfo`](#0x1_dkg_rounding_ReconstructThresholdInfo)
-  [Struct `Profile`](#0x1_dkg_rounding_Profile)
-  [Constants](#@Constants_0)
-  [Function `rounding`](#0x1_dkg_rounding_rounding)
-  [Function `binary_search`](#0x1_dkg_rounding_binary_search)
-  [Function `compute_profile`](#0x1_dkg_rounding_compute_profile)
-  [Function `compute_threshold`](#0x1_dkg_rounding_compute_threshold)
-  [Function `rounding_v0`](#0x1_dkg_rounding_rounding_v0)
-  [Function `get_total_weight`](#0x1_dkg_rounding_get_total_weight)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64">0x1::fixed_point64</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless">0x1::lossless</a>;
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



<pre><code><b>struct</b> <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a> <b>has</b> drop
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



<a id="0x1_dkg_rounding_ROUNDING_METHOD_BINARY_SEARCH"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_ROUNDING_METHOD_BINARY_SEARCH">ROUNDING_METHOD_BINARY_SEARCH</a>: u64 = 1;
</code></pre>



<a id="0x1_dkg_rounding_ROUNDING_METHOD_INFALLIBLE"></a>



<pre><code><b>const</b> <a href="dkg_rounding.md#0x1_dkg_rounding_ROUNDING_METHOD_INFALLIBLE">ROUNDING_METHOD_INFALLIBLE</a>: u64 = 2;
</code></pre>



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

    <b>let</b> total_weight_min = (n <b>as</b> u128);

    <b>let</b> n_div_2_plus_2_scaled = (n <b>as</b> u128) &lt;&lt;63 + 1&lt;&lt;65;
    <b>let</b> total_weight_max = <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_ceil">fixed_point64::ceil</a>(
        <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_raw_value">fixed_point64::create_from_raw_value</a>(
            <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_divide_u128">fixed_point64::divide_u128</a>(
                n_div_2_plus_2_scaled,
                <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_sub">fixed_point64::sub</a>(reconstruct_threshold_in_stake_ratio, secrecy_threshold_in_stake_ratio)
            )
        )
    );

    <b>let</b> stakes_total = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u128">lossless::from_u128</a>(0); //todo

    <b>let</b> secrecy_threshold_in_stake_ratio = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_fixed_point64">lossless::from_fixed_point64</a>(secrecy_threshold_in_stake_ratio);
    <b>let</b> reconstruct_threshold_in_stake_ratio = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_fixed_point64">lossless::from_fixed_point64</a>(reconstruct_threshold_in_stake_ratio);
    <b>let</b> fast_secrecy_threshold_in_stake_ratio = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_map">option::map</a>(fast_secrecy_threshold_in_stake_ratio, |v|<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_fixed_point64">lossless::from_fixed_point64</a>(v));

    <b>let</b> next_estimated_total_weight = total_weight_max;
    <b>let</b> delta = 1;
    <b>while</b> (next_estimated_total_weight &lt; total_weight_max) {
        next_estimated_total_weight = next_estimated_total_weight + delta;
        <b>let</b> weight_per_stake = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_div_ceil">lossless::div_ceil</a>(<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u128">lossless::from_u128</a>(next_estimated_total_weight), stakes_total);
        <b>let</b> profile = <a href="dkg_rounding.md#0x1_dkg_rounding_compute_profile">compute_profile</a>(secrecy_threshold_in_stake_ratio, fast_secrecy_threshold_in_stake_ratio, stakes, weight_per_stake);
        delta = delta * 2;
    };


    <b>let</b> default_path_config = <a href="dkg_rounding.md#0x1_dkg_rounding_WeightConfig">WeightConfig</a> {
        weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        reconsutruct_threshold: 0,
    };



    <a href="dkg_rounding.md#0x1_dkg_rounding_RoundingResult">RoundingResult</a> {
        weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        reconstruct_threshold_default_path: 0,
        reconstruct_threshold_fast_path: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    }
}
</code></pre>



</details>

<a id="0x1_dkg_rounding_binary_search"></a>

## Function `binary_search`



<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_binary_search">binary_search</a>(stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, total_weight_min: u128, total_weight_max: u128, secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>, reconstruct_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>, fast_secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>&gt;, default_path_config: &<b>mut</b> <a href="dkg_rounding.md#0x1_dkg_rounding_WeightConfig">dkg_rounding::WeightConfig</a>, fast_path_config: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_WeightConfig">dkg_rounding::WeightConfig</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_binary_search">binary_search</a>(
    stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    total_weight_min: u128,
    total_weight_max: u128,
    secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
    reconstruct_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
    fast_secrecy_threshold_in_stake_ratio: Option&lt;<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>&gt;,
    default_path_config: &<b>mut</b> <a href="dkg_rounding.md#0x1_dkg_rounding_WeightConfig">WeightConfig</a>,
    fast_path_config: &<b>mut</b> Option&lt;<a href="dkg_rounding.md#0x1_dkg_rounding_WeightConfig">WeightConfig</a>&gt;,
): u64 {
    <b>let</b> num_validators = (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&stakes) <b>as</b> u128);
    <b>if</b> (total_weight_min &lt; num_validators) {
        <b>return</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_1">BINARY_SEARCH_ERR_1</a>;
    };
    <b>if</b> (total_weight_max &lt; total_weight_min) {
        <b>return</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_2">BINARY_SEARCH_ERR_2</a>;
    };
    <b>if</b> (!<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_greater_than">lossless::greater_than</a>(&<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_product">lossless::product</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[secrecy_threshold_in_stake_ratio, <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u64">lossless::from_u64</a>(3)]), &<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u64">lossless::from_u64</a>(1))) {
        <b>return</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_3">BINARY_SEARCH_ERR_3</a>;
    };
    <b>if</b> (!<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_less_than">lossless::less_than</a>(&secrecy_threshold_in_stake_ratio, &reconstruct_threshold_in_stake_ratio)) {
        <b>return</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_4">BINARY_SEARCH_ERR_4</a>;
    };
    // <b>if</b> (lossless::greater(reconstruct_threshold_in_stake_ratio, <a href="../../aptos-stdlib/doc/fixed_point64.md#0x1_fixed_point64_create_from_rational">fixed_point64::create_from_rational</a>(2, 3))) {
    //     <b>return</b> <a href="dkg_rounding.md#0x1_dkg_rounding_BINARY_SEARCH_ERR_5">BINARY_SEARCH_ERR_5</a>;
    // };
    <b>let</b> stake_total = 0;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(stakes, |<a href="stake.md#0x1_stake">stake</a>| {
        <b>let</b> <a href="stake.md#0x1_stake">stake</a>: u64 = <a href="stake.md#0x1_stake">stake</a>;
        stake_total = stake_total + (<a href="stake.md#0x1_stake">stake</a> <b>as</b> u128);
    });

    <b>let</b> reconstruct_threshold_in_stakes = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_product">lossless::product</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u128">lossless::from_u128</a>(stake_total),
        reconstruct_threshold_in_stake_ratio,
    ]);
    <b>let</b> reconstruct_threshold_in_stakes = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_as_u128">lossless::as_u128</a>(reconstruct_threshold_in_stakes);

    <b>let</b> left = total_weight_min;
    <b>let</b> right = total_weight_max;
    <b>while</b> (left + 1 &lt; right) {
        <b>let</b> mid = (left + right) / 2;
        <b>let</b> weight_per_stake = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_div_ceil">lossless::div_ceil</a>(<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u128">lossless::from_u128</a>(mid), <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u128">lossless::from_u128</a>(stake_total));
        <b>let</b> profile = <a href="dkg_rounding.md#0x1_dkg_rounding_compute_profile">compute_profile</a>(
            secrecy_threshold_in_stake_ratio,
            fast_secrecy_threshold_in_stake_ratio,
            stakes,
            weight_per_stake,
        );
        <b>if</b> (profile.threshold_default_path.in_stakes &lt;= reconstruct_threshold_in_stakes) {
            right = mid;
        } <b>else</b> {
            left = mid;
        }
    };
    0
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


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_profile">compute_profile</a>(secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>, secrecy_threshold_in_stake_ratio_fast_path: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>&gt;, stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, weight_per_stake: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>): <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">dkg_rounding::Profile</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_profile">compute_profile</a>(
    secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
    secrecy_threshold_in_stake_ratio_fast_path: Option&lt;<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>&gt;,
    stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    weight_per_stake: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
): <a href="dkg_rounding.md#0x1_dkg_rounding_Profile">Profile</a> {
    <b>let</b> one = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u64">lossless::from_u64</a>(1);
    <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_min_assign">lossless::min_assign</a>(&<b>mut</b> weight_per_stake, one);

    // Initialize accumulators.
    <b>let</b> validator_weights = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> delta_down = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u64">lossless::from_u64</a>(0);
    <b>let</b> delta_up = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u64">lossless::from_u64</a>(0);
    <b>let</b> weight_total = 0;
    <b>let</b> stake_total = 0;

    // Assign weights.
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(stakes, |<a href="stake.md#0x1_stake">stake</a>|{
        <b>let</b> <a href="stake.md#0x1_stake">stake</a>: u64 = <a href="stake.md#0x1_stake">stake</a>;
        stake_total = stake_total + (<a href="stake.md#0x1_stake">stake</a> <b>as</b> u128);
        <b>let</b> ideal_weight = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_product">lossless::product</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u64">lossless::from_u64</a>(<a href="stake.md#0x1_stake">stake</a>), weight_per_stake]);
        <b>let</b> rounded_weight = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_round">lossless::round</a>(ideal_weight, one);
        <b>let</b> rounded_weight_u64 = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_as_u64">lossless::as_u64</a>(rounded_weight);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> validator_weights, rounded_weight_u64);
        weight_total = weight_total + (rounded_weight_u64 <b>as</b> u128);
        <b>if</b> (<a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_greater_than">lossless::greater_than</a>(&ideal_weight, &rounded_weight)) {
            <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_add_assign">lossless::add_assign</a>(&<b>mut</b> delta_down, <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_sub">lossless::sub</a>(ideal_weight, rounded_weight));
        } <b>else</b> {
            <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_add_assign">lossless::add_assign</a>(&<b>mut</b> delta_up, <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_sub">lossless::sub</a>(rounded_weight, ideal_weight));
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
        <b>let</b> t: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a> = t;
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


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_threshold">compute_threshold</a>(secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>, weight_per_stake: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>, stake_total: u128, weight_total: u128, delta_up: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>, delta_down: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>): <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">dkg_rounding::ReconstructThresholdInfo</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_compute_threshold">compute_threshold</a>(
    secrecy_threshold_in_stake_ratio: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
    weight_per_stake: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
    stake_total: u128,
    weight_total: u128,
    delta_up: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
    delta_down: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_Number">lossless::Number</a>,
): <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> {
    <b>let</b> reconstruct_threshold_in_weights = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_sum">lossless::sum</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_product">lossless::product</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
            secrecy_threshold_in_stake_ratio,
            <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u128">lossless::from_u128</a>(stake_total),
            weight_per_stake,
        ]),
        delta_up
    ]);
    <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_floor_assign">lossless::floor_assign</a>(&<b>mut</b> reconstruct_threshold_in_weights);
    <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_add_assign">lossless::add_assign</a>(&<b>mut</b> reconstruct_threshold_in_weights, <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u64">lossless::from_u64</a>(1));
    <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_min_assign">lossless::min_assign</a>(&<b>mut</b> reconstruct_threshold_in_weights, <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_from_u128">lossless::from_u128</a>(weight_total));

    <b>let</b> reconstruct_threshold_in_stakes = <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_div_ceil">lossless::div_ceil</a>(
        <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_sum">lossless::sum</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[reconstruct_threshold_in_weights, delta_down]),
        weight_per_stake,
    );

    <a href="dkg_rounding.md#0x1_dkg_rounding_ReconstructThresholdInfo">ReconstructThresholdInfo</a> {
        in_stakes: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_as_u128">lossless::as_u128</a>(reconstruct_threshold_in_stakes),
        in_weights: <a href="../../aptos-stdlib/doc/lossless.md#0x1_lossless_as_u128">lossless::as_u128</a>(reconstruct_threshold_in_weights),
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


<pre><code><b>native</b> <b>fun</b> <a href="dkg_rounding.md#0x1_dkg_rounding_rounding_v0">rounding_v0</a>(
    stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    secrecy_threshold_in_stake_ratio: FixedPoint64,
    reconstruct_threshold_in_stake_ratio: FixedPoint64,
    fast_secrecy_threshold_in_stake_ratio: Option&lt;FixedPoint64&gt;,
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
