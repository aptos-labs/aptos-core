
<a name="0x1_governance_proposal"></a>

# Module `0x1::governance_proposal`

Define the GovernanceProposal that will be used as part of on-chain governance by AptosGovernance.

This is separate from the AptosGovernance module to avoid circular dependency between AptosGovernance and Stake.


-  [Struct `GovernanceProposal`](#0x1_governance_proposal_GovernanceProposal)
-  [Function `create_proposal`](#0x1_governance_proposal_create_proposal)
-  [Function `create_empty_proposal`](#0x1_governance_proposal_create_empty_proposal)
-  [Specification](#@Specification_0)
    -  [Function `create_proposal`](#@Specification_0_create_proposal)
    -  [Function `create_empty_proposal`](#@Specification_0_create_empty_proposal)


<pre><code></code></pre>



<a name="0x1_governance_proposal_GovernanceProposal"></a>

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

<a name="0x1_governance_proposal_create_proposal"></a>

## Function `create_proposal`

Create and return a GovernanceProposal resource. Can only be called by AptosGovernance


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {
    <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {}
}
</code></pre>



</details>

<a name="0x1_governance_proposal_create_empty_proposal"></a>

## Function `create_empty_proposal`

Useful for AptosGovernance to create an empty proposal as proof.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">create_empty_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">create_empty_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {
    <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>()
}
</code></pre>



</details>

<a name="@Specification_0"></a>

## Specification


<a name="@Specification_0_create_proposal"></a>

### Function `create_proposal`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">create_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {};
</code></pre>



<a name="@Specification_0_create_empty_proposal"></a>

### Function `create_empty_proposal`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">create_empty_proposal</a>(): <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">GovernanceProposal</a> {};
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
