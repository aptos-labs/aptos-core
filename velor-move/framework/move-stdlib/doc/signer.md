
<a id="0x1_signer"></a>

# Module `0x1::signer`



-  [Function `borrow_address`](#0x1_signer_borrow_address)
-  [Function `address_of`](#0x1_signer_address_of)
-  [Specification](#@Specification_0)


<pre><code></code></pre>



<a id="0x1_signer_borrow_address"></a>

## Function `borrow_address`

signer is a builtin move type that represents an address that has been verfied by the VM.

VM Runtime representation is equivalent to following:
```
enum signer has drop {
Master { account: address },
Permissioned { account: address, permissions_address: address },
}
```

for bcs serialization:

```
struct signer has drop {
account: address,
}
```
^ The discrepency is needed to maintain backwards compatibility of signer serialization
semantics.

<code>borrow_address</code> borrows this inner field


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>;
</code></pre>



</details>

<a id="0x1_signer_address_of"></a>

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

<a id="@Specification_0"></a>

## Specification

Return true only if <code>s</code> is a transaction signer. This is a spec function only available in spec.


<a id="0x1_signer_is_txn_signer"></a>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_is_txn_signer">is_txn_signer</a>(s: <a href="signer.md#0x1_signer">signer</a>): bool;
</code></pre>


Return true only if <code>a</code> is a transaction signer address. This is a spec function only available in spec.


<a id="0x1_signer_is_txn_signer_addr"></a>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_is_txn_signer_addr">is_txn_signer_addr</a>(a: <b>address</b>): bool;
</code></pre>


[move-book]: https://velor.dev/move/book/SUMMARY
