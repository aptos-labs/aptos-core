
<a name="0x1_transaction_context"></a>

# Module `0x1::transaction_context`



-  [Struct `AUID`](#0x1_transaction_context_AUID)
-  [Constants](#@Constants_0)
-  [Function `get_txn_hash`](#0x1_transaction_context_get_txn_hash)
-  [Function `get_transaction_hash`](#0x1_transaction_context_get_transaction_hash)
-  [Function `generate_unique_address`](#0x1_transaction_context_generate_unique_address)
-  [Function `generate_auid_address`](#0x1_transaction_context_generate_auid_address)
-  [Function `get_script_hash`](#0x1_transaction_context_get_script_hash)
-  [Function `generate_auid`](#0x1_transaction_context_generate_auid)
-  [Function `auid_address`](#0x1_transaction_context_auid_address)
-  [Specification](#@Specification_1)
    -  [Function `get_txn_hash`](#@Specification_1_get_txn_hash)
    -  [Function `generate_unique_address`](#@Specification_1_generate_unique_address)
    -  [Function `get_script_hash`](#@Specification_1_get_script_hash)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
</code></pre>



<a name="0x1_transaction_context_AUID"></a>

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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_transaction_context_EAUID_NOT_SUPPORTED"></a>

AUID feature is not supported.


<pre><code><b>const</b> <a href="transaction_context.md#0x1_transaction_context_EAUID_NOT_SUPPORTED">EAUID_NOT_SUPPORTED</a>: u64 = 1;
</code></pre>



<a name="0x1_transaction_context_get_txn_hash"></a>

## Function `get_txn_hash`

Return the transaction hash of the current transaction.


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_transaction_context_get_transaction_hash"></a>

## Function `get_transaction_hash`

Return the transaction hash of the current transaction.
Internally calls the private function <code>get_txn_hash</code>.
This function is created for to feature gate the <code>get_txn_hash</code> function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">get_transaction_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_auids_enabled">features::auids_enabled</a>(), <a href="transaction_context.md#0x1_transaction_context_EAUID_NOT_SUPPORTED">EAUID_NOT_SUPPORTED</a>);
    <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>()
}
</code></pre>



</details>

<a name="0x1_transaction_context_generate_unique_address"></a>

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

<a name="0x1_transaction_context_generate_auid_address"></a>

## Function `generate_auid_address`

Return a aptos unique identifier. Internally calls
the private function <code>generate_unique_address</code>. This function is
created for to feature gate the <code>generate_unique_address</code> function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">generate_auid_address</a>(): <b>address</b> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_auids_enabled">features::auids_enabled</a>(), <a href="transaction_context.md#0x1_transaction_context_EAUID_NOT_SUPPORTED">EAUID_NOT_SUPPORTED</a>);
    <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>()
}
</code></pre>



</details>

<a name="0x1_transaction_context_get_script_hash"></a>

## Function `get_script_hash`

Return the script hash of the current entry function.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>native</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_transaction_context_generate_auid"></a>

## Function `generate_auid`

This method runs <code>generate_unique_address</code> native function and returns
the generated unique address wrapped in the AUID class.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid">generate_auid</a>(): <a href="transaction_context.md#0x1_transaction_context_AUID">transaction_context::AUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_auid">generate_auid</a>(): <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_auids_enabled">features::auids_enabled</a>(), <a href="transaction_context.md#0x1_transaction_context_EAUID_NOT_SUPPORTED">EAUID_NOT_SUPPORTED</a>);
    <b>return</b> <a href="transaction_context.md#0x1_transaction_context_AUID">AUID</a> {
        unique_address: <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>()
    }
}
</code></pre>



</details>

<a name="0x1_transaction_context_auid_address"></a>

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

<a name="@Specification_1"></a>

## Specification



<a name="0x1_transaction_context_spec_get_txn_hash"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_txn_hash">spec_get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a name="@Specification_1_get_txn_hash"></a>

### Function `get_txn_hash`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_txn_hash">get_txn_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="transaction_context.md#0x1_transaction_context_spec_get_txn_hash">spec_get_txn_hash</a>();
</code></pre>



<a name="@Specification_1_generate_unique_address"></a>

### Function `generate_unique_address`


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_generate_unique_address">generate_unique_address</a>(): <b>address</b>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>();
</code></pre>




<a name="0x1_transaction_context_spec_generate_unique_address"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_generate_unique_address">spec_generate_unique_address</a>(): <b>address</b>;
</code></pre>



<a name="@Specification_1_get_script_hash"></a>

### Function `get_script_hash`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_context.md#0x1_transaction_context_get_script_hash">get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">spec_get_script_hash</a>();
</code></pre>




<a name="0x1_transaction_context_spec_get_script_hash"></a>


<pre><code><b>fun</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">spec_get_script_hash</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
