
<a id="0x1_option"></a>

# Module `0x1::option`

This module defines the Option type and its methods to represent and handle an optional value.


-  [Struct `Option`](#0x1_option_Option)
-  [Constants](#@Constants_0)
-  [Function `none`](#0x1_option_none)
-  [Function `some`](#0x1_option_some)
-  [Function `from_vec`](#0x1_option_from_vec)
-  [Function `is_none`](#0x1_option_is_none)
-  [Function `is_some`](#0x1_option_is_some)
-  [Function `contains`](#0x1_option_contains)
-  [Function `borrow`](#0x1_option_borrow)
-  [Function `borrow_with_default`](#0x1_option_borrow_with_default)
-  [Function `get_with_default`](#0x1_option_get_with_default)
-  [Function `fill`](#0x1_option_fill)
-  [Function `extract`](#0x1_option_extract)
-  [Function `borrow_mut`](#0x1_option_borrow_mut)
-  [Function `swap`](#0x1_option_swap)
-  [Function `swap_or_fill`](#0x1_option_swap_or_fill)
-  [Function `destroy_with_default`](#0x1_option_destroy_with_default)
-  [Function `destroy_some`](#0x1_option_destroy_some)
-  [Function `destroy_none`](#0x1_option_destroy_none)
-  [Function `to_vec`](#0x1_option_to_vec)
-  [Function `for_each`](#0x1_option_for_each)
-  [Function `for_each_ref`](#0x1_option_for_each_ref)
-  [Function `for_each_mut`](#0x1_option_for_each_mut)
-  [Function `fold`](#0x1_option_fold)
-  [Function `map`](#0x1_option_map)
-  [Function `map_ref`](#0x1_option_map_ref)
-  [Function `filter`](#0x1_option_filter)
-  [Function `any`](#0x1_option_any)
-  [Function `destroy`](#0x1_option_destroy)
-  [Specification](#@Specification_1)
    -  [Helper Schema](#@Helper_Schema_2)
    -  [Struct `Option`](#@Specification_1_Option)
    -  [Function `none`](#@Specification_1_none)
    -  [Function `some`](#@Specification_1_some)
    -  [Function `from_vec`](#@Specification_1_from_vec)
    -  [Function `is_none`](#@Specification_1_is_none)
    -  [Function `is_some`](#@Specification_1_is_some)
    -  [Function `contains`](#@Specification_1_contains)
    -  [Function `borrow`](#@Specification_1_borrow)
    -  [Function `borrow_with_default`](#@Specification_1_borrow_with_default)
    -  [Function `get_with_default`](#@Specification_1_get_with_default)
    -  [Function `fill`](#@Specification_1_fill)
    -  [Function `extract`](#@Specification_1_extract)
    -  [Function `borrow_mut`](#@Specification_1_borrow_mut)
    -  [Function `swap`](#@Specification_1_swap)
    -  [Function `swap_or_fill`](#@Specification_1_swap_or_fill)
    -  [Function `destroy_with_default`](#@Specification_1_destroy_with_default)
    -  [Function `destroy_some`](#@Specification_1_destroy_some)
    -  [Function `destroy_none`](#@Specification_1_destroy_none)
    -  [Function `to_vec`](#@Specification_1_to_vec)


<pre><code><b>use</b> <a href="vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_option_Option"></a>

## Struct `Option`

Abstraction of a value that may or may not be present. Implemented with a vector of size
zero or one because Move bytecode does not have ADTs.


<pre><code><b>struct</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>vec: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_option_EOPTION_IS_SET"></a>

The <code><a href="option.md#0x1_option_Option">Option</a></code> is in an invalid state for the operation attempted.
The <code><a href="option.md#0x1_option_Option">Option</a></code> is <code>Some</code> while it should be <code>None</code>.


<pre><code><b>const</b> <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a>: u64 &#61; 262144;<br /></code></pre>



<a id="0x1_option_EOPTION_NOT_SET"></a>

The <code><a href="option.md#0x1_option_Option">Option</a></code> is in an invalid state for the operation attempted.
The <code><a href="option.md#0x1_option_Option">Option</a></code> is <code>None</code> while it should be <code>Some</code>.


<pre><code><b>const</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>: u64 &#61; 262145;<br /></code></pre>



<a id="0x1_option_EOPTION_VEC_TOO_LONG"></a>

Cannot construct an option from a vector with 2 or more elements.


<pre><code><b>const</b> <a href="option.md#0x1_option_EOPTION_VEC_TOO_LONG">EOPTION_VEC_TOO_LONG</a>: u64 &#61; 262146;<br /></code></pre>



<a id="0x1_option_none"></a>

## Function `none`

Return an empty <code><a href="option.md#0x1_option_Option">Option</a></code>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_none">none</a>&lt;Element&gt;(): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_none">none</a>&lt;Element&gt;(): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; &#123;<br />    <a href="option.md#0x1_option_Option">Option</a> &#123; vec: <a href="vector.md#0x1_vector_empty">vector::empty</a>() &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_some"></a>

## Function `some`

Return an <code><a href="option.md#0x1_option_Option">Option</a></code> containing <code>e</code>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_some">some</a>&lt;Element&gt;(e: Element): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_some">some</a>&lt;Element&gt;(e: Element): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; &#123;<br />    <a href="option.md#0x1_option_Option">Option</a> &#123; vec: <a href="vector.md#0x1_vector_singleton">vector::singleton</a>(e) &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_from_vec"></a>

## Function `from_vec`



<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_from_vec">from_vec</a>&lt;Element&gt;(vec: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_from_vec">from_vec</a>&lt;Element&gt;(vec: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; &#123;<br />    <b>assert</b>!(<a href="vector.md#0x1_vector_length">vector::length</a>(&amp;vec) &lt;&#61; 1, <a href="option.md#0x1_option_EOPTION_VEC_TOO_LONG">EOPTION_VEC_TOO_LONG</a>);<br />    <a href="option.md#0x1_option_Option">Option</a> &#123; vec &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_is_none"></a>

## Function `is_none`

Return true if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_none">is_none</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_none">is_none</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool &#123;<br />    <a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;t.vec)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_is_some"></a>

## Function `is_some`

Return true if <code>t</code> holds a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_some">is_some</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_some">is_some</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool &#123;<br />    !<a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;t.vec)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_contains"></a>

## Function `contains`

Return true if the value in <code>t</code> is equal to <code>e_ref</code>
Always returns <code><b>false</b></code> if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_contains">contains</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e_ref: &amp;Element): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_contains">contains</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e_ref: &amp;Element): bool &#123;<br />    <a href="vector.md#0x1_vector_contains">vector::contains</a>(&amp;t.vec, e_ref)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_borrow"></a>

## Function `borrow`

Return an immutable reference to the value inside <code>t</code>
Aborts if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow">borrow</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): &amp;Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow">borrow</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): &amp;Element &#123;<br />    <b>assert</b>!(<a href="option.md#0x1_option_is_some">is_some</a>(t), <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>);<br />    <a href="vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;t.vec, 0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_borrow_with_default"></a>

## Function `borrow_with_default`

Return a reference to the value inside <code>t</code> if it holds one
Return <code>default_ref</code> if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_with_default">borrow_with_default</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default_ref: &amp;Element): &amp;Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_with_default">borrow_with_default</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, default_ref: &amp;Element): &amp;Element &#123;<br />    <b>let</b> vec_ref &#61; &amp;t.vec;<br />    <b>if</b> (<a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(vec_ref)) default_ref<br />    <b>else</b> <a href="vector.md#0x1_vector_borrow">vector::borrow</a>(vec_ref, 0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_get_with_default"></a>

## Function `get_with_default`

Return the value inside <code>t</code> if it holds one
Return <code>default</code> if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_get_with_default">get_with_default</a>&lt;Element: <b>copy</b>, drop&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default: Element): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_get_with_default">get_with_default</a>&lt;Element: <b>copy</b> &#43; drop&gt;(<br />    t: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;,<br />    default: Element,<br />): Element &#123;<br />    <b>let</b> vec_ref &#61; &amp;t.vec;<br />    <b>if</b> (<a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(vec_ref)) default<br />    <b>else</b> &#42;<a href="vector.md#0x1_vector_borrow">vector::borrow</a>(vec_ref, 0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_fill"></a>

## Function `fill`

Convert the none option <code>t</code> to a some option by adding <code>e</code>.
Aborts if <code>t</code> already holds a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_fill">fill</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_fill">fill</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e: Element) &#123;<br />    <b>let</b> vec_ref &#61; &amp;<b>mut</b> t.vec;<br />    <b>if</b> (<a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(vec_ref)) <a href="vector.md#0x1_vector_push_back">vector::push_back</a>(vec_ref, e)<br />    <b>else</b> <b>abort</b> <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_extract"></a>

## Function `extract`

Convert a <code>some</code> option to a <code>none</code> by removing and returning the value stored inside <code>t</code>
Aborts if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_extract">extract</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_extract">extract</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): Element &#123;<br />    <b>assert</b>!(<a href="option.md#0x1_option_is_some">is_some</a>(t), <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>);<br />    <a href="vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> t.vec)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the value inside <code>t</code>
Aborts if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_mut">borrow_mut</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): &amp;<b>mut</b> Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_mut">borrow_mut</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): &amp;<b>mut</b> Element &#123;<br />    <b>assert</b>!(<a href="option.md#0x1_option_is_some">is_some</a>(t), <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>);<br />    <a href="vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&amp;<b>mut</b> t.vec, 0)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_swap"></a>

## Function `swap`

Swap the old value inside <code>t</code> with <code>e</code> and return the old value
Aborts if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap">swap</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap">swap</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e: Element): Element &#123;<br />    <b>assert</b>!(<a href="option.md#0x1_option_is_some">is_some</a>(t), <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>);<br />    <b>let</b> vec_ref &#61; &amp;<b>mut</b> t.vec;<br />    <b>let</b> old_value &#61; <a href="vector.md#0x1_vector_pop_back">vector::pop_back</a>(vec_ref);<br />    <a href="vector.md#0x1_vector_push_back">vector::push_back</a>(vec_ref, e);<br />    old_value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_swap_or_fill"></a>

## Function `swap_or_fill`

Swap the old value inside <code>t</code> with <code>e</code> and return the old value;
or if there is no old value, fill it with <code>e</code>.
Different from swap(), swap_or_fill() allows for <code>t</code> not holding a value.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap_or_fill">swap_or_fill</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap_or_fill">swap_or_fill</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e: Element): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; &#123;<br />    <b>let</b> vec_ref &#61; &amp;<b>mut</b> t.vec;<br />    <b>let</b> old_value &#61; <b>if</b> (<a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(vec_ref)) <a href="option.md#0x1_option_none">none</a>()<br />        <b>else</b> <a href="option.md#0x1_option_some">some</a>(<a href="vector.md#0x1_vector_pop_back">vector::pop_back</a>(vec_ref));<br />    <a href="vector.md#0x1_vector_push_back">vector::push_back</a>(vec_ref, e);<br />    old_value<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_destroy_with_default"></a>

## Function `destroy_with_default`

Destroys <code>t.</code> If <code>t</code> holds a value, return it. Returns <code>default</code> otherwise


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_with_default">destroy_with_default</a>&lt;Element: drop&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default: Element): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_with_default">destroy_with_default</a>&lt;Element: drop&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, default: Element): Element &#123;<br />    <b>let</b> <a href="option.md#0x1_option_Option">Option</a> &#123; vec &#125; &#61; t;<br />    <b>if</b> (<a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;<b>mut</b> vec)) default<br />    <b>else</b> <a href="vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> vec)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_destroy_some"></a>

## Function `destroy_some`

Unpack <code>t</code> and return its contents
Aborts if <code>t</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_some">destroy_some</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): Element<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_some">destroy_some</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): Element &#123;<br />    <b>assert</b>!(<a href="option.md#0x1_option_is_some">is_some</a>(&amp;t), <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>);<br />    <b>let</b> <a href="option.md#0x1_option_Option">Option</a> &#123; vec &#125; &#61; t;<br />    <b>let</b> elem &#61; <a href="vector.md#0x1_vector_pop_back">vector::pop_back</a>(&amp;<b>mut</b> vec);<br />    <a href="vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(vec);<br />    elem<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_destroy_none"></a>

## Function `destroy_none`

Unpack <code>t</code>
Aborts if <code>t</code> holds a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_none">destroy_none</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_none">destroy_none</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;) &#123;<br />    <b>assert</b>!(<a href="option.md#0x1_option_is_none">is_none</a>(&amp;t), <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a>);<br />    <b>let</b> <a href="option.md#0x1_option_Option">Option</a> &#123; vec &#125; &#61; t;<br />    <a href="vector.md#0x1_vector_destroy_empty">vector::destroy_empty</a>(vec)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_to_vec"></a>

## Function `to_vec`

Convert <code>t</code> into a vector of length 1 if it is <code>Some</code>,
and an empty vector otherwise


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_to_vec">to_vec</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_to_vec">to_vec</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; &#123;<br />    <b>let</b> <a href="option.md#0x1_option_Option">Option</a> &#123; vec &#125; &#61; t;<br />    vec<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_for_each"></a>

## Function `for_each`

Apply the function to the optional element, consuming it. Does nothing if no value present.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_for_each">for_each</a>&lt;Element&gt;(o: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: &#124;Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_for_each">for_each</a>&lt;Element&gt;(o: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: &#124;Element&#124;) &#123;<br />    <b>if</b> (<a href="option.md#0x1_option_is_some">is_some</a>(&amp;o)) &#123;<br />        f(<a href="option.md#0x1_option_destroy_some">destroy_some</a>(o))<br />    &#125; <b>else</b> &#123;<br />        <a href="option.md#0x1_option_destroy_none">destroy_none</a>(o)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to the optional element reference. Does nothing if no value present.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_for_each_ref">for_each_ref</a>&lt;Element&gt;(o: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_for_each_ref">for_each_ref</a>&lt;Element&gt;(o: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;) &#123;<br />    <b>if</b> (<a href="option.md#0x1_option_is_some">is_some</a>(o)) &#123;<br />        f(<a href="option.md#0x1_option_borrow">borrow</a>(o))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to the optional element reference. Does nothing if no value present.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_for_each_mut">for_each_mut</a>&lt;Element&gt;(o: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: &#124;&amp;<b>mut</b> Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_for_each_mut">for_each_mut</a>&lt;Element&gt;(o: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: &#124;&amp;<b>mut</b> Element&#124;) &#123;<br />    <b>if</b> (<a href="option.md#0x1_option_is_some">is_some</a>(o)) &#123;<br />        f(<a href="option.md#0x1_option_borrow_mut">borrow_mut</a>(o))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_fold"></a>

## Function `fold`

Folds the function over the optional element.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_fold">fold</a>&lt;Accumulator, Element&gt;(o: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, init: Accumulator, f: &#124;(Accumulator, Element)&#124;Accumulator): Accumulator<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_fold">fold</a>&lt;Accumulator, Element&gt;(<br />    o: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;,<br />    init: Accumulator,<br />    f: &#124;Accumulator,Element&#124;Accumulator<br />): Accumulator &#123;<br />    <b>if</b> (<a href="option.md#0x1_option_is_some">is_some</a>(&amp;o)) &#123;<br />        f(init, <a href="option.md#0x1_option_destroy_some">destroy_some</a>(o))<br />    &#125; <b>else</b> &#123;<br />        <a href="option.md#0x1_option_destroy_none">destroy_none</a>(o);<br />        init<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_map"></a>

## Function `map`

Maps the content of an option.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_map">map</a>&lt;Element, OtherElement&gt;(o: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: &#124;Element&#124;OtherElement): <a href="option.md#0x1_option_Option">option::Option</a>&lt;OtherElement&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_map">map</a>&lt;Element, OtherElement&gt;(o: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: &#124;Element&#124;OtherElement): <a href="option.md#0x1_option_Option">Option</a>&lt;OtherElement&gt; &#123;<br />    <b>if</b> (<a href="option.md#0x1_option_is_some">is_some</a>(&amp;o)) &#123;<br />        <a href="option.md#0x1_option_some">some</a>(f(<a href="option.md#0x1_option_destroy_some">destroy_some</a>(o)))<br />    &#125; <b>else</b> &#123;<br />        <a href="option.md#0x1_option_destroy_none">destroy_none</a>(o);<br />        <a href="option.md#0x1_option_none">none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_map_ref"></a>

## Function `map_ref`

Maps the content of an option without destroying the original option.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_map_ref">map_ref</a>&lt;Element, OtherElement&gt;(o: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;OtherElement): <a href="option.md#0x1_option_Option">option::Option</a>&lt;OtherElement&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_map_ref">map_ref</a>&lt;Element, OtherElement&gt;(<br />    o: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;OtherElement): <a href="option.md#0x1_option_Option">Option</a>&lt;OtherElement&gt; &#123;<br />    <b>if</b> (<a href="option.md#0x1_option_is_some">is_some</a>(o)) &#123;<br />        <a href="option.md#0x1_option_some">some</a>(f(<a href="option.md#0x1_option_borrow">borrow</a>(o)))<br />    &#125; <b>else</b> &#123;<br />        <a href="option.md#0x1_option_none">none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_filter"></a>

## Function `filter`

Filters the content of an option


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_filter">filter</a>&lt;Element: drop&gt;(o: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_filter">filter</a>&lt;Element:drop&gt;(o: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; &#123;<br />    <b>if</b> (<a href="option.md#0x1_option_is_some">is_some</a>(&amp;o) &amp;&amp; f(<a href="option.md#0x1_option_borrow">borrow</a>(&amp;o))) &#123;<br />        o<br />    &#125; <b>else</b> &#123;<br />        <a href="option.md#0x1_option_none">none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_any"></a>

## Function `any`

Returns true if the option contains an element which satisfies predicate.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_any">any</a>&lt;Element&gt;(o: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_any">any</a>&lt;Element&gt;(o: &amp;<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool &#123;<br />    <a href="option.md#0x1_option_is_some">is_some</a>(o) &amp;&amp; p(<a href="option.md#0x1_option_borrow">borrow</a>(o))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_option_destroy"></a>

## Function `destroy`

Utility function to destroy an option that is not droppable.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy">destroy</a>&lt;Element&gt;(o: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, d: &#124;Element&#124;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_destroy">destroy</a>&lt;Element&gt;(o: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, d: &#124;Element&#124;) &#123;<br />    <b>let</b> vec &#61; <a href="option.md#0x1_option_to_vec">to_vec</a>(o);<br />    <a href="vector.md#0x1_vector_destroy">vector::destroy</a>(vec, &#124;e&#124; d(e));<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Helper_Schema_2"></a>

### Helper Schema



<a id="0x1_option_AbortsIfNone"></a>


<pre><code><b>schema</b> <a href="option.md#0x1_option_AbortsIfNone">AbortsIfNone</a>&lt;Element&gt; &#123;<br />t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;;<br /><b>aborts_if</b> <a href="option.md#0x1_option_spec_is_none">spec_is_none</a>(t) <b>with</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_Option"></a>

### Struct `Option`


<pre><code><b>struct</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<dl>
<dt>
<code>vec: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;</code>
</dt>
<dd>

</dd>
</dl>


The size of vector is always less than equal to 1
because it&apos;s 0 for &quot;none&quot; or 1 for &quot;some&quot;.


<pre><code><b>invariant</b> len(vec) &lt;&#61; 1;<br /></code></pre>



<a id="@Specification_1_none"></a>

### Function `none`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_none">none</a>&lt;Element&gt;(): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_none">spec_none</a>&lt;Element&gt;();<br /></code></pre>




<a id="0x1_option_spec_none"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_none">spec_none</a>&lt;Element&gt;(): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; &#123;<br />   <a href="option.md#0x1_option_Option">Option</a>&#123; vec: vec() &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_some"></a>

### Function `some`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_some">some</a>&lt;Element&gt;(e: Element): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_some">spec_some</a>(e);<br /></code></pre>




<a id="0x1_option_spec_some"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_some">spec_some</a>&lt;Element&gt;(e: Element): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; &#123;<br />   <a href="option.md#0x1_option_Option">Option</a>&#123; vec: vec(e) &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_from_vec"></a>

### Function `from_vec`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_from_vec">from_vec</a>&lt;Element&gt;(vec: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="vector.md#0x1_vector_length">vector::length</a>(vec) &gt; 1;<br /></code></pre>



<a id="@Specification_1_is_none"></a>

### Function `is_none`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_none">is_none</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_is_none">spec_is_none</a>(t);<br /></code></pre>




<a id="0x1_option_spec_is_none"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_is_none">spec_is_none</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool &#123;<br />   <a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(t.vec)<br />&#125;<br /></code></pre>



<a id="@Specification_1_is_some"></a>

### Function `is_some`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_some">is_some</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t);<br /></code></pre>




<a id="0x1_option_spec_is_some"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_is_some">spec_is_some</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool &#123;<br />   !<a href="vector.md#0x1_vector_is_empty">vector::is_empty</a>(t.vec)<br />&#125;<br /></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_contains">contains</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e_ref: &amp;Element): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_contains">spec_contains</a>(t, e_ref);<br /></code></pre>




<a id="0x1_option_spec_contains"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_contains">spec_contains</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e: Element): bool &#123;<br />   <a href="option.md#0x1_option_is_some">is_some</a>(t) &amp;&amp; <a href="option.md#0x1_option_borrow">borrow</a>(t) &#61;&#61; e<br />&#125;<br /></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow">borrow</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): &amp;Element<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="option.md#0x1_option_AbortsIfNone">AbortsIfNone</a>&lt;Element&gt;;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t);<br /></code></pre>




<a id="0x1_option_spec_borrow"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): Element &#123;<br />   t.vec[0]<br />&#125;<br /></code></pre>



<a id="@Specification_1_borrow_with_default"></a>

### Function `borrow_with_default`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_with_default">borrow_with_default</a>&lt;Element&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default_ref: &amp;Element): &amp;Element<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; (<b>if</b> (<a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t)) <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t) <b>else</b> default_ref);<br /></code></pre>



<a id="@Specification_1_get_with_default"></a>

### Function `get_with_default`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_get_with_default">get_with_default</a>&lt;Element: <b>copy</b>, drop&gt;(t: &amp;<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default: Element): Element<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; (<b>if</b> (<a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t)) <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t) <b>else</b> default);<br /></code></pre>



<a id="@Specification_1_fill"></a>

### Function `fill`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_fill">fill</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t) <b>with</b> <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a>;<br /><b>ensures</b> <a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t);<br /><b>ensures</b> <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t) &#61;&#61; e;<br /></code></pre>



<a id="@Specification_1_extract"></a>

### Function `extract`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_extract">extract</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): Element<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="option.md#0x1_option_AbortsIfNone">AbortsIfNone</a>&lt;Element&gt;;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(<b>old</b>(t));<br /><b>ensures</b> <a href="option.md#0x1_option_spec_is_none">spec_is_none</a>(t);<br /></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_mut">borrow_mut</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): &amp;<b>mut</b> Element<br /></code></pre>




<pre><code><b>include</b> <a href="option.md#0x1_option_AbortsIfNone">AbortsIfNone</a>&lt;Element&gt;;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t);<br /><b>ensures</b> t &#61;&#61; <b>old</b>(t);<br /></code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap">swap</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element): Element<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="option.md#0x1_option_AbortsIfNone">AbortsIfNone</a>&lt;Element&gt;;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(<b>old</b>(t));<br /><b>ensures</b> <a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t);<br /><b>ensures</b> <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t) &#61;&#61; e;<br /></code></pre>



<a id="@Specification_1_swap_or_fill"></a>

### Function `swap_or_fill`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap_or_fill">swap_or_fill</a>&lt;Element&gt;(t: &amp;<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <b>old</b>(t);<br /><b>ensures</b> <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t) &#61;&#61; e;<br /></code></pre>



<a id="@Specification_1_destroy_with_default"></a>

### Function `destroy_with_default`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_with_default">destroy_with_default</a>&lt;Element: drop&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default: Element): Element<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; (<b>if</b> (<a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t)) <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t) <b>else</b> default);<br /></code></pre>



<a id="@Specification_1_destroy_some"></a>

### Function `destroy_some`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_some">destroy_some</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): Element<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="option.md#0x1_option_AbortsIfNone">AbortsIfNone</a>&lt;Element&gt;;<br /><b>ensures</b> result &#61;&#61; <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>(t);<br /></code></pre>



<a id="@Specification_1_destroy_none"></a>

### Function `destroy_none`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_none">destroy_none</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <a href="option.md#0x1_option_spec_is_some">spec_is_some</a>(t) <b>with</b> <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a>;<br /></code></pre>



<a id="@Specification_1_to_vec"></a>

### Function `to_vec`


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_to_vec">to_vec</a>&lt;Element&gt;(t: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; t.vec;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
