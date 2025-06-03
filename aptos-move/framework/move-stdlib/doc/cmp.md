
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
-  [Function `test_compare_preliminary_types`](#0x1_cmp_test_compare_preliminary_types)
-  [Function `test_compare_vec`](#0x1_cmp_test_compare_vec)
-  [Specification](#@Specification_0)
    -  [Enum `Ordering`](#@Specification_0_Ordering)
    -  [Function `compare`](#@Specification_0_compare)
    -  [Function `is_eq`](#@Specification_0_is_eq)
    -  [Function `is_ne`](#@Specification_0_is_ne)
    -  [Function `is_lt`](#@Specification_0_is_lt)
    -  [Function `is_le`](#@Specification_0_is_le)
    -  [Function `is_gt`](#@Specification_0_is_gt)
    -  [Function `is_ge`](#@Specification_0_is_ge)


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

<a id="0x1_cmp_test_compare_preliminary_types"></a>

## Function `test_compare_preliminary_types`



<pre><code><b>fun</b> <a href="cmp.md#0x1_cmp_test_compare_preliminary_types">test_compare_preliminary_types</a>(): <a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="cmp.md#0x1_cmp_test_compare_preliminary_types">test_compare_preliminary_types</a>(): <a href="cmp.md#0x1_cmp_Ordering">Ordering</a> {
    <b>let</b> a = 1;
    <b>let</b> b = 5;
    <b>spec</b> {
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(a, b) == Ordering::Less;
    };
    <b>let</b> x = <b>true</b>;
    <b>let</b> y = <b>false</b>;
    <b>spec</b> {
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(x, y) == Ordering::Greater;
    };
    <b>let</b> addr_1 = @0x1;
    <b>let</b> addr_2 = @0x2;
    <b>spec</b> {
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(addr_1, addr_1) == Ordering::Equal;
    };
    <a href="cmp.md#0x1_cmp_compare">compare</a>(&x, &y);
    <a href="cmp.md#0x1_cmp_compare">compare</a>(&a, &b)
}
</code></pre>



</details>

<a id="0x1_cmp_test_compare_vec"></a>

## Function `test_compare_vec`



<pre><code><b>fun</b> <a href="cmp.md#0x1_cmp_test_compare_vec">test_compare_vec</a>(v2: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="cmp.md#0x1_cmp_test_compare_vec">test_compare_vec</a>(v2: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;) {
    <b>let</b> v1 = <a href="vector.md#0x1_vector">vector</a>[1, 2, 3];
    <b>let</b> v1_1 = <a href="vector.md#0x1_vector">vector</a>[1, 2, 3];
    <b>let</b> v2 = <a href="vector.md#0x1_vector">vector</a>[1, 2];
    <b>let</b> v3 = <a href="vector.md#0x1_vector">vector</a>[1, 2, 4];
    <b>let</b> v4 = <a href="vector.md#0x1_vector">vector</a>[1, 2, 3, 4];
    <b>let</b> v5 = <a href="vector.md#0x1_vector">vector</a>[5];
    <b>let</b> v6 = <a href="vector.md#0x1_vector">vector</a>[<a href="vector.md#0x1_vector">vector</a>[1, 2, 3]];
    <b>spec</b> {
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v1, v1_1) == Ordering::Equal;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v1, v3) == Ordering::Less;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v1, v2) == Ordering::Greater;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v3, v1) == Ordering::Greater;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v4, v1) == Ordering::Greater;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v1, v4) == Ordering::Less;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v5, v1) == Ordering::Greater;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v1, v5) == Ordering::Less;
        <b>assert</b> <a href="cmp.md#0x1_cmp_compare">compare</a>(v6, v6) == Ordering::Equal;
    };
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_Ordering"></a>

### Enum `Ordering`


<pre><code>enum <a href="cmp.md#0x1_cmp_Ordering">Ordering</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<dl>

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
</dl>



<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_compare"></a>

### Function `compare`


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_compare">compare</a>&lt;T&gt;(first: &T, second: &T): <a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_is_eq"></a>

### Function `is_eq`


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_eq">is_eq</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_is_ne"></a>

### Function `is_ne`


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_ne">is_ne</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_is_lt"></a>

### Function `is_lt`


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_lt">is_lt</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_is_le"></a>

### Function `is_le`


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_le">is_le</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_is_gt"></a>

### Function `is_gt`


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_gt">is_gt</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>



<a id="@Specification_0_is_ge"></a>

### Function `is_ge`


<pre><code><b>public</b> <b>fun</b> <a href="cmp.md#0x1_cmp_is_ge">is_ge</a>(self: &<a href="cmp.md#0x1_cmp_Ordering">cmp::Ordering</a>): bool
</code></pre>




<pre><code><b>pragma</b> intrinsic;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
