
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

Native function to deserialize a type T.

Note that this function does not put any constraint on <code>T</code>. If code uses this function to
deserialized a linear value, its their responsibility that the data they deserialize is
owned.


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

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The address input bytes should be exactly 32 bytes long.</td>
<td>Low</td>
<td>The address_from_bytes function should assert if the length of the input bytes is 32.</td>
<td>Verified via <a href="#high-level-req-1">address_from_bytes</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_from_bytes&lt;T&gt;(bytes);<br/></code></pre>




<a id="0x1_util_spec_from_bytes"></a>


<pre><code>fun spec_from_bytes&lt;T&gt;(bytes: vector&lt;u8&gt;): T;<br/></code></pre>



<a id="@Specification_0_address_from_bytes"></a>

### Function `address_from_bytes`


<pre><code>public fun address_from_bytes(bytes: vector&lt;u8&gt;): address<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
aborts_if [abstract] len(bytes) !&#61; 32;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
