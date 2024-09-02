
<a id="0x1_supra_governance"></a>

# Module `0x1::supra_governance`


AptosGovernance represents the on-chain governance of the Aptos network. Voting power is calculated based on the
current epoch's voting power of the proposer or voter's backing stake pool. In addition, for it to count,
the stake pool's lockup needs to be at least as long as the proposal's duration.

It provides the following flow:
1. Proposers can create a proposal by calling AptosGovernance::create_proposal. The proposer's backing stake pool
needs to have the minimum proposer stake required. Off-chain components can subscribe to CreateProposalEvent to
track proposal creation and proposal ids.
2. Voters can vote on a proposal. Their voting power is derived from the backing stake pool. A stake pool can vote
on a proposal multiple times as long as the total voting power of these votes doesn't exceed its total voting power.


-  [Resource `GovernanceResponsbility`](#0x1_supra_governance_GovernanceResponsbility)
-  [Resource `GovernanceConfig`](#0x1_supra_governance_GovernanceConfig)
-  [Struct `RecordKey`](#0x1_supra_governance_RecordKey)
-  [Resource `VotingRecords`](#0x1_supra_governance_VotingRecords)
-  [Resource `VotingRecordsV2`](#0x1_supra_governance_VotingRecordsV2)
-  [Resource `ApprovedExecutionHashes`](#0x1_supra_governance_ApprovedExecutionHashes)
-  [Resource `GovernanceEvents`](#0x1_supra_governance_GovernanceEvents)
-  [Struct `CreateProposalEvent`](#0x1_supra_governance_CreateProposalEvent)
-  [Struct `VoteEvent`](#0x1_supra_governance_VoteEvent)
-  [Struct `UpdateConfigEvent`](#0x1_supra_governance_UpdateConfigEvent)
-  [Struct `CreateProposal`](#0x1_supra_governance_CreateProposal)
-  [Struct `Vote`](#0x1_supra_governance_Vote)
-  [Struct `UpdateConfig`](#0x1_supra_governance_UpdateConfig)
-  [Resource `SupraGovernanceConfig`](#0x1_supra_governance_SupraGovernanceConfig)
-  [Resource `SupraGovernanceEvents`](#0x1_supra_governance_SupraGovernanceEvents)
-  [Struct `SupraCreateProposalEvent`](#0x1_supra_governance_SupraCreateProposalEvent)
-  [Struct `SupraUpdateConfigEvent`](#0x1_supra_governance_SupraUpdateConfigEvent)
-  [Struct `SupraVoteEvent`](#0x1_supra_governance_SupraVoteEvent)
-  [Struct `SupraCreateProposal`](#0x1_supra_governance_SupraCreateProposal)
-  [Struct `SupraVote`](#0x1_supra_governance_SupraVote)
-  [Struct `SupraUpdateConfig`](#0x1_supra_governance_SupraUpdateConfig)
-  [Constants](#@Constants_0)
-  [Function `store_signer_cap`](#0x1_supra_governance_store_signer_cap)
-  [Function `old_initialize`](#0x1_supra_governance_old_initialize)
-  [Function `initialize`](#0x1_supra_governance_initialize)
-  [Function `update_supra_governance_config`](#0x1_supra_governance_update_supra_governance_config)
-  [Function `initialize_partial_voting`](#0x1_supra_governance_initialize_partial_voting)
-  [Function `get_voting_duration_secs`](#0x1_supra_governance_get_voting_duration_secs)
-  [Function `get_min_voting_threshold`](#0x1_supra_governance_get_min_voting_threshold)
-  [Function `get_voters_list`](#0x1_supra_governance_get_voters_list)
-  [Function `get_required_proposer_stake`](#0x1_supra_governance_get_required_proposer_stake)
-  [Function `has_entirely_voted`](#0x1_supra_governance_has_entirely_voted)
-  [Function `get_remaining_voting_power`](#0x1_supra_governance_get_remaining_voting_power)
-  [Function `create_proposal`](#0x1_supra_governance_create_proposal)
-  [Function `create_proposal_v2`](#0x1_supra_governance_create_proposal_v2)
-  [Function `create_proposal_v2_impl`](#0x1_supra_governance_create_proposal_v2_impl)
-  [Function `supra_create_proposal`](#0x1_supra_governance_supra_create_proposal)
-  [Function `supra_create_proposal_v2`](#0x1_supra_governance_supra_create_proposal_v2)
-  [Function `supra_create_proposal_v2_impl`](#0x1_supra_governance_supra_create_proposal_v2_impl)
-  [Function `vote`](#0x1_supra_governance_vote)
-  [Function `partial_vote`](#0x1_supra_governance_partial_vote)
-  [Function `vote_internal`](#0x1_supra_governance_vote_internal)
-  [Function `supra_vote`](#0x1_supra_governance_supra_vote)
-  [Function `supra_vote_internal`](#0x1_supra_governance_supra_vote_internal)
-  [Function `add_supra_approved_script_hash_script`](#0x1_supra_governance_add_supra_approved_script_hash_script)
-  [Function `add_approved_script_hash`](#0x1_supra_governance_add_approved_script_hash)
-  [Function `add_supra_approved_script_hash`](#0x1_supra_governance_add_supra_approved_script_hash)
-  [Function `resolve`](#0x1_supra_governance_resolve)
-  [Function `supra_resolve`](#0x1_supra_governance_supra_resolve)
-  [Function `resolve_multi_step_proposal`](#0x1_supra_governance_resolve_multi_step_proposal)
-  [Function `resolve_supra_multi_step_proposal`](#0x1_supra_governance_resolve_supra_multi_step_proposal)
-  [Function `remove_approved_hash`](#0x1_supra_governance_remove_approved_hash)
-  [Function `remove_supra_approved_hash`](#0x1_supra_governance_remove_supra_approved_hash)
-  [Function `reconfigure`](#0x1_supra_governance_reconfigure)
-  [Function `force_end_epoch`](#0x1_supra_governance_force_end_epoch)
-  [Function `force_end_epoch_test_only`](#0x1_supra_governance_force_end_epoch_test_only)
-  [Function `toggle_features`](#0x1_supra_governance_toggle_features)
-  [Function `get_signer_testnet_only`](#0x1_supra_governance_get_signer_testnet_only)
-  [Function `get_voting_power`](#0x1_supra_governance_get_voting_power)
-  [Function `get_signer`](#0x1_supra_governance_get_signer)
-  [Function `create_proposal_metadata`](#0x1_supra_governance_create_proposal_metadata)
-  [Function `assert_voting_initialization`](#0x1_supra_governance_assert_voting_initialization)
-  [Function `initialize_for_verification`](#0x1_supra_governance_initialize_for_verification)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="governance_proposal.md#0x1_governance_proposal">0x1::governance_proposal</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="multisig_voting.md#0x1_multisig_voting">0x1::multisig_voting</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="randomness_config.md#0x1_randomness_config">0x1::randomness_config</a>;
<b>use</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg">0x1::reconfiguration_with_dkg</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="voting.md#0x1_voting">0x1::voting</a>;
</code></pre>



<a id="0x1_supra_governance_GovernanceResponsbility"></a>

## Resource `GovernanceResponsbility`

Store the SignerCapabilities of accounts under the on-chain governance's control.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>signer_caps: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_GovernanceConfig"></a>

## Resource `GovernanceConfig`

Configurations of the AptosGovernance, set during Genesis and can be updated by the same process offered
by this AptosGovernance module.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>min_voting_threshold: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>required_proposer_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_duration_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_RecordKey"></a>

## Struct `RecordKey`



<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_RecordKey">RecordKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>stake_pool: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_VotingRecords"></a>

## Resource `VotingRecords`

Records to track the proposals each stake pool has been used to vote on.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="supra_governance.md#0x1_supra_governance_RecordKey">supra_governance::RecordKey</a>, bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_VotingRecordsV2"></a>

## Resource `VotingRecordsV2`

Records to track the voting power usage of each stake pool on each proposal.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<a href="supra_governance.md#0x1_supra_governance_RecordKey">supra_governance::RecordKey</a>, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_ApprovedExecutionHashes"></a>

## Resource `ApprovedExecutionHashes`

Used to track which execution script hashes have been approved by governance.
This is required to bypass cases where the execution scripts exceed the size limit imposed by mempool.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hashes: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_GovernanceEvents"></a>

## Resource `GovernanceEvents`

Events generated by interactions with the AptosGovernance module.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_CreateProposalEvent">supra_governance::CreateProposalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_config_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_UpdateConfigEvent">supra_governance::UpdateConfigEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_VoteEvent">supra_governance::VoteEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_CreateProposalEvent"></a>

## Struct `CreateProposalEvent`

Event emitted when a proposal is created.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_CreateProposalEvent">CreateProposalEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_VoteEvent"></a>

## Struct `VoteEvent`

Event emitted when there's a vote on a proposa;


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_VoteEvent">VoteEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>num_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>should_pass: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_UpdateConfigEvent"></a>

## Struct `UpdateConfigEvent`

Event emitted when the governance configs are updated.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_UpdateConfigEvent">UpdateConfigEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>min_voting_threshold: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>required_proposer_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_duration_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_CreateProposal"></a>

## Struct `CreateProposal`

Event emitted when a proposal is created.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="supra_governance.md#0x1_supra_governance_CreateProposal">CreateProposal</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_Vote"></a>

## Struct `Vote`

Event emitted when there's a vote on a proposa;


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="supra_governance.md#0x1_supra_governance_Vote">Vote</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>num_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>should_pass: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_UpdateConfig"></a>

## Struct `UpdateConfig`

Event emitted when the governance configs are updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="supra_governance.md#0x1_supra_governance_UpdateConfig">UpdateConfig</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>min_voting_threshold: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>required_proposer_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voting_duration_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraGovernanceConfig"></a>

## Resource `SupraGovernanceConfig`

Configurations of the SupraGovernance, set during Genesis and can be updated by the same process offered
by this SupraGovernance module.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voting_duration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>min_voting_threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraGovernanceEvents"></a>

## Resource `SupraGovernanceEvents`

Events generated by interactions with the SupraGovernance module.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraCreateProposalEvent">supra_governance::SupraCreateProposalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_config_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraUpdateConfigEvent">supra_governance::SupraUpdateConfigEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraVoteEvent">supra_governance::SupraVoteEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraCreateProposalEvent"></a>

## Struct `SupraCreateProposalEvent`

Event emitted when a proposal is created.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraCreateProposalEvent">SupraCreateProposalEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraUpdateConfigEvent"></a>

## Struct `SupraUpdateConfigEvent`

Event emitted when the governance configs are updated.


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraUpdateConfigEvent">SupraUpdateConfigEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voting_duration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>min_voting_threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraVoteEvent"></a>

## Struct `SupraVoteEvent`

Event emitted when there's a vote on a proposa;


<pre><code><b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraVoteEvent">SupraVoteEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>should_pass: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraCreateProposal"></a>

## Struct `SupraCreateProposal`

Event emitted when a proposal is created.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraCreateProposal">SupraCreateProposal</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraVote"></a>

## Struct `SupraVote`

Event emitted when there's a vote on a proposa;


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraVote">SupraVote</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>should_pass: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_supra_governance_SupraUpdateConfig"></a>

## Struct `SupraUpdateConfig`

Event emitted when the governance configs are updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="supra_governance.md#0x1_supra_governance_SupraUpdateConfig">SupraUpdateConfig</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voting_duration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>min_voting_threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_supra_governance_MAX_U64"></a>



<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED"></a>

This matches the same enum const in voting. We have to duplicate it as Move doesn't have support for enums yet.


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>: u64 = 1;
</code></pre>



<a id="0x1_supra_governance_ETHRESHOLD_EXCEEDS_VOTERS"></a>

Threshold should not exceeds voters


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_EXCEEDS_VOTERS">ETHRESHOLD_EXCEEDS_VOTERS</a>: u64 = 17;
</code></pre>



<a id="0x1_supra_governance_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE"></a>

Threshold value must be greater than 1


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE">ETHRESHOLD_MUST_BE_GREATER_THAN_ONE</a>: u64 = 18;
</code></pre>



<a id="0x1_supra_governance_EACCOUNT_NOT_AUTHORIZED"></a>

The account does not have permission to propose or vote


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EACCOUNT_NOT_AUTHORIZED">EACCOUNT_NOT_AUTHORIZED</a>: u64 = 15;
</code></pre>



<a id="0x1_supra_governance_EALREADY_VOTED"></a>

The specified stake pool has already been used to vote on the same proposal


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EALREADY_VOTED">EALREADY_VOTED</a>: u64 = 4;
</code></pre>



<a id="0x1_supra_governance_EINSUFFICIENT_PROPOSER_STAKE"></a>

The specified stake pool does not have sufficient stake to create a proposal


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>: u64 = 1;
</code></pre>



<a id="0x1_supra_governance_EINSUFFICIENT_STAKE_LOCKUP"></a>

The specified stake pool does not have long enough remaining lockup to create a proposal or vote


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EINSUFFICIENT_STAKE_LOCKUP">EINSUFFICIENT_STAKE_LOCKUP</a>: u64 = 3;
</code></pre>



<a id="0x1_supra_governance_EMETADATA_HASH_TOO_LONG"></a>

Metadata hash cannot be longer than 256 chars


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EMETADATA_HASH_TOO_LONG">EMETADATA_HASH_TOO_LONG</a>: u64 = 10;
</code></pre>



<a id="0x1_supra_governance_EMETADATA_LOCATION_TOO_LONG"></a>

Metadata location cannot be longer than 256 chars


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EMETADATA_LOCATION_TOO_LONG">EMETADATA_LOCATION_TOO_LONG</a>: u64 = 9;
</code></pre>



<a id="0x1_supra_governance_ENOT_DELEGATED_VOTER"></a>

This account is not the designated voter of the specified stake pool


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>: u64 = 2;
</code></pre>



<a id="0x1_supra_governance_ENO_VOTING_POWER"></a>

The specified stake pool must be part of the validator set


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_ENO_VOTING_POWER">ENO_VOTING_POWER</a>: u64 = 5;
</code></pre>



<a id="0x1_supra_governance_EPARTIAL_VOTING_NOT_INITIALIZED"></a>

Partial voting feature hasn't been properly initialized.


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EPARTIAL_VOTING_NOT_INITIALIZED">EPARTIAL_VOTING_NOT_INITIALIZED</a>: u64 = 13;
</code></pre>



<a id="0x1_supra_governance_EPROPOSAL_IS_EXPIRE"></a>

Proposal is expired


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_IS_EXPIRE">EPROPOSAL_IS_EXPIRE</a>: u64 = 16;
</code></pre>



<a id="0x1_supra_governance_EPROPOSAL_NOT_RESOLVABLE_YET"></a>

Proposal is not ready to be resolved. Waiting on time or votes


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_NOT_RESOLVABLE_YET">EPROPOSAL_NOT_RESOLVABLE_YET</a>: u64 = 6;
</code></pre>



<a id="0x1_supra_governance_EPROPOSAL_NOT_RESOLVED_YET"></a>

The proposal has not been resolved yet


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_NOT_RESOLVED_YET">EPROPOSAL_NOT_RESOLVED_YET</a>: u64 = 8;
</code></pre>



<a id="0x1_supra_governance_EUNAUTHORIZED"></a>

Account is not authorized to call this function.


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EUNAUTHORIZED">EUNAUTHORIZED</a>: u64 = 11;
</code></pre>



<a id="0x1_supra_governance_METADATA_HASH_KEY"></a>



<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_METADATA_HASH_KEY">METADATA_HASH_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [109, 101, 116, 97, 100, 97, 116, 97, 95, 104, 97, 115, 104];
</code></pre>



<a id="0x1_supra_governance_METADATA_LOCATION_KEY"></a>

Proposal metadata attribute keys.


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_METADATA_LOCATION_KEY">METADATA_LOCATION_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [109, 101, 116, 97, 100, 97, 116, 97, 95, 108, 111, 99, 97, 116, 105, 111, 110];
</code></pre>



<a id="0x1_supra_governance_store_signer_cap"></a>

## Function `store_signer_cap`

Can be called during genesis or by the governance itself.
Stores the signer capability for a given address.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_store_signer_cap">store_signer_cap</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>, signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_store_signer_cap">store_signer_cap</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    signer_address: <b>address</b>,
    signer_cap: SignerCapability,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved">system_addresses::assert_framework_reserved</a>(signer_address);

    <b>if</b> (!<b>exists</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@supra_framework)) {
        <b>move_to</b>(
            supra_framework,
            <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> { signer_caps: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, SignerCapability&gt;() }
        );
    };

    <b>let</b> signer_caps = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@supra_framework).signer_caps;
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(signer_caps, signer_address, signer_cap);
}
</code></pre>



</details>

<a id="0x1_supra_governance_old_initialize"></a>

## Function `old_initialize`



<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_old_initialize">old_initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_old_initialize">old_initialize</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    min_voting_threshold: u128,
    required_proposer_stake: u64,
    voting_duration_secs: u64,
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);

    <a href="voting.md#0x1_voting_register">voting::register</a>&lt;GovernanceProposal&gt;(supra_framework);
    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a> {
        voting_duration_secs,
        min_voting_threshold,
        required_proposer_stake,
    });
    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> {
        create_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_CreateProposalEvent">CreateProposalEvent</a>&gt;(supra_framework),
        update_config_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_UpdateConfigEvent">UpdateConfigEvent</a>&gt;(supra_framework),
        vote_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_VoteEvent">VoteEvent</a>&gt;(supra_framework),
    });
    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a> {
        votes: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
    });
    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
        hashes: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;()
    });
}
</code></pre>



</details>

<a id="0x1_supra_governance_initialize"></a>

## Function `initialize`

Initializes the state for Aptos Governance. Can only be called during Genesis with a signer
for the supra_framework (0x1) account.
This function is private because it's called directly from the vm.


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_initialize">initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, voting_duration_secs: u64, min_voting_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_initialize">initialize</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    voting_duration_secs: u64,
    min_voting_threshold: u64,
    voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
) {
    <a href="multisig_voting.md#0x1_multisig_voting_register">multisig_voting::register</a>&lt;GovernanceProposal&gt;(supra_framework);

    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&voters) &gt;= min_voting_threshold && min_voting_threshold &gt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&voters) / 2, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_EXCEEDS_VOTERS">ETHRESHOLD_EXCEEDS_VOTERS</a>));
    <b>assert</b>!(min_voting_threshold &gt; 1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE">ETHRESHOLD_MUST_BE_GREATER_THAN_ONE</a>));

    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a> {
        voting_duration_secs,
        min_voting_threshold,
        voters,
    });
    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a> {
        create_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraCreateProposalEvent">SupraCreateProposalEvent</a>&gt;(supra_framework),
        update_config_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraUpdateConfigEvent">SupraUpdateConfigEvent</a>&gt;(supra_framework),
        vote_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraVoteEvent">SupraVoteEvent</a>&gt;(supra_framework),
    });
    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
        hashes: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(),
    })
}
</code></pre>



</details>

<a id="0x1_supra_governance_update_supra_governance_config"></a>

## Function `update_supra_governance_config`

Update the governance configurations. This can only be called as part of resolving a proposal in this same
AptosGovernance.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_update_supra_governance_config">update_supra_governance_config</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, voting_duration_secs: u64, min_voting_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_update_supra_governance_config">update_supra_governance_config</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    voting_duration_secs: u64,
    min_voting_threshold: u64,
    voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);

    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&voters) &gt;= min_voting_threshold && min_voting_threshold &gt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&voters) / 2, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_EXCEEDS_VOTERS">ETHRESHOLD_EXCEEDS_VOTERS</a>));
    <b>assert</b>!(min_voting_threshold &gt; 1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE">ETHRESHOLD_MUST_BE_GREATER_THAN_ONE</a>));

    <b>let</b> supra_governance_config = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>&gt;(@supra_framework);
    supra_governance_config.voting_duration_secs = voting_duration_secs;
    supra_governance_config.min_voting_threshold = min_voting_threshold;
    supra_governance_config.voters = voters;

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="supra_governance.md#0x1_supra_governance_SupraUpdateConfig">SupraUpdateConfig</a> {
                min_voting_threshold,
                voting_duration_secs,
                voters,
            },
        )
    };
    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a>&gt;(@supra_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraUpdateConfigEvent">SupraUpdateConfigEvent</a>&gt;(
        &<b>mut</b> events.update_config_events,
        <a href="supra_governance.md#0x1_supra_governance_SupraUpdateConfigEvent">SupraUpdateConfigEvent</a> {
            min_voting_threshold,
            voting_duration_secs,
            voters,
        },
    );
}
</code></pre>



</details>

<a id="0x1_supra_governance_initialize_partial_voting"></a>

## Function `initialize_partial_voting`

Initializes the state for Aptos Governance partial voting. Can only be called through Aptos governance
proposals with a signer for the supra_framework (0x1) account.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_initialize_partial_voting">initialize_partial_voting</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_initialize_partial_voting">initialize_partial_voting</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);

    <b>move_to</b>(supra_framework, <a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a> {
        votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),
    });
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_voting_duration_secs"></a>

## Function `get_voting_duration_secs`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_voting_duration_secs">get_voting_duration_secs</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_voting_duration_secs">get_voting_duration_secs</a>(): u64 <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a> {
    <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>&gt;(@supra_framework).voting_duration_secs
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_min_voting_threshold"></a>

## Function `get_min_voting_threshold`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_min_voting_threshold">get_min_voting_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_min_voting_threshold">get_min_voting_threshold</a>(): u64 <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a> {
    <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>&gt;(@supra_framework).min_voting_threshold
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_voters_list"></a>

## Function `get_voters_list`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_voters_list">get_voters_list</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_voters_list">get_voters_list</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a> {
    <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>&gt;(@supra_framework).voters
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_required_proposer_stake"></a>

## Function `get_required_proposer_stake`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_required_proposer_stake">get_required_proposer_stake</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_required_proposer_stake">get_required_proposer_stake</a>(): u64 <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a> {
    <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@supra_framework).required_proposer_stake
}
</code></pre>



</details>

<a id="0x1_supra_governance_has_entirely_voted"></a>

## Function `has_entirely_voted`

Return true if a stake pool has already voted on a proposal before partial governance voting is enabled.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_has_entirely_voted">has_entirely_voted</a>(stake_pool: <b>address</b>, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_has_entirely_voted">has_entirely_voted</a>(stake_pool: <b>address</b>, proposal_id: u64): bool <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a> {
    <b>let</b> record_key = <a href="supra_governance.md#0x1_supra_governance_RecordKey">RecordKey</a> {
        stake_pool,
        proposal_id,
    };
    // If a <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> already voted on a proposal before partial governance <a href="voting.md#0x1_voting">voting</a> is enabled,
    // there is a record in <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a>.
    <b>let</b> voting_records = <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a>&gt;(@supra_framework);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&voting_records.votes, record_key)
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_remaining_voting_power"></a>

## Function `get_remaining_voting_power`

Return remaining voting power of a stake pool on a proposal.
Note: a stake pool's voting power on a proposal could increase over time(e.g. rewards/new stake).


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_remaining_voting_power">get_remaining_voting_power</a>(stake_pool: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_remaining_voting_power">get_remaining_voting_power</a>(
    stake_pool: <b>address</b>,
    proposal_id: u64
): u64 <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a>, <a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a> {
    <a href="supra_governance.md#0x1_supra_governance_assert_voting_initialization">assert_voting_initialization</a>();

    <b>let</b> proposal_expiration = <a href="voting.md#0x1_voting_get_proposal_expiration_secs">voting::get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(
        @supra_framework,
        proposal_id
    );
    <b>let</b> lockup_until = <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(stake_pool);
    // The voter's <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be locked up at least <b>as</b> long <b>as</b> the proposal's expiration.
    // Also no one can vote on a expired proposal.
    <b>if</b> (proposal_expiration &gt; lockup_until || <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; proposal_expiration) {
        <b>return</b> 0
    };

    // If a <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> already voted on a proposal before partial governance <a href="voting.md#0x1_voting">voting</a> is enabled, the <a href="stake.md#0x1_stake">stake</a> pool
    // cannot vote on the proposal even after partial governance <a href="voting.md#0x1_voting">voting</a> is enabled.
    <b>if</b> (<a href="supra_governance.md#0x1_supra_governance_has_entirely_voted">has_entirely_voted</a>(stake_pool, proposal_id)) {
        <b>return</b> 0
    };
    <b>let</b> record_key = <a href="supra_governance.md#0x1_supra_governance_RecordKey">RecordKey</a> {
        stake_pool,
        proposal_id,
    };
    <b>let</b> used_voting_power = 0u64;
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>()) {
        <b>let</b> voting_records_v2 = <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@supra_framework);
        used_voting_power = *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_with_default">smart_table::borrow_with_default</a>(&voting_records_v2.votes, record_key, &0);
    };
    <a href="supra_governance.md#0x1_supra_governance_get_voting_power">get_voting_power</a>(stake_pool) - used_voting_power
}
</code></pre>



</details>

<a id="0x1_supra_governance_create_proposal"></a>

## Function `create_proposal`

Create a single-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal">create_proposal</a>(proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal">create_proposal</a>(
    proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    stake_pool: <b>address</b>,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> {
    <a href="supra_governance.md#0x1_supra_governance_create_proposal_v2">create_proposal_v2</a>(proposer, stake_pool, execution_hash, metadata_location, metadata_hash, <b>false</b>);
}
</code></pre>



</details>

<a id="0x1_supra_governance_create_proposal_v2"></a>

## Function `create_proposal_v2`

Create a single-step or multi-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal_v2">create_proposal_v2</a>(proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal_v2">create_proposal_v2</a>(
    proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    stake_pool: <b>address</b>,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_multi_step_proposal: bool,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> {
    <a href="supra_governance.md#0x1_supra_governance_create_proposal_v2_impl">create_proposal_v2_impl</a>(
        proposer,
        stake_pool,
        execution_hash,
        metadata_location,
        metadata_hash,
        is_multi_step_proposal
    );
}
</code></pre>



</details>

<a id="0x1_supra_governance_create_proposal_v2_impl"></a>

## Function `create_proposal_v2_impl`

Create a single-step or multi-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.
Return proposal_id when a proposal is successfully created.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal_v2_impl">create_proposal_v2_impl</a>(proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal_v2_impl">create_proposal_v2_impl</a>(
    proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    stake_pool: <b>address</b>,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_multi_step_proposal: bool,
): u64 <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> {
    <b>let</b> proposer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(proposer);
    <b>assert</b>!(
        <a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(stake_pool) == proposer_address,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>)
    );

    // The proposer's <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be at least the required bond amount.
    <b>let</b> governance_config = <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@supra_framework);
    <b>let</b> stake_balance = <a href="supra_governance.md#0x1_supra_governance_get_voting_power">get_voting_power</a>(stake_pool);
    <b>assert</b>!(
        stake_balance &gt;= governance_config.required_proposer_stake,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>),
    );

    // The proposer's <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be locked up at least <b>as</b> long <b>as</b> the proposal's <a href="voting.md#0x1_voting">voting</a> period.
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> proposal_expiration = current_time + governance_config.voting_duration_secs;
    <b>assert</b>!(
        <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(stake_pool) &gt;= proposal_expiration,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EINSUFFICIENT_STAKE_LOCKUP">EINSUFFICIENT_STAKE_LOCKUP</a>),
    );

    // Create and validate proposal metadata.
    <b>let</b> proposal_metadata = <a href="supra_governance.md#0x1_supra_governance_create_proposal_metadata">create_proposal_metadata</a>(metadata_location, metadata_hash);

    // We want <b>to</b> allow early resolution of proposals <b>if</b> more than 50% of the total supply of the network coins
    // <b>has</b> voted. This doesn't take into subsequent inflation/deflation (rewards are issued every epoch and gas fees
    // are burnt after every transaction), but inflation/delation is very unlikely <b>to</b> have a major impact on total
    // supply during the <a href="voting.md#0x1_voting">voting</a> period.
    <b>let</b> total_voting_token_supply = <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;SupraCoin&gt;();
    <b>let</b> early_resolution_vote_threshold = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u128&gt;();
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&total_voting_token_supply)) {
        <b>let</b> total_supply = *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&total_voting_token_supply);
        // 50% + 1 <b>to</b> avoid rounding errors.
        early_resolution_vote_threshold = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(total_supply / 2 + 1);
    };

    <b>let</b> proposal_id = <a href="voting.md#0x1_voting_create_proposal_v2">voting::create_proposal_v2</a>(
        proposer_address,
        @supra_framework,
        <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">governance_proposal::create_proposal</a>(),
        execution_hash,
        governance_config.min_voting_threshold,
        proposal_expiration,
        early_resolution_vote_threshold,
        proposal_metadata,
        is_multi_step_proposal,
    );

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="supra_governance.md#0x1_supra_governance_CreateProposal">CreateProposal</a> {
                proposal_id,
                proposer: proposer_address,
                stake_pool,
                execution_hash,
                proposal_metadata,
            },
        );
    };
    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a>&gt;(@supra_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="supra_governance.md#0x1_supra_governance_CreateProposalEvent">CreateProposalEvent</a>&gt;(
        &<b>mut</b> events.create_proposal_events,
        <a href="supra_governance.md#0x1_supra_governance_CreateProposalEvent">CreateProposalEvent</a> {
            proposal_id,
            proposer: proposer_address,
            stake_pool,
            execution_hash,
            proposal_metadata,
        },
    );
    proposal_id
}
</code></pre>



</details>

<a id="0x1_supra_governance_supra_create_proposal"></a>

## Function `supra_create_proposal`

Create a single-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal">supra_create_proposal</a>(proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal">supra_create_proposal</a>(
    proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a> {
    <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal_v2">supra_create_proposal_v2</a>(proposer, execution_hash, metadata_location, metadata_hash, <b>false</b>);
}
</code></pre>



</details>

<a id="0x1_supra_governance_supra_create_proposal_v2"></a>

## Function `supra_create_proposal_v2`

Create a single-step or multi-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal_v2">supra_create_proposal_v2</a>(proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal_v2">supra_create_proposal_v2</a>(
    proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_multi_step_proposal: bool,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a> {
    <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal_v2_impl">supra_create_proposal_v2_impl</a>(
        proposer,
        execution_hash,
        metadata_location,
        metadata_hash,
        is_multi_step_proposal
    );
}
</code></pre>



</details>

<a id="0x1_supra_governance_supra_create_proposal_v2_impl"></a>

## Function `supra_create_proposal_v2_impl`

Create a single-step or multi-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.
Return proposal_id when a proposal is successfully created.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal_v2_impl">supra_create_proposal_v2_impl</a>(proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_create_proposal_v2_impl">supra_create_proposal_v2_impl</a>(
    proposer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    is_multi_step_proposal: bool,
): u64 <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a> {
    <b>let</b> proposer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(proposer);
    <b>let</b> supra_governance_config = <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>&gt;(@supra_framework);

    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&supra_governance_config.voters, &proposer_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="supra_governance.md#0x1_supra_governance_EACCOUNT_NOT_AUTHORIZED">EACCOUNT_NOT_AUTHORIZED</a>));

    <b>let</b> proposal_expiration = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() + supra_governance_config.voting_duration_secs;

    // Create and validate proposal metadata.
    <b>let</b> proposal_metadata = <a href="supra_governance.md#0x1_supra_governance_create_proposal_metadata">create_proposal_metadata</a>(metadata_location, metadata_hash);

    <b>let</b> proposal_id = <a href="multisig_voting.md#0x1_multisig_voting_create_proposal_v2">multisig_voting::create_proposal_v2</a>(
        proposer_address,
        @supra_framework,
        <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">governance_proposal::create_proposal</a>(),
        execution_hash,
        supra_governance_config.min_voting_threshold,
        supra_governance_config.voters,
        proposal_expiration,
        proposal_metadata,
        is_multi_step_proposal,
    );

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="supra_governance.md#0x1_supra_governance_SupraCreateProposal">SupraCreateProposal</a> {
                proposal_id,
                proposer: proposer_address,
                execution_hash,
                proposal_metadata,
            },
        );
    };
    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a>&gt;(@supra_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraCreateProposalEvent">SupraCreateProposalEvent</a>&gt;(
        &<b>mut</b> events.create_proposal_events,
        <a href="supra_governance.md#0x1_supra_governance_SupraCreateProposalEvent">SupraCreateProposalEvent</a> {
            proposal_id,
            proposer: proposer_address,
            execution_hash,
            proposal_metadata,
        },
    );
    proposal_id
}
</code></pre>



</details>

<a id="0x1_supra_governance_vote"></a>

## Function `vote`

Vote on proposal with <code>proposal_id</code> and all voting power from <code>stake_pool</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_vote">vote</a>(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_vote">vote</a>(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    stake_pool: <b>address</b>,
    proposal_id: u64,
    should_pass: bool,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a>, <a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> {
    <a href="supra_governance.md#0x1_supra_governance_vote_internal">vote_internal</a>(voter, stake_pool, proposal_id, <a href="supra_governance.md#0x1_supra_governance_MAX_U64">MAX_U64</a>, should_pass);
}
</code></pre>



</details>

<a id="0x1_supra_governance_partial_vote"></a>

## Function `partial_vote`

Vote on proposal with <code>proposal_id</code> and specified voting power from <code>stake_pool</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_partial_vote">partial_vote</a>(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_partial_vote">partial_vote</a>(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    stake_pool: <b>address</b>,
    proposal_id: u64,
    voting_power: u64,
    should_pass: bool,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a>, <a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> {
    <a href="supra_governance.md#0x1_supra_governance_vote_internal">vote_internal</a>(voter, stake_pool, proposal_id, voting_power, should_pass);
}
</code></pre>



</details>

<a id="0x1_supra_governance_vote_internal"></a>

## Function `vote_internal`

Vote on proposal with <code>proposal_id</code> and specified voting_power from <code>stake_pool</code>.
If voting_power is more than all the left voting power of <code>stake_pool</code>, use all the left voting power.
If a stake pool has already voted on a proposal before partial governance voting is enabled, the stake pool
cannot vote on the proposal even after partial governance voting is enabled.


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_vote_internal">vote_internal</a>(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_vote_internal">vote_internal</a>(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    stake_pool: <b>address</b>,
    proposal_id: u64,
    voting_power: u64,
    should_pass: bool,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a>, <a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a> {
    <b>let</b> voter_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);
    <b>assert</b>!(<a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(stake_pool) == voter_address, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>));

    // The voter's <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be locked up at least <b>as</b> long <b>as</b> the proposal's expiration.
    <b>let</b> proposal_expiration = <a href="voting.md#0x1_voting_get_proposal_expiration_secs">voting::get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(
        @supra_framework,
        proposal_id
    );
    <b>assert</b>!(
        <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(stake_pool) &gt;= proposal_expiration,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EINSUFFICIENT_STAKE_LOCKUP">EINSUFFICIENT_STAKE_LOCKUP</a>),
    );

    // If a <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> already voted on a proposal before partial governance <a href="voting.md#0x1_voting">voting</a> is enabled,
    // `get_remaining_voting_power` returns 0.
    <b>let</b> staking_pool_voting_power = <a href="supra_governance.md#0x1_supra_governance_get_remaining_voting_power">get_remaining_voting_power</a>(stake_pool, proposal_id);
    voting_power = <b>min</b>(voting_power, staking_pool_voting_power);

    // Short-circuit <b>if</b> the voter <b>has</b> no <a href="voting.md#0x1_voting">voting</a> power.
    <b>assert</b>!(voting_power &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_ENO_VOTING_POWER">ENO_VOTING_POWER</a>));

    <a href="voting.md#0x1_voting_vote">voting::vote</a>&lt;GovernanceProposal&gt;(
        &<a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">governance_proposal::create_empty_proposal</a>(),
        @supra_framework,
        proposal_id,
        voting_power,
        should_pass,
    );

    <b>let</b> record_key = <a href="supra_governance.md#0x1_supra_governance_RecordKey">RecordKey</a> {
        stake_pool,
        proposal_id,
    };
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>()) {
        <b>let</b> voting_records_v2 = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@supra_framework);
        <b>let</b> used_voting_power = <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(&<b>mut</b> voting_records_v2.votes, record_key, 0);
        // This calculation should never overflow because the used <a href="voting.md#0x1_voting">voting</a> cannot exceed the total <a href="voting.md#0x1_voting">voting</a> power of this <a href="stake.md#0x1_stake">stake</a> pool.
        *used_voting_power = *used_voting_power + voting_power;
    } <b>else</b> {
        <b>let</b> voting_records = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_VotingRecords">VotingRecords</a>&gt;(@supra_framework);
        <b>assert</b>!(
            !<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&voting_records.votes, record_key),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EALREADY_VOTED">EALREADY_VOTED</a>));
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> voting_records.votes, record_key, <b>true</b>);
    };

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="supra_governance.md#0x1_supra_governance_Vote">Vote</a> {
                proposal_id,
                voter: voter_address,
                stake_pool,
                num_votes: voting_power,
                should_pass,
            },
        );
    };
    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceEvents">GovernanceEvents</a>&gt;(@supra_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="supra_governance.md#0x1_supra_governance_VoteEvent">VoteEvent</a>&gt;(
        &<b>mut</b> events.vote_events,
        <a href="supra_governance.md#0x1_supra_governance_VoteEvent">VoteEvent</a> {
            proposal_id,
            voter: voter_address,
            stake_pool,
            num_votes: voting_power,
            should_pass,
        },
    );

    <b>let</b> proposal_state = <a href="voting.md#0x1_voting_get_proposal_state">voting::get_proposal_state</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);
    <b>if</b> (proposal_state == <a href="supra_governance.md#0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>) {
        <a href="supra_governance.md#0x1_supra_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id);
    }
}
</code></pre>



</details>

<a id="0x1_supra_governance_supra_vote"></a>

## Function `supra_vote`

Vote on proposal with <code>proposal_id</code> and all voting power from <code>stake_pool</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_vote">supra_vote</a>(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, proposal_id: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_vote">supra_vote</a>(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    proposal_id: u64,
    should_pass: bool,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a> {
    <a href="supra_governance.md#0x1_supra_governance_supra_vote_internal">supra_vote_internal</a>(voter, proposal_id, should_pass);
}
</code></pre>



</details>

<a id="0x1_supra_governance_supra_vote_internal"></a>

## Function `supra_vote_internal`

Vote on proposal with <code>proposal_id</code> and specified voting_power from <code>stake_pool</code>.
If voting_power is more than all the left voting power of <code>stake_pool</code>, use all the left voting power.
If a stake pool has already voted on a proposal before partial governance voting is enabled, the stake pool
cannot vote on the proposal even after partial governance voting is enabled.


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_vote_internal">supra_vote_internal</a>(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, proposal_id: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_vote_internal">supra_vote_internal</a>(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    proposal_id: u64,
    should_pass: bool,
) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a>, <a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a> {
    <b>let</b> voter_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);

    <b>let</b> supra_governance_config = <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceConfig">SupraGovernanceConfig</a>&gt;(@supra_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&supra_governance_config.voters, &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="supra_governance.md#0x1_supra_governance_EACCOUNT_NOT_AUTHORIZED">EACCOUNT_NOT_AUTHORIZED</a>));

    // The voter's <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be locked up at least <b>as</b> long <b>as</b> the proposal's expiration.
    <b>let</b> proposal_expiration = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_expiration_secs">multisig_voting::get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(
        @supra_framework,
        proposal_id
    );
    <b>assert</b>!(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;= proposal_expiration, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_IS_EXPIRE">EPROPOSAL_IS_EXPIRE</a>));

    <a href="multisig_voting.md#0x1_multisig_voting_vote">multisig_voting::vote</a>&lt;GovernanceProposal&gt;(
        voter,
        &<a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">governance_proposal::create_empty_proposal</a>(),
        @supra_framework,
        proposal_id,
        should_pass,
    );

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="supra_governance.md#0x1_supra_governance_SupraVote">SupraVote</a> {
                proposal_id,
                voter: voter_address,
                should_pass,
            },
        );
    };
    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraGovernanceEvents">SupraGovernanceEvents</a>&gt;(@supra_framework);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="supra_governance.md#0x1_supra_governance_SupraVoteEvent">SupraVoteEvent</a>&gt;(
        &<b>mut</b> events.vote_events,
        <a href="supra_governance.md#0x1_supra_governance_SupraVoteEvent">SupraVoteEvent</a> {
            proposal_id,
            voter: voter_address,
            should_pass,
        },
    );

    <b>let</b> proposal_state = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_state">multisig_voting::get_proposal_state</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);
    <b>if</b> (proposal_state == <a href="supra_governance.md#0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>) {
        <a href="supra_governance.md#0x1_supra_governance_add_supra_approved_script_hash">add_supra_approved_script_hash</a>(proposal_id);
    }
}
</code></pre>



</details>

<a id="0x1_supra_governance_add_supra_approved_script_hash_script"></a>

## Function `add_supra_approved_script_hash_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_add_supra_approved_script_hash_script">add_supra_approved_script_hash_script</a>(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_add_supra_approved_script_hash_script">add_supra_approved_script_hash_script</a>(proposal_id: u64) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
    <a href="supra_governance.md#0x1_supra_governance_add_supra_approved_script_hash">add_supra_approved_script_hash</a>(proposal_id)
}
</code></pre>



</details>

<a id="0x1_supra_governance_add_approved_script_hash"></a>

## Function `add_approved_script_hash`

Add the execution script hash of a successful governance proposal to the approved list.
This is needed to bypass the mempool transaction size limit for approved governance proposal transactions that
are too large (e.g. module upgrades).


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id: u64) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
    <b>let</b> approved_hashes = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@supra_framework);

    // Ensure the proposal can be resolved.
    <b>let</b> proposal_state = <a href="voting.md#0x1_voting_get_proposal_state">voting::get_proposal_state</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);
    <b>assert</b>!(proposal_state == <a href="supra_governance.md#0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_NOT_RESOLVABLE_YET">EPROPOSAL_NOT_RESOLVABLE_YET</a>));

    <b>let</b> execution_hash = <a href="voting.md#0x1_voting_get_execution_hash">voting::get_execution_hash</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);

    // If this is a multi-step proposal, the proposal id will already exist in the <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.
    // We will <b>update</b> execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> in <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> <b>to</b> be the next_execution_hash.
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&approved_hashes.hashes, &proposal_id)) {
        <b>let</b> current_execution_hash = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> approved_hashes.hashes, &proposal_id);
        *current_execution_hash = execution_hash;
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> approved_hashes.hashes, proposal_id, execution_hash);
    }
}
</code></pre>



</details>

<a id="0x1_supra_governance_add_supra_approved_script_hash"></a>

## Function `add_supra_approved_script_hash`

Add the execution script hash of a successful governance proposal to the approved list.
This is needed to bypass the mempool transaction size limit for approved governance proposal transactions that
are too large (e.g. module upgrades).


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_add_supra_approved_script_hash">add_supra_approved_script_hash</a>(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_add_supra_approved_script_hash">add_supra_approved_script_hash</a>(proposal_id: u64) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
    <b>let</b> approved_hashes = <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@supra_framework);

    // Ensure the proposal can be resolved.
    <b>let</b> proposal_state = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_state">multisig_voting::get_proposal_state</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);
    <b>assert</b>!(proposal_state == <a href="supra_governance.md#0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_NOT_RESOLVABLE_YET">EPROPOSAL_NOT_RESOLVABLE_YET</a>));

    <b>let</b> execution_hash = <a href="multisig_voting.md#0x1_multisig_voting_get_execution_hash">multisig_voting::get_execution_hash</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);

    // If this is a multi-step proposal, the proposal id will already exist in the <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.
    // We will <b>update</b> execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> in <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> <b>to</b> be the next_execution_hash.
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&approved_hashes.hashes, &proposal_id)) {
        <b>let</b> current_execution_hash = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> approved_hashes.hashes, &proposal_id);
        *current_execution_hash = execution_hash;
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> approved_hashes.hashes, proposal_id, execution_hash);
    }
}
</code></pre>



</details>

<a id="0x1_supra_governance_resolve"></a>

## Function `resolve`

Resolve a successful single-step proposal. This would fail if the proposal is not successful (not enough votes or more no
than yes).


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_resolve">resolve</a>(proposal_id: u64, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_resolve">resolve</a>(
    proposal_id: u64,
    signer_address: <b>address</b>
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> {
    <a href="voting.md#0x1_voting_resolve">voting::resolve</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);
    <a href="supra_governance.md#0x1_supra_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id);
    <a href="supra_governance.md#0x1_supra_governance_get_signer">get_signer</a>(signer_address)
}
</code></pre>



</details>

<a id="0x1_supra_governance_supra_resolve"></a>

## Function `supra_resolve`

Resolve a successful single-step proposal. This would fail if the proposal is not successful (not enough votes or more no
than yes).


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_resolve">supra_resolve</a>(proposal_id: u64, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_supra_resolve">supra_resolve</a>(
    proposal_id: u64,
    signer_address: <b>address</b>
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> {
    <a href="multisig_voting.md#0x1_multisig_voting_resolve">multisig_voting::resolve</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id);
    <a href="supra_governance.md#0x1_supra_governance_remove_supra_approved_hash">remove_supra_approved_hash</a>(proposal_id);
    <a href="supra_governance.md#0x1_supra_governance_get_signer">get_signer</a>(signer_address)
}
</code></pre>



</details>

<a id="0x1_supra_governance_resolve_multi_step_proposal"></a>

## Function `resolve_multi_step_proposal`

Resolve a successful multi-step proposal. This would fail if the proposal is not successful.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_resolve_multi_step_proposal">resolve_multi_step_proposal</a>(proposal_id: u64, signer_address: <b>address</b>, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_resolve_multi_step_proposal">resolve_multi_step_proposal</a>(
    proposal_id: u64,
    signer_address: <b>address</b>,
    next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a>, <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
    <a href="voting.md#0x1_voting_resolve_proposal_v2">voting::resolve_proposal_v2</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id, next_execution_hash);
    // If the current step is the last step of this multi-step proposal,
    // we will remove the execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> from the <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&next_execution_hash) == 0) {
        <a href="supra_governance.md#0x1_supra_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id);
    } <b>else</b> {
        // If the current step is not the last step of this proposal,
        // we replace the current execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> <b>with</b> the next execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
        // in the <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.
        <a href="supra_governance.md#0x1_supra_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id)
    };
    <a href="supra_governance.md#0x1_supra_governance_get_signer">get_signer</a>(signer_address)
}
</code></pre>



