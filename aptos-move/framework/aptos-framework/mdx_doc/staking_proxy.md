
<a id="0x1_staking_proxy"></a>

# Module `0x1::staking_proxy`



-  [Function `set_operator`](#0x1_staking_proxy_set_operator)
-  [Function `set_voter`](#0x1_staking_proxy_set_voter)
-  [Function `set_vesting_contract_operator`](#0x1_staking_proxy_set_vesting_contract_operator)
-  [Function `set_staking_contract_operator`](#0x1_staking_proxy_set_staking_contract_operator)
-  [Function `set_stake_pool_operator`](#0x1_staking_proxy_set_stake_pool_operator)
-  [Function `set_vesting_contract_voter`](#0x1_staking_proxy_set_vesting_contract_voter)
-  [Function `set_staking_contract_voter`](#0x1_staking_proxy_set_staking_contract_voter)
-  [Function `set_stake_pool_voter`](#0x1_staking_proxy_set_stake_pool_voter)
-  [Specification](#@Specification_0)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `set_operator`](#@Specification_0_set_operator)
    -  [Function `set_voter`](#@Specification_0_set_voter)
    -  [Function `set_vesting_contract_operator`](#@Specification_0_set_vesting_contract_operator)
    -  [Function `set_staking_contract_operator`](#@Specification_0_set_staking_contract_operator)
    -  [Function `set_stake_pool_operator`](#@Specification_0_set_stake_pool_operator)
    -  [Function `set_vesting_contract_voter`](#@Specification_0_set_vesting_contract_voter)
    -  [Function `set_staking_contract_voter`](#@Specification_0_set_staking_contract_voter)
    -  [Function `set_stake_pool_voter`](#@Specification_0_set_stake_pool_voter)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="staking_contract.md#0x1_staking_contract">0x1::staking_contract</a>;<br /><b>use</b> <a href="vesting.md#0x1_vesting">0x1::vesting</a>;<br /></code></pre>



<a id="0x1_staking_proxy_set_operator"></a>

## Function `set_operator`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_operator">set_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_operator">set_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>) &#123;<br />    <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_operator">set_vesting_contract_operator</a>(owner, old_operator, new_operator);<br />    <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_operator">set_staking_contract_operator</a>(owner, old_operator, new_operator);<br />    <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_operator">set_stake_pool_operator</a>(owner, new_operator);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_proxy_set_voter"></a>

## Function `set_voter`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_voter">set_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_voter">set_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>) &#123;<br />    <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_voter">set_vesting_contract_voter</a>(owner, operator, new_voter);<br />    <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_voter">set_staking_contract_voter</a>(owner, operator, new_voter);<br />    <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_voter">set_stake_pool_voter</a>(owner, new_voter);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_proxy_set_vesting_contract_operator"></a>

## Function `set_vesting_contract_operator`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_operator">set_vesting_contract_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_operator">set_vesting_contract_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>) &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>let</b> vesting_contracts &#61; &amp;<a href="vesting.md#0x1_vesting_vesting_contracts">vesting::vesting_contracts</a>(owner_address);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(vesting_contracts, &#124;vesting_contract&#124; &#123;<br />        <b>let</b> vesting_contract &#61; &#42;vesting_contract;<br />        <b>if</b> (<a href="vesting.md#0x1_vesting_operator">vesting::operator</a>(vesting_contract) &#61;&#61; old_operator) &#123;<br />            <b>let</b> current_commission_percentage &#61; <a href="vesting.md#0x1_vesting_operator_commission_percentage">vesting::operator_commission_percentage</a>(vesting_contract);<br />            <a href="vesting.md#0x1_vesting_update_operator">vesting::update_operator</a>(owner, vesting_contract, new_operator, current_commission_percentage);<br />        &#125;;<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_proxy_set_staking_contract_operator"></a>

## Function `set_staking_contract_operator`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_operator">set_staking_contract_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_operator">set_staking_contract_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>) &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>if</b> (<a href="staking_contract.md#0x1_staking_contract_staking_contract_exists">staking_contract::staking_contract_exists</a>(owner_address, old_operator)) &#123;<br />        <b>let</b> current_commission_percentage &#61; <a href="staking_contract.md#0x1_staking_contract_commission_percentage">staking_contract::commission_percentage</a>(owner_address, old_operator);<br />        <a href="staking_contract.md#0x1_staking_contract_switch_operator">staking_contract::switch_operator</a>(owner, old_operator, new_operator, current_commission_percentage);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_proxy_set_stake_pool_operator"></a>

## Function `set_stake_pool_operator`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_operator">set_stake_pool_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_operator">set_stake_pool_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>) &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>if</b> (<a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(owner_address)) &#123;<br />        <a href="stake.md#0x1_stake_set_operator">stake::set_operator</a>(owner, new_operator);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_proxy_set_vesting_contract_voter"></a>

## Function `set_vesting_contract_voter`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_voter">set_vesting_contract_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_voter">set_vesting_contract_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>) &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>let</b> vesting_contracts &#61; &amp;<a href="vesting.md#0x1_vesting_vesting_contracts">vesting::vesting_contracts</a>(owner_address);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(vesting_contracts, &#124;vesting_contract&#124; &#123;<br />        <b>let</b> vesting_contract &#61; &#42;vesting_contract;<br />        <b>if</b> (<a href="vesting.md#0x1_vesting_operator">vesting::operator</a>(vesting_contract) &#61;&#61; operator) &#123;<br />            <a href="vesting.md#0x1_vesting_update_voter">vesting::update_voter</a>(owner, vesting_contract, new_voter);<br />        &#125;;<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_proxy_set_staking_contract_voter"></a>

## Function `set_staking_contract_voter`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_voter">set_staking_contract_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_voter">set_staking_contract_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>) &#123;<br />    <b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>if</b> (<a href="staking_contract.md#0x1_staking_contract_staking_contract_exists">staking_contract::staking_contract_exists</a>(owner_address, operator)) &#123;<br />        <a href="staking_contract.md#0x1_staking_contract_update_voter">staking_contract::update_voter</a>(owner, operator, new_voter);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_staking_proxy_set_stake_pool_voter"></a>

## Function `set_stake_pool_voter`



<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_voter">set_stake_pool_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_voter">set_stake_pool_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>) &#123;<br />    <b>if</b> (<a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner))) &#123;<br />        <a href="stake.md#0x1_stake_set_delegated_voter">stake::set_delegated_voter</a>(owner, new_voter);<br />    &#125;;<br />&#125;<br /></code></pre>



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
<td>When updating the Vesting operator, it should be updated throughout all depending units.</td>
<td>Medium</td>
<td>The VestingContract contains a StakingInfo object that has an operator field, and this operator is mapped to a StakingContract object that in turn encompasses a StakePool object where the operator matches.</td>
<td>Audited that it ensures the two operator fields hold the new value after the update.</td>
</tr>

<tr>
<td>2</td>
<td>When updating the Vesting voter, it should be updated throughout all depending units.</td>
<td>Medium</td>
<td>The VestingContract contains a StakingInfo object that has an operator field, and this operator is mapped to a StakingContract object that in turn encompasses a StakePool object where the operator matches.</td>
<td>Audited that it ensures the two operator fields hold the new value after the update.</td>
</tr>

<tr>
<td>3</td>
<td>The operator and voter of a Vesting Contract should only be updated by the owner of the contract.</td>
<td>High</td>
<td>The owner&#45;operator&#45;voter model, as defined in the documentation, grants distinct abilities to each role. Therefore, it&apos;s crucial to ensure that only the owner has the authority to modify the operator or voter, to prevent the compromise of the StakePool.</td>
<td>Audited that it ensures the signer owns the AdminStore resource and that the operator or voter intended for the update actually exists.</td>
</tr>

<tr>
<td>4</td>
<td>The operator and voter of a Staking Contract should only be updated by the owner of the contract.</td>
<td>High</td>
<td>The owner&#45;operator&#45;voter model, as defined in the documentation, grants distinct abilities to each role. Therefore, it&apos;s crucial to ensure that only the owner has the authority to modify the operator or voter, to prevent the compromise of the StakePool.</td>
<td>Audited the patterns of updating operators and voters in the staking contract.</td>
</tr>

<tr>
<td>5</td>
<td>Staking Contract&apos;s operators should be unique inside a store.</td>
<td>Medium</td>
<td>Duplicates among operators could result in incorrectly updating the operator or voter associated with the incorrect StakingContract.</td>
<td>Enforced via <a href="https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/staking_contract.move#L87">SimpleMap</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_0_set_operator"></a>

### Function `set_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_operator">set_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>


Aborts if conditions of SetStakePoolOperator are not met


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakePoolOperator">SetStakePoolOperator</a>;<br /><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakingContractOperator">SetStakingContractOperator</a>;<br /></code></pre>



<a id="@Specification_0_set_voter"></a>

### Function `set_voter`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_voter">set_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>


Aborts if conditions of SetStackingContractVoter and SetStackPoolVoterAbortsIf are not met


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakingContractVoter">SetStakingContractVoter</a>;<br /><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakePoolVoterAbortsIf">SetStakePoolVoterAbortsIf</a>;<br /></code></pre>



<a id="@Specification_0_set_vesting_contract_operator"></a>

### Function `set_vesting_contract_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_operator">set_vesting_contract_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_0_set_staking_contract_operator"></a>

### Function `set_staking_contract_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_operator">set_staking_contract_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, old_operator: <b>address</b>, new_operator: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>pragma</b> verify &#61; <b>false</b>;<br /><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakingContractOperator">SetStakingContractOperator</a>;<br /></code></pre>




<a id="0x1_staking_proxy_SetStakingContractOperator"></a>


<pre><code><b>schema</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakingContractOperator">SetStakingContractOperator</a> &#123;<br />owner: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />old_operator: <b>address</b>;<br />new_operator: <b>address</b>;<br /><b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>let</b> store &#61; <b>global</b>&lt;Store&gt;(owner_address);<br /><b>let</b> staking_contract_exists &#61; <b>exists</b>&lt;Store&gt;(owner_address) &amp;&amp; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(store.staking_contracts, old_operator);<br /><b>aborts_if</b> staking_contract_exists &amp;&amp; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(store.staking_contracts, new_operator);<br /><b>let</b> <b>post</b> post_store &#61; <b>global</b>&lt;Store&gt;(owner_address);<br /><b>ensures</b> staking_contract_exists &#61;&#61;&gt; !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_store.staking_contracts, old_operator);<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, old_operator);<br /><b>let</b> stake_pool &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address);<br /><b>let</b> active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.active);<br /><b>let</b> pending_active &#61; <a href="coin.md#0x1_coin_value">coin::value</a>(stake_pool.pending_active);<br /><b>let</b> total_active_stake &#61; active &#43; pending_active;<br /><b>let</b> accumulated_rewards &#61; total_active_stake &#45; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.principal;<br /><b>let</b> commission_amount &#61; accumulated_rewards &#42; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage / 100;<br /><b>aborts_if</b> staking_contract_exists &amp;&amp; !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address);<br /><b>ensures</b> staking_contract_exists &#61;&#61;&gt;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_store.staking_contracts, new_operator).principal &#61;&#61; total_active_stake &#45; commission_amount;<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;<br /><b>let</b> current_commission_percentage &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.commission_percentage;<br /><b>aborts_if</b> staking_contract_exists &amp;&amp; commission_amount !&#61; 0 &amp;&amp; !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>ensures</b> staking_contract_exists &amp;&amp; commission_amount !&#61; 0 &#61;&#61;&gt;<br />    <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address).operator_address &#61;&#61; new_operator<br />    &amp;&amp; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_store.staking_contracts, new_operator).commission_percentage &#61;&#61; current_commission_percentage;<br /><b>ensures</b> staking_contract_exists &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_store.staking_contracts, new_operator);<br />&#125;<br /></code></pre>



<a id="@Specification_0_set_stake_pool_operator"></a>

### Function `set_stake_pool_operator`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_operator">set_stake_pool_operator</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_operator: <b>address</b>)<br /></code></pre>


Aborts if stake_pool is exists and when OwnerCapability or stake_pool_exists
One of them are not exists


<pre><code><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakePoolOperator">SetStakePoolOperator</a>;<br /></code></pre>




<a id="0x1_staking_proxy_SetStakePoolOperator"></a>


<pre><code><b>schema</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakePoolOperator">SetStakePoolOperator</a> &#123;<br />owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />new_operator: <b>address</b>;<br /><b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>let</b> ownership_cap &#61; <b>borrow_global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>&gt;(owner_address);<br /><b>let</b> pool_address &#61; ownership_cap.pool_address;<br /><b>aborts_if</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(owner_address) &amp;&amp; !(<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>&gt;(owner_address) &amp;&amp; <a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(pool_address));<br /><b>ensures</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(owner_address) &#61;&#61;&gt; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address).operator_address &#61;&#61; new_operator;<br />&#125;<br /></code></pre>



<a id="@Specification_0_set_vesting_contract_voter"></a>

### Function `set_vesting_contract_voter`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_vesting_contract_voter">set_vesting_contract_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_0_set_staking_contract_voter"></a>

### Function `set_staking_contract_voter`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_staking_contract_voter">set_staking_contract_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, operator: <b>address</b>, new_voter: <b>address</b>)<br /></code></pre>




<pre><code><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakingContractVoter">SetStakingContractVoter</a>;<br /></code></pre>


Make sure staking_contract_exists first
Then abort if the resource is not exist


<a id="0x1_staking_proxy_SetStakingContractVoter"></a>


<pre><code><b>schema</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakingContractVoter">SetStakingContractVoter</a> &#123;<br />owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />operator: <b>address</b>;<br />new_voter: <b>address</b>;<br /><b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>let</b> staker &#61; owner_address;<br /><b>let</b> store &#61; <b>global</b>&lt;Store&gt;(staker);<br /><b>let</b> staking_contract_exists &#61; <b>exists</b>&lt;Store&gt;(staker) &amp;&amp; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(store.staking_contracts, operator);<br /><b>let</b> staker_address &#61; owner_address;<br /><b>let</b> <a href="staking_contract.md#0x1_staking_contract">staking_contract</a> &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(store.staking_contracts, operator);<br /><b>let</b> pool_address &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.pool_address;<br /><b>let</b> pool_address1 &#61; <a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address;<br /><b>aborts_if</b> staking_contract_exists &amp;&amp; !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> staking_contract_exists &amp;&amp; !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(<a href="staking_contract.md#0x1_staking_contract">staking_contract</a>.owner_cap.pool_address);<br /><b>ensures</b> staking_contract_exists &#61;&#61;&gt; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address1).delegated_voter &#61;&#61; new_voter;<br />&#125;<br /></code></pre>



<a id="@Specification_0_set_stake_pool_voter"></a>

### Function `set_stake_pool_voter`


<pre><code><b>public</b> entry <b>fun</b> <a href="staking_proxy.md#0x1_staking_proxy_set_stake_pool_voter">set_stake_pool_voter</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_voter: <b>address</b>)<br /></code></pre>




<pre><code><b>include</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakePoolVoterAbortsIf">SetStakePoolVoterAbortsIf</a>;<br /></code></pre>




<a id="0x1_staking_proxy_SetStakePoolVoterAbortsIf"></a>


<pre><code><b>schema</b> <a href="staking_proxy.md#0x1_staking_proxy_SetStakePoolVoterAbortsIf">SetStakePoolVoterAbortsIf</a> &#123;<br />owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />new_voter: <b>address</b>;<br /><b>let</b> owner_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>let</b> ownership_cap &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>&gt;(owner_address);<br /><b>let</b> pool_address &#61; ownership_cap.pool_address;<br /><b>aborts_if</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(owner_address) &amp;&amp; !(<b>exists</b>&lt;<a href="stake.md#0x1_stake_OwnerCapability">stake::OwnerCapability</a>&gt;(owner_address) &amp;&amp; <a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(pool_address));<br /><b>ensures</b> <a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(owner_address) &#61;&#61;&gt; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address).delegated_voter &#61;&#61; new_voter;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
