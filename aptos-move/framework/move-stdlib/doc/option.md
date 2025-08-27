
<a id="0x1_option"></a>

# Module `0x1::option`

This module defines the Option type and its methods to represent and handle an optional value.


-  [Enum `Option`](#0x1_option_Option)
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


<pre><code><b>use</b> <a href="mem.md#0x1_mem">0x1::mem</a>;
</code></pre>



<a id="0x1_option_Option"></a>

## Enum `Option`

Abstraction of a value that may or may not be present. Implemented with a vector of size
zero or one because Move bytecode does not have ADTs.


<pre><code>enum <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>None</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>Some</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>e: Element</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_option_EOPTION_IS_SET"></a>

The <code><a href="option.md#0x1_option_Option">Option</a></code> is in an invalid state for the operation attempted.
The <code><a href="option.md#0x1_option_Option">Option</a></code> is <code>Some</code> while it should be <code>None</code>.


<pre><code><b>const</b> <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a>: u64 = 262144;
</code></pre>



<a id="0x1_option_EOPTION_NOT_SET"></a>

The <code><a href="option.md#0x1_option_Option">Option</a></code> is in an invalid state for the operation attempted.
The <code><a href="option.md#0x1_option_Option">Option</a></code> is <code>None</code> while it should be <code>Some</code>.


<pre><code><b>const</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>: u64 = 262145;
</code></pre>



<a id="0x1_option_EOPTION_VEC_TOO_LONG"></a>

Cannot construct an option from a vector with 2 or more elements.


<pre><code><b>const</b> <a href="option.md#0x1_option_EOPTION_VEC_TOO_LONG">EOPTION_VEC_TOO_LONG</a>: u64 = 262146;
</code></pre>



<a id="0x1_option_none"></a>

## Function `none`

Return an empty <code><a href="option.md#0x1_option_Option">Option</a></code>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_none">none</a>&lt;Element&gt;(): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_none">none</a>&lt;Element&gt;(): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; {
    Option::None
}
</code></pre>



</details>

<a id="0x1_option_some"></a>

## Function `some`

Return an <code><a href="option.md#0x1_option_Option">Option</a></code> containing <code>e</code>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_some">some</a>&lt;Element&gt;(e: Element): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_some">some</a>&lt;Element&gt;(e: Element): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; {
    Option::Some { e }
}
</code></pre>



</details>

<a id="0x1_option_from_vec"></a>

## Function `from_vec`



<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_from_vec">from_vec</a>&lt;Element&gt;(vec: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_from_vec">from_vec</a>&lt;Element&gt;(vec: <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; {
    <b>assert</b>!(vec.length() &lt;= 1, <a href="option.md#0x1_option_EOPTION_VEC_TOO_LONG">EOPTION_VEC_TOO_LONG</a>);
    <b>if</b> (vec.is_empty()) {
        vec.destroy_empty();
        Option::None
    } <b>else</b> {
        <b>let</b> e = vec.pop_back();
        vec.destroy_empty();
        Option::Some { e }
    }
}
</code></pre>



</details>

<a id="0x1_option_is_none"></a>

## Function `is_none`

Return true if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_none">is_none</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_none">is_none</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool {
    self is Option::None
}
</code></pre>



</details>

<a id="0x1_option_is_some"></a>

## Function `is_some`

Return true if <code>self</code> holds a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_some">is_some</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_is_some">is_some</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool {
    self is Option::Some
}
</code></pre>



</details>

<a id="0x1_option_contains"></a>

## Function `contains`

Return true if the value in <code>self</code> is equal to <code>e_ref</code>
Always returns <code><b>false</b></code> if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_contains">contains</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e_ref: &Element): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_contains">contains</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e_ref: &Element): bool {
    match (self) {
        Option::None =&gt; <b>false</b>,
        Option::Some { e } =&gt; e == e_ref,
    }
}
</code></pre>



</details>

<a id="0x1_option_borrow"></a>

## Function `borrow`

Return an immutable reference to the value inside <code>self</code>
Aborts if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow">borrow</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): &Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow">borrow</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): &Element {
    match (self) {
        Option::None =&gt; {
            <b>abort</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>
        },
        Option::Some { e } =&gt; e,
    }
}
</code></pre>



