
<a name="0x42_N"></a>

# Module `0x42::N`



- Attributes:
    - `#[attr8]`



-  [Function `bar`](#0x42_N_bar)


<pre><code></code></pre>



<a name="0x42_N_bar"></a>

## Function `bar`



<pre><code>#[attr10]
<b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_N_bar">bar</a>()
</code></pre>



##### Implementation


<pre><code><b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_N_bar">bar</a>() {}
</code></pre>



<a name="0x42_M"></a>

# Module `0x42::M`



- Attributes:
    - `#[attr2]`
    - `#[attr7]`



-  [Struct `S`](#0x42_M_S)
-  [Struct `T`](#0x42_M_T)
-  [Constants](#@Constants_0)
-  [Function `foo`](#0x42_M_foo)


<pre><code><b>use</b> <a href="attribute_placement.md#0x42_N">0x42::N</a>;
</code></pre>



<a name="0x42_M_S"></a>

## Struct `S`



<pre><code>#[attr4]
<b>struct</b> <a href="attribute_placement.md#0x42_M_S">S</a>
</code></pre>



##### Fields


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


<a name="0x42_M_T"></a>

## Struct `T`



<pre><code>#[attr4b]
#[resource_group(#[scope = <b>global</b>])]
<b>struct</b> <a href="attribute_placement.md#0x42_M_T">T</a>
</code></pre>



##### Fields


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


<a name="@Constants_0"></a>

## Constants


<a name="0x42_M_C"></a>



<pre><code><b>const</b> <a href="attribute_placement.md#0x42_M_C">C</a>: u64 = 0;
</code></pre>



<a name="0x42_M_foo"></a>

## Function `foo`



<pre><code>#[attr6]
#[resource_group_member(#[group = 0x1::string::String])]
<b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_M_foo">foo</a>()
</code></pre>



##### Implementation


<pre><code><b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_M_foo">foo</a>() { <a href="attribute_placement.md#0x42_N_bar">N::bar</a>() }
</code></pre>



##### Specification



<a name="0x1_main"></a>

# Module `0x1::main`



- Attributes:
    - `#[attr11]`



-  [Constants](#@Constants_0)
-  [Function `main`](#0x1_main_main)


<pre><code><b>use</b> <a href="attribute_placement.md#0x42_M">0x42::M</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1_main_C"></a>



<pre><code><b>const</b> <a href="attribute_placement.md#0x1_main_C">C</a>: u64 = 0;
</code></pre>



<a name="0x1_main_main"></a>

## Function `main`



<pre><code>#[attr14]
<b>fun</b> <a href="attribute_placement.md#0x1_main">main</a>()
</code></pre>



##### Implementation


<pre><code><b>fun</b> <a href="attribute_placement.md#0x1_main">main</a>() {
    <a href="attribute_placement.md#0x42_M_foo">M::foo</a>();
}
</code></pre>



##### Specification