</details>

<a id="0x1_supra_governance_resolve_supra_multi_step_proposal"></a>

## Function `resolve_supra_multi_step_proposal`

Resolve a successful multi-step proposal. This would fail if the proposal is not successful.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_resolve_supra_multi_step_proposal">resolve_supra_multi_step_proposal</a>(proposal_id: u64, signer_address: <b>address</b>, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_resolve_supra_multi_step_proposal">resolve_supra_multi_step_proposal</a>(
    proposal_id: u64,
    signer_address: <b>address</b>,
    next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a>, <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
    <a href="multisig_voting.md#0x1_multisig_voting_resolve_proposal_v2">multisig_voting::resolve_proposal_v2</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id, next_execution_hash);
    // If the current step is the last step of this multi-step proposal,
    // we will remove the execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> from the <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&next_execution_hash) == 0) {
        <a href="supra_governance.md#0x1_supra_governance_remove_supra_approved_hash">remove_supra_approved_hash</a>(proposal_id);
    } <b>else</b> {
        // If the current step is not the last step of this proposal,
        // we replace the current execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> <b>with</b> the next execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
        // in the <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.
        <a href="supra_governance.md#0x1_supra_governance_add_supra_approved_script_hash">add_supra_approved_script_hash</a>(proposal_id)
    };
    <a href="supra_governance.md#0x1_supra_governance_get_signer">get_signer</a>(signer_address)
}
</code></pre>



