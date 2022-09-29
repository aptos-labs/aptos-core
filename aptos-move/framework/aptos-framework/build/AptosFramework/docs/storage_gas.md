
<a name="0x1_storage_gas"></a>

# Module `0x1::storage_gas`



-  [Resource `StorageGas`](#0x1_storage_gas_StorageGas)
-  [Struct `Point`](#0x1_storage_gas_Point)
-  [Struct `UsageGasConfig`](#0x1_storage_gas_UsageGasConfig)
-  [Struct `GasCurve`](#0x1_storage_gas_GasCurve)
-  [Resource `StorageGasConfig`](#0x1_storage_gas_StorageGasConfig)
-  [Constants](#@Constants_0)
-  [Function `base_8192_exponential_curve`](#0x1_storage_gas_base_8192_exponential_curve)
-  [Function `new_point`](#0x1_storage_gas_new_point)
-  [Function `new_gas_curve`](#0x1_storage_gas_new_gas_curve)
-  [Function `new_usage_gas_config`](#0x1_storage_gas_new_usage_gas_config)
-  [Function `new_storage_gas_config`](#0x1_storage_gas_new_storage_gas_config)
-  [Function `set_config`](#0x1_storage_gas_set_config)
-  [Function `initialize`](#0x1_storage_gas_initialize)
-  [Function `validate_points`](#0x1_storage_gas_validate_points)
-  [Function `calculate_gas`](#0x1_storage_gas_calculate_gas)
-  [Function `interpolate`](#0x1_storage_gas_interpolate)
-  [Function `calculate_read_gas`](#0x1_storage_gas_calculate_read_gas)
-  [Function `calculate_create_gas`](#0x1_storage_gas_calculate_create_gas)
-  [Function `calculate_write_gas`](#0x1_storage_gas_calculate_write_gas)
-  [Function `on_reconfig`](#0x1_storage_gas_on_reconfig)


<pre><code><b>use</b> <a href="">0x1::error</a>;
<b>use</b> <a href="state_storage.md#0x1_state_storage">0x1::state_storage</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a name="0x1_storage_gas_StorageGas"></a>

## Resource `StorageGas`

This updates at reconfig and guarantees not to change elsewhere, safe
for gas calculation.

Specifically, it is updated by executing a reconfig transaction based
on the usage at the begining of the current epoch. The gas schedule
derived from these parameter will be for gas calculation of the entire
next epoch.
-- The data is one epoch older than ideal, but VM doesn't need to worry
about reloading gas parameters after the first txn of an epoch.


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>per_item_read: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>per_item_create: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>per_item_write: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>per_byte_read: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>per_byte_create: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>per_byte_write: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_storage_gas_Point"></a>

## Struct `Point`



<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>y: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<details>
<summary>Specification</summary>



<pre><code><b>invariant</b> x &lt;= <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;
<b>invariant</b> y &lt;= <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;
</code></pre>



</details>

<a name="0x1_storage_gas_UsageGasConfig"></a>

## Struct `UsageGasConfig`



<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>target_usage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>read_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a></code>
</dt>
<dd>

</dd>
<dt>
<code>create_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a></code>
</dt>
<dd>

</dd>
<dt>
<code>write_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<details>
<summary>Specification</summary>



<pre><code><b>invariant</b> target_usage &gt; 0;
<b>invariant</b> target_usage &lt;= <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;
</code></pre>




<pre><code><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework);
</code></pre>



</details>

<a name="0x1_storage_gas_GasCurve"></a>

## Struct `GasCurve`

The curve assumes there are two points (0, 0) and (10000, 10000) on both ends. Moreover, points must also
satisfy the following rules:
1. the x values must be strictly increasing and between (0, 10000);
2. the y values must be non-decreasing and between (0, 10000);
So the curve will be identified as point (0, 0) and (10000, 10000) interpolated with the points. The y value
between two points will be calculated by neighboring points as if there is a linear line connecting these two
points.


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>min_gas: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>max_gas: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>points: <a href="">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<details>
<summary>Specification</summary>



<pre><code><b>invariant</b> min_gas &lt;= max_gas;
<b>invariant</b> max_gas &lt;= <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;
<b>invariant</b> (len(points) &gt; 0 ==&gt; points[0].x &gt; 0)
    && (len(points) &gt; 0 ==&gt; points[len(points) - 1].x &lt; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>)
    && (<b>forall</b> i in 0..len(points) - 1: (points[i].x &lt; points[i + 1].x && points[i].y &lt;= points[i + 1].y));
</code></pre>



</details>

<a name="0x1_storage_gas_StorageGasConfig"></a>

## Resource `StorageGasConfig`



<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>item_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>byte_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_storage_gas_MAX_U64"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a name="0x1_storage_gas_BASIS_POINT_DENOMINATION"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>: u64 = 10000;
</code></pre>



<a name="0x1_storage_gas_EINVALID_GAS_RANGE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EINVALID_GAS_RANGE">EINVALID_GAS_RANGE</a>: u64 = 2;
</code></pre>



<a name="0x1_storage_gas_EINVALID_MONOTONICALLY_NON_DECREASING_CURVE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EINVALID_MONOTONICALLY_NON_DECREASING_CURVE">EINVALID_MONOTONICALLY_NON_DECREASING_CURVE</a>: u64 = 5;
</code></pre>



<a name="0x1_storage_gas_EINVALID_POINT_RANGE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EINVALID_POINT_RANGE">EINVALID_POINT_RANGE</a>: u64 = 6;
</code></pre>



<a name="0x1_storage_gas_ESTORAGE_GAS"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS">ESTORAGE_GAS</a>: u64 = 1;
</code></pre>



<a name="0x1_storage_gas_ESTORAGE_GAS_CONFIG"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS_CONFIG">ESTORAGE_GAS_CONFIG</a>: u64 = 0;
</code></pre>



<a name="0x1_storage_gas_ETARGET_USAGE_TOO_BIG"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_ETARGET_USAGE_TOO_BIG">ETARGET_USAGE_TOO_BIG</a>: u64 = 4;
</code></pre>



<a name="0x1_storage_gas_EZERO_TARGET_USAGE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EZERO_TARGET_USAGE">EZERO_TARGET_USAGE</a>: u64 = 3;
</code></pre>



<a name="0x1_storage_gas_base_8192_exponential_curve"></a>

## Function `base_8192_exponential_curve`

P(x) = min_price + (base ^ (utilization / target_usage) - 1) / (base - 1) * (max_price - min_price)


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(min_gas: u64, max_gas: u64): <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(min_gas: u64, max_gas: u64): <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> {
    <a href="storage_gas.md#0x1_storage_gas_new_gas_curve">new_gas_curve</a>(min_gas, max_gas,
        <a href="">vector</a>[
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(1000, 2),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(2000, 6),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(3000, 17),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(4000, 44),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(5000, 109),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(6000, 271),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(7000, 669),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(8000, 1648),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(9000, 4061),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(9500, 6372),
            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(9900, 9138),
        ]
    )
}
</code></pre>



</details>

<a name="0x1_storage_gas_new_point"></a>

## Function `new_point`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(x: u64, y: u64): <a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(x: u64, y: u64): <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> {
    <b>assert</b>!(
        x &lt;= <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> && y &lt;= <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>,
        <a href="_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_POINT_RANGE">EINVALID_POINT_RANGE</a>)
    );
    <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x, y }
}
</code></pre>



</details>

<a name="0x1_storage_gas_new_gas_curve"></a>

## Function `new_gas_curve`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_gas_curve">new_gas_curve</a>(min_gas: u64, max_gas: u64, points: <a href="">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;): <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_gas_curve">new_gas_curve</a>(min_gas: u64, max_gas: u64, points: <a href="">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a>&gt;): <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> {
    <b>assert</b>!(max_gas &gt;= min_gas, <a href="_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_GAS_RANGE">EINVALID_GAS_RANGE</a>));
    <b>assert</b>!(max_gas &lt;= <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, <a href="_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_GAS_RANGE">EINVALID_GAS_RANGE</a>));
    <a href="storage_gas.md#0x1_storage_gas_validate_points">validate_points</a>(&points);
    <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> {
        min_gas,
        max_gas,
        points
    }
}
</code></pre>



</details>

<a name="0x1_storage_gas_new_usage_gas_config"></a>

## Function `new_usage_gas_config`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_usage_gas_config">new_usage_gas_config</a>(target_usage: u64, read_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>, create_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>, write_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>): <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_usage_gas_config">new_usage_gas_config</a>(target_usage: u64, read_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>, create_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>, write_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>): <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> {
    <b>assert</b>!(target_usage &gt; 0, <a href="_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EZERO_TARGET_USAGE">EZERO_TARGET_USAGE</a>));
    <b>assert</b>!(target_usage &lt;= <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, <a href="_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_ETARGET_USAGE_TOO_BIG">ETARGET_USAGE_TOO_BIG</a>));
    <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> {
        target_usage,
        read_curve,
        create_curve,
        write_curve,
    }
}
</code></pre>



</details>

<a name="0x1_storage_gas_new_storage_gas_config"></a>

## Function `new_storage_gas_config`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_storage_gas_config">new_storage_gas_config</a>(item_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, byte_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>): <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_storage_gas_config">new_storage_gas_config</a>(item_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, byte_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>): <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> {
    <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> {
        item_config,
        byte_config
    }
}
</code></pre>



</details>

<a name="0x1_storage_gas_set_config"></a>

## Function `set_config`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_set_config">set_config</a>(aptos_framework: &<a href="">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_set_config">set_config</a>(aptos_framework: &<a href="">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>) <b>acquires</b> <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    *<b>borrow_global_mut</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework) = config;
}
</code></pre>



</details>

<a name="0x1_storage_gas_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_initialize">initialize</a>(aptos_framework: &<a href="">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_initialize">initialize</a>(aptos_framework: &<a href="">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework),
        <a href="_already_exists">error::already_exists</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS_CONFIG">ESTORAGE_GAS_CONFIG</a>)
    );

    <b>let</b> item_config = <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> {
        target_usage: 1000000000, // 1 billion
        read_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(80000, 80000 * 100),
        create_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(2000000, 2000000 * 100),
        write_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(400000, 400000 * 100),
    };
    <b>let</b> byte_config = <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> {
        target_usage: 500000000000, // 500 GB
        read_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(40, 40 * 100),
        create_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(1000, 1000 * 100),
        write_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(200, 200 * 100),
    };
    <b>move_to</b>(aptos_framework, <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> {
        item_config,
        byte_config,
    });

    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework),
        <a href="_already_exists">error::already_exists</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS">ESTORAGE_GAS</a>)
    );
    <b>move_to</b>(aptos_framework, <a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a> {
        per_item_read: 8000,
        per_item_create: 1280000,
        per_item_write: 160000,
        per_byte_read: 1000,
        per_byte_create: 10000,
        per_byte_write: 10000,
    });
}
</code></pre>



</details>

<a name="0x1_storage_gas_validate_points"></a>

## Function `validate_points`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_validate_points">validate_points</a>(points: &<a href="">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_validate_points">validate_points</a>(points: &<a href="">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a>&gt;) {
    <b>let</b> len = <a href="_length">vector::length</a>(points);
    <b>spec</b> {
        <b>assume</b> len &lt; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a>;
    };
    <b>let</b> i = 0;
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> <b>forall</b> j in 0..i: {
                <b>let</b> cur = <b>if</b> (j == 0) { <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: 0, y: 0 } } <b>else</b> { points[j - 1] };
                <b>let</b> next = <b>if</b> (j == len) { <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> } } <b>else</b> { points[j] };
                cur.x &lt; next.x && cur.y &lt;= next.y
            };
        };
        i &lt;= len
    }) {
        <b>let</b> cur = <b>if</b> (i == 0) { &<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: 0, y: 0 } } <b>else</b> { <a href="_borrow">vector::borrow</a>(points, i - 1) };
        <b>let</b> next = <b>if</b> (i == len) { &<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> } } <b>else</b> { <a href="_borrow">vector::borrow</a>(points, i) };
        <b>assert</b>!(cur.x &lt; next.x && cur.y &lt;= next.y, <a href="_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_MONOTONICALLY_NON_DECREASING_CURVE">EINVALID_MONOTONICALLY_NON_DECREASING_CURVE</a>));
        i = i + 1;
    }
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>exists</b> i in 0..len(points) - 1: (
    points[i].x &gt;= points[i + 1].x || points[i].y &gt; points[i + 1].y
);
<b>aborts_if</b> [abstract] len(points) &gt; 0 && points[0].x == 0;
<b>aborts_if</b> [abstract]  len(points) &gt; 0 && points[len(points) - 1].x == <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;
</code></pre>