</details>

<a id="0x1_option_borrow_with_default"></a>

## Function `borrow_with_default`

Return a reference to the value inside <code>self</code> if it holds one
Return <code>default_ref</code> if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_with_default">borrow_with_default</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default_ref: &Element): &Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_with_default">borrow_with_default</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, default_ref: &Element): &Element {
    match (self) {
        Option::None =&gt; default_ref,
        Option::Some { e } =&gt; e,
    }
}
</code></pre>



</details>

<a id="0x1_option_get_with_default"></a>

## Function `get_with_default`

Return the value inside <code>self</code> if it holds one
Return <code>default</code> if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_get_with_default">get_with_default</a>&lt;Element: <b>copy</b>, drop&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default: Element): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_get_with_default">get_with_default</a>&lt;Element: <b>copy</b> + drop&gt;(
    self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;,
    default: Element,
): Element {
    match (self) {
        Option::None =&gt; default,
        Option::Some { e } =&gt; *e,
    }
}
</code></pre>



</details>

<a id="0x1_option_fill"></a>

## Function `fill`

Convert the none option <code>self</code> to a some option by adding <code>e</code>.
Aborts if <code>self</code> already holds a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_fill">fill</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_fill">fill</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e: Element) {
    <b>let</b> <b>old</b> = <a href="mem.md#0x1_mem_replace">mem::replace</a>(self, Option::Some { e });
    match (<b>old</b>) {
        Option::None =&gt; {},
        Option::Some { e: _ } =&gt; {
           <b>abort</b> <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a>
        },
    }
}
</code></pre>



</details>

<a id="0x1_option_extract"></a>

## Function `extract`

Convert a <code>some</code> option to a <code>none</code> by removing and returning the value stored inside <code>self</code>
Aborts if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_extract">extract</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_extract">extract</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): Element {
    <b>let</b> inner = <a href="mem.md#0x1_mem_replace">mem::replace</a>(self, Option::None);
    match (inner) {
        Option::Some { e } =&gt; e,
        Option::None =&gt; {
           <b>abort</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>
        },
    }
}
</code></pre>



</details>

<a id="0x1_option_borrow_mut"></a>

## Function `borrow_mut`

Return a mutable reference to the value inside <code>self</code>
Aborts if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_mut">borrow_mut</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): &<b>mut</b> Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_borrow_mut">borrow_mut</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): &<b>mut</b> Element {
    match (self) {
        Option::None =&gt; {
            <b>abort</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>
        },
        Option::Some { e } =&gt; e,
    }
}
</code></pre>



</details>

<a id="0x1_option_swap"></a>

## Function `swap`

Swap the old value inside <code>self</code> with <code>e</code> and return the old value
Aborts if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap">swap</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, el: Element): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap">swap</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, el: Element): Element {
    match (self) {
        Option::None =&gt; {
            <b>abort</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>
        },
        Option::Some { e } =&gt; {
            <a href="mem.md#0x1_mem_replace">mem::replace</a>(e, el)
        },
    }
}
</code></pre>



</details>

<a id="0x1_option_swap_or_fill"></a>

## Function `swap_or_fill`

Swap the old value inside <code>self</code> with <code>e</code> and return the old value;
or if there is no old value, fill it with <code>e</code>.
Different from swap(), swap_or_fill() allows for <code>self</code> not holding a value.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap_or_fill">swap_or_fill</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, e: Element): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_swap_or_fill">swap_or_fill</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e: Element): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; {
    <a href="mem.md#0x1_mem_replace">mem::replace</a>(self, Option::Some { e })
}
</code></pre>



</details>

<a id="0x1_option_destroy_with_default"></a>

## Function `destroy_with_default`

Destroys <code>self.</code> If <code>self</code> holds a value, return it. Returns <code>default</code> otherwise


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_with_default">destroy_with_default</a>&lt;Element: drop&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, default: Element): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_with_default">destroy_with_default</a>&lt;Element: drop&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, default: Element): Element {
    match (self) {
        Option::None =&gt; default,
        Option::Some { e } =&gt; e,
    }
}
</code></pre>



