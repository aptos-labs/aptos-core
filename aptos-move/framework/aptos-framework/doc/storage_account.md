
<a name="0x1_storage_account"></a>

# Module `0x1::storage_account`

A storage account is a lightweight way to allocate global storage:
- Cheaper than using a table since no handle generation/hashing.
- Cheaper than a resource account, which allocates an Account too.
- Cheaper than creating an object & immediately deleting ObjectCore.


-  [Struct `SignerCapability`](#0x1_storage_account_SignerCapability)
-  [Function `create_storage_account`](#0x1_storage_account_create_storage_account)
-  [Function `create_storage_account_and_capability`](#0x1_storage_account_create_storage_account_and_capability)
-  [Function `get_signer_capability_address`](#0x1_storage_account_get_signer_capability_address)
-  [Function `create_signer_with_capability`](#0x1_storage_account_create_signer_with_capability)


<pre><code><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
</code></pre>



<a name="0x1_storage_account_SignerCapability"></a>

## Struct `SignerCapability`



<pre><code><b>struct</b> <a href="storage_account.md#0x1_storage_account_SignerCapability">SignerCapability</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_storage_account_create_storage_account"></a>

## Function `create_storage_account`



<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_create_storage_account">create_storage_account</a>(): (<b>address</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_create_storage_account">create_storage_account</a>(): (<b>address</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>let</b> storage_addr = <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">transaction_context::generate_auid_address</a>();
    (storage_addr, <a href="create_signer.md#0x1_create_signer">create_signer</a>(storage_addr))
}
</code></pre>



</details>

<a name="0x1_storage_account_create_storage_account_and_capability"></a>

## Function `create_storage_account_and_capability`



<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_create_storage_account_and_capability">create_storage_account_and_capability</a>(): (<b>address</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="storage_account.md#0x1_storage_account_SignerCapability">storage_account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_create_storage_account_and_capability">create_storage_account_and_capability</a>(): (
    <b>address</b>,
    <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="storage_account.md#0x1_storage_account_SignerCapability">SignerCapability</a>,
) {
    <b>let</b> addr = <a href="transaction_context.md#0x1_transaction_context_generate_auid_address">transaction_context::generate_auid_address</a>();
    (addr, <a href="create_signer.md#0x1_create_signer">create_signer</a>(addr), <a href="storage_account.md#0x1_storage_account_SignerCapability">SignerCapability</a> { <a href="account.md#0x1_account">account</a>: addr })
}
</code></pre>



</details>

<a name="0x1_storage_account_get_signer_capability_address"></a>

## Function `get_signer_capability_address`



<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_get_signer_capability_address">get_signer_capability_address</a>(cap: &<a href="storage_account.md#0x1_storage_account_SignerCapability">storage_account::SignerCapability</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_get_signer_capability_address">get_signer_capability_address</a>(cap: &<a href="storage_account.md#0x1_storage_account_SignerCapability">SignerCapability</a>): <b>address</b> {
    cap.<a href="account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a name="0x1_storage_account_create_signer_with_capability"></a>

## Function `create_signer_with_capability`



<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_create_signer_with_capability">create_signer_with_capability</a>(cap: &<a href="storage_account.md#0x1_storage_account_SignerCapability">storage_account::SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_account.md#0x1_storage_account_create_signer_with_capability">create_signer_with_capability</a>(cap: &<a href="storage_account.md#0x1_storage_account_SignerCapability">SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="create_signer.md#0x1_create_signer">create_signer</a>(cap.<a href="account.md#0x1_account">account</a>)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
