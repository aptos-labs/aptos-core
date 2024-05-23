
<a id="0x1_create_signer"></a>

# Module `0x1::create_signer`

Provides a common place for exporting <code>create_signer</code> across the Aptos Framework.<br/><br/> To use create_signer, add the module below, such that:<br/> <code>friend aptos_framework::friend_wants_create_signer</code><br/> where <code>friend_wants_create_signer</code> is the module that needs <code>create_signer</code>.<br/><br/> Note, that this is only available within the Aptos Framework.<br/><br/> This exists to make auditing straight forward and to limit the need to depend<br/> on account to have access to this.


-  [Function `create_signer`](#0x1_create_signer_create_signer)
-  [Specification](#@Specification_0)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create_signer`](#@Specification_0_create_signer)


<pre><code></code></pre>



<a id="0x1_create_signer_create_signer"></a>

## Function `create_signer`



<pre><code>public(friend) fun create_signer(addr: address): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) native fun create_signer(addr: address): signer;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;Obtaining a signer for an arbitrary account should only be available within the Aptos Framework.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The create_signer::create_signer function only allows friend modules to retrieve the signer for an arbitrarily address.&lt;/td&gt;<br/>&lt;td&gt;Enforced through function visibility.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The account owner should have the ability to create a signer for their account.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;Before an Account resource is created, a signer is created for the specified new_address, and later, the Account resource is assigned to this signer.&lt;/td&gt;<br/>&lt;td&gt;Enforced by the &lt;a href&#61;&quot;https://github.com/aptos&#45;labs/aptos&#45;core/blob/main/third_party/move/move&#45;vm/types/src/values/values_impl.rs&#35;L1129&quot;&gt;move vm&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;An account should only be able to create a signer for another account if that account has granted it signing capabilities.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The Account resource holds a signer_capability_offer field which allows the owner to share the signer capability with other accounts.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;account.md&#35;high&#45;level&#45;spec&#45;3&quot;&gt;AccountContainsAddr&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;A signer should be returned for addresses that are not registered as accounts.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The signer is just a struct that wraps an address, allows for non&#45;accounts to have a signer.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;0x1_create_signer_create_signer&quot;&gt;create_signer&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_0_create_signer"></a>

### Function `create_signer`


<pre><code>public(friend) fun create_signer(addr: address): signer<br/></code></pre>


Convert address to singer and return.


<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] signer::address_of(result) &#61;&#61; addr;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
