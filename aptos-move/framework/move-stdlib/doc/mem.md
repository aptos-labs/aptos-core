
<a id="0x1_mem"></a>

# Module `0x1::mem`

Module with methods for safe memory manipulation.


-  [Function `swap`](#0x1_mem_swap)
-  [Function `replace`](#0x1_mem_replace)
-  [Specification](#@Specification_0)
    -  [Function `swap`](#@Specification_0_swap)
    -  [Function `replace`](#@Specification_0_replace)


<pre><code></code></pre>



<a id="0x1_mem_swap"></a>

## Function `swap`

Swap contents of two passed mutable references.

Move prevents from having two mutable references to the same value,
so <code>left</code> and <code>right</code> references are always distinct.


<pre><code><b>public</b> <b>fun</b> <a href="mem.md#0x1_mem_swap">swap</a>&lt;T&gt;(left: &<b>mut</b> T, right: &<b>mut</b> T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="mem.md#0x1_mem_swap">swap</a>&lt;T&gt;(left: &<b>mut</b> T, right: &<b>mut</b> T);
</code></pre>



</details>

<a id="0x1_mem_replace"></a>

## Function `replace`

Replace the value reference points to with the given new value,
and return the value it had before.


<pre><code><b>public</b> <b>fun</b> <a href="mem.md#0x1_mem_replace">replace</a>&lt;T&gt;(ref: &<b>mut</b> T, new: T): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mem.md#0x1_mem_replace">replace</a>&lt;T&gt;(ref: &<b>mut</b> T, new: T): T {
    <a href="mem.md#0x1_mem_swap">swap</a>(ref, &<b>mut</b> new);
    new
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_swap"></a>

### Function `swap`


<pre><code><b>public</b> <b>fun</b> <a href="mem.md#0x1_mem_swap">swap</a>&lt;T&gt;(left: &<b>mut</b> T, right: &<b>mut</b> T)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> right == <b>old</b>(left);
<b>ensures</b> left == <b>old</b>(right);
</code></pre>



<a id="@Specification_0_replace"></a>

### Function `replace`


<pre><code><b>public</b> <b>fun</b> <a href="mem.md#0x1_mem_replace">replace</a>&lt;T&gt;(ref: &<b>mut</b> T, new: T): T
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <b>old</b>(ref);
<b>ensures</b> ref == new;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