</details>

<a id="0x1_supra_governance_remove_approved_hash"></a>

## Function `remove_approved_hash`

Remove an approved proposal's execution script hash.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id: u64) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
    <b>assert</b>!(
        <a href="voting.md#0x1_voting_is_resolved">voting::is_resolved</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_NOT_RESOLVED_YET">EPROPOSAL_NOT_RESOLVED_YET</a>),
    );

    <b>let</b> approved_hashes = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@supra_framework).hashes;
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(approved_hashes, &proposal_id)) {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(approved_hashes, &proposal_id);
    };
}
</code></pre>



</details>

<a id="0x1_supra_governance_remove_supra_approved_hash"></a>

## Function `remove_supra_approved_hash`

Remove an approved proposal's execution script hash.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_remove_supra_approved_hash">remove_supra_approved_hash</a>(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_remove_supra_approved_hash">remove_supra_approved_hash</a>(proposal_id: u64) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> {
    <b>assert</b>!(
        <a href="multisig_voting.md#0x1_multisig_voting_is_resolved">multisig_voting::is_resolved</a>&lt;GovernanceProposal&gt;(@supra_framework, proposal_id),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EPROPOSAL_NOT_RESOLVED_YET">EPROPOSAL_NOT_RESOLVED_YET</a>),
    );

    <b>let</b> approved_hashes = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="supra_governance.md#0x1_supra_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@supra_framework).hashes;
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(approved_hashes, &proposal_id)) {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(approved_hashes, &proposal_id);
    };
}
</code></pre>



