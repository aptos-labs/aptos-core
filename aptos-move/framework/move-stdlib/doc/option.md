
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


<pre><code>use 0x1::vector;<br/></code></pre>



<a id="0x1_option_Option"></a>

## Struct `Option`

Abstraction of a value that may or may not be present. Implemented with a vector of size
zero or one because Move bytecode does not have ADTs.


<pre><code>struct Option&lt;Element&gt; has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>vec: vector&lt;Element&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_option_EOPTION_IS_SET"></a>

The <code>Option</code> is in an invalid state for the operation attempted.
The <code>Option</code> is <code>Some</code> while it should be <code>None</code>.


<pre><code>const EOPTION_IS_SET: u64 &#61; 262144;<br/></code></pre>



<a id="0x1_option_EOPTION_NOT_SET"></a>

The <code>Option</code> is in an invalid state for the operation attempted.
The <code>Option</code> is <code>None</code> while it should be <code>Some</code>.


<pre><code>const EOPTION_NOT_SET: u64 &#61; 262145;<br/></code></pre>



<a id="0x1_option_EOPTION_VEC_TOO_LONG"></a>

Cannot construct an option from a vector with 2 or more elements.


<pre><code>const EOPTION_VEC_TOO_LONG: u64 &#61; 262146;<br/></code></pre>



<a id="0x1_option_none"></a>

## Function `none`

Return an empty <code>Option</code>


<pre><code>public fun none&lt;Element&gt;(): option::Option&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun none&lt;Element&gt;(): Option&lt;Element&gt; &#123;<br/>    Option &#123; vec: vector::empty() &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_some"></a>

## Function `some`

Return an <code>Option</code> containing <code>e</code>


<pre><code>public fun some&lt;Element&gt;(e: Element): option::Option&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun some&lt;Element&gt;(e: Element): Option&lt;Element&gt; &#123;<br/>    Option &#123; vec: vector::singleton(e) &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_from_vec"></a>

## Function `from_vec`



<pre><code>public fun from_vec&lt;Element&gt;(vec: vector&lt;Element&gt;): option::Option&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun from_vec&lt;Element&gt;(vec: vector&lt;Element&gt;): Option&lt;Element&gt; &#123;<br/>    assert!(vector::length(&amp;vec) &lt;&#61; 1, EOPTION_VEC_TOO_LONG);<br/>    Option &#123; vec &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_is_none"></a>

## Function `is_none`

Return true if <code>t</code> does not hold a value


<pre><code>public fun is_none&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_none&lt;Element&gt;(t: &amp;Option&lt;Element&gt;): bool &#123;<br/>    vector::is_empty(&amp;t.vec)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_is_some"></a>

## Function `is_some`

Return true if <code>t</code> holds a value


<pre><code>public fun is_some&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_some&lt;Element&gt;(t: &amp;Option&lt;Element&gt;): bool &#123;<br/>    !vector::is_empty(&amp;t.vec)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_contains"></a>

## Function `contains`

Return true if the value in <code>t</code> is equal to <code>e_ref</code>
Always returns <code>false</code> if <code>t</code> does not hold a value


<pre><code>public fun contains&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;, e_ref: &amp;Element): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains&lt;Element&gt;(t: &amp;Option&lt;Element&gt;, e_ref: &amp;Element): bool &#123;<br/>    vector::contains(&amp;t.vec, e_ref)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_borrow"></a>

## Function `borrow`

Return an immutable reference to the value inside <code>t</code>
Aborts if <code>t</code> does not hold a value


<pre><code>public fun borrow&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;): &amp;Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow&lt;Element&gt;(t: &amp;Option&lt;Element&gt;): &amp;Element &#123;<br/>    assert!(is_some(t), EOPTION_NOT_SET);<br/>    vector::borrow(&amp;t.vec, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_borrow_with_default"></a>

## Function `borrow_with_default`

Return a reference to the value inside <code>t</code> if it holds one
Return <code>default_ref</code> if <code>t</code> does not hold a value


<pre><code>public fun borrow_with_default&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;, default_ref: &amp;Element): &amp;Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_with_default&lt;Element&gt;(t: &amp;Option&lt;Element&gt;, default_ref: &amp;Element): &amp;Element &#123;<br/>    let vec_ref &#61; &amp;t.vec;<br/>    if (vector::is_empty(vec_ref)) default_ref<br/>    else vector::borrow(vec_ref, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_get_with_default"></a>

