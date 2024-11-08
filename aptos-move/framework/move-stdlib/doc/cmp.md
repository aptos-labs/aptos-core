
<a id="0x1_cmp"></a>

# Module `0x1::cmp`



-  [Enum `Ordering`](#0x1_cmp_Ordering)
-  [Function `compare`](#0x1_cmp_compare)
-  [Function `is_eq`](#0x1_cmp_is_eq)
-  [Function `is_ne`](#0x1_cmp_is_ne)
-  [Function `is_lt`](#0x1_cmp_is_lt)
-  [Function `is_le`](#0x1_cmp_is_le)
-  [Function `is_gt`](#0x1_cmp_is_gt)
-  [Function `is_ge`](#0x1_cmp_is_ge)


<pre><code></code></pre>



<a id="0x1_cmp_Ordering"></a>

## Enum `Ordering`



<pre><code>enum <a href="cmp.md#0x1_cmp_Ordering">Ordering</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Less</summary>


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
<summary>Greater</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_cmp_compare"></a>

## Function `compare`

Compares two values with the natural ordering:
- native types are compared identically to <code>&lt;</code> and other operators
- complex types
- Structs and vectors - are compared lexicographically - first field/element is compared first,
and if equal we proceed to the next.
- enum's are compared first by their variant, and if equal - they are compared as structs are.


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare">compare</a>&lt;T&gt;(first: &T, second: &T): <a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare">compare</a>&lt;T&gt;(first: &T, second: &T): <a href="cmp.md#0x1_cmp_Ordering">Ordering</a>;
</code></pre>



</details>

<a id="0x1_cmp_is_eq"></a>

## Function `is_eq`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_eq">is_eq</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_eq">is_eq</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self is Ordering::Equal
}
</code></pre>



</details>

<a id="0x1_cmp_is_ne"></a>

## Function `is_ne`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_ne">is_ne</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_ne">is_ne</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    !(self is Ordering::Equal)
}
</code></pre>



</details>

<a id="0x1_cmp_is_lt"></a>

## Function `is_lt`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_lt">is_lt</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_lt">is_lt</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self is Ordering::Less
}
</code></pre>



</details>

<a id="0x1_cmp_is_le"></a>

## Function `is_le`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_le">is_le</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_le">is_le</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    !(self is Ordering::Greater)
}
</code></pre>



</details>

<a id="0x1_cmp_is_gt"></a>

## Function `is_gt`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_gt">is_gt</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_gt">is_gt</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    self is Ordering::Greater
}
</code></pre>



</details>

<a id="0x1_cmp_is_ge"></a>

## Function `is_ge`



<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_ge">is_ge</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_ge">is_ge</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">Ordering</a>): bool {
    !(self is Ordering::Less)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