</details>

<a name="0x1_storage_gas_calculate_gas"></a>

## Function `calculate_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(max_usage: u64, current_usage: u64, curve: &<a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(max_usage: u64, current_usage: u64, curve: &<a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>): u64 {
    <b>let</b> capped_current_usage = <b>if</b> (current_usage &gt; max_usage) max_usage <b>else</b> current_usage;
    <b>let</b> points = &curve.points;
    <b>let</b> num_points = <a href="_length">vector::length</a>(points);
    <b>let</b> current_usage_bps = capped_current_usage * <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> / max_usage;

    // Check the corner case that current_usage_bps drops before the first point.
    <b>let</b> (left, right) = <b>if</b> (num_points == 0) {
        (&<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: 0, y: 0 }, &<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> })
    } <b>else</b> <b>if</b> (current_usage_bps &lt; <a href="_borrow">vector::borrow</a>(points, 0).x) {
        (&<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: 0, y: 0 }, <a href="_borrow">vector::borrow</a>(points, 0))
    } <b>else</b> <b>if</b> (<a href="_borrow">vector::borrow</a>(points, num_points - 1).x &lt;= current_usage_bps) {
        (<a href="_borrow">vector::borrow</a>(points, num_points - 1), &<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> { x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> })
    } <b>else</b> {
        <b>let</b> (i, j) = (0, num_points - 2);
        <b>while</b> ({
            <b>spec</b> {
                <b>invariant</b> i &lt;= j;
                <b>invariant</b> j &lt; num_points - 1;
                <b>invariant</b> points[i].x &lt;= current_usage_bps;
                <b>invariant</b> current_usage_bps &lt; points[j + 1].x;
            };
            i &lt; j
        }) {
            <b>let</b> mid = j - (j - i) / 2;
            <b>if</b> (current_usage_bps &lt; <a href="_borrow">vector::borrow</a>(points, mid).x) {
                <b>spec</b> {
                    // j is strictly decreasing.
                    <b>assert</b> mid - 1 &lt; j;
                };
                j = mid - 1;
            } <b>else</b> {
                <b>spec</b> {
                    // i is strictly increasing.
                    <b>assert</b> i &lt; mid;
                };
                i = mid;
            };
        };
        (<a href="_borrow">vector::borrow</a>(points, i), <a href="_borrow">vector::borrow</a>(points, i + 1))
    };
    <b>let</b> y_interpolated = <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(left.x, right.x, left.y, right.y, current_usage_bps);
    <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(0, <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, curve.min_gas, curve.max_gas, y_interpolated)
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
<b>requires</b> max_usage &gt; 0;
<b>requires</b> max_usage &lt;= <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> [abstract] result == <a href="storage_gas.md#0x1_storage_gas_spec_calculate_gas">spec_calculate_gas</a>(max_usage, current_usage, curve);
</code></pre>



