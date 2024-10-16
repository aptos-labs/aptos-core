
<a id="0x1_cmp"></a>

# Module `0x1::cmp`



-  [Enum `Ordering`](#0x1_cmp_Ordering)
-  [Constants](#@Constants_0)
-  [Function `compare`](#0x1_cmp_compare)
-  [Function `is_equal`](#0x1_cmp_is_equal)
-  [Function `is_less_than`](#0x1_cmp_is_less_than)
-  [Function `is_less_or_equal`](#0x1_cmp_is_less_or_equal)
-  [Function `is_greater_than`](#0x1_cmp_is_greater_than)
-  [Function `is__greater_or_equal`](#0x1_cmp_is__greater_or_equal)


<pre><code></code></pre>



<a id="0x1_cmp_Ordering"></a>

## Enum `Ordering`



<pre><code>enum <a href="cmp.md#0x1_cmp_Ordering">Ordering</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>LessThan</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>Equal</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>GreaterThan</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_cmp_EQUAL"></a>



<pre><code><b>const</b> <a href="cmp.md#0x1_cmp_EQUAL">EQUAL</a>: u8 = 1;
</code></pre>



<a id="0x1_cmp_GREATER_THAN"></a>



<pre><code><b>const</b> <a href="cmp.md#0x1_cmp_GREATER_THAN">GREATER_THAN</a>: u8 = 2;
</code></pre>



<a id="0x1_cmp_LESS_THAN"></a>



<pre><code><b>const</b> <a href="cmp.md#0x1_cmp_LESS_THAN">LESS_THAN</a>: u8 = 0;
</code></pre>



<a id="0x1_cmp_compare"></a>

## Function `compare`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare">compare</a>&lt;T&gt;(first: &T, second: &T): <a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare">compare</a>&lt;T&gt;(first: &T, second: &T): <a href="cmp.md#0x1_cmp_Ordering">Ordering</a>;
</code></pre>



</details>

<a id="0x1_cmp_is_equal"></a>

## Function `is_equal`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_equal">is_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_equal">is_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self is Ordering::Equal
}
</code></pre>



</details>

<a id="0x1_cmp_is_less_than"></a>

## Function `is_less_than`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_less_than">is_less_than</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_less_than">is_less_than</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self is Ordering::LessThan
}
</code></pre>



</details>

<a id="0x1_cmp_is_less_or_equal"></a>

## Function `is_less_or_equal`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_less_or_equal">is_less_or_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_less_or_equal">is_less_or_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    !(self is Ordering::GreaterThan)
}
</code></pre>



</details>

<a id="0x1_cmp_is_greater_than"></a>

## Function `is_greater_than`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_greater_than">is_greater_than</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_greater_than">is_greater_than</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self is Ordering::GreaterThan
}
</code></pre>



</details>

<a id="0x1_cmp_is__greater_or_equal"></a>

## Function `is__greater_or_equal`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is__greater_or_equal">is__greater_or_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is__greater_or_equal">is__greater_or_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    !(self is Ordering::LessThan)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
