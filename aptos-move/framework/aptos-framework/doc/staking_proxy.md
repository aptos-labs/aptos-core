
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


<pre><code>use 0x1::signer;<br/>use 0x1::stake;<br/>use 0x1::staking_contract;<br/>use 0x1::vesting;<br/></code></pre>



<a id="0x1_staking_proxy_set_operator"></a>

## Function `set_operator`



<pre><code>public entry fun set_operator(owner: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_operator(owner: &amp;signer, old_operator: address, new_operator: address) &#123;<br/>    set_vesting_contract_operator(owner, old_operator, new_operator);<br/>    set_staking_contract_operator(owner, old_operator, new_operator);<br/>    set_stake_pool_operator(owner, new_operator);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_proxy_set_voter"></a>

## Function `set_voter`



<pre><code>public entry fun set_voter(owner: &amp;signer, operator: address, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_voter(owner: &amp;signer, operator: address, new_voter: address) &#123;<br/>    set_vesting_contract_voter(owner, operator, new_voter);<br/>    set_staking_contract_voter(owner, operator, new_voter);<br/>    set_stake_pool_voter(owner, new_voter);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_proxy_set_vesting_contract_operator"></a>

## Function `set_vesting_contract_operator`



<pre><code>public entry fun set_vesting_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_vesting_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address) &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    let vesting_contracts &#61; &amp;vesting::vesting_contracts(owner_address);<br/>    vector::for_each_ref(vesting_contracts, &#124;vesting_contract&#124; &#123;<br/>        let vesting_contract &#61; &#42;vesting_contract;<br/>        if (vesting::operator(vesting_contract) &#61;&#61; old_operator) &#123;<br/>            let current_commission_percentage &#61; vesting::operator_commission_percentage(vesting_contract);<br/>            vesting::update_operator(owner, vesting_contract, new_operator, current_commission_percentage);<br/>        &#125;;<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_proxy_set_staking_contract_operator"></a>

## Function `set_staking_contract_operator`



<pre><code>public entry fun set_staking_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_staking_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address) &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    if (staking_contract::staking_contract_exists(owner_address, old_operator)) &#123;<br/>        let current_commission_percentage &#61; staking_contract::commission_percentage(owner_address, old_operator);<br/>        staking_contract::switch_operator(owner, old_operator, new_operator, current_commission_percentage);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_proxy_set_stake_pool_operator"></a>

## Function `set_stake_pool_operator`