</details>

<a id="0x1_option_destroy_some"></a>

## Function `destroy_some`

Unpack <code>self</code> and return its contents
Aborts if <code>self</code> does not hold a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_some">destroy_some</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): Element
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_some">destroy_some</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): Element {
    match (self) {
        Option::None =&gt; {
            <b>abort</b> <a href="option.md#0x1_option_EOPTION_NOT_SET">EOPTION_NOT_SET</a>
        },
        Option::Some { e } =&gt; e,
    }
}
</code></pre>



</details>

<a id="0x1_option_destroy_none"></a>

## Function `destroy_none`

Unpack <code>self</code>
Aborts if <code>self</code> holds a value


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_none">destroy_none</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy_none">destroy_none</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;) {
    match (self) {
        Option::None =&gt; {},
        Option::Some { e: _ } =&gt; {
            <b>abort</b> <a href="option.md#0x1_option_EOPTION_IS_SET">EOPTION_IS_SET</a>
        },
    }
}
</code></pre>



</details>

<a id="0x1_option_to_vec"></a>

## Function `to_vec`

Convert <code>self</code> into a vector of length 1 if it is <code>Some</code>,
and an empty vector otherwise


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_to_vec">to_vec</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_to_vec">to_vec</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): <a href="vector.md#0x1_vector">vector</a>&lt;Element&gt; {
    match (self) {
        Option::None =&gt; <a href="vector.md#0x1_vector_empty">vector::empty</a>(),
        Option::Some { e } =&gt; <a href="vector.md#0x1_vector_singleton">vector::singleton</a>(e),
    }
}
</code></pre>



</details>

<a id="0x1_option_for_each"></a>

## Function `for_each`

Apply the function to the optional element, consuming it. Does nothing if no value present.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_for_each">for_each</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: |Element|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_for_each">for_each</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: |Element|) {
    <b>if</b> (self.<a href="option.md#0x1_option_is_some">is_some</a>()) {
        f(self.<a href="option.md#0x1_option_destroy_some">destroy_some</a>())
    } <b>else</b> {
        self.<a href="option.md#0x1_option_destroy_none">destroy_none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_option_for_each_ref"></a>

## Function `for_each_ref`

Apply the function to the optional element reference. Does nothing if no value present.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_for_each_ref">for_each_ref</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: |&Element|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_for_each_ref">for_each_ref</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: |&Element|) {
    <b>if</b> (self.<a href="option.md#0x1_option_is_some">is_some</a>()) {
        f(self.<a href="option.md#0x1_option_borrow">borrow</a>())
    }
}
</code></pre>



</details>

<a id="0x1_option_for_each_mut"></a>

## Function `for_each_mut`

Apply the function to the optional element reference. Does nothing if no value present.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_for_each_mut">for_each_mut</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: |&<b>mut</b> Element|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_for_each_mut">for_each_mut</a>&lt;Element&gt;(self: &<b>mut</b> <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: |&<b>mut</b> Element|) {
    <b>if</b> (self.<a href="option.md#0x1_option_is_some">is_some</a>()) {
        f(self.<a href="option.md#0x1_option_borrow_mut">borrow_mut</a>())
    }
}
</code></pre>



</details>

<a id="0x1_option_fold"></a>

## Function `fold`

Folds the function over the optional element.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_fold">fold</a>&lt;Accumulator, Element&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, init: Accumulator, f: |Accumulator, Element|Accumulator): Accumulator
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_fold">fold</a>&lt;Accumulator, Element&gt;(
    self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;,
    init: Accumulator,
    f: |Accumulator,Element|Accumulator
): Accumulator {
    <b>if</b> (self.<a href="option.md#0x1_option_is_some">is_some</a>()) {
        f(init, self.<a href="option.md#0x1_option_destroy_some">destroy_some</a>())
    } <b>else</b> {
        self.<a href="option.md#0x1_option_destroy_none">destroy_none</a>();
        init
    }
}
</code></pre>



</details>

<a id="0x1_option_map"></a>

## Function `map`

