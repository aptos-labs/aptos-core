
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


<pre><code>use 0x1::signer;
use 0x1::stake;
use 0x1::staking_contract;
use 0x1::vesting;
</code></pre>



<a id="0x1_staking_proxy_set_operator"></a>

## Function `set_operator`



<pre><code>public entry fun set_operator(owner: &amp;signer, old_operator: address, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_operator(owner: &amp;signer, old_operator: address, new_operator: address) &#123;
    set_vesting_contract_operator(owner, old_operator, new_operator);
    set_staking_contract_operator(owner, old_operator, new_operator);
    set_stake_pool_operator(owner, new_operator);
&#125;
</code></pre>



</details>

<a id="0x1_staking_proxy_set_voter"></a>

## Function `set_voter`



<pre><code>public entry fun set_voter(owner: &amp;signer, operator: address, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_voter(owner: &amp;signer, operator: address, new_voter: address) &#123;
    set_vesting_contract_voter(owner, operator, new_voter);
    set_staking_contract_voter(owner, operator, new_voter);
    set_stake_pool_voter(owner, new_voter);
&#125;
</code></pre>



</details>

<a id="0x1_staking_proxy_set_vesting_contract_operator"></a>

## Function `set_vesting_contract_operator`



<pre><code>public entry fun set_vesting_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_vesting_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address) &#123;
    let owner_address &#61; signer::address_of(owner);
    let vesting_contracts &#61; &amp;vesting::vesting_contracts(owner_address);
    vector::for_each_ref(vesting_contracts, &#124;vesting_contract&#124; &#123;
        let vesting_contract &#61; &#42;vesting_contract;
        if (vesting::operator(vesting_contract) &#61;&#61; old_operator) &#123;
            let current_commission_percentage &#61; vesting::operator_commission_percentage(vesting_contract);
            vesting::update_operator(owner, vesting_contract, new_operator, current_commission_percentage);
        &#125;;
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_staking_proxy_set_staking_contract_operator"></a>

## Function `set_staking_contract_operator`



<pre><code>public entry fun set_staking_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_staking_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address) &#123;
    let owner_address &#61; signer::address_of(owner);
    if (staking_contract::staking_contract_exists(owner_address, old_operator)) &#123;
        let current_commission_percentage &#61; staking_contract::commission_percentage(owner_address, old_operator);
        staking_contract::switch_operator(owner, old_operator, new_operator, current_commission_percentage);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_staking_proxy_set_stake_pool_operator"></a>

## Function `set_stake_pool_operator`



<pre><code>public entry fun set_stake_pool_operator(owner: &amp;signer, new_operator: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_stake_pool_operator(owner: &amp;signer, new_operator: address) &#123;
    let owner_address &#61; signer::address_of(owner);
    if (stake::stake_pool_exists(owner_address)) &#123;
        stake::set_operator(owner, new_operator);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_staking_proxy_set_vesting_contract_voter"></a>

## Function `set_vesting_contract_voter`



<pre><code>public entry fun set_vesting_contract_voter(owner: &amp;signer, operator: address, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_vesting_contract_voter(owner: &amp;signer, operator: address, new_voter: address) &#123;
    let owner_address &#61; signer::address_of(owner);
    let vesting_contracts &#61; &amp;vesting::vesting_contracts(owner_address);
    vector::for_each_ref(vesting_contracts, &#124;vesting_contract&#124; &#123;
        let vesting_contract &#61; &#42;vesting_contract;
        if (vesting::operator(vesting_contract) &#61;&#61; operator) &#123;
            vesting::update_voter(owner, vesting_contract, new_voter);
        &#125;;
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_staking_proxy_set_staking_contract_voter"></a>

## Function `set_staking_contract_voter`



<pre><code>public entry fun set_staking_contract_voter(owner: &amp;signer, operator: address, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_staking_contract_voter(owner: &amp;signer, operator: address, new_voter: address) &#123;
    let owner_address &#61; signer::address_of(owner);
    if (staking_contract::staking_contract_exists(owner_address, operator)) &#123;
        staking_contract::update_voter(owner, operator, new_voter);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_staking_proxy_set_stake_pool_voter"></a>

## Function `set_stake_pool_voter`



<pre><code>public entry fun set_stake_pool_voter(owner: &amp;signer, new_voter: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_stake_pool_voter(owner: &amp;signer, new_voter: address) &#123;
    if (stake::stake_pool_exists(signer::address_of(owner))) &#123;
        stake::set_delegated_voter(owner, new_voter);
    &#125;;
&#125;
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
<td>The owner-operator-voter model, as defined in the documentation, grants distinct abilities to each role. Therefore, it's crucial to ensure that only the owner has the authority to modify the operator or voter, to prevent the compromise of the StakePool.</td>
<td>Audited that it ensures the signer owns the AdminStore resource and that the operator or voter intended for the update actually exists.</td>
</tr>

<tr>
<td>4</td>
<td>The operator and voter of a Staking Contract should only be updated by the owner of the contract.</td>
<td>High</td>
<td>The owner-operator-voter model, as defined in the documentation, grants distinct abilities to each role. Therefore, it's crucial to ensure that only the owner has the authority to modify the operator or voter, to prevent the compromise of the StakePool.</td>
<td>Audited the patterns of updating operators and voters in the staking contract.</td>
</tr>

<tr>
<td>5</td>
<td>Staking Contract's operators should be unique inside a store.</td>
<td>Medium</td>
<td>Duplicates among operators could result in incorrectly updating the operator or voter associated with the incorrect StakingContract.</td>
<td>Enforced via <a href="https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/staking_contract.move#L87">SimpleMap</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_0_set_operator"></a>

### Function `set_operator`


<pre><code>public entry fun set_operator(owner: &amp;signer, old_operator: address, new_operator: address)
</code></pre>


Aborts if conditions of SetStakePoolOperator are not met


<pre><code>pragma verify &#61; false;
pragma aborts_if_is_partial;
include SetStakePoolOperator;
include SetStakingContractOperator;
</code></pre>



<a id="@Specification_0_set_voter"></a>

### Function `set_voter`


<pre><code>public entry fun set_voter(owner: &amp;signer, operator: address, new_voter: address)
</code></pre>


Aborts if conditions of SetStackingContractVoter and SetStackPoolVoterAbortsIf are not met


<pre><code>pragma aborts_if_is_partial;
include SetStakingContractVoter;
include SetStakePoolVoterAbortsIf;
</code></pre>



<a id="@Specification_0_set_vesting_contract_operator"></a>

### Function `set_vesting_contract_operator`


<pre><code>public entry fun set_vesting_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_0_set_staking_contract_operator"></a>

### Function `set_staking_contract_operator`


<pre><code>public entry fun set_staking_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
pragma verify &#61; false;
include SetStakingContractOperator;
</code></pre>




<a id="0x1_staking_proxy_SetStakingContractOperator"></a>


<pre><code>schema SetStakingContractOperator &#123;
    owner: signer;
    old_operator: address;
    new_operator: address;
    let owner_address &#61; signer::address_of(owner);
    let store &#61; global&lt;Store&gt;(owner_address);
    let staking_contract_exists &#61; exists&lt;Store&gt;(owner_address) &amp;&amp; simple_map::spec_contains_key(store.staking_contracts, old_operator);
    aborts_if staking_contract_exists &amp;&amp; simple_map::spec_contains_key(store.staking_contracts, new_operator);
    let post post_store &#61; global&lt;Store&gt;(owner_address);
    ensures staking_contract_exists &#61;&#61;&gt; !simple_map::spec_contains_key(post_store.staking_contracts, old_operator);
    let staking_contract &#61; simple_map::spec_get(store.staking_contracts, old_operator);
    let stake_pool &#61; global&lt;stake::StakePool&gt;(staking_contract.pool_address);
    let active &#61; coin::value(stake_pool.active);
    let pending_active &#61; coin::value(stake_pool.pending_active);
    let total_active_stake &#61; active &#43; pending_active;
    let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;
    let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;
    aborts_if staking_contract_exists &amp;&amp; !exists&lt;stake::StakePool&gt;(staking_contract.pool_address);
    ensures staking_contract_exists &#61;&#61;&gt;
        simple_map::spec_get(post_store.staking_contracts, new_operator).principal &#61;&#61; total_active_stake &#45; commission_amount;
    let pool_address &#61; staking_contract.owner_cap.pool_address;
    let current_commission_percentage &#61; staking_contract.commission_percentage;
    aborts_if staking_contract_exists &amp;&amp; commission_amount !&#61; 0 &amp;&amp; !exists&lt;stake::StakePool&gt;(pool_address);
    ensures staking_contract_exists &amp;&amp; commission_amount !&#61; 0 &#61;&#61;&gt;
        global&lt;stake::StakePool&gt;(pool_address).operator_address &#61;&#61; new_operator
        &amp;&amp; simple_map::spec_get(post_store.staking_contracts, new_operator).commission_percentage &#61;&#61; current_commission_percentage;
    ensures staking_contract_exists &#61;&#61;&gt; simple_map::spec_contains_key(post_store.staking_contracts, new_operator);
&#125;
</code></pre>



<a id="@Specification_0_set_stake_pool_operator"></a>

### Function `set_stake_pool_operator`


<pre><code>public entry fun set_stake_pool_operator(owner: &amp;signer, new_operator: address)
</code></pre>


Aborts if stake_pool is exists and when OwnerCapability or stake_pool_exists
One of them are not exists


<pre><code>include SetStakePoolOperator;
</code></pre>




<a id="0x1_staking_proxy_SetStakePoolOperator"></a>


<pre><code>schema SetStakePoolOperator &#123;
    owner: &amp;signer;
    new_operator: address;
    let owner_address &#61; signer::address_of(owner);
    let ownership_cap &#61; borrow_global&lt;stake::OwnerCapability&gt;(owner_address);
    let pool_address &#61; ownership_cap.pool_address;
    aborts_if stake::stake_pool_exists(owner_address) &amp;&amp; !(exists&lt;stake::OwnerCapability&gt;(owner_address) &amp;&amp; stake::stake_pool_exists(pool_address));
    ensures stake::stake_pool_exists(owner_address) &#61;&#61;&gt; global&lt;stake::StakePool&gt;(pool_address).operator_address &#61;&#61; new_operator;
&#125;
</code></pre>



<a id="@Specification_0_set_vesting_contract_voter"></a>

### Function `set_vesting_contract_voter`


<pre><code>public entry fun set_vesting_contract_voter(owner: &amp;signer, operator: address, new_voter: address)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_0_set_staking_contract_voter"></a>

### Function `set_staking_contract_voter`


<pre><code>public entry fun set_staking_contract_voter(owner: &amp;signer, operator: address, new_voter: address)
</code></pre>




<pre><code>include SetStakingContractVoter;
</code></pre>


Make sure staking_contract_exists first
Then abort if the resource is not exist


<a id="0x1_staking_proxy_SetStakingContractVoter"></a>


<pre><code>schema SetStakingContractVoter &#123;
    owner: &amp;signer;
    operator: address;
    new_voter: address;
    let owner_address &#61; signer::address_of(owner);
    let staker &#61; owner_address;
    let store &#61; global&lt;Store&gt;(staker);
    let staking_contract_exists &#61; exists&lt;Store&gt;(staker) &amp;&amp; simple_map::spec_contains_key(store.staking_contracts, operator);
    let staker_address &#61; owner_address;
    let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);
    let pool_address &#61; staking_contract.pool_address;
    let pool_address1 &#61; staking_contract.owner_cap.pool_address;
    aborts_if staking_contract_exists &amp;&amp; !exists&lt;stake::StakePool&gt;(pool_address);
    aborts_if staking_contract_exists &amp;&amp; !exists&lt;stake::StakePool&gt;(staking_contract.owner_cap.pool_address);
    ensures staking_contract_exists &#61;&#61;&gt; global&lt;stake::StakePool&gt;(pool_address1).delegated_voter &#61;&#61; new_voter;
&#125;
</code></pre>



<a id="@Specification_0_set_stake_pool_voter"></a>

### Function `set_stake_pool_voter`


<pre><code>public entry fun set_stake_pool_voter(owner: &amp;signer, new_voter: address)
</code></pre>




<pre><code>include SetStakePoolVoterAbortsIf;
</code></pre>




<a id="0x1_staking_proxy_SetStakePoolVoterAbortsIf"></a>


<pre><code>schema SetStakePoolVoterAbortsIf &#123;
    owner: &amp;signer;
    new_voter: address;
    let owner_address &#61; signer::address_of(owner);
    let ownership_cap &#61; global&lt;stake::OwnerCapability&gt;(owner_address);
    let pool_address &#61; ownership_cap.pool_address;
    aborts_if stake::stake_pool_exists(owner_address) &amp;&amp; !(exists&lt;stake::OwnerCapability&gt;(owner_address) &amp;&amp; stake::stake_pool_exists(pool_address));
    ensures stake::stake_pool_exists(owner_address) &#61;&#61;&gt; global&lt;stake::StakePool&gt;(pool_address).delegated_voter &#61;&#61; new_voter;
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
