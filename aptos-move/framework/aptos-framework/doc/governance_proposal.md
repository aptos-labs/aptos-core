
<a id="0x1_governance_proposal"></a>

# Module `0x1::governance_proposal`

Define the GovernanceProposal that will be used as part of on&#45;chain governance by AptosGovernance.<br/><br/> This is separate from the AptosGovernance module to avoid circular dependency between AptosGovernance and Stake.


-  [Struct `GovernanceProposal`](#0x1_governance_proposal_GovernanceProposal)
-  [Function `create_proposal`](#0x1_governance_proposal_create_proposal)
-  [Function `create_empty_proposal`](#0x1_governance_proposal_create_empty_proposal)
-  [Specification](#@Specification_0)
    -  [Function `create_proposal`](#@Specification_0_create_proposal)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `create_empty_proposal`](#@Specification_0_create_empty_proposal)


<pre><code></code></pre>



<a id="0x1_governance_proposal_GovernanceProposal"></a>

## Struct `GovernanceProposal`



<pre><code>struct GovernanceProposal has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_governance_proposal_create_proposal"></a>

## Function `create_proposal`

Create and return a GovernanceProposal resource. Can only be called by AptosGovernance


<pre><code>public(friend) fun create_proposal(): governance_proposal::GovernanceProposal<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_proposal(): GovernanceProposal &#123;<br/>    GovernanceProposal &#123;&#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_governance_proposal_create_empty_proposal"></a>

## Function `create_empty_proposal`

Useful for AptosGovernance to create an empty proposal as proof.


<pre><code>public(friend) fun create_empty_proposal(): governance_proposal::GovernanceProposal<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_empty_proposal(): GovernanceProposal &#123;<br/>    create_proposal()<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_create_proposal"></a>

### Function `create_proposal`


<pre><code>public(friend) fun create_proposal(): governance_proposal::GovernanceProposal<br/></code></pre>





<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;Creating a proposal should never abort but should always return a governance proposal resource.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;Both create_proposal and create_empty_proposal functions return a GovernanceProposal resource.&lt;/td&gt;<br/>&lt;td&gt;Enforced via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.1&quot;&gt;create_proposal&lt;/a&gt; and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1.2&quot;&gt;create_empty_proposal&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The governance proposal module should only be accessible to the aptos governance.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;Both create_proposal and create_empty_proposal functions are only available to the friend module aptos_framework::aptos_governance.&lt;/td&gt;<br/>&lt;td&gt;Enforced via friend module relationship.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>aborts_if false;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures result &#61;&#61; GovernanceProposal &#123;&#125;;<br/></code></pre>



<a id="@Specification_0_create_empty_proposal"></a>

### Function `create_empty_proposal`


<pre><code>public(friend) fun create_empty_proposal(): governance_proposal::GovernanceProposal<br/></code></pre>




<pre><code>aborts_if false;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures result &#61;&#61; GovernanceProposal &#123;&#125;;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
