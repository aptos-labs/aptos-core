
<a id="0x1_util"></a>

# Module `0x1::util`

Utility functions used by the framework modules.


-  [Function `from_bytes`](#0x1_util_from_bytes)
-  [Function `address_from_bytes`](#0x1_util_address_from_bytes)
-  [Specification](#@Specification_0)
    -  [Function `from_bytes`](#@Specification_0_from_bytes)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `address_from_bytes`](#@Specification_0_address_from_bytes)


<pre><code></code></pre>



<a id="0x1_util_from_bytes"></a>

## Function `from_bytes`

Native function to deserialize a type T.<br/><br/> Note that this function does not put any constraint on <code>T</code>. If code uses this function to<br/> deserialized a linear value, its their responsibility that the data they deserialize is<br/> owned.


<pre><code>public(friend) fun from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) native fun from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T;<br/></code></pre>



</details>

<a id="0x1_util_address_from_bytes"></a>

## Function `address_from_bytes`



<pre><code>public fun address_from_bytes(bytes: vector&lt;u8&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_from_bytes(bytes: vector&lt;u8&gt;): address &#123;<br/>    from_bytes(bytes)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_from_bytes"></a>

### Function `from_bytes`


<pre><code>public(friend) fun from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T<br/></code></pre>





<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The address input bytes should be exactly 32 bytes long.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The address_from_bytes function should assert if the length of the input bytes is 32.&lt;/td&gt;<br/>&lt;td&gt;Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;address_from_bytes&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_from_bytes&lt;T&gt;(bytes);<br/></code></pre>




<a id="0x1_util_spec_from_bytes"></a>


<pre><code>fun spec_from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T;<br/></code></pre>



<a id="@Specification_0_address_from_bytes"></a>

### Function `address_from_bytes`


<pre><code>public fun address_from_bytes(bytes: vector&lt;u8&gt;): address<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
aborts_if [abstract] len(bytes) !&#61; 32;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
