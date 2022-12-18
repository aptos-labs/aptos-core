
<a name="0x1_simple_set"></a>

# Module `0x1::simple_set`

This module provides a solution for small unsorted sets, that is it has the properties that
1) Each item must be unique
2) The items in set are unsorted
3) Adds and removals take O(N) time


-  [Struct `SimpleSet`](#0x1_simple_set_SimpleSet)
-  [Function `length`](#0x1_simple_set_length)
-  [Function `empty`](#0x1_simple_set_empty)
-  [Function `contains`](#0x1_simple_set_contains)
-  [Function `destroy_empty`](#0x1_simple_set_destroy_empty)
-  [Function `insert`](#0x1_simple_set_insert)
-  [Function `remove`](#0x1_simple_set_remove)
-  [Function `find`](#0x1_simple_set_find)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_simple_set_SimpleSet"></a>

## Struct `SimpleSet`



<pre><code><b>struct</b> <a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>data: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Key&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_simple_set_length"></a>

## Function `length`

Return the number of keys in the set.


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_length">length</a>&lt;Key&gt;(set: &<a href="simple_set.md#0x1_simple_set_SimpleSet">simple_set::SimpleSet</a>&lt;Key&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_length">length</a>&lt;Key&gt;(set: &<a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt;): u64 {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&set.data)
}
</code></pre>



</details>

<a name="0x1_simple_set_empty"></a>

## Function `empty`

Create an empty set.


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_empty">empty</a>&lt;Key: <b>copy</b>, drop, store&gt;(): <a href="simple_set.md#0x1_simple_set_SimpleSet">simple_set::SimpleSet</a>&lt;Key&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_empty">empty</a>&lt;Key: store + <b>copy</b> + drop&gt;(): <a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt; {
    <a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a> {
        data: <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;Key&gt;(),
    }
}
</code></pre>



</details>

<a name="0x1_simple_set_contains"></a>

## Function `contains`

Return true if the set contains <code>key</code>, or false vice versa.


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_contains">contains</a>&lt;Key&gt;(set: &<a href="simple_set.md#0x1_simple_set_SimpleSet">simple_set::SimpleSet</a>&lt;Key&gt;, key: &Key): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_contains">contains</a>&lt;Key&gt;(
    set: &<a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt;,
    key: &Key,
): bool {
    <b>let</b> maybe_idx = <a href="simple_set.md#0x1_simple_set_find">find</a>(set, key);
    <a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&maybe_idx)
}
</code></pre>



</details>

<a name="0x1_simple_set_destroy_empty"></a>

## Function `destroy_empty`

Destroy the set. Aborts if set is not empty.


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_destroy_empty">destroy_empty</a>&lt;Key&gt;(set: <a href="simple_set.md#0x1_simple_set_SimpleSet">simple_set::SimpleSet</a>&lt;Key&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_destroy_empty">destroy_empty</a>&lt;Key&gt;(set: <a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt;) {
    <b>let</b> <a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a> { data } = set;
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(data);
}
</code></pre>



</details>

<a name="0x1_simple_set_insert"></a>

## Function `insert`

Insert <code>key</code> into the set.
Return <code><b>true</b></code> if <code>key</code> did not already exist in the set and <code><b>false</b></code> vice versa.


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_insert">insert</a>&lt;Key: drop&gt;(set: &<b>mut</b> <a href="simple_set.md#0x1_simple_set_SimpleSet">simple_set::SimpleSet</a>&lt;Key&gt;, key: Key): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_insert">insert</a>&lt;Key: drop&gt;(
    set: &<b>mut</b> <a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt;,
    key: Key,
): bool {
    <b>let</b> maybe_idx = <a href="simple_set.md#0x1_simple_set_find">find</a>(set, &key);
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&maybe_idx)) {
        <b>false</b>
    } <b>else</b> {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> set.data, key);
        <b>true</b>
    }
}
</code></pre>



</details>

<a name="0x1_simple_set_remove"></a>

## Function `remove`

Remove <code>key</code> into the set.
Return <code><b>true</b></code> if <code>key</code> already existed in the set and <code><b>false</b></code> vice versa.


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_remove">remove</a>&lt;Key: drop&gt;(set: &<b>mut</b> <a href="simple_set.md#0x1_simple_set_SimpleSet">simple_set::SimpleSet</a>&lt;Key&gt;, key: &Key): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="simple_set.md#0x1_simple_set_remove">remove</a>&lt;Key: drop&gt;(
    set: &<b>mut</b> <a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt;,
    key: &Key,
): bool {
    <b>let</b> maybe_idx = <a href="simple_set.md#0x1_simple_set_find">find</a>(set, key);
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&maybe_idx)) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> set.data, *<a href="../../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&maybe_idx));
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a name="0x1_simple_set_find"></a>

## Function `find`

Find the potential index of <code>key</code> in the underlying data vector.


<pre><code><b>fun</b> <a href="simple_set.md#0x1_simple_set_find">find</a>&lt;Key&gt;(set: &<a href="simple_set.md#0x1_simple_set_SimpleSet">simple_set::SimpleSet</a>&lt;Key&gt;, key: &Key): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="simple_set.md#0x1_simple_set_find">find</a>&lt;Key&gt;(
    set: &<a href="simple_set.md#0x1_simple_set_SimpleSet">SimpleSet</a>&lt;Key&gt;,
    key: &Key,
): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;{
    <b>let</b> leng = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&set.data);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; leng) {
        <b>let</b> cur = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&set.data, i);
        <b>if</b> (cur == key){
            <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(i)
        };
        i = i + 1;
    };
    <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u64&gt;()
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