## Function `get_with_default`

Return the value inside <code>t</code> if it holds one
Return <code>default</code> if <code>t</code> does not hold a value


<pre><code>public fun get_with_default&lt;Element: copy, drop&gt;(t: &amp;option::Option&lt;Element&gt;, default: Element): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_with_default&lt;Element: copy &#43; drop&gt;(<br/>    t: &amp;Option&lt;Element&gt;,<br/>    default: Element,<br/>): Element &#123;<br/>    let vec_ref &#61; &amp;t.vec;<br/>    if (vector::is_empty(vec_ref)) default<br/>    else &#42;vector::borrow(vec_ref, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_fill"></a>

## Function `fill`

Convert the none option <code>t</code> to a some option by adding <code>e</code>.
Aborts if <code>t</code> already holds a value


<pre><code>public fun fill&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;, e: Element)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun fill&lt;Element&gt;(t: &amp;mut Option&lt;Element&gt;, e: Element) &#123;<br/>    let vec_ref &#61; &amp;mut t.vec;<br/>    if (vector::is_empty(vec_ref)) vector::push_back(vec_ref, e)<br/>    else abort EOPTION_IS_SET<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_extract"></a>

## Function `extract`

Convert a <code>some</code> option to a <code>none</code> by removing and returning the value stored inside <code>t</code>
Aborts if <code>t</code> does not hold a value


<pre><code>public fun extract&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract&lt;Element&gt;(t: &amp;mut Option&lt;Element&gt;): Element &#123;<br/>    assert!(is_some(t), EOPTION_NOT_SET);<br/>    vector::pop_back(&amp;mut t.vec)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the value inside <code>t</code>
Aborts if <code>t</code> does not hold a value


<pre><code>public fun borrow_mut&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;): &amp;mut Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun borrow_mut&lt;Element&gt;(t: &amp;mut Option&lt;Element&gt;): &amp;mut Element &#123;<br/>    assert!(is_some(t), EOPTION_NOT_SET);<br/>    vector::borrow_mut(&amp;mut t.vec, 0)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_swap"></a>

## Function `swap`

Swap the old value inside <code>t</code> with <code>e</code> and return the old value
Aborts if <code>t</code> does not hold a value


<pre><code>public fun swap&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;, e: Element): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap&lt;Element&gt;(t: &amp;mut Option&lt;Element&gt;, e: Element): Element &#123;<br/>    assert!(is_some(t), EOPTION_NOT_SET);<br/>    let vec_ref &#61; &amp;mut t.vec;<br/>    let old_value &#61; vector::pop_back(vec_ref);<br/>    vector::push_back(vec_ref, e);<br/>    old_value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_swap_or_fill"></a>

## Function `swap_or_fill`

Swap the old value inside <code>t</code> with <code>e</code> and return the old value;
or if there is no old value, fill it with <code>e</code>.
Different from swap(), swap_or_fill() allows for <code>t</code> not holding a value.


<pre><code>public fun swap_or_fill&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;, e: Element): option::Option&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun swap_or_fill&lt;Element&gt;(t: &amp;mut Option&lt;Element&gt;, e: Element): Option&lt;Element&gt; &#123;<br/>    let vec_ref &#61; &amp;mut t.vec;<br/>    let old_value &#61; if (vector::is_empty(vec_ref)) none()<br/>        else some(vector::pop_back(vec_ref));<br/>    vector::push_back(vec_ref, e);<br/>    old_value<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_destroy_with_default"></a>

## Function `destroy_with_default`

Destroys <code>t.</code> If <code>t</code> holds a value, return it. Returns <code>default</code> otherwise


<pre><code>public fun destroy_with_default&lt;Element: drop&gt;(t: option::Option&lt;Element&gt;, default: Element): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_with_default&lt;Element: drop&gt;(t: Option&lt;Element&gt;, default: Element): Element &#123;<br/>    let Option &#123; vec &#125; &#61; t;<br/>    if (vector::is_empty(&amp;mut vec)) default<br/>    else vector::pop_back(&amp;mut vec)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_destroy_some"></a>

## Function `destroy_some`

Unpack <code>t</code> and return its contents
Aborts if <code>t</code> does not hold a value


<pre><code>public fun destroy_some&lt;Element&gt;(t: option::Option&lt;Element&gt;): Element<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_some&lt;Element&gt;(t: Option&lt;Element&gt;): Element &#123;<br/>    assert!(is_some(&amp;t), EOPTION_NOT_SET);<br/>    let Option &#123; vec &#125; &#61; t;<br/>    let elem &#61; vector::pop_back(&amp;mut vec);<br/>    vector::destroy_empty(vec);<br/>    elem<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_destroy_none"></a>

