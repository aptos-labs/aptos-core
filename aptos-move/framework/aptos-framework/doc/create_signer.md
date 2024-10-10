
<a id="0x1_create_signer"></a>

# Module `0x1::create_signer`

Provides a common place for exporting <code><a href="create_signer.md#0x1_create_signer">create_signer</a></code> across the Aptos Framework.

To use create_signer, add the module below, such that:
<code><b>friend</b> aptos_framework::friend_wants_create_signer</code>
where <code>friend_wants_create_signer</code> is the module that needs <code><a href="create_signer.md#0x1_create_signer">create_signer</a></code>.

Note, that this is only available within the Aptos Framework.

This exists to make auditing straight forward and to limit the need to depend
on account to have access to this.


-  [Function `create_signer`](#0x1_create_signer_create_signer)
-  [Specification](#@Specification_0)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create_signer`](#@Specification_0_create_signer)


<pre><code></code></pre>



<a id="0x1_create_signer_create_signer"></a>

## Function `create_signer`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="create_signer.md#0x1_create_signer">create_signer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>native</b> <b>fun</b> <a href="create_signer.md#0x1_create_signer">create_signer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Obtaining a signer for an arbitrary account should only be available within the Aptos Framework.</td>
<td>Critical</td>
<td>The create_signer::create_signer function only allows friend modules to retrieve the signer for an arbitrarily address.</td>
<td>Enforced through function visibility.</td>
</tr>

<tr>
<td>2</td>
<td>The account owner should have the ability to create a signer for their account.</td>
<td>Medium</td>
<td>Before an Account resource is created, a signer is created for the specified new_address, and later, the Account resource is assigned to this signer.</td>
<td>Enforced by the <a href="https://github.com/aptos-labs/aptos-core/blob/main/third_party/move/move-vm/types/src/values/values_impl.rs#L1129">move vm</a>.</td>
</tr>

<tr>
<td>3</td>
<td>An account should only be able to create a signer for another account if that account has granted it signing capabilities.</td>
<td>Critical</td>
<td>The Account resource holds a signer_capability_offer field which allows the owner to share the signer capability with other accounts.</td>
<td>Formally verified via <a href="account.md#high-level-spec-3">AccountContainsAddr</a>.</td>
</tr>

<tr>
<td>4</td>
<td>A signer should be returned for addresses that are not registered as accounts.</td>
<td>Low</td>
<td>The signer is just a struct that wraps an address, allows for non-accounts to have a signer.</td>
<td>Formally verified via <a href="#0x1_create_signer_create_signer">create_signer</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_0_create_signer"></a>

### Function `create_signer`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="create_signer.md#0x1_create_signer">create_signer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>


Convert address to singer and return.


<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) == addr;
<b>ensures</b> [abstract] result == <a href="create_signer.md#0x1_create_signer_spec_create_signer">spec_create_signer</a>(addr);
</code></pre>




<a id="0x1_create_signer_spec_create_signer"></a>


<pre><code><b>fun</b> <a href="create_signer.md#0x1_create_signer_spec_create_signer">spec_create_signer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