Maps the content of an option.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_map">map</a>&lt;Element, OtherElement&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: |Element|OtherElement): <a href="option.md#0x1_option_Option">option::Option</a>&lt;OtherElement&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_map">map</a>&lt;Element, OtherElement&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: |Element|OtherElement): <a href="option.md#0x1_option_Option">Option</a>&lt;OtherElement&gt; {
    <b>if</b> (self.<a href="option.md#0x1_option_is_some">is_some</a>()) {
        <a href="option.md#0x1_option_some">some</a>(f(self.<a href="option.md#0x1_option_destroy_some">destroy_some</a>()))
    } <b>else</b> {
        self.<a href="option.md#0x1_option_destroy_none">destroy_none</a>();
        <a href="option.md#0x1_option_none">none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_option_map_ref"></a>

## Function `map_ref`

Maps the content of an option without destroying the original option.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_map_ref">map_ref</a>&lt;Element, OtherElement&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: |&Element|OtherElement): <a href="option.md#0x1_option_Option">option::Option</a>&lt;OtherElement&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_map_ref">map_ref</a>&lt;Element, OtherElement&gt;(
    self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: |&Element|OtherElement): <a href="option.md#0x1_option_Option">Option</a>&lt;OtherElement&gt; {
    <b>if</b> (self.<a href="option.md#0x1_option_is_some">is_some</a>()) {
        <a href="option.md#0x1_option_some">some</a>(f(self.<a href="option.md#0x1_option_borrow">borrow</a>()))
    } <b>else</b> {
        <a href="option.md#0x1_option_none">none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_option_filter"></a>

## Function `filter`

Filters the content of an option


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_filter">filter</a>&lt;Element: drop&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, f: |&Element|bool): <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_filter">filter</a>&lt;Element:drop&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, f: |&Element|bool): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; {
    <b>if</b> (self.<a href="option.md#0x1_option_is_some">is_some</a>() && f(self.<a href="option.md#0x1_option_borrow">borrow</a>())) {
        self
    } <b>else</b> {
        <a href="option.md#0x1_option_none">none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_option_any"></a>

## Function `any`

Returns true if the option contains an element which satisfies predicate.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_any">any</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, p: |&Element|bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_any">any</a>&lt;Element&gt;(self: &<a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, p: |&Element|bool): bool {
    self.<a href="option.md#0x1_option_is_some">is_some</a>() && p(self.<a href="option.md#0x1_option_borrow">borrow</a>())
}
</code></pre>



</details>

<a id="0x1_option_destroy"></a>

## Function `destroy`

Utility function to destroy an option that is not droppable.


<pre><code><b>public</b> <b>fun</b> <a href="option.md#0x1_option_destroy">destroy</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">option::Option</a>&lt;Element&gt;, d: |Element|)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="option.md#0x1_option_destroy">destroy</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, d: |Element|) {
    <b>let</b> vec = self.<a href="option.md#0x1_option_to_vec">to_vec</a>();
    vec.<a href="option.md#0x1_option_destroy">destroy</a>(|e| d(e));
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<a id="0x1_option_spec_is_some"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_is_some">spec_is_some</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool {
   <b>true</b>
}
</code></pre>




<a id="0x1_option_spec_is_none"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_is_none">spec_is_none</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): bool {
   <b>false</b>
}
</code></pre>




<a id="0x1_option_spec_borrow"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_borrow">spec_borrow</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;): Element {
   <b>abort</b> 0
}
</code></pre>




<a id="0x1_option_spec_some"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_some">spec_some</a>&lt;Element&gt;(e: Element): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; {
   <b>abort</b> 0
}
</code></pre>




<a id="0x1_option_spec_none"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_none">spec_none</a>&lt;Element&gt;(): <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt; {
   <b>abort</b> 0
}
</code></pre>




<a id="0x1_option_spec_contains"></a>


<pre><code><b>fun</b> <a href="option.md#0x1_option_spec_contains">spec_contains</a>&lt;Element&gt;(self: <a href="option.md#0x1_option_Option">Option</a>&lt;Element&gt;, e: Element): bool {
   <b>false</b>
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
