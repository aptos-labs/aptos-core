
<a id="0x1_box_or_inline"></a>

# Module `0x1::box_or_inline`



-  [Enum `BoxOrInline`](#0x1_box_or_inline_BoxOrInline)
-  [Struct `Dummy`](#0x1_box_or_inline_Dummy)
-  [Constants](#@Constants_0)
-  [Function `new_inline`](#0x1_box_or_inline_new_inline)
-  [Function `new_box`](#0x1_box_or_inline_new_box)
-  [Function `borrow`](#0x1_box_or_inline_borrow)
-  [Function `borrow_mut`](#0x1_box_or_inline_borrow_mut)
-  [Function `destroy`](#0x1_box_or_inline_destroy)
-  [Function `move_to_inline`](#0x1_box_or_inline_move_to_inline)
-  [Function `move_to_box`](#0x1_box_or_inline_move_to_box)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/mem.md#0x1_mem">0x1::mem</a>;
<b>use</b> <a href="table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x1_box_or_inline_BoxOrInline"></a>

## Enum `BoxOrInline`



<pre><code>enum <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt; <b>has</b> store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Inline</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: T</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>BoxInTable</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="table.md#0x1_table">table</a>: <a href="table.md#0x1_table_Table">table::Table</a>&lt;bool, T&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_box_or_inline_Dummy"></a>

## Struct `Dummy`



<pre><code><b>struct</b> <a href="box_or_inline.md#0x1_box_or_inline_Dummy">Dummy</a> <b>has</b> store
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_box_or_inline_ONLY_KEY"></a>



<pre><code><b>const</b> <a href="box_or_inline.md#0x1_box_or_inline_ONLY_KEY">ONLY_KEY</a>: bool = <b>true</b>;
</code></pre>



<a id="0x1_box_or_inline_new_inline"></a>

## Function `new_inline`



<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_new_inline">new_inline</a>&lt;T: store&gt;(value: T): <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">box_or_inline::BoxOrInline</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_new_inline">new_inline</a>&lt;T: store&gt;(value: T): <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt; {
    BoxOrInline::Inline { value }
}
</code></pre>



</details>

<a id="0x1_box_or_inline_new_box"></a>

## Function `new_box`



<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_new_box">new_box</a>&lt;T: store&gt;(value: T): <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">box_or_inline::BoxOrInline</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_new_box">new_box</a>&lt;T: store&gt;(value: T): <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt; {
    <b>let</b> <a href="table.md#0x1_table">table</a> = <a href="table.md#0x1_table_new">table::new</a>();
    <a href="table.md#0x1_table">table</a>.add(<a href="box_or_inline.md#0x1_box_or_inline_ONLY_KEY">ONLY_KEY</a>, value);
    BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a> }
}
</code></pre>



</details>

<a id="0x1_box_or_inline_borrow"></a>

## Function `borrow`



<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">box_or_inline::BoxOrInline</a>&lt;T&gt;): &T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_borrow">borrow</a>&lt;T: store&gt;(self: &<a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt;): &T {
    match (self) {
        BoxOrInline::Inline { value } =&gt; value,
        BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a> } =&gt; <a href="table.md#0x1_table">table</a>.<a href="box_or_inline.md#0x1_box_or_inline_borrow">borrow</a>(<a href="box_or_inline.md#0x1_box_or_inline_ONLY_KEY">ONLY_KEY</a>),
    }
}
</code></pre>



</details>

<a id="0x1_box_or_inline_borrow_mut"></a>

## Function `borrow_mut`



<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">box_or_inline::BoxOrInline</a>&lt;T&gt;): &<b>mut</b> T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_borrow_mut">borrow_mut</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt;): &<b>mut</b> T {
    match (self) {
        BoxOrInline::Inline { value } =&gt; value,
        BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a> } =&gt; <a href="table.md#0x1_table">table</a>.<a href="box_or_inline.md#0x1_box_or_inline_borrow_mut">borrow_mut</a>(<a href="box_or_inline.md#0x1_box_or_inline_ONLY_KEY">ONLY_KEY</a>),
    }
}
</code></pre>



</details>

<a id="0x1_box_or_inline_destroy"></a>

## Function `destroy`



<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_destroy">destroy</a>&lt;T: store&gt;(self: <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">box_or_inline::BoxOrInline</a>&lt;T&gt;): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_destroy">destroy</a>&lt;T: store&gt;(self: <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt;): T {
    match (self) {
        BoxOrInline::Inline { value } =&gt; value,
        BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a> } =&gt; {
            <b>let</b> value = <a href="table.md#0x1_table">table</a>.remove(<a href="box_or_inline.md#0x1_box_or_inline_ONLY_KEY">ONLY_KEY</a>);
            <a href="table.md#0x1_table">table</a>.destroy_known_empty_unsafe();
            value
        },
    }
}
</code></pre>



</details>

<a id="0x1_box_or_inline_move_to_inline"></a>

## Function `move_to_inline`



<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_move_to_inline">move_to_inline</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">box_or_inline::BoxOrInline</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_move_to_inline">move_to_inline</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt;) {
    match (self) {
        BoxOrInline::Inline { value: _ } =&gt; {},
        BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a> } =&gt; {
            <b>let</b> value = <a href="table.md#0x1_table">table</a>.remove(<a href="box_or_inline.md#0x1_box_or_inline_ONLY_KEY">ONLY_KEY</a>);
            <b>let</b> BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a> } = <a href="../../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(self, BoxOrInline::Inline { value });
            <a href="table.md#0x1_table">table</a>.destroy_known_empty_unsafe();
        },
    }
}
</code></pre>



</details>

<a id="0x1_box_or_inline_move_to_box"></a>

## Function `move_to_box`



<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_move_to_box">move_to_box</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">box_or_inline::BoxOrInline</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="box_or_inline.md#0x1_box_or_inline_move_to_box">move_to_box</a>&lt;T: store&gt;(self: &<b>mut</b> <a href="box_or_inline.md#0x1_box_or_inline_BoxOrInline">BoxOrInline</a>&lt;T&gt;) {
    match (self) {
        BoxOrInline::Inline { value: _ } =&gt; {
            <b>let</b> BoxOrInline::Inline { value } = <a href="../../move-stdlib/doc/mem.md#0x1_mem_replace">mem::replace</a>(self, BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a>: <a href="table.md#0x1_table_new">table::new</a>() });
            self.<a href="table.md#0x1_table">table</a>.add(<a href="box_or_inline.md#0x1_box_or_inline_ONLY_KEY">ONLY_KEY</a>, value);
        },
        BoxOrInline::BoxInTable { <a href="table.md#0x1_table">table</a>: _ } =&gt; {},
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
