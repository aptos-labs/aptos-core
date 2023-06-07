
<a name="0x42_N"></a>

# `#[attr8]`<br>Module `0x42::N`



-  [`#[attr10]`<br>Function `bar`](#0x42_N_bar)


<pre><code></code></pre>



<a name="0x42_N_bar"></a>

## `#[attr10]`<br>Function `bar`



<pre><code><b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_N_bar">bar</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_N_bar">bar</a>() {}
</code></pre>



</details>



<a name="0x42_M"></a>

# `#[attr2]`<br>`#[attr7]`<br>Module `0x42::M`



-  [`#[attr4]`<br>Struct `S`](#0x42_M_S)
-  [`#[attr4b]`<br>`#[resource_group(scope = global)]`<br>Struct `T`](#0x42_M_T)
-  [Constants](#@Constants_0)
-  [`#[attr6]`<br>`#[resource_group_member(group = 0x1::string::String)]`<br>Function `foo`](#0x42_M_foo)


<pre><code><b>use</b> <a href="attribute_placement.md#0x42_N">0x42::N</a>;
</code></pre>



<a name="0x42_M_S"></a>

## `#[attr4]`<br>Struct `S`



<pre><code><b>struct</b> <a href="attribute_placement.md#0x42_M_S">S</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x42_M_T"></a>

## `#[attr4b]`<br>`#[resource_group(scope = global)]`<br>Struct `T`



<pre><code><b>struct</b> <a href="attribute_placement.md#0x42_M_T">T</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x42_M_C"></a>



<pre><code><b>const</b> <a href="attribute_placement.md#0x42_M_C">C</a>: u64 = 0;
</code></pre>



<a name="0x42_M_foo"></a>

## `#[attr6]`<br>`#[resource_group_member(group = 0x1::string::String)]`<br>Function `foo`



<pre><code><b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_M_foo">foo</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="attribute_placement.md#0x42_M_foo">foo</a>() { <a href="attribute_placement.md#0x42_N_bar">N::bar</a>() }
</code></pre>



</details>

<details>
<summary>Specification</summary>



</details>



<a name="0x1_main"></a>

# `#[attr11]`<br>Module `0x1::main`



-  [Constants](#@Constants_0)
-  [`#[attr14]`<br>Function `main`](#0x1_main_main)


<pre><code><b>use</b> <a href="attribute_placement.md#0x42_M">0x42::M</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1_main_C"></a>



<pre><code><b>const</b> <a href="attribute_placement.md#0x1_main_C">C</a>: u64 = 0;
</code></pre>



<a name="0x1_main_main"></a>

## `#[attr14]`<br>Function `main`



<pre><code><b>fun</b> <a href="attribute_placement.md#0x1_main">main</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="attribute_placement.md#0x1_main">main</a>() {
    <a href="attribute_placement.md#0x42_M_foo">M::foo</a>();
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



</details>