</details>

<a id="0x1_supra_governance_reconfigure"></a>

## Function `reconfigure`

Manually reconfigure. Called at the end of a governance txn that alters on-chain configs.

WARNING: this function always ensures a reconfiguration starts, but when the reconfiguration finishes depends.
- If feature <code>RECONFIGURE_WITH_DKG</code> is disabled, it finishes immediately.
- At the end of the calling transaction, we will be in a new epoch.
- If feature <code>RECONFIGURE_WITH_DKG</code> is enabled, it starts DKG, and the new epoch will start in a block prologue after DKG finishes.

This behavior affects when an update of an on-chain config (e.g. <code>ConsensusConfig</code>, <code>Features</code>) takes effect,
since such updates are applied whenever we enter an new epoch.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_reconfigure">reconfigure</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_reconfigure">reconfigure</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>if</b> (<a href="consensus_config.md#0x1_consensus_config_validator_txn_enabled">consensus_config::validator_txn_enabled</a>() && <a href="randomness_config.md#0x1_randomness_config_enabled">randomness_config::enabled</a>()) {
        <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">reconfiguration_with_dkg::try_start</a>();
    } <b>else</b> {
        <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">reconfiguration_with_dkg::finish</a>(supra_framework);
    }
}
</code></pre>



</details>