</details>

<a name="0x1_storage_gas_interpolate"></a>

## Function `interpolate`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64 {
    y0 + (x - x0) * (y1 - y0) / (x1 - x0)
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
<b>requires</b> x0 &lt; x1;
<b>requires</b> y0 &lt;= y1;
<b>requires</b> x0 &lt;= x && x &lt;= x1;
<b>requires</b> x1 * y1 &lt;= <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> y0 &lt;= result && result &lt;= y1;
</code></pre>



</details>

<a name="0x1_storage_gas_calculate_read_gas"></a>

## Function `calculate_read_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(config: &<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, usage: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(config: &<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, usage: u64): u64 {
    <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(config.target_usage, usage, &config.read_curve)
}
</code></pre>



</details>

<a name="0x1_storage_gas_calculate_create_gas"></a>

## Function `calculate_create_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(config: &<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, usage: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(config: &<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, usage: u64): u64 {
    <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(config.target_usage, usage, &config.create_curve)
}
</code></pre>



</details>

<a name="0x1_storage_gas_calculate_write_gas"></a>

## Function `calculate_write_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(config: &<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, usage: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(config: &<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, usage: u64): u64 {
    <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(config.target_usage, usage, &config.write_curve)
}
</code></pre>



</details>

<a name="0x1_storage_gas_on_reconfig"></a>