## Function `destroy_none`

Unpack <code>t</code>
Aborts if <code>t</code> holds a value


<pre><code>public fun destroy_none&lt;Element&gt;(t: option::Option&lt;Element&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_none&lt;Element&gt;(t: Option&lt;Element&gt;) &#123;<br/>    assert!(is_none(&amp;t), EOPTION_IS_SET);<br/>    let Option &#123; vec &#125; &#61; t;<br/>    vector::destroy_empty(vec)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_to_vec"></a>

## Function `to_vec`

Convert <code>t</code> into a vector of length 1 if it is <code>Some</code>,
and an empty vector otherwise


<pre><code>public fun to_vec&lt;Element&gt;(t: option::Option&lt;Element&gt;): vector&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun to_vec&lt;Element&gt;(t: Option&lt;Element&gt;): vector&lt;Element&gt; &#123;<br/>    let Option &#123; vec &#125; &#61; t;<br/>    vec<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_for_each"></a>

## Function `for_each`

Apply the function to the optional element, consuming it. Does nothing if no value present.


<pre><code>public fun for_each&lt;Element&gt;(o: option::Option&lt;Element&gt;, f: &#124;Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each&lt;Element&gt;(o: Option&lt;Element&gt;, f: &#124;Element&#124;) &#123;<br/>    if (is_some(&amp;o)) &#123;<br/>        f(destroy_some(o))<br/>    &#125; else &#123;<br/>        destroy_none(o)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to the optional element reference. Does nothing if no value present.


<pre><code>public fun for_each_ref&lt;Element&gt;(o: &amp;option::Option&lt;Element&gt;, f: &#124;&amp;Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_ref&lt;Element&gt;(o: &amp;Option&lt;Element&gt;, f: &#124;&amp;Element&#124;) &#123;<br/>    if (is_some(o)) &#123;<br/>        f(borrow(o))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to the optional element reference. Does nothing if no value present.


<pre><code>public fun for_each_mut&lt;Element&gt;(o: &amp;mut option::Option&lt;Element&gt;, f: &#124;&amp;mut Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun for_each_mut&lt;Element&gt;(o: &amp;mut Option&lt;Element&gt;, f: &#124;&amp;mut Element&#124;) &#123;<br/>    if (is_some(o)) &#123;<br/>        f(borrow_mut(o))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_fold"></a>

## Function `fold`

Folds the function over the optional element.


<pre><code>public fun fold&lt;Accumulator, Element&gt;(o: option::Option&lt;Element&gt;, init: Accumulator, f: &#124;(Accumulator, Element)&#124;Accumulator): Accumulator<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun fold&lt;Accumulator, Element&gt;(<br/>    o: Option&lt;Element&gt;,<br/>    init: Accumulator,<br/>    f: &#124;Accumulator,Element&#124;Accumulator<br/>): Accumulator &#123;<br/>    if (is_some(&amp;o)) &#123;<br/>        f(init, destroy_some(o))<br/>    &#125; else &#123;<br/>        destroy_none(o);<br/>        init<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_map"></a>

## Function `map`

Maps the content of an option.


<pre><code>public fun map&lt;Element, OtherElement&gt;(o: option::Option&lt;Element&gt;, f: &#124;Element&#124;OtherElement): option::Option&lt;OtherElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map&lt;Element, OtherElement&gt;(o: Option&lt;Element&gt;, f: &#124;Element&#124;OtherElement): Option&lt;OtherElement&gt; &#123;<br/>    if (is_some(&amp;o)) &#123;<br/>        some(f(destroy_some(o)))<br/>    &#125; else &#123;<br/>        destroy_none(o);<br/>        none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_map_ref"></a>

## Function `map_ref`

Maps the content of an option without destroying the original option.


<pre><code>public fun map_ref&lt;Element, OtherElement&gt;(o: &amp;option::Option&lt;Element&gt;, f: &#124;&amp;Element&#124;OtherElement): option::Option&lt;OtherElement&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun map_ref&lt;Element, OtherElement&gt;(<br/>    o: &amp;Option&lt;Element&gt;, f: &#124;&amp;Element&#124;OtherElement): Option&lt;OtherElement&gt; &#123;<br/>    if (is_some(o)) &#123;<br/>        some(f(borrow(o)))<br/>    &#125; else &#123;<br/>        none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_filter"></a>

## Function `filter`

Filters the content of an option


<pre><code>public fun filter&lt;Element: drop&gt;(o: option::Option&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): option::Option&lt;Element&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun filter&lt;Element:drop&gt;(o: Option&lt;Element&gt;, f: &#124;&amp;Element&#124;bool): Option&lt;Element&gt; &#123;<br/>    if (is_some(&amp;o) &amp;&amp; f(borrow(&amp;o))) &#123;<br/>        o<br/>    &#125; else &#123;<br/>        none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_any"></a>

## Function `any`

Returns true if the option contains an element which satisfies predicate.


<pre><code>public fun any&lt;Element&gt;(o: &amp;option::Option&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun any&lt;Element&gt;(o: &amp;Option&lt;Element&gt;, p: &#124;&amp;Element&#124;bool): bool &#123;<br/>    is_some(o) &amp;&amp; p(borrow(o))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_option_destroy"></a>

## Function `destroy`

Utility function to destroy an option that is not droppable.


<pre><code>public fun destroy&lt;Element&gt;(o: option::Option&lt;Element&gt;, d: &#124;Element&#124;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public inline fun destroy&lt;Element&gt;(o: Option&lt;Element&gt;, d: &#124;Element&#124;) &#123;<br/>    let vec &#61; to_vec(o);<br/>    vector::destroy(vec, &#124;e&#124; d(e));<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<pre><code>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Helper_Schema_2"></a>

### Helper Schema



<a id="0x1_option_AbortsIfNone"></a>


<pre><code>schema AbortsIfNone&lt;Element&gt; &#123;<br/>t: Option&lt;Element&gt;;<br/>aborts_if spec_is_none(t) with EOPTION_NOT_SET;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_Option"></a>

### Struct `Option`


<pre><code>struct Option&lt;Element&gt; has copy, drop, store<br/></code></pre>



<dl>
<dt>
<code>vec: vector&lt;Element&gt;</code>
</dt>
<dd>

</dd>
</dl>


The size of vector is always less than equal to 1
because it's 0 for "none" or 1 for "some".


<pre><code>invariant len(vec) &lt;&#61; 1;<br/></code></pre>



<a id="@Specification_1_none"></a>

### Function `none`


<pre><code>public fun none&lt;Element&gt;(): option::Option&lt;Element&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_none&lt;Element&gt;();<br/></code></pre>




<a id="0x1_option_spec_none"></a>


<pre><code>fun spec_none&lt;Element&gt;(): Option&lt;Element&gt; &#123;<br/>   Option&#123; vec: vec() &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_some"></a>

### Function `some`


<pre><code>public fun some&lt;Element&gt;(e: Element): option::Option&lt;Element&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_some(e);<br/></code></pre>




<a id="0x1_option_spec_some"></a>


<pre><code>fun spec_some&lt;Element&gt;(e: Element): Option&lt;Element&gt; &#123;<br/>   Option&#123; vec: vec(e) &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_from_vec"></a>

### Function `from_vec`


<pre><code>public fun from_vec&lt;Element&gt;(vec: vector&lt;Element&gt;): option::Option&lt;Element&gt;<br/></code></pre>




<pre><code>aborts_if vector::length(vec) &gt; 1;<br/></code></pre>



<a id="@Specification_1_is_none"></a>

### Function `is_none`


<pre><code>public fun is_none&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_is_none(t);<br/></code></pre>




<a id="0x1_option_spec_is_none"></a>


<pre><code>fun spec_is_none&lt;Element&gt;(t: Option&lt;Element&gt;): bool &#123;<br/>   vector::is_empty(t.vec)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_is_some"></a>

### Function `is_some`


<pre><code>public fun is_some&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_is_some(t);<br/></code></pre>




<a id="0x1_option_spec_is_some"></a>


<pre><code>fun spec_is_some&lt;Element&gt;(t: Option&lt;Element&gt;): bool &#123;<br/>   !vector::is_empty(t.vec)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code>public fun contains&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;, e_ref: &amp;Element): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_contains(t, e_ref);<br/></code></pre>




<a id="0x1_option_spec_contains"></a>


<pre><code>fun spec_contains&lt;Element&gt;(t: Option&lt;Element&gt;, e: Element): bool &#123;<br/>   is_some(t) &amp;&amp; borrow(t) &#61;&#61; e<br/>&#125;<br/></code></pre>



<a id="@Specification_1_borrow"></a>

### Function `borrow`


<pre><code>public fun borrow&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;): &amp;Element<br/></code></pre>




<pre><code>pragma opaque;<br/>include AbortsIfNone&lt;Element&gt;;<br/>ensures result &#61;&#61; spec_borrow(t);<br/></code></pre>




<a id="0x1_option_spec_borrow"></a>


<pre><code>fun spec_borrow&lt;Element&gt;(t: Option&lt;Element&gt;): Element &#123;<br/>   t.vec[0]<br/>&#125;<br/></code></pre>



<a id="@Specification_1_borrow_with_default"></a>

### Function `borrow_with_default`


<pre><code>public fun borrow_with_default&lt;Element&gt;(t: &amp;option::Option&lt;Element&gt;, default_ref: &amp;Element): &amp;Element<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; (if (spec_is_some(t)) spec_borrow(t) else default_ref);<br/></code></pre>



<a id="@Specification_1_get_with_default"></a>

### Function `get_with_default`


<pre><code>public fun get_with_default&lt;Element: copy, drop&gt;(t: &amp;option::Option&lt;Element&gt;, default: Element): Element<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; (if (spec_is_some(t)) spec_borrow(t) else default);<br/></code></pre>



<a id="@Specification_1_fill"></a>

### Function `fill`


<pre><code>public fun fill&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;, e: Element)<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if spec_is_some(t) with EOPTION_IS_SET;<br/>ensures spec_is_some(t);<br/>ensures spec_borrow(t) &#61;&#61; e;<br/></code></pre>



<a id="@Specification_1_extract"></a>

### Function `extract`


<pre><code>public fun extract&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;): Element<br/></code></pre>




<pre><code>pragma opaque;<br/>include AbortsIfNone&lt;Element&gt;;<br/>ensures result &#61;&#61; spec_borrow(old(t));<br/>ensures spec_is_none(t);<br/></code></pre>



<a id="@Specification_1_borrow_mut"></a>

### Function `borrow_mut`


<pre><code>public fun borrow_mut&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;): &amp;mut Element<br/></code></pre>




<pre><code>include AbortsIfNone&lt;Element&gt;;<br/>ensures result &#61;&#61; spec_borrow(t);<br/>ensures t &#61;&#61; old(t);<br/></code></pre>



<a id="@Specification_1_swap"></a>

### Function `swap`


<pre><code>public fun swap&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;, e: Element): Element<br/></code></pre>




<pre><code>pragma opaque;<br/>include AbortsIfNone&lt;Element&gt;;<br/>ensures result &#61;&#61; spec_borrow(old(t));<br/>ensures spec_is_some(t);<br/>ensures spec_borrow(t) &#61;&#61; e;<br/></code></pre>



<a id="@Specification_1_swap_or_fill"></a>

### Function `swap_or_fill`


<pre><code>public fun swap_or_fill&lt;Element&gt;(t: &amp;mut option::Option&lt;Element&gt;, e: Element): option::Option&lt;Element&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; old(t);<br/>ensures spec_borrow(t) &#61;&#61; e;<br/></code></pre>



<a id="@Specification_1_destroy_with_default"></a>

### Function `destroy_with_default`


<pre><code>public fun destroy_with_default&lt;Element: drop&gt;(t: option::Option&lt;Element&gt;, default: Element): Element<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; (if (spec_is_some(t)) spec_borrow(t) else default);<br/></code></pre>



<a id="@Specification_1_destroy_some"></a>

### Function `destroy_some`


<pre><code>public fun destroy_some&lt;Element&gt;(t: option::Option&lt;Element&gt;): Element<br/></code></pre>




<pre><code>pragma opaque;<br/>include AbortsIfNone&lt;Element&gt;;<br/>ensures result &#61;&#61; spec_borrow(t);<br/></code></pre>



<a id="@Specification_1_destroy_none"></a>

### Function `destroy_none`


<pre><code>public fun destroy_none&lt;Element&gt;(t: option::Option&lt;Element&gt;)<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if spec_is_some(t) with EOPTION_IS_SET;<br/></code></pre>



<a id="@Specification_1_to_vec"></a>

### Function `to_vec`


<pre><code>public fun to_vec&lt;Element&gt;(t: option::Option&lt;Element&gt;): vector&lt;Element&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if false;<br/>ensures result &#61;&#61; t.vec;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
