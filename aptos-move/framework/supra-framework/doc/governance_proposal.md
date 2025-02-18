
<a id="0x1_governance_proposal"></a>

# Module `0x1::governance_proposal`

Define the GovernanceProposal that will be used as part of on-chain governance by SupraGovernance.

This is separate from the SupraGovernance module to avoid circular dependency between SupraGovernance and Stake.


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



<pre><code><b>struct</b> <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> <b>has</b> drop, store
</code></pre>



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

Create and return a GovernanceProposal resource. Can only be called by SupraGovernance


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {
    <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {}
}
</code></pre>



</details>

<a id="0x1_governance_proposal_create_empty_proposal"></a>

## Function `create_empty_proposal`

Useful for SupraGovernance to create an empty proposal as proof.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">create_empty_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">create_empty_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {
    <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>()
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_create_proposal"></a>

### Function `create_proposal`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>





<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Creating a proposal should never abort but should always return a governance proposal resource.</td>
<td>Medium</td>
<td>Both create_proposal and create_empty_proposal functions return a GovernanceProposal resource.</td>
<td>Enforced via <a href="#high-level-req-1.1">create_proposal</a> and <a href="#high-level-req-1.2">create_empty_proposal</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The governance proposal module should only be accessible to the supra governance.</td>
<td>Medium</td>
<td>Both create_proposal and create_empty_proposal functions are only available to the friend module supra_framework::supra_governance.</td>
<td>Enforced via friend module relationship.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>aborts_if</b> <b>false</b>;
// This enforces <a id="high-level-req-1.1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> result == <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {};
</code></pre>



<a id="@Specification_0_create_empty_proposal"></a>

### Function `create_empty_proposal`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">create_empty_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
// This enforces <a id="high-level-req-1.2" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> result == <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {};
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