<a id="0x1_supra_governance_force_end_epoch"></a>

## Function `force_end_epoch`

Change epoch immediately.
If <code>RECONFIGURE_WITH_DKG</code> is enabled and we are in the middle of a DKG,
stop waiting for DKG and enter the new epoch without randomness.

WARNING: currently only used by tests. In most cases you should use <code><a href="supra_governance.md#0x1_supra_governance_reconfigure">reconfigure</a>()</code> instead.
TODO: migrate these tests to be aware of async reconfiguration.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_force_end_epoch">force_end_epoch</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_force_end_epoch">force_end_epoch</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">reconfiguration_with_dkg::finish</a>(supra_framework);
}
</code></pre>



</details>

<a id="0x1_supra_governance_force_end_epoch_test_only"></a>

## Function `force_end_epoch_test_only`

<code><a href="supra_governance.md#0x1_supra_governance_force_end_epoch">force_end_epoch</a>()</code> equivalent but only called in testnet,
where the core resources account exists and has been granted power to mint Aptos coins.


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_force_end_epoch_test_only">force_end_epoch_test_only</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_force_end_epoch_test_only">force_end_epoch_test_only</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> {
    <b>let</b> core_signer = <a href="supra_governance.md#0x1_supra_governance_get_signer_testnet_only">get_signer_testnet_only</a>(supra_framework, @0x1);
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(&core_signer);
    <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">reconfiguration_with_dkg::finish</a>(&core_signer);
}
</code></pre>



