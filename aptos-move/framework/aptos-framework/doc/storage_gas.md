
<a id="0x1_storage_gas"></a>

# Module `0x1::storage_gas`

Gas parameters for global storage.<br/><br/> &#35; General overview sections<br/><br/> [Definitions](&#35;definitions)<br/><br/> &#42; [Utilization dimensions](&#35;utilization&#45;dimensions)<br/> &#42; [Utilization ratios](&#35;utilization&#45;ratios)<br/> &#42; [Gas curve lookup](&#35;gas&#45;curve&#45;lookup)<br/> &#42; [Item&#45;wise operations](&#35;item&#45;wise&#45;operations)<br/> &#42; [Byte&#45;wise operations](&#35;byte&#45;wise&#45;operations)<br/><br/> [Function dependencies](&#35;function&#45;dependencies)<br/><br/> &#42; [Initialization](&#35;initialization)<br/> &#42; [Reconfiguration](&#35;reconfiguration)<br/> &#42; [Setting configurations](&#35;setting&#45;configurations)<br/><br/> &#35; Definitions<br/><br/> &#35;&#35; Utilization dimensions<br/><br/> Global storage gas fluctuates each epoch based on total utilization,<br/> which is defined across two dimensions:<br/><br/> 1. The number of &quot;items&quot; in global storage.<br/> 2. The number of bytes in global storage.<br/><br/> &quot;Items&quot; include:<br/><br/> 1. Resources having the <code>key</code> attribute, which have been moved into<br/>    global storage via a <code>move_to()</code> operation.<br/> 2.  Table entries.<br/><br/> &#35;&#35; Utilization ratios<br/><br/> <code>initialize()</code> sets an arbitrary &quot;target&quot; utilization for both<br/> item&#45;wise and byte&#45;wise storage, then each epoch, gas parameters are<br/> reconfigured based on the &quot;utilization ratio&quot; for each of the two<br/> utilization dimensions. The utilization ratio for a given dimension,<br/> either item&#45;wise or byte&#45;wise, is taken as the quotient of actual<br/> utilization and target utilization. For example, given a 500 GB<br/> target and 250 GB actual utilization, the byte&#45;wise utilization<br/> ratio is 50%.<br/><br/> See <code>base_8192_exponential_curve()</code> for mathematical definitions.<br/><br/> &#35;&#35; Gas curve lookup<br/><br/> The utilization ratio in a given epoch is used as a lookup value in<br/> a Eulerian approximation to an exponential curve, known as a<br/> <code>GasCurve</code>, which is defined in <code>base_8192_exponential_curve()</code>,<br/> based on a minimum gas charge and a maximum gas charge.<br/><br/> The minimum gas charge and maximum gas charge at the endpoints of<br/> the curve are set in <code>initialize()</code>, and correspond to the following<br/> operations defined in <code>StorageGas</code>:<br/><br/> 1. Per&#45;item read<br/> 2. Per&#45;item create<br/> 3. Per&#45;item write<br/> 4. Per&#45;byte read<br/> 5. Per&#45;byte create<br/> 6. Per&#45;byte write<br/><br/> For example, if the byte&#45;wise utilization ratio is 50%, then<br/> per&#45;byte reads will charge the minimum per&#45;byte gas cost, plus<br/> 1.09% of the difference between the maximum and the minimum cost.<br/> See <code>base_8192_exponential_curve()</code> for a supporting calculation.<br/><br/> &#35;&#35; Item&#45;wise operations<br/><br/> 1. Per&#45;item read gas is assessed whenever an item is read from<br/>    global storage via <code>borrow_global&lt;T&gt;()</code> or via a table entry read<br/>    operation.<br/> 2. Per&#45;item create gas is assessed whenever an item is created in<br/>    global storage via <code>move_to&lt;T&gt;()</code> or via a table entry creation<br/>    operation.<br/> 3. Per&#45;item write gas is assessed whenever an item is overwritten in<br/>    global storage via <code>borrow_global_mut&lt;T&gt;</code> or via a table entry<br/>    mutation operation.<br/><br/> &#35;&#35; Byte&#45;wise operations<br/><br/> Byte&#45;wise operations are assessed in a manner similar to per&#45;item<br/> operations, but account for the number of bytes affected by the<br/> given operation. Notably, this number denotes the total number of<br/> bytes in an &#42;entire item&#42;.<br/><br/> For example, if an operation mutates a <code>u8</code> field in a resource that<br/> has 5 other <code>u128</code> fields, the per&#45;byte gas write cost will account<br/> for $(5 &#42; 128) / 8 &#43; 1 &#61; 81$ bytes. Vectors are similarly treated<br/> as fields.<br/><br/> &#35; Function dependencies<br/><br/> The below dependency chart uses <code>mermaid.js</code> syntax, which can be<br/> automatically rendered into a diagram (depending on the browser)<br/> when viewing the documentation file generated from source code. If<br/> a browser renders the diagrams with coloring that makes it difficult<br/> to read, try a different browser.<br/><br/> &#35;&#35; Initialization<br/><br/> ```mermaid<br/><br/> flowchart LR<br/><br/> initialize &#45;&#45;&gt; base_8192_exponential_curve<br/> base_8192_exponential_curve &#45;&#45;&gt; new_gas_curve<br/> base_8192_exponential_curve &#45;&#45;&gt; new_point<br/> new_gas_curve &#45;&#45;&gt; validate_points<br/><br/> ```<br/><br/> &#35;&#35; Reconfiguration<br/><br/> ```mermaid<br/><br/> flowchart LR<br/><br/> calculate_gas &#45;&#45;&gt; Interpolate %% capitalized<br/> calculate_read_gas &#45;&#45;&gt; calculate_gas<br/> calculate_create_gas &#45;&#45;&gt; calculate_gas<br/> calculate_write_gas &#45;&#45;&gt; calculate_gas<br/> on_reconfig &#45;&#45;&gt; calculate_read_gas<br/> on_reconfig &#45;&#45;&gt; calculate_create_gas<br/> on_reconfig &#45;&#45;&gt; calculate_write_gas<br/> reconfiguration::reconfigure &#45;&#45;&gt; on_reconfig<br/><br/> ```<br/><br/> Here, the function <code>interpolate()</code> is spelled <code>Interpolate</code> because<br/> <code>interpolate</code> is a reserved word in <code>mermaid.js</code>.<br/><br/> &#35;&#35; Setting configurations<br/><br/> ```mermaid<br/><br/> flowchart LR<br/><br/> gas_schedule::set_storage_gas_config &#45;&#45;&gt; set_config<br/><br/> ```<br/><br/> &#35; Complete docgen index<br/><br/> The below index is automatically generated from source code:


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
-  [Specification](#@Specification_1)
    -  [Struct `Point`](#@Specification_1_Point)
    -  [Struct `UsageGasConfig`](#@Specification_1_UsageGasConfig)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Struct `GasCurve`](#@Specification_1_GasCurve)
    -  [Function `base_8192_exponential_curve`](#@Specification_1_base_8192_exponential_curve)
    -  [Function `new_point`](#@Specification_1_new_point)
    -  [Function `new_gas_curve`](#@Specification_1_new_gas_curve)
    -  [Function `new_usage_gas_config`](#@Specification_1_new_usage_gas_config)
    -  [Function `new_storage_gas_config`](#@Specification_1_new_storage_gas_config)
    -  [Function `set_config`](#@Specification_1_set_config)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `validate_points`](#@Specification_1_validate_points)
    -  [Function `calculate_gas`](#@Specification_1_calculate_gas)
    -  [Function `interpolate`](#@Specification_1_interpolate)
    -  [Function `on_reconfig`](#@Specification_1_on_reconfig)


<pre><code>use 0x1::error;<br/>use 0x1::state_storage;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_storage_gas_StorageGas"></a>

## Resource `StorageGas`

Storage parameters, reconfigured each epoch.<br/><br/> Parameters are updated during reconfiguration via<br/> <code>on_reconfig()</code>, based on storage utilization at the beginning<br/> of the epoch in which the reconfiguration transaction is<br/> executed. The gas schedule derived from these parameters will<br/> then be used to calculate gas for the entirety of the<br/> following epoch, such that the data is one epoch older than<br/> ideal. Notably, however, per this approach, the virtual machine<br/> does not need to reload gas parameters after the<br/> first transaction of an epoch.


<pre><code>struct StorageGas has key<br/></code></pre>



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

A point in a Eulerian curve approximation, with each coordinate<br/> given in basis points:<br/><br/> &#124; Field value &#124; Percentage &#124;<br/> &#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;<br/> &#124; <code>1</code>         &#124; 00.01 %    &#124;<br/> &#124; <code>10</code>        &#124; 00.10 %    &#124;<br/> &#124; <code>100</code>       &#124; 01.00 %    &#124;<br/> &#124; <code>1000</code>      &#124; 10.00 %    &#124;


<pre><code>struct Point has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>x: u64</code>
</dt>
<dd>
 x&#45;coordinate basis points, corresponding to utilization<br/> ratio in <code>base_8192_exponential_curve()</code>.
</dd>
<dt>
<code>y: u64</code>
</dt>
<dd>
 y&#45;coordinate basis points, corresponding to utilization<br/> multiplier in <code>base_8192_exponential_curve()</code>.
</dd>
</dl>


</details>

<a id="0x1_storage_gas_UsageGasConfig"></a>

## Struct `UsageGasConfig`

A gas configuration for either per&#45;item or per&#45;byte costs.<br/><br/> Contains a target usage amount, as well as a Eulerian<br/> approximation of an exponential curve for reads, creations, and<br/> overwrites. See <code>StorageGasConfig</code>.


<pre><code>struct UsageGasConfig has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>target_usage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>read_curve: storage_gas::GasCurve</code>
</dt>
<dd>

</dd>
<dt>
<code>create_curve: storage_gas::GasCurve</code>
</dt>
<dd>

</dd>
<dt>
<code>write_curve: storage_gas::GasCurve</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_storage_gas_GasCurve"></a>

## Struct `GasCurve`

Eulerian approximation of an exponential curve.<br/><br/> Assumes the following endpoints:<br/><br/> &#42; $(x_0, y_0) &#61; (0, 0)$<br/> &#42; $(x_f, y_f) &#61; (10000, 10000)$<br/><br/> Intermediate points must satisfy:<br/><br/> 1. $x_i &gt; x_&#123;i &#45; 1&#125;$ ( $x$ is strictly increasing).<br/> 2. $0 \leq x_i \leq 10000$ ( $x$ is between 0 and 10000).<br/> 3. $y_i \geq y_&#123;i &#45; 1&#125;$ ( $y$ is non&#45;decreasing).<br/> 4. $0 \leq y_i \leq 10000$ ( $y$ is between 0 and 10000).<br/><br/> Lookup between two successive points is calculated via linear<br/> interpolation, e.g., as if there were a straight line between<br/> them.<br/><br/> See <code>base_8192_exponential_curve()</code>.


<pre><code>struct GasCurve has copy, drop, store<br/></code></pre>



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
<code>points: vector&lt;storage_gas::Point&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_storage_gas_StorageGasConfig"></a>

## Resource `StorageGasConfig`

Gas configurations for per&#45;item and per&#45;byte prices.


<pre><code>struct StorageGasConfig has copy, drop, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>item_config: storage_gas::UsageGasConfig</code>
</dt>
<dd>
 Per&#45;item gas configuration.
</dd>
<dt>
<code>byte_config: storage_gas::UsageGasConfig</code>
</dt>
<dd>
 Per&#45;byte gas configuration.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_storage_gas_MAX_U64"></a>



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_storage_gas_BASIS_POINT_DENOMINATION"></a>



<pre><code>const BASIS_POINT_DENOMINATION: u64 &#61; 10000;<br/></code></pre>



<a id="0x1_storage_gas_EINVALID_GAS_RANGE"></a>



<pre><code>const EINVALID_GAS_RANGE: u64 &#61; 2;<br/></code></pre>



<a id="0x1_storage_gas_EINVALID_MONOTONICALLY_NON_DECREASING_CURVE"></a>



<pre><code>const EINVALID_MONOTONICALLY_NON_DECREASING_CURVE: u64 &#61; 5;<br/></code></pre>



<a id="0x1_storage_gas_EINVALID_POINT_RANGE"></a>



<pre><code>const EINVALID_POINT_RANGE: u64 &#61; 6;<br/></code></pre>



<a id="0x1_storage_gas_ESTORAGE_GAS"></a>



<pre><code>const ESTORAGE_GAS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_storage_gas_ESTORAGE_GAS_CONFIG"></a>



<pre><code>const ESTORAGE_GAS_CONFIG: u64 &#61; 0;<br/></code></pre>



<a id="0x1_storage_gas_ETARGET_USAGE_TOO_BIG"></a>



<pre><code>const ETARGET_USAGE_TOO_BIG: u64 &#61; 4;<br/></code></pre>



<a id="0x1_storage_gas_EZERO_TARGET_USAGE"></a>



<pre><code>const EZERO_TARGET_USAGE: u64 &#61; 3;<br/></code></pre>



<a id="0x1_storage_gas_base_8192_exponential_curve"></a>

## Function `base_8192_exponential_curve`

Default exponential curve having base 8192.<br/><br/> &#35; Function definition<br/><br/> Gas price as a function of utilization ratio is defined as:<br/><br/> $$g(u_r) &#61; g_&#123;min&#125; &#43; \frac&#123;(b^&#123;u_r&#125; &#45; 1)&#125;&#123;b &#45; 1&#125; \Delta_g$$<br/><br/> $$g(u_r) &#61; g_&#123;min&#125; &#43; u_m \Delta_g$$<br/><br/> &#124; Variable                            &#124; Description            &#124;<br/> &#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;<br/> &#124; $g_&#123;min&#125;$                           &#124; <code>min_gas</code>              &#124;<br/> &#124; $g_&#123;max&#125;$                           &#124; <code>max_gas</code>              &#124;<br/> &#124; $\Delta_&#123;g&#125; &#61; g_&#123;max&#125; &#45; g_&#123;min&#125;$    &#124; Gas delta              &#124;<br/> &#124; $u$                                 &#124; Utilization            &#124;<br/> &#124; $u_t$                               &#124; Target utilization     &#124;<br/> &#124; $u_r &#61; u / u_t$                     &#124; Utilization ratio      &#124;<br/> &#124; $u_m &#61; \frac&#123;(b^&#123;u_r&#125; &#45; 1)&#125;&#123;b &#45; 1&#125;$ &#124; Utilization multiplier &#124;<br/> &#124; $b &#61; 8192$                          &#124; Exponent base          &#124;<br/><br/> &#35; Example<br/><br/> Hence for a utilization ratio of 50% ( $u_r &#61; 0.5$ ):<br/><br/> $$g(0.5) &#61; g_&#123;min&#125; &#43; \frac&#123;8192^&#123;0.5&#125; &#45; 1&#125;&#123;8192 &#45; 1&#125; \Delta_g$$<br/><br/> $$g(0.5) \approx g_&#123;min&#125; &#43; 0.0109 \Delta_g$$<br/><br/> Which means that the price above <code>min_gas</code> is approximately<br/> 1.09% of the difference between <code>max_gas</code> and <code>min_gas</code>.<br/><br/> &#35; Utilization multipliers<br/><br/> &#124; $u_r$ &#124; $u_m$ (approximate) &#124;<br/> &#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;<br/> &#124; 10%   &#124; 0.02%               &#124;<br/> &#124; 20%   &#124; 0.06%               &#124;<br/> &#124; 30%   &#124; 0.17%               &#124;<br/> &#124; 40%   &#124; 0.44%               &#124;<br/> &#124; 50%   &#124; 1.09%               &#124;<br/> &#124; 60%   &#124; 2.71%               &#124;<br/> &#124; 70%   &#124; 6.69%               &#124;<br/> &#124; 80%   &#124; 16.48%              &#124;<br/> &#124; 90%   &#124; 40.61%              &#124;<br/> &#124; 95%   &#124; 63.72%              &#124;<br/> &#124; 99%   &#124; 91.38%              &#124;


<pre><code>public fun base_8192_exponential_curve(min_gas: u64, max_gas: u64): storage_gas::GasCurve<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun base_8192_exponential_curve(min_gas: u64, max_gas: u64): GasCurve &#123;<br/>    new_gas_curve(min_gas, max_gas,<br/>        vector[<br/>            new_point(1000, 2),<br/>            new_point(2000, 6),<br/>            new_point(3000, 17),<br/>            new_point(4000, 44),<br/>            new_point(5000, 109),<br/>            new_point(6000, 271),<br/>            new_point(7000, 669),<br/>            new_point(8000, 1648),<br/>            new_point(9000, 4061),<br/>            new_point(9500, 6372),<br/>            new_point(9900, 9138),<br/>        ]<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_new_point"></a>

## Function `new_point`



<pre><code>public fun new_point(x: u64, y: u64): storage_gas::Point<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_point(x: u64, y: u64): Point &#123;<br/>    assert!(<br/>        x &lt;&#61; BASIS_POINT_DENOMINATION &amp;&amp; y &lt;&#61; BASIS_POINT_DENOMINATION,<br/>        error::invalid_argument(EINVALID_POINT_RANGE)<br/>    );<br/>    Point &#123; x, y &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_new_gas_curve"></a>

## Function `new_gas_curve`



<pre><code>public fun new_gas_curve(min_gas: u64, max_gas: u64, points: vector&lt;storage_gas::Point&gt;): storage_gas::GasCurve<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_gas_curve(min_gas: u64, max_gas: u64, points: vector&lt;Point&gt;): GasCurve &#123;<br/>    assert!(max_gas &gt;&#61; min_gas, error::invalid_argument(EINVALID_GAS_RANGE));<br/>    assert!(max_gas &lt;&#61; MAX_U64 / BASIS_POINT_DENOMINATION, error::invalid_argument(EINVALID_GAS_RANGE));<br/>    validate_points(&amp;points);<br/>    GasCurve &#123;<br/>        min_gas,<br/>        max_gas,<br/>        points<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_new_usage_gas_config"></a>

## Function `new_usage_gas_config`



<pre><code>public fun new_usage_gas_config(target_usage: u64, read_curve: storage_gas::GasCurve, create_curve: storage_gas::GasCurve, write_curve: storage_gas::GasCurve): storage_gas::UsageGasConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_usage_gas_config(target_usage: u64, read_curve: GasCurve, create_curve: GasCurve, write_curve: GasCurve): UsageGasConfig &#123;<br/>    assert!(target_usage &gt; 0, error::invalid_argument(EZERO_TARGET_USAGE));<br/>    assert!(target_usage &lt;&#61; MAX_U64 / BASIS_POINT_DENOMINATION, error::invalid_argument(ETARGET_USAGE_TOO_BIG));<br/>    UsageGasConfig &#123;<br/>        target_usage,<br/>        read_curve,<br/>        create_curve,<br/>        write_curve,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_new_storage_gas_config"></a>

## Function `new_storage_gas_config`



<pre><code>public fun new_storage_gas_config(item_config: storage_gas::UsageGasConfig, byte_config: storage_gas::UsageGasConfig): storage_gas::StorageGasConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_storage_gas_config(item_config: UsageGasConfig, byte_config: UsageGasConfig): StorageGasConfig &#123;<br/>    StorageGasConfig &#123;<br/>        item_config,<br/>        byte_config<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_set_config"></a>

## Function `set_config`



<pre><code>public(friend) fun set_config(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun set_config(aptos_framework: &amp;signer, config: StorageGasConfig) acquires StorageGasConfig &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    &#42;borrow_global_mut&lt;StorageGasConfig&gt;(@aptos_framework) &#61; config;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_initialize"></a>

## Function `initialize`

Initialize per&#45;item and per&#45;byte gas prices.<br/><br/> Target utilization is set to 2 billion items and 1 TB.<br/><br/> <code>GasCurve</code> endpoints are initialized as follows:<br/><br/> &#124; Data style &#124; Operation &#124; Minimum gas &#124; Maximum gas &#124;<br/> &#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#45;&#124;<br/> &#124; Per item   &#124; Read      &#124; 300K        &#124; 300K &#42; 100  &#124;<br/> &#124; Per item   &#124; Create    &#124; 300k        &#124; 300k &#42; 100    &#124;<br/> &#124; Per item   &#124; Write     &#124; 300K        &#124; 300K &#42; 100  &#124;<br/> &#124; Per byte   &#124; Read      &#124; 300         &#124; 300 &#42; 100   &#124;<br/> &#124; Per byte   &#124; Create    &#124; 5K          &#124; 5K &#42; 100    &#124;<br/> &#124; Per byte   &#124; Write     &#124; 5K          &#124; 5K &#42; 100    &#124;<br/><br/> <code>StorageGas</code> values are additionally initialized, but per<br/> <code>on_reconfig()</code>, they will be reconfigured for each subsequent<br/> epoch after initialization.<br/><br/> See <code>base_8192_exponential_curve()</code> fore more information on<br/> target utilization.


<pre><code>public fun initialize(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(<br/>        !exists&lt;StorageGasConfig&gt;(@aptos_framework),<br/>        error::already_exists(ESTORAGE_GAS_CONFIG)<br/>    );<br/><br/>    let k: u64 &#61; 1000;<br/>    let m: u64 &#61; 1000 &#42; 1000;<br/><br/>    let item_config &#61; UsageGasConfig &#123;<br/>        target_usage: 2 &#42; k &#42; m, // 2 billion<br/>        read_curve: base_8192_exponential_curve(300 &#42; k, 300 &#42; k &#42; 100),<br/>        create_curve: base_8192_exponential_curve(300 &#42; k, 300 &#42; k &#42; 100),<br/>        write_curve: base_8192_exponential_curve(300 &#42; k, 300 &#42; k &#42; 100),<br/>    &#125;;<br/>    let byte_config &#61; UsageGasConfig &#123;<br/>        target_usage: 1 &#42; m &#42; m, // 1TB<br/>        read_curve: base_8192_exponential_curve(300, 300 &#42; 100),<br/>        create_curve: base_8192_exponential_curve(5 &#42; k,  5 &#42; k &#42; 100),<br/>        write_curve: base_8192_exponential_curve(5 &#42; k,  5 &#42; k &#42; 100),<br/>    &#125;;<br/>    move_to(aptos_framework, StorageGasConfig &#123;<br/>        item_config,<br/>        byte_config,<br/>    &#125;);<br/><br/>    assert!(<br/>        !exists&lt;StorageGas&gt;(@aptos_framework),<br/>        error::already_exists(ESTORAGE_GAS)<br/>    );<br/>    move_to(aptos_framework, StorageGas &#123;<br/>        per_item_read: 300 &#42; k,<br/>        per_item_create: 5 &#42; m,<br/>        per_item_write: 300 &#42; k,<br/>        per_byte_read: 300,<br/>        per_byte_create: 5 &#42; k,<br/>        per_byte_write: 5 &#42; k,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_validate_points"></a>

## Function `validate_points`



<pre><code>fun validate_points(points: &amp;vector&lt;storage_gas::Point&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_points(points: &amp;vector&lt;Point&gt;) &#123;<br/>    let len &#61; vector::length(points);<br/>    spec &#123;<br/>        assume len &lt; MAX_U64;<br/>    &#125;;<br/>    let i &#61; 0;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant forall j in 0..i: &#123;<br/>                let cur &#61; if (j &#61;&#61; 0) &#123; Point &#123; x: 0, y: 0 &#125; &#125; else &#123; points[j &#45; 1] &#125;;<br/>                let next &#61; if (j &#61;&#61; len) &#123; Point &#123; x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION &#125; &#125; else &#123; points[j] &#125;;<br/>                cur.x &lt; next.x &amp;&amp; cur.y &lt;&#61; next.y<br/>            &#125;;<br/>        &#125;;<br/>        i &lt;&#61; len<br/>    &#125;) &#123;<br/>        let cur &#61; if (i &#61;&#61; 0) &#123; &amp;Point &#123; x: 0, y: 0 &#125; &#125; else &#123; vector::borrow(points, i &#45; 1) &#125;;<br/>        let next &#61; if (i &#61;&#61; len) &#123; &amp;Point &#123; x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION &#125; &#125; else &#123; vector::borrow(points, i) &#125;;<br/>        assert!(cur.x &lt; next.x &amp;&amp; cur.y &lt;&#61; next.y, error::invalid_argument(EINVALID_MONOTONICALLY_NON_DECREASING_CURVE));<br/>        i &#61; i &#43; 1;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_calculate_gas"></a>

## Function `calculate_gas`



<pre><code>fun calculate_gas(max_usage: u64, current_usage: u64, curve: &amp;storage_gas::GasCurve): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_gas(max_usage: u64, current_usage: u64, curve: &amp;GasCurve): u64 &#123;<br/>    let capped_current_usage &#61; if (current_usage &gt; max_usage) max_usage else current_usage;<br/>    let points &#61; &amp;curve.points;<br/>    let num_points &#61; vector::length(points);<br/>    let current_usage_bps &#61; capped_current_usage &#42; BASIS_POINT_DENOMINATION / max_usage;<br/><br/>    // Check the corner case that current_usage_bps drops before the first point.<br/>    let (left, right) &#61; if (num_points &#61;&#61; 0) &#123;<br/>        (&amp;Point &#123; x: 0, y: 0 &#125;, &amp;Point &#123; x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION &#125;)<br/>    &#125; else if (current_usage_bps &lt; vector::borrow(points, 0).x) &#123;<br/>        (&amp;Point &#123; x: 0, y: 0 &#125;, vector::borrow(points, 0))<br/>    &#125; else if (vector::borrow(points, num_points &#45; 1).x &lt;&#61; current_usage_bps) &#123;<br/>        (vector::borrow(points, num_points &#45; 1), &amp;Point &#123; x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION &#125;)<br/>    &#125; else &#123;<br/>        let (i, j) &#61; (0, num_points &#45; 2);<br/>        while (&#123;<br/>            spec &#123;<br/>                invariant i &lt;&#61; j;<br/>                invariant j &lt; num_points &#45; 1;<br/>                invariant points[i].x &lt;&#61; current_usage_bps;<br/>                invariant current_usage_bps &lt; points[j &#43; 1].x;<br/>            &#125;;<br/>            i &lt; j<br/>        &#125;) &#123;<br/>            let mid &#61; j &#45; (j &#45; i) / 2;<br/>            if (current_usage_bps &lt; vector::borrow(points, mid).x) &#123;<br/>                spec &#123;<br/>                    // j is strictly decreasing.<br/>                    assert mid &#45; 1 &lt; j;<br/>                &#125;;<br/>                j &#61; mid &#45; 1;<br/>            &#125; else &#123;<br/>                spec &#123;<br/>                    // i is strictly increasing.<br/>                    assert i &lt; mid;<br/>                &#125;;<br/>                i &#61; mid;<br/>            &#125;;<br/>        &#125;;<br/>        (vector::borrow(points, i), vector::borrow(points, i &#43; 1))<br/>    &#125;;<br/>    let y_interpolated &#61; interpolate(left.x, right.x, left.y, right.y, current_usage_bps);<br/>    interpolate(0, BASIS_POINT_DENOMINATION, curve.min_gas, curve.max_gas, y_interpolated)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_interpolate"></a>

## Function `interpolate`



<pre><code>fun interpolate(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun interpolate(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64 &#123;<br/>    y0 &#43; (x &#45; x0) &#42; (y1 &#45; y0) / (x1 &#45; x0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_calculate_read_gas"></a>

## Function `calculate_read_gas`



<pre><code>fun calculate_read_gas(config: &amp;storage_gas::UsageGasConfig, usage: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_read_gas(config: &amp;UsageGasConfig, usage: u64): u64 &#123;<br/>    calculate_gas(config.target_usage, usage, &amp;config.read_curve)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_calculate_create_gas"></a>

## Function `calculate_create_gas`



<pre><code>fun calculate_create_gas(config: &amp;storage_gas::UsageGasConfig, usage: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_create_gas(config: &amp;UsageGasConfig, usage: u64): u64 &#123;<br/>    calculate_gas(config.target_usage, usage, &amp;config.create_curve)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_calculate_write_gas"></a>

## Function `calculate_write_gas`



<pre><code>fun calculate_write_gas(config: &amp;storage_gas::UsageGasConfig, usage: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun calculate_write_gas(config: &amp;UsageGasConfig, usage: u64): u64 &#123;<br/>    calculate_gas(config.target_usage, usage, &amp;config.write_curve)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_storage_gas_on_reconfig"></a>

## Function `on_reconfig`



<pre><code>public(friend) fun on_reconfig()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_reconfig() acquires StorageGas, StorageGasConfig &#123;<br/>    assert!(<br/>        exists&lt;StorageGasConfig&gt;(@aptos_framework),<br/>        error::not_found(ESTORAGE_GAS_CONFIG)<br/>    );<br/>    assert!(<br/>        exists&lt;StorageGas&gt;(@aptos_framework),<br/>        error::not_found(ESTORAGE_GAS)<br/>    );<br/>    let (items, bytes) &#61; state_storage::current_items_and_bytes();<br/>    let gas_config &#61; borrow_global&lt;StorageGasConfig&gt;(@aptos_framework);<br/>    let gas &#61; borrow_global_mut&lt;StorageGas&gt;(@aptos_framework);<br/>    gas.per_item_read &#61; calculate_read_gas(&amp;gas_config.item_config, items);<br/>    gas.per_item_create &#61; calculate_create_gas(&amp;gas_config.item_config, items);<br/>    gas.per_item_write &#61; calculate_write_gas(&amp;gas_config.item_config, items);<br/>    gas.per_byte_read &#61; calculate_read_gas(&amp;gas_config.byte_config, bytes);<br/>    gas.per_byte_create &#61; calculate_create_gas(&amp;gas_config.byte_config, bytes);<br/>    gas.per_byte_write &#61; calculate_write_gas(&amp;gas_config.byte_config, bytes);<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<a id="0x1_storage_gas_spec_calculate_gas"></a>


<pre><code>fun spec_calculate_gas(max_usage: u64, current_usage: u64, curve: GasCurve): u64;<br/></code></pre>




<a id="0x1_storage_gas_NewGasCurveAbortsIf"></a>


<pre><code>schema NewGasCurveAbortsIf &#123;<br/>min_gas: u64;<br/>max_gas: u64;<br/>aborts_if max_gas &lt; min_gas;<br/>aborts_if max_gas &gt; MAX_U64 / BASIS_POINT_DENOMINATION;<br/>&#125;<br/></code></pre>


A non decreasing curve must ensure that next is greater than cur.


<a id="0x1_storage_gas_ValidatePointsAbortsIf"></a>


<pre><code>schema ValidatePointsAbortsIf &#123;<br/>points: vector&lt;Point&gt;;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
    aborts_if exists i in 0..len(points) &#45; 1: (<br/>    points[i].x &gt;&#61; points[i &#43; 1].x &#124;&#124; points[i].y &gt; points[i &#43; 1].y<br/>);<br/>aborts_if len(points) &gt; 0 &amp;&amp; points[0].x &#61;&#61; 0;<br/>aborts_if len(points) &gt; 0 &amp;&amp; points[len(points) &#45; 1].x &#61;&#61; BASIS_POINT_DENOMINATION;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_Point"></a>

### Struct `Point`


<pre><code>struct Point has copy, drop, store<br/></code></pre>



<dl>
<dt>
<code>x: u64</code>
</dt>
<dd>
 x&#45;coordinate basis points, corresponding to utilization<br/> ratio in <code>base_8192_exponential_curve()</code>.
</dd>
<dt>
<code>y: u64</code>
</dt>
<dd>
 y&#45;coordinate basis points, corresponding to utilization<br/> multiplier in <code>base_8192_exponential_curve()</code>.
</dd>
</dl>



<pre><code>invariant x &lt;&#61; BASIS_POINT_DENOMINATION;<br/>invariant y &lt;&#61; BASIS_POINT_DENOMINATION;<br/></code></pre>



<a id="@Specification_1_UsageGasConfig"></a>

### Struct `UsageGasConfig`


<pre><code>struct UsageGasConfig has copy, drop, store<br/></code></pre>



<dl>
<dt>
<code>target_usage: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>read_curve: storage_gas::GasCurve</code>
</dt>
<dd>

</dd>
<dt>
<code>create_curve: storage_gas::GasCurve</code>
</dt>
<dd>

</dd>
<dt>
<code>write_curve: storage_gas::GasCurve</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant target_usage &gt; 0;<br/>invariant target_usage &lt;&#61; MAX_U64 / BASIS_POINT_DENOMINATION;<br/></code></pre>





<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The module&apos;s initialization guarantees the creation of the StorageGasConfig resource with a precise configuration, including accurate gas curves for per&#45;item and per&#45;byte operations.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The initialize function is responsible for setting up the initial state of the module, ensuring the fulfillment of the following conditions: (1) the creation of the StorageGasConfig resource, indicating its existence witqhin the module&apos;s context, and (2) the configuration of the StorageGasConfig resource includes the precise gas curves that define the behavior of per&#45;item and per&#45;byte operations.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;initialize&lt;/a&gt;. Moreover, the native gas logic has been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The gas curve approximates an exponential curve based on a minimum and maximum gas charge.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The validate_points function ensures that the provided vector of points represents a monotonically non&#45;decreasing curve.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;validate_points&lt;/a&gt;. Moreover, the configuration logic has been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The initialized gas curve structure has values set according to the provided parameters.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The new_gas_curve function initializes the GasCurve structure with values provided as parameters.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;new_gas_curve&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The initialized usage gas configuration structure has values set according to the provided parameters.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The new_usage_gas_config function initializes the UsageGasConfig structure with values provided as parameters.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;new_usage_gas_config&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;StorageGasConfig&gt;(@aptos_framework);<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;StorageGas&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_GasCurve"></a>

### Struct `GasCurve`


<pre><code>struct GasCurve has copy, drop, store<br/></code></pre>



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
<code>points: vector&lt;storage_gas::Point&gt;</code>
</dt>
<dd>

</dd>
</dl>


Invariant 1: The minimum gas charge does not exceed the maximum gas charge.


<pre><code>invariant min_gas &lt;&#61; max_gas;<br/></code></pre>


Invariant 2: The maximum gas charge is capped by MAX_U64 scaled down by the basis point denomination.


<pre><code>invariant max_gas &lt;&#61; MAX_U64 / BASIS_POINT_DENOMINATION;<br/></code></pre>


Invariant 3: The x&#45;coordinate increases monotonically and the y&#45;coordinate increasing strictly monotonically,<br/> that is, the gas&#45;curve is a monotonically increasing function.


<pre><code>invariant (len(points) &gt; 0 &#61;&#61;&gt; points[0].x &gt; 0)<br/>    &amp;&amp; (len(points) &gt; 0 &#61;&#61;&gt; points[len(points) &#45; 1].x &lt; BASIS_POINT_DENOMINATION)<br/>    &amp;&amp; (forall i in 0..len(points) &#45; 1: (points[i].x &lt; points[i &#43; 1].x &amp;&amp; points[i].y &lt;&#61; points[i &#43; 1].y));<br/></code></pre>



<a id="@Specification_1_base_8192_exponential_curve"></a>

### Function `base_8192_exponential_curve`


<pre><code>public fun base_8192_exponential_curve(min_gas: u64, max_gas: u64): storage_gas::GasCurve<br/></code></pre>




<pre><code>include NewGasCurveAbortsIf;<br/></code></pre>



<a id="@Specification_1_new_point"></a>

### Function `new_point`


<pre><code>public fun new_point(x: u64, y: u64): storage_gas::Point<br/></code></pre>




<pre><code>aborts_if x &gt; BASIS_POINT_DENOMINATION &#124;&#124; y &gt; BASIS_POINT_DENOMINATION;<br/>ensures result.x &#61;&#61; x;<br/>ensures result.y &#61;&#61; y;<br/></code></pre>



<a id="@Specification_1_new_gas_curve"></a>

### Function `new_gas_curve`


<pre><code>public fun new_gas_curve(min_gas: u64, max_gas: u64, points: vector&lt;storage_gas::Point&gt;): storage_gas::GasCurve<br/></code></pre>


A non decreasing curve must ensure that next is greater than cur.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>include NewGasCurveAbortsIf;<br/>include ValidatePointsAbortsIf;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
ensures result &#61;&#61; GasCurve &#123;<br/>    min_gas,<br/>    max_gas,<br/>    points<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_new_usage_gas_config"></a>

### Function `new_usage_gas_config`


<pre><code>public fun new_usage_gas_config(target_usage: u64, read_curve: storage_gas::GasCurve, create_curve: storage_gas::GasCurve, write_curve: storage_gas::GasCurve): storage_gas::UsageGasConfig<br/></code></pre>




<pre><code>aborts_if target_usage &#61;&#61; 0;<br/>aborts_if target_usage &gt; MAX_U64 / BASIS_POINT_DENOMINATION;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
ensures result &#61;&#61; UsageGasConfig &#123;<br/>    target_usage,<br/>    read_curve,<br/>    create_curve,<br/>    write_curve,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_new_storage_gas_config"></a>

### Function `new_storage_gas_config`


<pre><code>public fun new_storage_gas_config(item_config: storage_gas::UsageGasConfig, byte_config: storage_gas::UsageGasConfig): storage_gas::StorageGasConfig<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result.item_config &#61;&#61; item_config;<br/>ensures result.byte_config &#61;&#61; byte_config;<br/></code></pre>



<a id="@Specification_1_set_config"></a>

### Function `set_config`


<pre><code>public(friend) fun set_config(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)<br/></code></pre>


Signer address must be @aptos_framework and StorageGasConfig exists.


<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>aborts_if !exists&lt;StorageGasConfig&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public fun initialize(aptos_framework: &amp;signer)<br/></code></pre>


Signer address must be @aptos_framework.<br/> Address @aptos_framework does not exist StorageGasConfig and StorageGas before the function call is restricted<br/> and exists after the function is executed.


<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>pragma verify_duration_estimate &#61; 120;<br/>aborts_if exists&lt;StorageGasConfig&gt;(@aptos_framework);<br/>aborts_if exists&lt;StorageGas&gt;(@aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures exists&lt;StorageGasConfig&gt;(@aptos_framework);<br/>ensures exists&lt;StorageGas&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_validate_points"></a>

### Function `validate_points`


<pre><code>fun validate_points(points: &amp;vector&lt;storage_gas::Point&gt;)<br/></code></pre>


A non decreasing curve must ensure that next is greater than cur.


<pre><code>pragma aborts_if_is_strict &#61; false;<br/>pragma verify &#61; false;<br/>pragma opaque;<br/>include ValidatePointsAbortsIf;<br/></code></pre>



<a id="@Specification_1_calculate_gas"></a>

### Function `calculate_gas`


<pre><code>fun calculate_gas(max_usage: u64, current_usage: u64, curve: &amp;storage_gas::GasCurve): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>pragma verify_duration_estimate &#61; 120;<br/>requires max_usage &gt; 0;<br/>requires max_usage &lt;&#61; MAX_U64 / BASIS_POINT_DENOMINATION;<br/>aborts_if false;<br/>ensures [abstract] result &#61;&#61; spec_calculate_gas(max_usage, current_usage, curve);<br/></code></pre>



<a id="@Specification_1_interpolate"></a>

### Function `interpolate`


<pre><code>fun interpolate(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64<br/></code></pre>




<pre><code>pragma opaque;<br/>pragma intrinsic;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_on_reconfig"></a>

### Function `on_reconfig`


<pre><code>public(friend) fun on_reconfig()<br/></code></pre>


Address @aptos_framework must exist StorageGasConfig and StorageGas and StateStorageUsage.


<pre><code>requires chain_status::is_operating();<br/>aborts_if !exists&lt;StorageGasConfig&gt;(@aptos_framework);<br/>aborts_if !exists&lt;StorageGas&gt;(@aptos_framework);<br/>aborts_if !exists&lt;state_storage::StateStorageUsage&gt;(@aptos_framework);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
