
<a id="0x1_storage_gas"></a>

# Module `0x1::storage_gas`

Gas parameters for global storage.


<a id="@General_overview_sections_0"></a>

## General overview sections


[Definitions](#definitions)

&#42; [Utilization dimensions](#utilization&#45;dimensions)
&#42; [Utilization ratios](#utilization&#45;ratios)
&#42; [Gas curve lookup](#gas&#45;curve&#45;lookup)
&#42; [Item&#45;wise operations](#item&#45;wise&#45;operations)
&#42; [Byte&#45;wise operations](#byte&#45;wise&#45;operations)

[Function dependencies](#function&#45;dependencies)

&#42; [Initialization](#initialization)
&#42; [Reconfiguration](#reconfiguration)
&#42; [Setting configurations](#setting&#45;configurations)


<a id="@Definitions_1"></a>

## Definitions



<a id="@Utilization_dimensions_2"></a>

### Utilization dimensions


Global storage gas fluctuates each epoch based on total utilization,
which is defined across two dimensions:

1. The number of &quot;items&quot; in global storage.
2. The number of bytes in global storage.

&quot;Items&quot; include:

1. Resources having the <code>key</code> attribute, which have been moved into
global storage via a <code><b>move_to</b>()</code> operation.
2.  Table entries.


<a id="@Utilization_ratios_3"></a>

### Utilization ratios


<code><a href="storage_gas.md#0x1_storage_gas_initialize">initialize</a>()</code> sets an arbitrary &quot;target&quot; utilization for both
item&#45;wise and byte&#45;wise storage, then each epoch, gas parameters are
reconfigured based on the &quot;utilization ratio&quot; for each of the two
utilization dimensions. The utilization ratio for a given dimension,
either item&#45;wise or byte&#45;wise, is taken as the quotient of actual
utilization and target utilization. For example, given a 500 GB
target and 250 GB actual utilization, the byte&#45;wise utilization
ratio is 50%.

See <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code> for mathematical definitions.


<a id="@Gas_curve_lookup_4"></a>

### Gas curve lookup


The utilization ratio in a given epoch is used as a lookup value in
a Eulerian approximation to an exponential curve, known as a
<code><a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a></code>, which is defined in <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code>,
based on a minimum gas charge and a maximum gas charge.

The minimum gas charge and maximum gas charge at the endpoints of
the curve are set in <code><a href="storage_gas.md#0x1_storage_gas_initialize">initialize</a>()</code>, and correspond to the following
operations defined in <code><a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a></code>:

1. Per&#45;item read
2. Per&#45;item create
3. Per&#45;item write
4. Per&#45;byte read
5. Per&#45;byte create
6. Per&#45;byte write

For example, if the byte&#45;wise utilization ratio is 50%, then
per&#45;byte reads will charge the minimum per&#45;byte gas cost, plus
1.09% of the difference between the maximum and the minimum cost.
See <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code> for a supporting calculation.


<a id="@Item&#45;wise_operations_5"></a>

### Item&#45;wise operations


1. Per&#45;item read gas is assessed whenever an item is read from
global storage via <code><b>borrow_global</b>&lt;T&gt;()</code> or via a table entry read
operation.
2. Per&#45;item create gas is assessed whenever an item is created in
global storage via <code><b>move_to</b>&lt;T&gt;()</code> or via a table entry creation
operation.
3. Per&#45;item write gas is assessed whenever an item is overwritten in
global storage via <code><b>borrow_global_mut</b>&lt;T&gt;</code> or via a table entry
mutation operation.


<a id="@Byte&#45;wise_operations_6"></a>

### Byte&#45;wise operations


Byte&#45;wise operations are assessed in a manner similar to per&#45;item
operations, but account for the number of bytes affected by the
given operation. Notably, this number denotes the total number of
bytes in an &#42;entire item&#42;.

For example, if an operation mutates a <code>u8</code> field in a resource that
has 5 other <code>u128</code> fields, the per&#45;byte gas write cost will account
for $(5 &#42; 128) / 8 &#43; 1 &#61; 81$ bytes. Vectors are similarly treated
as fields.


<a id="@Function_dependencies_7"></a>

## Function dependencies


The below dependency chart uses <code>mermaid.js</code> syntax, which can be
automatically rendered into a diagram (depending on the browser)
when viewing the documentation file generated from source code. If
a browser renders the diagrams with coloring that makes it difficult
to read, try a different browser.


<a id="@Initialization_8"></a>

### Initialization


```mermaid

flowchart LR

initialize &#45;&#45;&gt; base_8192_exponential_curve
base_8192_exponential_curve &#45;&#45;&gt; new_gas_curve
base_8192_exponential_curve &#45;&#45;&gt; new_point
new_gas_curve &#45;&#45;&gt; validate_points

```


<a id="@Reconfiguration_9"></a>

### Reconfiguration


```mermaid

flowchart LR

calculate_gas &#45;&#45;&gt; Interpolate %% capitalized
calculate_read_gas &#45;&#45;&gt; calculate_gas
calculate_create_gas &#45;&#45;&gt; calculate_gas
calculate_write_gas &#45;&#45;&gt; calculate_gas
on_reconfig &#45;&#45;&gt; calculate_read_gas
on_reconfig &#45;&#45;&gt; calculate_create_gas
on_reconfig &#45;&#45;&gt; calculate_write_gas
reconfiguration::reconfigure &#45;&#45;&gt; on_reconfig

```

Here, the function <code><a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>()</code> is spelled <code>Interpolate</code> because
<code>interpolate</code> is a reserved word in <code>mermaid.js</code>.


<a id="@Setting_configurations_10"></a>

### Setting configurations


```mermaid

flowchart LR

gas_schedule::set_storage_gas_config &#45;&#45;&gt; set_config

```


<a id="@Complete_docgen_index_11"></a>

## Complete docgen index


The below index is automatically generated from source code:


-  [General overview sections](#@General_overview_sections_0)
-  [Definitions](#@Definitions_1)
    -  [Utilization dimensions](#@Utilization_dimensions_2)
    -  [Utilization ratios](#@Utilization_ratios_3)
    -  [Gas curve lookup](#@Gas_curve_lookup_4)
    -  [Item&#45;wise operations](#@Item&#45;wise_operations_5)
    -  [Byte&#45;wise operations](#@Byte&#45;wise_operations_6)
-  [Function dependencies](#@Function_dependencies_7)
    -  [Initialization](#@Initialization_8)
    -  [Reconfiguration](#@Reconfiguration_9)
    -  [Setting configurations](#@Setting_configurations_10)
-  [Complete docgen index](#@Complete_docgen_index_11)
-  [Resource `StorageGas`](#0x1_storage_gas_StorageGas)
-  [Struct `Point`](#0x1_storage_gas_Point)
-  [Struct `UsageGasConfig`](#0x1_storage_gas_UsageGasConfig)
-  [Struct `GasCurve`](#0x1_storage_gas_GasCurve)
-  [Resource `StorageGasConfig`](#0x1_storage_gas_StorageGasConfig)
-  [Constants](#@Constants_12)
-  [Function `base_8192_exponential_curve`](#0x1_storage_gas_base_8192_exponential_curve)
    -  [Function definition](#@Function_definition_13)
    -  [Example](#@Example_14)
    -  [Utilization multipliers](#@Utilization_multipliers_15)
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
-  [Specification](#@Specification_16)
    -  [Struct `Point`](#@Specification_16_Point)
    -  [Struct `UsageGasConfig`](#@Specification_16_UsageGasConfig)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Struct `GasCurve`](#@Specification_16_GasCurve)
    -  [Function `base_8192_exponential_curve`](#@Specification_16_base_8192_exponential_curve)
    -  [Function `new_point`](#@Specification_16_new_point)
    -  [Function `new_gas_curve`](#@Specification_16_new_gas_curve)
    -  [Function `new_usage_gas_config`](#@Specification_16_new_usage_gas_config)
    -  [Function `new_storage_gas_config`](#@Specification_16_new_storage_gas_config)
    -  [Function `set_config`](#@Specification_16_set_config)
    -  [Function `initialize`](#@Specification_16_initialize)
    -  [Function `validate_points`](#@Specification_16_validate_points)
    -  [Function `calculate_gas`](#@Specification_16_calculate_gas)
    -  [Function `interpolate`](#@Specification_16_interpolate)
    -  [Function `on_reconfig`](#@Specification_16_on_reconfig)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="state_storage.md#0x1_state_storage">0x1::state_storage</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_storage_gas_StorageGas"></a>

## Resource `StorageGas`

Storage parameters, reconfigured each epoch.

Parameters are updated during reconfiguration via
<code><a href="storage_gas.md#0x1_storage_gas_on_reconfig">on_reconfig</a>()</code>, based on storage utilization at the beginning
of the epoch in which the reconfiguration transaction is
executed. The gas schedule derived from these parameters will
then be used to calculate gas for the entirety of the
following epoch, such that the data is one epoch older than
ideal. Notably, however, per this approach, the virtual machine
does not need to reload gas parameters after the
first transaction of an epoch.


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>per_item_read: u64</code>
</dt>
<dd>
 Cost to read an item from global storage.
</dd>
<dt>
<code>per_item_create: u64</code>
</dt>
<dd>
 Cost to create an item in global storage.
</dd>
<dt>
<code>per_item_write: u64</code>
</dt>
<dd>
 Cost to overwrite an item in global storage.
</dd>
<dt>
<code>per_byte_read: u64</code>
</dt>
<dd>
 Cost to read a byte from global storage.
</dd>
<dt>
<code>per_byte_create: u64</code>
</dt>
<dd>
 Cost to create a byte in global storage.
</dd>
<dt>
<code>per_byte_write: u64</code>
</dt>
<dd>
 Cost to overwrite a byte in global storage.
</dd>
</dl>


</details>

<a id="0x1_storage_gas_Point"></a>

## Struct `Point`

A point in a Eulerian curve approximation, with each coordinate
given in basis points:

&#124; Field value &#124; Percentage &#124;
&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;
&#124; <code>1</code>         &#124; 00.01 %    &#124;
&#124; <code>10</code>        &#124; 00.10 %    &#124;
&#124; <code>100</code>       &#124; 01.00 %    &#124;
&#124; <code>1000</code>      &#124; 10.00 %    &#124;


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x: u64</code>
</dt>
<dd>
 x&#45;coordinate basis points, corresponding to utilization
 ratio in <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code>.
</dd>
<dt>
<code>y: u64</code>
</dt>
<dd>
 y&#45;coordinate basis points, corresponding to utilization
 multiplier in <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code>.
</dd>
</dl>


</details>

<a id="0x1_storage_gas_UsageGasConfig"></a>

## Struct `UsageGasConfig`

A gas configuration for either per&#45;item or per&#45;byte costs.

Contains a target usage amount, as well as a Eulerian
approximation of an exponential curve for reads, creations, and
overwrites. See <code><a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a></code>.


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_storage_gas_GasCurve"></a>

## Struct `GasCurve`

Eulerian approximation of an exponential curve.

Assumes the following endpoints:

&#42; $(x_0, y_0) &#61; (0, 0)$
&#42; $(x_f, y_f) &#61; (10000, 10000)$

Intermediate points must satisfy:

1. $x_i &gt; x_&#123;i &#45; 1&#125;$ ( $x$ is strictly increasing).
2. $0 \leq x_i \leq 10000$ ( $x$ is between 0 and 10000).
3. $y_i \geq y_&#123;i &#45; 1&#125;$ ( $y$ is non&#45;decreasing).
4. $0 \leq y_i \leq 10000$ ( $y$ is between 0 and 10000).

Lookup between two successive points is calculated via linear
interpolation, e.g., as if there were a straight line between
them.

See <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code>.


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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
<code>points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_storage_gas_StorageGasConfig"></a>

## Resource `StorageGasConfig`

Gas configurations for per&#45;item and per&#45;byte prices.


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> <b>has</b> <b>copy</b>, drop, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>item_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a></code>
</dt>
<dd>
 Per&#45;item gas configuration.
</dd>
<dt>
<code>byte_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a></code>
</dt>
<dd>
 Per&#45;byte gas configuration.
</dd>
</dl>


</details>

<a id="@Constants_12"></a>

## Constants


<a id="0x1_storage_gas_MAX_U64"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a>: u64 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_storage_gas_BASIS_POINT_DENOMINATION"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>: u64 &#61; 10000;<br /></code></pre>



<a id="0x1_storage_gas_EINVALID_GAS_RANGE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EINVALID_GAS_RANGE">EINVALID_GAS_RANGE</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_storage_gas_EINVALID_MONOTONICALLY_NON_DECREASING_CURVE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EINVALID_MONOTONICALLY_NON_DECREASING_CURVE">EINVALID_MONOTONICALLY_NON_DECREASING_CURVE</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_storage_gas_EINVALID_POINT_RANGE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EINVALID_POINT_RANGE">EINVALID_POINT_RANGE</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_storage_gas_ESTORAGE_GAS"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS">ESTORAGE_GAS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_storage_gas_ESTORAGE_GAS_CONFIG"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS_CONFIG">ESTORAGE_GAS_CONFIG</a>: u64 &#61; 0;<br /></code></pre>



<a id="0x1_storage_gas_ETARGET_USAGE_TOO_BIG"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_ETARGET_USAGE_TOO_BIG">ETARGET_USAGE_TOO_BIG</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_storage_gas_EZERO_TARGET_USAGE"></a>



<pre><code><b>const</b> <a href="storage_gas.md#0x1_storage_gas_EZERO_TARGET_USAGE">EZERO_TARGET_USAGE</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_storage_gas_base_8192_exponential_curve"></a>

## Function `base_8192_exponential_curve`

Default exponential curve having base 8192.


<a id="@Function_definition_13"></a>

### Function definition


Gas price as a function of utilization ratio is defined as:

$$g(u_r) &#61; g_&#123;min&#125; &#43; \frac&#123;(b^&#123;u_r&#125; &#45; 1)&#125;&#123;b &#45; 1&#125; \Delta_g$$

$$g(u_r) &#61; g_&#123;min&#125; &#43; u_m \Delta_g$$

&#124; Variable                            &#124; Description            &#124;
&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;
&#124; $g_&#123;min&#125;$                           &#124; <code>min_gas</code>              &#124;
&#124; $g_&#123;max&#125;$                           &#124; <code>max_gas</code>              &#124;
&#124; $\Delta_&#123;g&#125; &#61; g_&#123;max&#125; &#45; g_&#123;min&#125;$    &#124; Gas delta              &#124;
&#124; $u$                                 &#124; Utilization            &#124;
&#124; $u_t$                               &#124; Target utilization     &#124;
&#124; $u_r &#61; u / u_t$                     &#124; Utilization ratio      &#124;
&#124; $u_m &#61; \frac&#123;(b^&#123;u_r&#125; &#45; 1)&#125;&#123;b &#45; 1&#125;$ &#124; Utilization multiplier &#124;
&#124; $b &#61; 8192$                          &#124; Exponent base          &#124;


<a id="@Example_14"></a>

### Example


Hence for a utilization ratio of 50% ( $u_r &#61; 0.5$ ):

$$g(0.5) &#61; g_&#123;min&#125; &#43; \frac&#123;8192^&#123;0.5&#125; &#45; 1&#125;&#123;8192 &#45; 1&#125; \Delta_g$$

$$g(0.5) \approx g_&#123;min&#125; &#43; 0.0109 \Delta_g$$

Which means that the price above <code>min_gas</code> is approximately
1.09% of the difference between <code>max_gas</code> and <code>min_gas</code>.


<a id="@Utilization_multipliers_15"></a>

### Utilization multipliers


&#124; $u_r$ &#124; $u_m$ (approximate) &#124;
&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;
&#124; 10%   &#124; 0.02%               &#124;
&#124; 20%   &#124; 0.06%               &#124;
&#124; 30%   &#124; 0.17%               &#124;
&#124; 40%   &#124; 0.44%               &#124;
&#124; 50%   &#124; 1.09%               &#124;
&#124; 60%   &#124; 2.71%               &#124;
&#124; 70%   &#124; 6.69%               &#124;
&#124; 80%   &#124; 16.48%              &#124;
&#124; 90%   &#124; 40.61%              &#124;
&#124; 95%   &#124; 63.72%              &#124;
&#124; 99%   &#124; 91.38%              &#124;


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(min_gas: u64, max_gas: u64): <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(min_gas: u64, max_gas: u64): <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> &#123;<br />    <a href="storage_gas.md#0x1_storage_gas_new_gas_curve">new_gas_curve</a>(min_gas, max_gas,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(1000, 2),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(2000, 6),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(3000, 17),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(4000, 44),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(5000, 109),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(6000, 271),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(7000, 669),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(8000, 1648),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(9000, 4061),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(9500, 6372),<br />            <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(9900, 9138),<br />        ]<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_new_point"></a>

## Function `new_point`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(x: u64, y: u64): <a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(x: u64, y: u64): <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123;<br />    <b>assert</b>!(<br />        x &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> &amp;&amp; y &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_POINT_RANGE">EINVALID_POINT_RANGE</a>)<br />    );<br />    <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x, y &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_new_gas_curve"></a>

## Function `new_gas_curve`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_gas_curve">new_gas_curve</a>(min_gas: u64, max_gas: u64, points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;): <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_gas_curve">new_gas_curve</a>(min_gas: u64, max_gas: u64, points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a>&gt;): <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> &#123;<br />    <b>assert</b>!(max_gas &gt;&#61; min_gas, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_GAS_RANGE">EINVALID_GAS_RANGE</a>));<br />    <b>assert</b>!(max_gas &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_GAS_RANGE">EINVALID_GAS_RANGE</a>));<br />    <a href="storage_gas.md#0x1_storage_gas_validate_points">validate_points</a>(&amp;points);<br />    <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> &#123;<br />        min_gas,<br />        max_gas,<br />        points<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_new_usage_gas_config"></a>

## Function `new_usage_gas_config`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_usage_gas_config">new_usage_gas_config</a>(target_usage: u64, read_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>, create_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>, write_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>): <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_usage_gas_config">new_usage_gas_config</a>(target_usage: u64, read_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>, create_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>, write_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>): <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> &#123;<br />    <b>assert</b>!(target_usage &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EZERO_TARGET_USAGE">EZERO_TARGET_USAGE</a>));<br />    <b>assert</b>!(target_usage &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_ETARGET_USAGE_TOO_BIG">ETARGET_USAGE_TOO_BIG</a>));<br />    <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> &#123;<br />        target_usage,<br />        read_curve,<br />        create_curve,<br />        write_curve,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_new_storage_gas_config"></a>

## Function `new_storage_gas_config`



<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_storage_gas_config">new_storage_gas_config</a>(item_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, byte_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>): <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_storage_gas_config">new_storage_gas_config</a>(item_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, byte_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>): <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> &#123;<br />    <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> &#123;<br />        item_config,<br />        byte_config<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_set_config"></a>

## Function `set_config`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_set_config">set_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_set_config">set_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>) <b>acquires</b> <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    &#42;<b>borrow_global_mut</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework) &#61; config;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_initialize"></a>

## Function `initialize`

Initialize per&#45;item and per&#45;byte gas prices.

Target utilization is set to 2 billion items and 1 TB.

<code><a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a></code> endpoints are initialized as follows:

&#124; Data style &#124; Operation &#124; Minimum gas &#124; Maximum gas &#124;
&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;
&#124; Per item   &#124; Read      &#124; 300K        &#124; 300K &#42; 100  &#124;
&#124; Per item   &#124; Create    &#124; 300k        &#124; 300k &#42; 100    &#124;
&#124; Per item   &#124; Write     &#124; 300K        &#124; 300K &#42; 100  &#124;
&#124; Per byte   &#124; Read      &#124; 300         &#124; 300 &#42; 100   &#124;
&#124; Per byte   &#124; Create    &#124; 5K          &#124; 5K &#42; 100    &#124;
&#124; Per byte   &#124; Write     &#124; 5K          &#124; 5K &#42; 100    &#124;

<code><a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a></code> values are additionally initialized, but per
<code><a href="storage_gas.md#0x1_storage_gas_on_reconfig">on_reconfig</a>()</code>, they will be reconfigured for each subsequent
epoch after initialization.

See <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code> fore more information on
target utilization.


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(<br />        !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS_CONFIG">ESTORAGE_GAS_CONFIG</a>)<br />    );<br /><br />    <b>let</b> k: u64 &#61; 1000;<br />    <b>let</b> m: u64 &#61; 1000 &#42; 1000;<br /><br />    <b>let</b> item_config &#61; <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> &#123;<br />        target_usage: 2 &#42; k &#42; m, // 2 billion<br />        read_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(300 &#42; k, 300 &#42; k &#42; 100),<br />        create_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(300 &#42; k, 300 &#42; k &#42; 100),<br />        write_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(300 &#42; k, 300 &#42; k &#42; 100),<br />    &#125;;<br />    <b>let</b> byte_config &#61; <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> &#123;<br />        target_usage: 1 &#42; m &#42; m, // 1TB<br />        read_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(300, 300 &#42; 100),<br />        create_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(5 &#42; k,  5 &#42; k &#42; 100),<br />        write_curve: <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(5 &#42; k,  5 &#42; k &#42; 100),<br />    &#125;;<br />    <b>move_to</b>(aptos_framework, <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> &#123;<br />        item_config,<br />        byte_config,<br />    &#125;);<br /><br />    <b>assert</b>!(<br />        !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS">ESTORAGE_GAS</a>)<br />    );<br />    <b>move_to</b>(aptos_framework, <a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a> &#123;<br />        per_item_read: 300 &#42; k,<br />        per_item_create: 5 &#42; m,<br />        per_item_write: 300 &#42; k,<br />        per_byte_read: 300,<br />        per_byte_create: 5 &#42; k,<br />        per_byte_write: 5 &#42; k,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_validate_points"></a>

## Function `validate_points`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_validate_points">validate_points</a>(points: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_validate_points">validate_points</a>(points: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a>&gt;) &#123;<br />    <b>let</b> len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(points);<br />    <b>spec</b> &#123;<br />        <b>assume</b> len &lt; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a>;<br />    &#125;;<br />    <b>let</b> i &#61; 0;<br />    <b>while</b> (&#123;<br />        <b>spec</b> &#123;<br />            <b>invariant</b> <b>forall</b> j in 0..i: &#123;<br />                <b>let</b> cur &#61; <b>if</b> (j &#61;&#61; 0) &#123; <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: 0, y: 0 &#125; &#125; <b>else</b> &#123; points[j &#45; 1] &#125;;<br />                <b>let</b> next &#61; <b>if</b> (j &#61;&#61; len) &#123; <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> &#125; &#125; <b>else</b> &#123; points[j] &#125;;<br />                cur.x &lt; next.x &amp;&amp; cur.y &lt;&#61; next.y<br />            &#125;;<br />        &#125;;<br />        i &lt;&#61; len<br />    &#125;) &#123;<br />        <b>let</b> cur &#61; <b>if</b> (i &#61;&#61; 0) &#123; &amp;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: 0, y: 0 &#125; &#125; <b>else</b> &#123; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, i &#45; 1) &#125;;<br />        <b>let</b> next &#61; <b>if</b> (i &#61;&#61; len) &#123; &amp;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> &#125; &#125; <b>else</b> &#123; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, i) &#125;;<br />        <b>assert</b>!(cur.x &lt; next.x &amp;&amp; cur.y &lt;&#61; next.y, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="storage_gas.md#0x1_storage_gas_EINVALID_MONOTONICALLY_NON_DECREASING_CURVE">EINVALID_MONOTONICALLY_NON_DECREASING_CURVE</a>));<br />        i &#61; i &#43; 1;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_calculate_gas"></a>

## Function `calculate_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(max_usage: u64, current_usage: u64, curve: &amp;<a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(max_usage: u64, current_usage: u64, curve: &amp;<a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>): u64 &#123;<br />    <b>let</b> capped_current_usage &#61; <b>if</b> (current_usage &gt; max_usage) max_usage <b>else</b> current_usage;<br />    <b>let</b> points &#61; &amp;curve.points;<br />    <b>let</b> num_points &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(points);<br />    <b>let</b> current_usage_bps &#61; capped_current_usage &#42; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> / max_usage;<br /><br />    // Check the corner case that current_usage_bps drops before the first point.<br />    <b>let</b> (left, right) &#61; <b>if</b> (num_points &#61;&#61; 0) &#123;<br />        (&amp;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: 0, y: 0 &#125;, &amp;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> &#125;)<br />    &#125; <b>else</b> <b>if</b> (current_usage_bps &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, 0).x) &#123;<br />        (&amp;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: 0, y: 0 &#125;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, 0))<br />    &#125; <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, num_points &#45; 1).x &lt;&#61; current_usage_bps) &#123;<br />        (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, num_points &#45; 1), &amp;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a> &#123; x: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, y: <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> &#125;)<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> (i, j) &#61; (0, num_points &#45; 2);<br />        <b>while</b> (&#123;<br />            <b>spec</b> &#123;<br />                <b>invariant</b> i &lt;&#61; j;<br />                <b>invariant</b> j &lt; num_points &#45; 1;<br />                <b>invariant</b> points[i].x &lt;&#61; current_usage_bps;<br />                <b>invariant</b> current_usage_bps &lt; points[j &#43; 1].x;<br />            &#125;;<br />            i &lt; j<br />        &#125;) &#123;<br />            <b>let</b> mid &#61; j &#45; (j &#45; i) / 2;<br />            <b>if</b> (current_usage_bps &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, mid).x) &#123;<br />                <b>spec</b> &#123;<br />                    // j is strictly decreasing.<br />                    <b>assert</b> mid &#45; 1 &lt; j;<br />                &#125;;<br />                j &#61; mid &#45; 1;<br />            &#125; <b>else</b> &#123;<br />                <b>spec</b> &#123;<br />                    // i is strictly increasing.<br />                    <b>assert</b> i &lt; mid;<br />                &#125;;<br />                i &#61; mid;<br />            &#125;;<br />        &#125;;<br />        (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, i), <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(points, i &#43; 1))<br />    &#125;;<br />    <b>let</b> y_interpolated &#61; <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(left.x, right.x, left.y, right.y, current_usage_bps);<br />    <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(0, <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>, curve.min_gas, curve.max_gas, y_interpolated)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_interpolate"></a>

## Function `interpolate`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64 &#123;<br />    y0 &#43; (x &#45; x0) &#42; (y1 &#45; y0) / (x1 &#45; x0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_calculate_read_gas"></a>

## Function `calculate_read_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(config: &amp;<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, usage: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(config: &amp;<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, usage: u64): u64 &#123;<br />    <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(config.target_usage, usage, &amp;config.read_curve)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_calculate_create_gas"></a>

## Function `calculate_create_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(config: &amp;<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, usage: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(config: &amp;<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, usage: u64): u64 &#123;<br />    <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(config.target_usage, usage, &amp;config.create_curve)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_calculate_write_gas"></a>

## Function `calculate_write_gas`



<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(config: &amp;<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, usage: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(config: &amp;<a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a>, usage: u64): u64 &#123;<br />    <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(config.target_usage, usage, &amp;config.write_curve)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_storage_gas_on_reconfig"></a>

## Function `on_reconfig`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_on_reconfig">on_reconfig</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_on_reconfig">on_reconfig</a>() <b>acquires</b> <a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>, <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a> &#123;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS_CONFIG">ESTORAGE_GAS_CONFIG</a>)<br />    );<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="storage_gas.md#0x1_storage_gas_ESTORAGE_GAS">ESTORAGE_GAS</a>)<br />    );<br />    <b>let</b> (items, bytes) &#61; <a href="state_storage.md#0x1_state_storage_current_items_and_bytes">state_storage::current_items_and_bytes</a>();<br />    <b>let</b> gas_config &#61; <b>borrow_global</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);<br />    <b>let</b> gas &#61; <b>borrow_global_mut</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework);<br />    gas.per_item_read &#61; <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(&amp;gas_config.item_config, items);<br />    gas.per_item_create &#61; <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(&amp;gas_config.item_config, items);<br />    gas.per_item_write &#61; <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(&amp;gas_config.item_config, items);<br />    gas.per_byte_read &#61; <a href="storage_gas.md#0x1_storage_gas_calculate_read_gas">calculate_read_gas</a>(&amp;gas_config.byte_config, bytes);<br />    gas.per_byte_create &#61; <a href="storage_gas.md#0x1_storage_gas_calculate_create_gas">calculate_create_gas</a>(&amp;gas_config.byte_config, bytes);<br />    gas.per_byte_write &#61; <a href="storage_gas.md#0x1_storage_gas_calculate_write_gas">calculate_write_gas</a>(&amp;gas_config.byte_config, bytes);<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_16"></a>

## Specification



<a id="0x1_storage_gas_spec_calculate_gas"></a>


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_spec_calculate_gas">spec_calculate_gas</a>(max_usage: u64, current_usage: u64, curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a>): u64;<br /></code></pre>




<a id="0x1_storage_gas_NewGasCurveAbortsIf"></a>


<pre><code><b>schema</b> <a href="storage_gas.md#0x1_storage_gas_NewGasCurveAbortsIf">NewGasCurveAbortsIf</a> &#123;<br />min_gas: u64;<br />max_gas: u64;<br /><b>aborts_if</b> max_gas &lt; min_gas;<br /><b>aborts_if</b> max_gas &gt; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br />&#125;<br /></code></pre>


A non decreasing curve must ensure that next is greater than cur.


<a id="0x1_storage_gas_ValidatePointsAbortsIf"></a>


<pre><code><b>schema</b> <a href="storage_gas.md#0x1_storage_gas_ValidatePointsAbortsIf">ValidatePointsAbortsIf</a> &#123;<br />points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">Point</a>&gt;;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
    <b>aborts_if</b> <b>exists</b> i in 0..len(points) &#45; 1: (<br />    points[i].x &gt;&#61; points[i &#43; 1].x &#124;&#124; points[i].y &gt; points[i &#43; 1].y<br />);<br /><b>aborts_if</b> len(points) &gt; 0 &amp;&amp; points[0].x &#61;&#61; 0;<br /><b>aborts_if</b> len(points) &gt; 0 &amp;&amp; points[len(points) &#45; 1].x &#61;&#61; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_16_Point"></a>

### Struct `Point`


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_Point">Point</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<dl>
<dt>
<code>x: u64</code>
</dt>
<dd>
 x&#45;coordinate basis points, corresponding to utilization
 ratio in <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code>.
</dd>
<dt>
<code>y: u64</code>
</dt>
<dd>
 y&#45;coordinate basis points, corresponding to utilization
 multiplier in <code><a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>()</code>.
</dd>
</dl>



<pre><code><b>invariant</b> x &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br /><b>invariant</b> y &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br /></code></pre>



<a id="@Specification_16_UsageGasConfig"></a>

### Struct `UsageGasConfig`


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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



<pre><code><b>invariant</b> target_usage &gt; 0;<br /><b>invariant</b> target_usage &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br /></code></pre>





<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The module&apos;s initialization guarantees the creation of the StorageGasConfig resource with a precise configuration, including accurate gas curves for per&#45;item and per&#45;byte operations.</td>
<td>Medium</td>
<td>The initialize function is responsible for setting up the initial state of the module, ensuring the fulfillment of the following conditions: (1) the creation of the StorageGasConfig resource, indicating its existence witqhin the module&apos;s context, and (2) the configuration of the StorageGasConfig resource includes the precise gas curves that define the behavior of per&#45;item and per&#45;byte operations.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>. Moreover, the native gas logic has been manually audited.</td>
</tr>

<tr>
<td>2</td>
<td>The gas curve approximates an exponential curve based on a minimum and maximum gas charge.</td>
<td>High</td>
<td>The validate_points function ensures that the provided vector of points represents a monotonically non&#45;decreasing curve.</td>
<td>Formally verified via <a href="#high-level-req-2">validate_points</a>. Moreover, the configuration logic has been manually audited.</td>
</tr>

<tr>
<td>3</td>
<td>The initialized gas curve structure has values set according to the provided parameters.</td>
<td>Low</td>
<td>The new_gas_curve function initializes the GasCurve structure with values provided as parameters.</td>
<td>Formally verified via <a href="#high-level-req-3">new_gas_curve</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The initialized usage gas configuration structure has values set according to the provided parameters.</td>
<td>Low</td>
<td>The new_usage_gas_config function initializes the UsageGasConfig structure with values provided as parameters.</td>
<td>Formally verified via <a href="#high-level-req-4">new_usage_gas_config</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_16_GasCurve"></a>

### Struct `GasCurve`


<pre><code><b>struct</b> <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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
<code>points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


Invariant 1: The minimum gas charge does not exceed the maximum gas charge.


<pre><code><b>invariant</b> min_gas &lt;&#61; max_gas;<br /></code></pre>


Invariant 2: The maximum gas charge is capped by MAX_U64 scaled down by the basis point denomination.


<pre><code><b>invariant</b> max_gas &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br /></code></pre>


Invariant 3: The x&#45;coordinate increases monotonically and the y&#45;coordinate increasing strictly monotonically,
that is, the gas&#45;curve is a monotonically increasing function.


<pre><code><b>invariant</b> (len(points) &gt; 0 &#61;&#61;&gt; points[0].x &gt; 0)<br />    &amp;&amp; (len(points) &gt; 0 &#61;&#61;&gt; points[len(points) &#45; 1].x &lt; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>)<br />    &amp;&amp; (<b>forall</b> i in 0..len(points) &#45; 1: (points[i].x &lt; points[i &#43; 1].x &amp;&amp; points[i].y &lt;&#61; points[i &#43; 1].y));<br /></code></pre>



<a id="@Specification_16_base_8192_exponential_curve"></a>

### Function `base_8192_exponential_curve`


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_base_8192_exponential_curve">base_8192_exponential_curve</a>(min_gas: u64, max_gas: u64): <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a><br /></code></pre>




<pre><code><b>include</b> <a href="storage_gas.md#0x1_storage_gas_NewGasCurveAbortsIf">NewGasCurveAbortsIf</a>;<br /></code></pre>



<a id="@Specification_16_new_point"></a>

### Function `new_point`


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_point">new_point</a>(x: u64, y: u64): <a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a><br /></code></pre>




<pre><code><b>aborts_if</b> x &gt; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a> &#124;&#124; y &gt; <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br /><b>ensures</b> result.x &#61;&#61; x;<br /><b>ensures</b> result.y &#61;&#61; y;<br /></code></pre>



<a id="@Specification_16_new_gas_curve"></a>

### Function `new_gas_curve`


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_gas_curve">new_gas_curve</a>(min_gas: u64, max_gas: u64, points: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;): <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a><br /></code></pre>


A non decreasing curve must ensure that next is greater than cur.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="storage_gas.md#0x1_storage_gas_NewGasCurveAbortsIf">NewGasCurveAbortsIf</a>;<br /><b>include</b> <a href="storage_gas.md#0x1_storage_gas_ValidatePointsAbortsIf">ValidatePointsAbortsIf</a>;<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> result &#61;&#61; <a href="storage_gas.md#0x1_storage_gas_GasCurve">GasCurve</a> &#123;<br />    min_gas,<br />    max_gas,<br />    points<br />&#125;;<br /></code></pre>



<a id="@Specification_16_new_usage_gas_config"></a>

### Function `new_usage_gas_config`


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_usage_gas_config">new_usage_gas_config</a>(target_usage: u64, read_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>, create_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>, write_curve: <a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>): <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a><br /></code></pre>




<pre><code><b>aborts_if</b> target_usage &#61;&#61; 0;<br /><b>aborts_if</b> target_usage &gt; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>ensures</b> result &#61;&#61; <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">UsageGasConfig</a> &#123;<br />    target_usage,<br />    read_curve,<br />    create_curve,<br />    write_curve,<br />&#125;;<br /></code></pre>



<a id="@Specification_16_new_storage_gas_config"></a>

### Function `new_storage_gas_config`


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_new_storage_gas_config">new_storage_gas_config</a>(item_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>, byte_config: <a href="storage_gas.md#0x1_storage_gas_UsageGasConfig">storage_gas::UsageGasConfig</a>): <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a><br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result.item_config &#61;&#61; item_config;<br /><b>ensures</b> result.byte_config &#61;&#61; byte_config;<br /></code></pre>



<a id="@Specification_16_set_config"></a>

### Function `set_config`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_set_config">set_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)<br /></code></pre>


Signer address must be @aptos_framework and StorageGasConfig exists.


<pre><code><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_16_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


Signer address must be @aptos_framework.
Address @aptos_framework does not exist StorageGasConfig and StorageGas before the function call is restricted
and exists after the function is executed.


<pre><code><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br /><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_16_validate_points"></a>

### Function `validate_points`


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_validate_points">validate_points</a>(points: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_gas.md#0x1_storage_gas_Point">storage_gas::Point</a>&gt;)<br /></code></pre>


A non decreasing curve must ensure that next is greater than cur.


<pre><code><b>pragma</b> aborts_if_is_strict &#61; <b>false</b>;<br /><b>pragma</b> verify &#61; <b>false</b>;<br /><b>pragma</b> opaque;<br /><b>include</b> <a href="storage_gas.md#0x1_storage_gas_ValidatePointsAbortsIf">ValidatePointsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_16_calculate_gas"></a>

### Function `calculate_gas`


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_calculate_gas">calculate_gas</a>(max_usage: u64, current_usage: u64, curve: &amp;<a href="storage_gas.md#0x1_storage_gas_GasCurve">storage_gas::GasCurve</a>): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>requires</b> max_usage &gt; 0;<br /><b>requires</b> max_usage &lt;&#61; <a href="storage_gas.md#0x1_storage_gas_MAX_U64">MAX_U64</a> / <a href="storage_gas.md#0x1_storage_gas_BASIS_POINT_DENOMINATION">BASIS_POINT_DENOMINATION</a>;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="storage_gas.md#0x1_storage_gas_spec_calculate_gas">spec_calculate_gas</a>(max_usage, current_usage, curve);<br /></code></pre>



<a id="@Specification_16_interpolate"></a>

### Function `interpolate`


<pre><code><b>fun</b> <a href="storage_gas.md#0x1_storage_gas_interpolate">interpolate</a>(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>pragma</b> intrinsic;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_16_on_reconfig"></a>

### Function `on_reconfig`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="storage_gas.md#0x1_storage_gas_on_reconfig">on_reconfig</a>()<br /></code></pre>


Address @aptos_framework must exist StorageGasConfig and StorageGas and StateStorageUsage.


<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">StorageGasConfig</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGas">StorageGas</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="state_storage.md#0x1_state_storage_StateStorageUsage">state_storage::StateStorageUsage</a>&gt;(@aptos_framework);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