</details>

<a id="0x1_supra_governance_toggle_features"></a>

## Function `toggle_features`

Update feature flags and also trigger reconfiguration.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_toggle_features">toggle_features</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_toggle_features">toggle_features</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_change_feature_flags_for_next_epoch">features::change_feature_flags_for_next_epoch</a>(supra_framework, enable, disable);
    <a href="supra_governance.md#0x1_supra_governance_reconfigure">reconfigure</a>(supra_framework);
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_signer_testnet_only"></a>

## Function `get_signer_testnet_only`

Only called in testnet where the core resources account exists and has been granted power to mint Aptos coins.


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_signer_testnet_only">get_signer_testnet_only</a>(core_resources: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_signer_testnet_only">get_signer_testnet_only</a>(
    core_resources: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_core_resource">system_addresses::assert_core_resource</a>(core_resources);
    // Core resources <a href="account.md#0x1_account">account</a> only <b>has</b> mint <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> in tests/testnets.
    <b>assert</b>!(<a href="supra_coin.md#0x1_supra_coin_has_mint_capability">supra_coin::has_mint_capability</a>(core_resources), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="supra_governance.md#0x1_supra_governance_EUNAUTHORIZED">EUNAUTHORIZED</a>));
    <a href="supra_governance.md#0x1_supra_governance_get_signer">get_signer</a>(signer_address)
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_voting_power"></a>