## Function `on_reconfig`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_on_reconfig">on_reconfig</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_on_reconfig">on_reconfig</a>() <b>acquires</b> <a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>, <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework),
        <a href="_not_found">error::not_found</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS_CONFIG">ESTORAGE_GAS_CONFIG</a>)
    );
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework),
        <a href="_not_found">error::not_found</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS">ESTORAGE_GAS</a>)
    );
    <b>let</b> (items, bytes) = <a href="state_storage.md#0x1_state_storage_current_items_and_bytes">state_storage::current_items_and_bytes</a>();
    <b>let</b> gas_config = <b>borrow_global</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);
    <b>let</b> gas = <b>borrow_global_mut</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework);
    gas.per_item_read = <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(&gas_config.item_config, items);
    gas.per_item_create = <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(&gas_config.item_config, items);
    gas.per_item_write = <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(&gas_config.item_config, items);
    gas.per_byte_read = <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(&gas_config.byte_config, bytes);
    gas.per_byte_create = <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(&gas_config.byte_config, bytes);
    gas.per_byte_write = <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(&gas_config.byte_config, bytes);
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>aborts_if</b> <b>false</b>;
</code></pre>




<a name="0x1_storage_gas_spec_calculate_gas"></a>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_spec_calculate_gas">spec_calculate_gas</a>(max_usage: u64, current_usage: u64, curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>): u64;
</code></pre>



</details>
