
<a id="0x1_signer"></a>

# Module `0x1::signer`



-  [Function `borrow_address`](#0x1_signer_borrow_address)
-  [Function `address_of`](#0x1_signer_address_of)
-  [Specification](#@Specification_0)


<pre><code></code></pre>



<a id="0x1_signer_borrow_address"></a>

## Function `borrow_address`

Borrows the address of the signer
Conceptually, you can think of the <code>signer</code> as being a struct wrapper around an
address
```
struct signer has drop { addr: address }
```
<code>borrow_address</code> borrows this inner field


<pre><code>public fun borrow_address(s: &amp;signer): &amp;address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native public fun borrow_address(s: &amp;signer): &amp;address;
</code></pre>



</details>

<a id="0x1_signer_address_of"></a>

## Function `address_of`



<pre><code>public fun address_of(s: &amp;signer): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun address_of(s: &amp;signer): address &#123;
    &#42;borrow_address(s)
&#125;
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification

Return true only if <code>s</code> is a transaction signer. This is a spec function only available in spec.


<a id="0x1_signer_is_txn_signer"></a>


<pre><code>native fun is_txn_signer(s: signer): bool;
</code></pre>


Return true only if <code>a</code> is a transaction signer address. This is a spec function only available in spec.


<a id="0x1_signer_is_txn_signer_addr"></a>


<pre><code>native fun is_txn_signer_addr(a: address): bool;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