<pre><code>public entry fun set_stake_pool_operator(owner: &amp;signer, new_operator: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_stake_pool_operator(owner: &amp;signer, new_operator: address) &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    if (stake::stake_pool_exists(owner_address)) &#123;<br/>        stake::set_operator(owner, new_operator);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_proxy_set_vesting_contract_voter"></a>

## Function `set_vesting_contract_voter`



<pre><code>public entry fun set_vesting_contract_voter(owner: &amp;signer, operator: address, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_vesting_contract_voter(owner: &amp;signer, operator: address, new_voter: address) &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    let vesting_contracts &#61; &amp;vesting::vesting_contracts(owner_address);<br/>    vector::for_each_ref(vesting_contracts, &#124;vesting_contract&#124; &#123;<br/>        let vesting_contract &#61; &#42;vesting_contract;<br/>        if (vesting::operator(vesting_contract) &#61;&#61; operator) &#123;<br/>            vesting::update_voter(owner, vesting_contract, new_voter);<br/>        &#125;;<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_proxy_set_staking_contract_voter"></a>

## Function `set_staking_contract_voter`



<pre><code>public entry fun set_staking_contract_voter(owner: &amp;signer, operator: address, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_staking_contract_voter(owner: &amp;signer, operator: address, new_voter: address) &#123;<br/>    let owner_address &#61; signer::address_of(owner);<br/>    if (staking_contract::staking_contract_exists(owner_address, operator)) &#123;<br/>        staking_contract::update_voter(owner, operator, new_voter);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_staking_proxy_set_stake_pool_voter"></a>

## Function `set_stake_pool_voter`



<pre><code>public entry fun set_stake_pool_voter(owner: &amp;signer, new_voter: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_stake_pool_voter(owner: &amp;signer, new_voter: address) &#123;<br/>    if (stake::stake_pool_exists(signer::address_of(owner))) &#123;<br/>        stake::set_delegated_voter(owner, new_voter);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;When updating the Vesting operator, it should be updated throughout all depending units.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The VestingContract contains a StakingInfo object that has an operator field, and this operator is mapped to a StakingContract object that in turn encompasses a StakePool object where the operator matches.&lt;/td&gt;<br/>&lt;td&gt;Audited that it ensures the two operator fields hold the new value after the update.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;When updating the Vesting voter, it should be updated throughout all depending units.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The VestingContract contains a StakingInfo object that has an operator field, and this operator is mapped to a StakingContract object that in turn encompasses a StakePool object where the operator matches.&lt;/td&gt;<br/>&lt;td&gt;Audited that it ensures the two operator fields hold the new value after the update.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The operator and voter of a Vesting Contract should only be updated by the owner of the contract.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The owner&#45;operator&#45;voter model, as defined in the documentation, grants distinct abilities to each role. Therefore, it&apos;s crucial to ensure that only the owner has the authority to modify the operator or voter, to prevent the compromise of the StakePool.&lt;/td&gt;<br/>&lt;td&gt;Audited that it ensures the signer owns the AdminStore resource and that the operator or voter intended for the update actually exists.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The operator and voter of a Staking Contract should only be updated by the owner of the contract.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The owner&#45;operator&#45;voter model, as defined in the documentation, grants distinct abilities to each role. Therefore, it&apos;s crucial to ensure that only the owner has the authority to modify the operator or voter, to prevent the compromise of the StakePool.&lt;/td&gt;<br/>&lt;td&gt;Audited the patterns of updating operators and voters in the staking contract.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;Staking Contract&apos;s operators should be unique inside a store.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;Duplicates among operators could result in incorrectly updating the operator or voter associated with the incorrect StakingContract.&lt;/td&gt;<br/>&lt;td&gt;Enforced via &lt;a href&#61;&quot;https://github.com/aptos&#45;labs/aptos&#45;core/blob/main/aptos&#45;move/framework/aptos&#45;framework/sources/staking_contract.move&#35;L87&quot;&gt;SimpleMap&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_0_set_operator"></a>

### Function `set_operator`


<pre><code>public entry fun set_operator(owner: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>


Aborts if conditions of SetStakePoolOperator are not met


<pre><code>pragma verify &#61; false;<br/>pragma aborts_if_is_partial;<br/>include SetStakePoolOperator;<br/>include SetStakingContractOperator;<br/></code></pre>



<a id="@Specification_0_set_voter"></a>

### Function `set_voter`


<pre><code>public entry fun set_voter(owner: &amp;signer, operator: address, new_voter: address)<br/></code></pre>


Aborts if conditions of SetStackingContractVoter and SetStackPoolVoterAbortsIf are not met


<pre><code>pragma aborts_if_is_partial;<br/>include SetStakingContractVoter;<br/>include SetStakePoolVoterAbortsIf;<br/></code></pre>



<a id="@Specification_0_set_vesting_contract_operator"></a>

### Function `set_vesting_contract_operator`


<pre><code>public entry fun set_vesting_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_0_set_staking_contract_operator"></a>

### Function `set_staking_contract_operator`


<pre><code>public entry fun set_staking_contract_operator(owner: &amp;signer, old_operator: address, new_operator: address)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>pragma verify &#61; false;<br/>include SetStakingContractOperator;<br/></code></pre>




<a id="0x1_staking_proxy_SetStakingContractOperator"></a>


<pre><code>schema SetStakingContractOperator &#123;<br/>owner: signer;<br/>old_operator: address;<br/>new_operator: address;<br/>let owner_address &#61; signer::address_of(owner);<br/>let store &#61; global&lt;Store&gt;(owner_address);<br/>let staking_contract_exists &#61; exists&lt;Store&gt;(owner_address) &amp;&amp; simple_map::spec_contains_key(store.staking_contracts, old_operator);<br/>aborts_if staking_contract_exists &amp;&amp; simple_map::spec_contains_key(store.staking_contracts, new_operator);<br/>let post post_store &#61; global&lt;Store&gt;(owner_address);<br/>ensures staking_contract_exists &#61;&#61;&gt; !simple_map::spec_contains_key(post_store.staking_contracts, old_operator);<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, old_operator);<br/>let stake_pool &#61; global&lt;stake::StakePool&gt;(staking_contract.pool_address);<br/>let active &#61; coin::value(stake_pool.active);<br/>let pending_active &#61; coin::value(stake_pool.pending_active);<br/>let total_active_stake &#61; active &#43; pending_active;<br/>let accumulated_rewards &#61; total_active_stake &#45; staking_contract.principal;<br/>let commission_amount &#61; accumulated_rewards &#42; staking_contract.commission_percentage / 100;<br/>aborts_if staking_contract_exists &amp;&amp; !exists&lt;stake::StakePool&gt;(staking_contract.pool_address);<br/>ensures staking_contract_exists &#61;&#61;&gt;<br/>    simple_map::spec_get(post_store.staking_contracts, new_operator).principal &#61;&#61; total_active_stake &#45; commission_amount;<br/>let pool_address &#61; staking_contract.owner_cap.pool_address;<br/>let current_commission_percentage &#61; staking_contract.commission_percentage;<br/>aborts_if staking_contract_exists &amp;&amp; commission_amount !&#61; 0 &amp;&amp; !exists&lt;stake::StakePool&gt;(pool_address);<br/>ensures staking_contract_exists &amp;&amp; commission_amount !&#61; 0 &#61;&#61;&gt;<br/>    global&lt;stake::StakePool&gt;(pool_address).operator_address &#61;&#61; new_operator<br/>    &amp;&amp; simple_map::spec_get(post_store.staking_contracts, new_operator).commission_percentage &#61;&#61; current_commission_percentage;<br/>ensures staking_contract_exists &#61;&#61;&gt; simple_map::spec_contains_key(post_store.staking_contracts, new_operator);<br/>&#125;<br/></code></pre>



<a id="@Specification_0_set_stake_pool_operator"></a>

### Function `set_stake_pool_operator`


<pre><code>public entry fun set_stake_pool_operator(owner: &amp;signer, new_operator: address)<br/></code></pre>


Aborts if stake_pool is exists and when OwnerCapability or stake_pool_exists<br/> One of them are not exists


<pre><code>include SetStakePoolOperator;<br/></code></pre>




<a id="0x1_staking_proxy_SetStakePoolOperator"></a>


<pre><code>schema SetStakePoolOperator &#123;<br/>owner: &amp;signer;<br/>new_operator: address;<br/>let owner_address &#61; signer::address_of(owner);<br/>let ownership_cap &#61; borrow_global&lt;stake::OwnerCapability&gt;(owner_address);<br/>let pool_address &#61; ownership_cap.pool_address;<br/>aborts_if stake::stake_pool_exists(owner_address) &amp;&amp; !(exists&lt;stake::OwnerCapability&gt;(owner_address) &amp;&amp; stake::stake_pool_exists(pool_address));<br/>ensures stake::stake_pool_exists(owner_address) &#61;&#61;&gt; global&lt;stake::StakePool&gt;(pool_address).operator_address &#61;&#61; new_operator;<br/>&#125;<br/></code></pre>



<a id="@Specification_0_set_vesting_contract_voter"></a>

### Function `set_vesting_contract_voter`


<pre><code>public entry fun set_vesting_contract_voter(owner: &amp;signer, operator: address, new_voter: address)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_0_set_staking_contract_voter"></a>

### Function `set_staking_contract_voter`


<pre><code>public entry fun set_staking_contract_voter(owner: &amp;signer, operator: address, new_voter: address)<br/></code></pre>




<pre><code>include SetStakingContractVoter;<br/></code></pre>


Make sure staking_contract_exists first<br/> Then abort if the resource is not exist


<a id="0x1_staking_proxy_SetStakingContractVoter"></a>


<pre><code>schema SetStakingContractVoter &#123;<br/>owner: &amp;signer;<br/>operator: address;<br/>new_voter: address;<br/>let owner_address &#61; signer::address_of(owner);<br/>let staker &#61; owner_address;<br/>let store &#61; global&lt;Store&gt;(staker);<br/>let staking_contract_exists &#61; exists&lt;Store&gt;(staker) &amp;&amp; simple_map::spec_contains_key(store.staking_contracts, operator);<br/>let staker_address &#61; owner_address;<br/>let staking_contract &#61; simple_map::spec_get(store.staking_contracts, operator);<br/>let pool_address &#61; staking_contract.pool_address;<br/>let pool_address1 &#61; staking_contract.owner_cap.pool_address;<br/>aborts_if staking_contract_exists &amp;&amp; !exists&lt;stake::StakePool&gt;(pool_address);<br/>aborts_if staking_contract_exists &amp;&amp; !exists&lt;stake::StakePool&gt;(staking_contract.owner_cap.pool_address);<br/>ensures staking_contract_exists &#61;&#61;&gt; global&lt;stake::StakePool&gt;(pool_address1).delegated_voter &#61;&#61; new_voter;<br/>&#125;<br/></code></pre>



<a id="@Specification_0_set_stake_pool_voter"></a>

### Function `set_stake_pool_voter`


<pre><code>public entry fun set_stake_pool_voter(owner: &amp;signer, new_voter: address)<br/></code></pre>




<pre><code>include SetStakePoolVoterAbortsIf;<br/></code></pre>




<a id="0x1_staking_proxy_SetStakePoolVoterAbortsIf"></a>


<pre><code>schema SetStakePoolVoterAbortsIf &#123;<br/>owner: &amp;signer;<br/>new_voter: address;<br/>let owner_address &#61; signer::address_of(owner);<br/>let ownership_cap &#61; global&lt;stake::OwnerCapability&gt;(owner_address);<br/>let pool_address &#61; ownership_cap.pool_address;<br/>aborts_if stake::stake_pool_exists(owner_address) &amp;&amp; !(exists&lt;stake::OwnerCapability&gt;(owner_address) &amp;&amp; stake::stake_pool_exists(pool_address));<br/>ensures stake::stake_pool_exists(owner_address) &#61;&#61;&gt; global&lt;stake::StakePool&gt;(pool_address).delegated_voter &#61;&#61; new_voter;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
