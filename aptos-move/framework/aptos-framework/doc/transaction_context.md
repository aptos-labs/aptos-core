
<a id="0x1_transaction_context"></a>

# Module `0x1::transaction_context`



-  [Struct `AUID`](#0x1_transaction_context_AUID)
-  [Function `get_txn_hash`](#0x1_transaction_context_get_txn_hash)
-  [Function `get_transaction_hash`](#0x1_transaction_context_get_transaction_hash)
-  [Function `generate_unique_address`](#0x1_transaction_context_generate_unique_address)
-  [Function `generate_auid_address`](#0x1_transaction_context_generate_auid_address)
-  [Function `get_script_hash`](#0x1_transaction_context_get_script_hash)
-  [Function `generate_auid`](#0x1_transaction_context_generate_auid)
-  [Function `auid_address`](#0x1_transaction_context_auid_address)
-  [Specification](#@Specification_0)
    -  [Function `get_txn_hash`](#@Specification_0_get_txn_hash)
    -  [Function `get_transaction_hash`](#@Specification_0_get_transaction_hash)
    -  [Function `generate_unique_address`](#@Specification_0_generate_unique_address)
    -  [Function `generate_auid_address`](#@Specification_0_generate_auid_address)
    -  [Function `get_script_hash`](#@Specification_0_get_script_hash)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `auid_address`](#@Specification_0_auid_address)


<pre><code></code></pre>



<a id="0x1_transaction_context_AUID"></a>

## Struct `AUID`

A wrapper denoting aptos unique identifer (AUID)
for storing an address


<pre><code><b>struct</b> <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>unique_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_context_get_txn_hash"></a>

## Function `get_txn_hash`

Return the transaction hash of the current transaction.


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_transaction_context_get_transaction_hash"></a>

## Function `get_transaction_hash`

Return the transaction hash of the current transaction.
Internally calls the private function <code>get_txn_hash</code>.
This function is created for to feature gate the <code>get_txn_hash</code> function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_generate_unique_address"></a>

## Function `generate_unique_address`

Return a universally unique identifier (of type address) generated
by hashing the transaction hash of this transaction and a sequence number
specific to this transaction. This function can be called any
number of times inside a single transaction. Each such call increments
the sequence number and generates a new unique address.
Uses Scheme in types/src/transaction/authenticator.rs for domain separation
from other ways of generating unique addresses.


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>(): <b>address</b>;
</code></pre>



</details>

<a id="0x1_transaction_context_generate_auid_address"></a>

## Function `generate_auid_address`

Return a aptos unique identifier. Internally calls
the private function <code>generate_unique_address</code>. This function is
created for to feature gate the <code>generate_unique_address</code> function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b> {
    <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>()
}
</code></pre>



</details>

<a id="0x1_transaction_context_get_script_hash"></a>

## Function `get_script_hash`

Return the script hash of the current entry function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="0x1_transaction_context_generate_auid"></a>

## Function `generate_auid`

This method runs <code>generate_unique_address</code> native function and returns
the generated unique address wrapped in the AUID class.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid">generate_auid</a>(): <a href="transaction_context.md#0x1_transaction_context_AUID">transaction_context::AUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid">generate_auid</a>(): <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> {
    <b>return</b> <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> {
        unique_address: <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>()
    }
}
</code></pre>



</details>

<a id="0x1_transaction_context_auid_address"></a>

## Function `auid_address`



<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_auid_address">auid_address</a>(auid: &<a href="transaction_context.md#0x1_transaction_context_AUID">transaction_context::AUID</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_auid_address">auid_address</a>(auid: &<a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a>): <b>address</b> {
    auid.unique_address
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_get_txn_hash"></a>

### Function `get_txn_hash`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> result == <a href="transaction_context.md#0x1_transaction_context_spec_get_txn_hash">spec_get_txn_hash</a>();
</code></pre>




<a id="0x1_transaction_context_spec_get_txn_hash"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_txn_hash">spec_get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a id="@Specification_0_get_transaction_hash"></a>

### Function `get_transaction_hash`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> [abstract] len(result) == 32;
</code></pre>



<a id="@Specification_0_generate_unique_address"></a>

### Function `generate_unique_address`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>(): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>();
</code></pre>




<a id="0x1_transaction_context_spec_generate_unique_address"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>(): <b>address</b>;
</code></pre>



<a id="@Specification_0_generate_auid_address"></a>

### Function `generate_auid_address`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>ensures</b> [abstract] result == <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>();
</code></pre>



<a id="@Specification_0_get_script_hash"></a>

### Function `get_script_hash`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>





<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Fetching the transaction hash should return a vector with 32 bytes.</td>
<td>Medium</td>
<td>The get_transaction_hash function calls the native function get_txn_hash, which fetches the NativeTransactionContext struct and returns the txn_hash field.</td>
<td>Audited that the native function returns the txn hash, whose size is 32 bytes. This has been modeled as the abstract postcondition that the returned vector is of length 32. Formally verified via <a href="#high-level-req-1">get_txn_hash</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Fetching the unique address should never abort.</td>
<td>Low</td>
<td>The function auid_address returns the unique address from a supplied AUID resource.</td>
<td>Formally verified via <a href="#high-level-req-2">auid_address</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Generating the unique address should return a vector with 32 bytes.</td>
<td>Medium</td>
<td>The generate_auid_address function checks calls the native function generate_unique_address which fetches the NativeTransactionContext struct, increments the auid_counter by one, and then creates a new authentication key from a preimage, which is then returned.</td>
<td>Audited that the native function returns an address, and the length of an address is 32 bytes. This has been modeled as the abstract postcondition that the returned vector is of length 32. Formally verified via <a href="#high-level-req-3">generate_auid_address</a>.</td>
</tr>

<tr>
<td>4</td>
<td>Fetching the script hash of the current entry function should never fail and should return a vector with 32 bytes if the transaction payload is a script, otherwise an empty vector.</td>
<td>Low</td>
<td>The native function get_script_hash returns the NativeTransactionContext.script_hash field.</td>
<td>Audited that the native function holds the required property. This has been modeled as the abstract spec. Formally verified via <a href="#high-level-req-4">get_script_hash</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> opaque;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">spec_get_script_hash</a>();
<b>ensures</b> [abstract] len(result) == 32;
</code></pre>




<a id="0x1_transaction_context_spec_get_script_hash"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">spec_get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a id="@Specification_0_auid_address"></a>

### Function `auid_address`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_auid_address">auid_address</a>(auid: &<a href="transaction_context.md#0x1_transaction_context_AUID">transaction_context::AUID</a>): <b>address</b>
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
