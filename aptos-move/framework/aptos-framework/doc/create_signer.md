
<a name="0x1_create_signer"></a>

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
    -  [Function `create_signer`](#@Specification_0_create_signer)


<pre><code></code></pre>



<a name="0x1_create_signer_create_signer"></a>

## Function `create_signer`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="create_signer.md#0x1_create_signer">create_signer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>native</b> <b>fun</b> <a href="create_signer.md#0x1_create_signer">create_signer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



</details>

<a name="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a name="@Specification_0_create_signer"></a>

### Function `create_signer`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="create_signer.md#0x1_create_signer">create_signer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>


Convert address to singer and return.


<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) == addr;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