## Function `get_voting_power`

Return the voting power a stake pool has with respect to governance proposals.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_voting_power">get_voting_power</a>(pool_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_voting_power">get_voting_power</a>(pool_address: <b>address</b>): u64 {
    <b>let</b> allow_validator_set_change = <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(&<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());
    <b>if</b> (allow_validator_set_change) {
        <b>let</b> (active, _, pending_active, pending_inactive) = <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);
        // We calculate the <a href="voting.md#0x1_voting">voting</a> power <b>as</b> total non-inactive stakes of the pool. Even <b>if</b> the validator is not in the
        // active validator set, <b>as</b> long <b>as</b> they have a lockup (separately checked in create_proposal and <a href="voting.md#0x1_voting">voting</a>), their
        // <a href="stake.md#0x1_stake">stake</a> would still count in their <a href="voting.md#0x1_voting">voting</a> power for governance proposals.
        active + pending_active + pending_inactive
    } <b>else</b> {
        <a href="stake.md#0x1_stake_get_current_epoch_voting_power">stake::get_current_epoch_voting_power</a>(pool_address)
    }
}
</code></pre>



</details>

<a id="0x1_supra_governance_get_signer"></a>

## Function `get_signer`

Return a signer for making changes to 0x1 as part of on-chain governance proposal process.


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_signer">get_signer</a>(signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_get_signer">get_signer</a>(signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a> {
    <b>let</b> governance_responsibility = <b>borrow_global</b>&lt;<a href="supra_governance.md#0x1_supra_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@supra_framework);
    <b>let</b> signer_cap = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&governance_responsibility.signer_caps, &signer_address);
    create_signer_with_capability(signer_cap)
}
</code></pre>



