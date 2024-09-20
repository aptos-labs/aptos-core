
<a id="0x1_cmp"></a>

# Module `0x1::cmp`



-  [Struct `Ordering`](#0x1_cmp_Ordering)
-  [Constants](#@Constants_0)
-  [Function `compare_impl`](#0x1_cmp_compare_impl)
-  [Function `compare`](#0x1_cmp_compare)
-  [Function `is_equal`](#0x1_cmp_is_equal)
-  [Function `is_less_then`](#0x1_cmp_is_less_then)
-  [Function `is_less_or_equal`](#0x1_cmp_is_less_or_equal)


<pre><code></code></pre>



<a id="0x1_cmp_Ordering"></a>

## Struct `Ordering`



<pre><code><b>struct</b> <a href="cmp.md#0x1_cmp_Ordering">Ordering</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: u8</code>
</dt>
<dd>

</dd>
</dl>


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



<a id="0x1_cmp_compare_impl"></a>

## Function `compare_impl`

As there are no signed values in move, all values are shifted by 1 up.
An int value:
1   iff both values are the same
0   iff first value is smaller than the second
2   iff first value is larger than the second


<pre><code><b>fun</b> <a href="cmp.md#0x1_cmp_compare_impl">compare_impl</a>&lt;T&gt;(first: &T, second: &T): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare_impl">compare_impl</a>&lt;T&gt;(first: &T, second: &T): u8;
</code></pre>



</details>

<a id="0x1_cmp_compare"></a>

## Function `compare`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare">compare</a>&lt;T&gt;(first: &T, second: &T): <a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare">compare</a>&lt;T&gt;(first: &T, second: &T): <a href="cmp.md#0x1_cmp_Ordering">Ordering</a> {
    <a href="cmp.md#0x1_cmp_Ordering">Ordering</a> {
        value: <a href="cmp.md#0x1_cmp_compare_impl">compare_impl</a>(first, second),
    }
}
</code></pre>



</details>

<a id="0x1_cmp_is_equal"></a>

## Function `is_equal`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_equal">is_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_equal">is_equal</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self.value == <a href="cmp.md#0x1_cmp_EQUAL">EQUAL</a>
}
</code></pre>



</details>

<a id="0x1_cmp_is_less_then"></a>

## Function `is_less_then`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_less_then">is_less_then</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_less_then">is_less_then</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self.value == <a href="cmp.md#0x1_cmp_LESS_THAN">LESS_THAN</a>
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
    self.value != <a href="cmp.md#0x1_cmp_GREATER_THAN">GREATER_THAN</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
