
<a id="0x1_signer"></a>

# Module `0x1::signer`



-  [Constants](#@Constants_0)
-  [Function `borrow_address`](#0x1_signer_borrow_address)
-  [Function `borrow_address_unpermissioned`](#0x1_signer_borrow_address_unpermissioned)
-  [Function `address_of`](#0x1_signer_address_of)
-  [Function `address_of_unpermissioned`](#0x1_signer_address_of_unpermissioned)
-  [Function `is_permissioned_signer`](#0x1_signer_is_permissioned_signer)
-  [Specification](#@Specification_1)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_signer_ENOT_MASTER_SIGNER"></a>

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
Access address of a permissioned signer;


<pre><code><b>const</b> <a href="signer.md#0x1_signer_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>: u64 = 1;
</code></pre>



<a id="0x1_signer_borrow_address"></a>

## Function `borrow_address`

<code>borrow_address</code> borrows this inner field, abort if <code>s</code> is a permissioned signer.


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address">borrow_address</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>;
</code></pre>



</details>

<a id="0x1_signer_borrow_address_unpermissioned"></a>

## Function `borrow_address_unpermissioned`

<code>borrow_address_unpermissioned</code> borrows this inner field, without checking if <code>s</code> is a permissioned signer.


<pre><code><b>fun</b> <a href="signer.md#0x1_signer_borrow_address_unpermissioned">borrow_address_unpermissioned</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_borrow_address_unpermissioned">borrow_address_unpermissioned</a>(s: &<a href="signer.md#0x1_signer">signer</a>): &<b>address</b>;
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

<a id="0x1_signer_address_of_unpermissioned"></a>

## Function `address_of_unpermissioned`



<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_of_unpermissioned">address_of_unpermissioned</a>(s: &<a href="signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signer.md#0x1_signer_address_of_unpermissioned">address_of_unpermissioned</a>(s: &<a href="signer.md#0x1_signer">signer</a>): <b>address</b> {
    *<a href="signer.md#0x1_signer_borrow_address_unpermissioned">borrow_address_unpermissioned</a>(s)
}
</code></pre>



</details>

<a id="0x1_signer_is_permissioned_signer"></a>

## Function `is_permissioned_signer`



<pre><code><b>fun</b> <a href="signer.md#0x1_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="signer.md#0x1_signer">signer</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_is_permissioned_signer">is_permissioned_signer</a>(s: &<a href="signer.md#0x1_signer">signer</a>): bool;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification

Return true only if <code>s</code> is a transaction signer. This is a spec function only available in spec.


<a id="0x1_signer_is_txn_signer"></a>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_is_txn_signer">is_txn_signer</a>(s: <a href="signer.md#0x1_signer">signer</a>): bool;
</code></pre>


Return true only if <code>a</code> is a transaction signer address. This is a spec function only available in spec.


<a id="0x1_signer_is_txn_signer_addr"></a>


<pre><code><b>native</b> <b>fun</b> <a href="signer.md#0x1_signer_is_txn_signer_addr">is_txn_signer_addr</a>(a: <b>address</b>): bool;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