</details>

<a id="0x1_supra_governance_create_proposal_metadata"></a>

## Function `create_proposal_metadata`



<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal_metadata">create_proposal_metadata</a>(metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_create_proposal_metadata">create_proposal_metadata</a>(
    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&utf8(metadata_location)) &lt;= 256, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EMETADATA_LOCATION_TOO_LONG">EMETADATA_LOCATION_TOO_LONG</a>));
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&utf8(metadata_hash)) &lt;= 256, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="supra_governance.md#0x1_supra_governance_EMETADATA_HASH_TOO_LONG">EMETADATA_HASH_TOO_LONG</a>));

    <b>let</b> metadata = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> metadata, utf8(<a href="supra_governance.md#0x1_supra_governance_METADATA_LOCATION_KEY">METADATA_LOCATION_KEY</a>), metadata_location);
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> metadata, utf8(<a href="supra_governance.md#0x1_supra_governance_METADATA_HASH_KEY">METADATA_HASH_KEY</a>), metadata_hash);
    metadata
}
</code></pre>



</details>

<a id="0x1_supra_governance_assert_voting_initialization"></a>

## Function `assert_voting_initialization`



<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_assert_voting_initialization">assert_voting_initialization</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="supra_governance.md#0x1_supra_governance_assert_voting_initialization">assert_voting_initialization</a>() {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>()) {
        <b>assert</b>!(<b>exists</b>&lt;<a href="supra_governance.md#0x1_supra_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@supra_framework), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="supra_governance.md#0x1_supra_governance_EPARTIAL_VOTING_NOT_INITIALIZED">EPARTIAL_VOTING_NOT_INITIALIZED</a>));
    };
}
</code></pre>



</details>

<a id="0x1_supra_governance_initialize_for_verification"></a>

## Function `initialize_for_verification`



<pre><code>#[verify_only]
<b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_initialize_for_verification">initialize_for_verification</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, voting_duration_secs: u64, supra_min_voting_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="supra_governance.md#0x1_supra_governance_initialize_for_verification">initialize_for_verification</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    voting_duration_secs: u64,
    supra_min_voting_threshold: u64,
    voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
) {
    // <a href="supra_governance.md#0x1_supra_governance_old_initialize">old_initialize</a>(supra_framework, min_voting_threshold, required_proposer_stake, voting_duration_secs);
    <a href="supra_governance.md#0x1_supra_governance_initialize">initialize</a>(supra_framework, voting_duration_secs, supra_min_voting_threshold, voters);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
