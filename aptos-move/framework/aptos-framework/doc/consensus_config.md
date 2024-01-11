
<a id="0x1_consensus_config"></a>

# Module `0x1::consensus_config`

Maintains the consensus config for the blockchain. The config is stored in a
Reconfiguration, and may be updated by root.


-  [Resource `ConsensusConfig`](#0x1_consensus_config_ConsensusConfig)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_consensus_config_initialize)
-  [Function `set`](#0x1_consensus_config_set)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `set`](#@Specification_1_set)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_consensus_config_ConsensusConfig"></a>

## Resource `ConsensusConfig`



<pre><code><b>struct</b> <a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_consensus_config_EINVALID_CONFIG"></a>

The provided on chain config bytes are empty or invalid


<pre><code><b>const</b> <a href="consensus_config.md#0x1_consensus_config_EINVALID_CONFIG">EINVALID_CONFIG</a>: u64 = 1;
</code></pre>



<a id="0x1_consensus_config_initialize"></a>

## Function `initialize`

Publishes the ConsensusConfig config.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="consensus_config.md#0x1_consensus_config_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="consensus_config.md#0x1_consensus_config_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&config) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="consensus_config.md#0x1_consensus_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));
    <b>move_to</b>(aptos_framework, <a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a> { config });
}
</code></pre>



</details>

<a id="0x1_consensus_config_set"></a>

## Function `set`

This can be called by on-chain governance to update on-chain consensus configs.


<pre><code><b>public</b> <b>fun</b> <a href="consensus_config.md#0x1_consensus_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="consensus_config.md#0x1_consensus_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&config) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="consensus_config.md#0x1_consensus_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));

    <b>let</b> config_ref = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a>&gt;(@aptos_framework).config;
    *config_ref = config;

    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so validator nodes can sync on the updated configs.
    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>During genesis, the Aptos framework account should be assigned the consensus config resource.</td>
<td>Medium</td>
<td>The consensus_config::initialize function calls the assert_aptos_framework function to ensure that the signer is the aptos_framework and then assigns the ConsensusConfig resource to it.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Only aptos framework account is allowed to update the consensus configuration.</td>
<td>Medium</td>
<td>The consensus_config::set function ensures that the signer is aptos_framework.</td>
<td>Formally verified via <a href="#high-level-req-2">set</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Only a valid configuration can be used during initialization and update.</td>
<td>Medium</td>
<td>Both the initialize and set functions validate the config by ensuring its length to be greater than 0.</td>
<td>Formally verified via <a href="#high-level-req-3.1">initialize</a> and <a href="#high-level-req-3.2">set</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="consensus_config.md#0x1_consensus_config_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


Ensure caller is admin.
Aborts if StateStorageUsage already exists.


<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a>&gt;(@aptos_framework);
// This enforces <a id="high-level-req-3.1" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> !(len(config) &gt; 0);
<b>ensures</b> <b>global</b>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a>&gt;(addr) == <a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a> { config };
</code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code><b>public</b> <b>fun</b> <a href="consensus_config.md#0x1_consensus_config_set">set</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


Ensure the caller is admin and <code><a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a></code> should be existed.
When setting now time must be later than last_reconfiguration_time.


<pre><code><b>pragma</b> verify_duration_estimate = 120;
<b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;
<b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a>&gt;(@aptos_framework);
// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> !(len(config) &gt; 0);
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>requires</b> <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &gt;= <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();
<b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);
<b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
<b>ensures</b> <b>global</b>&lt;<a href="consensus_config.md#0x1_consensus_config_ConsensusConfig">ConsensusConfig</a>&gt;(@aptos_framework).config == config;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
