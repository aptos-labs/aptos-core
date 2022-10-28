
<a name="0x1_signer"></a>

# Module `0x1::signer`



-  [Function `borrow_address`](#0x1_signer_borrow_address)
-  [Function `address_of`](#0x1_signer_address_of)
-  [Specification](#@Specification_0)


<pre><code></code></pre>



<details>
<summary>Show all the modules that "signer" depends on directly or indirectly</summary>


![](img/signer_forward_dep.svg)


</details>

<details>
<summary>Show all the modules that depend on "signer" directly or indirectly</summary>


![](img/signer_backward_dep.svg)


</details>

<a name="0x1_signer_borrow_address"></a>

## Function `borrow_address`



<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>;
</code></pre>



</details>

<details>
<summary>Show all the functions that "borrow_address" calls</summary>


![](img/signer_borrow_address_forward_call_graph.svg)


</details>

<details>
<summary>Show all the functions that call "borrow_address"</summary>


![](img/signer_borrow_address_backward_call_graph.svg)


</details>

<a name="0x1_signer_address_of"></a>

## Function `address_of`



<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_of">address_of</a>(s: &<a href="signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_of">address_of</a>(s: &<a href="signer.md#0x1_signer">signer</a>): <b>address</b> {
    *<a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s)
}
</code></pre>



</details>

<details>
<summary>Show all the functions that "address_of" calls</summary>


![](img/signer_address_of_forward_call_graph.svg)


</details>

<details>
<summary>Show all the functions that call "address_of"</summary>


![](img/signer_address_of_backward_call_graph.svg)


</details>

<a name="@Specification_0"></a>

## Specification

Return true only if <code>s</code> is a transaction signer. This is a spec function only available in spec.


<a name="0x1_signer_is_txn_signer"></a>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_is_txn_signer">is_txn_signer</a>(s: <a href="signer.md#0x1_signer">signer</a>): bool;
</code></pre>


Return true only if <code>a</code> is a transaction signer address. This is a spec function only available in spec.


<a name="0x1_signer_is_txn_signer_addr"></a>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_is_txn_signer_addr">is_txn_signer_addr</a>(a: <b>address</b>): bool;
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
