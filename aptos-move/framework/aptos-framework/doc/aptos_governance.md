
<a id="0x1_aptos_governance"></a>

# Module `0x1::aptos_governance`


AptosGovernance represents the on&#45;chain governance of the Aptos network. Voting power is calculated based on the
current epoch&apos;s voting power of the proposer or voter&apos;s backing stake pool. In addition, for it to count,
the stake pool&apos;s lockup needs to be at least as long as the proposal&apos;s duration.

It provides the following flow:
1. Proposers can create a proposal by calling AptosGovernance::create_proposal. The proposer&apos;s backing stake pool
needs to have the minimum proposer stake required. Off&#45;chain components can subscribe to CreateProposalEvent to
track proposal creation and proposal ids.
2. Voters can vote on a proposal. Their voting power is derived from the backing stake pool. A stake pool can vote
on a proposal multiple times as long as the total voting power of these votes doesn&apos;t exceed its total voting power.


-  [Resource `GovernanceResponsbility`](#0x1_aptos_governance_GovernanceResponsbility)
-  [Resource `GovernanceConfig`](#0x1_aptos_governance_GovernanceConfig)
-  [Struct `RecordKey`](#0x1_aptos_governance_RecordKey)
-  [Resource `VotingRecords`](#0x1_aptos_governance_VotingRecords)
-  [Resource `VotingRecordsV2`](#0x1_aptos_governance_VotingRecordsV2)
-  [Resource `ApprovedExecutionHashes`](#0x1_aptos_governance_ApprovedExecutionHashes)
-  [Resource `GovernanceEvents`](#0x1_aptos_governance_GovernanceEvents)
-  [Struct `CreateProposalEvent`](#0x1_aptos_governance_CreateProposalEvent)
-  [Struct `VoteEvent`](#0x1_aptos_governance_VoteEvent)
-  [Struct `UpdateConfigEvent`](#0x1_aptos_governance_UpdateConfigEvent)
-  [Struct `CreateProposal`](#0x1_aptos_governance_CreateProposal)
-  [Struct `Vote`](#0x1_aptos_governance_Vote)
-  [Struct `UpdateConfig`](#0x1_aptos_governance_UpdateConfig)
-  [Constants](#@Constants_0)
-  [Function `store_signer_cap`](#0x1_aptos_governance_store_signer_cap)
-  [Function `initialize`](#0x1_aptos_governance_initialize)
-  [Function `update_governance_config`](#0x1_aptos_governance_update_governance_config)
-  [Function `initialize_partial_voting`](#0x1_aptos_governance_initialize_partial_voting)
-  [Function `get_voting_duration_secs`](#0x1_aptos_governance_get_voting_duration_secs)
-  [Function `get_min_voting_threshold`](#0x1_aptos_governance_get_min_voting_threshold)
-  [Function `get_required_proposer_stake`](#0x1_aptos_governance_get_required_proposer_stake)
-  [Function `has_entirely_voted`](#0x1_aptos_governance_has_entirely_voted)
-  [Function `get_remaining_voting_power`](#0x1_aptos_governance_get_remaining_voting_power)
-  [Function `create_proposal`](#0x1_aptos_governance_create_proposal)
-  [Function `create_proposal_v2`](#0x1_aptos_governance_create_proposal_v2)
-  [Function `create_proposal_v2_impl`](#0x1_aptos_governance_create_proposal_v2_impl)
-  [Function `vote`](#0x1_aptos_governance_vote)
-  [Function `partial_vote`](#0x1_aptos_governance_partial_vote)
-  [Function `vote_internal`](#0x1_aptos_governance_vote_internal)
-  [Function `add_approved_script_hash_script`](#0x1_aptos_governance_add_approved_script_hash_script)
-  [Function `add_approved_script_hash`](#0x1_aptos_governance_add_approved_script_hash)
-  [Function `resolve`](#0x1_aptos_governance_resolve)
-  [Function `resolve_multi_step_proposal`](#0x1_aptos_governance_resolve_multi_step_proposal)
-  [Function `remove_approved_hash`](#0x1_aptos_governance_remove_approved_hash)
-  [Function `reconfigure`](#0x1_aptos_governance_reconfigure)
-  [Function `force_end_epoch`](#0x1_aptos_governance_force_end_epoch)
-  [Function `force_end_epoch_test_only`](#0x1_aptos_governance_force_end_epoch_test_only)
-  [Function `toggle_features`](#0x1_aptos_governance_toggle_features)
-  [Function `get_signer_testnet_only`](#0x1_aptos_governance_get_signer_testnet_only)
-  [Function `get_voting_power`](#0x1_aptos_governance_get_voting_power)
-  [Function `get_signer`](#0x1_aptos_governance_get_signer)
-  [Function `create_proposal_metadata`](#0x1_aptos_governance_create_proposal_metadata)
-  [Function `assert_voting_initialization`](#0x1_aptos_governance_assert_voting_initialization)
-  [Function `initialize_for_verification`](#0x1_aptos_governance_initialize_for_verification)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `store_signer_cap`](#@Specification_1_store_signer_cap)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `update_governance_config`](#@Specification_1_update_governance_config)
    -  [Function `initialize_partial_voting`](#@Specification_1_initialize_partial_voting)
    -  [Function `get_voting_duration_secs`](#@Specification_1_get_voting_duration_secs)
    -  [Function `get_min_voting_threshold`](#@Specification_1_get_min_voting_threshold)
    -  [Function `get_required_proposer_stake`](#@Specification_1_get_required_proposer_stake)
    -  [Function `has_entirely_voted`](#@Specification_1_has_entirely_voted)
    -  [Function `get_remaining_voting_power`](#@Specification_1_get_remaining_voting_power)
    -  [Function `create_proposal`](#@Specification_1_create_proposal)
    -  [Function `create_proposal_v2`](#@Specification_1_create_proposal_v2)
    -  [Function `create_proposal_v2_impl`](#@Specification_1_create_proposal_v2_impl)
    -  [Function `vote`](#@Specification_1_vote)
    -  [Function `partial_vote`](#@Specification_1_partial_vote)
    -  [Function `vote_internal`](#@Specification_1_vote_internal)
    -  [Function `add_approved_script_hash_script`](#@Specification_1_add_approved_script_hash_script)
    -  [Function `add_approved_script_hash`](#@Specification_1_add_approved_script_hash)
    -  [Function `resolve`](#@Specification_1_resolve)
    -  [Function `resolve_multi_step_proposal`](#@Specification_1_resolve_multi_step_proposal)
    -  [Function `remove_approved_hash`](#@Specification_1_remove_approved_hash)
    -  [Function `reconfigure`](#@Specification_1_reconfigure)
    -  [Function `force_end_epoch`](#@Specification_1_force_end_epoch)
    -  [Function `force_end_epoch_test_only`](#@Specification_1_force_end_epoch_test_only)
    -  [Function `toggle_features`](#@Specification_1_toggle_features)
    -  [Function `get_signer_testnet_only`](#@Specification_1_get_signer_testnet_only)
    -  [Function `get_voting_power`](#@Specification_1_get_voting_power)
    -  [Function `get_signer`](#@Specification_1_get_signer)
    -  [Function `create_proposal_metadata`](#@Specification_1_create_proposal_metadata)
    -  [Function `assert_voting_initialization`](#@Specification_1_assert_voting_initialization)
    -  [Function `initialize_for_verification`](#@Specification_1_initialize_for_verification)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;<br /><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;<br /><b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="governance_proposal.md#0x1_governance_proposal">0x1::governance_proposal</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="randomness_config.md#0x1_randomness_config">0x1::randomness_config</a>;<br /><b>use</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg">0x1::reconfiguration_with_dkg</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="voting.md#0x1_voting">0x1::voting</a>;<br /></code></pre>



<a id="0x1_aptos_governance_GovernanceResponsbility"></a>

## Resource `GovernanceResponsbility`

Store the SignerCapabilities of accounts under the on&#45;chain governance&apos;s control.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a> <b>has</b> key<br /></code></pre>



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

<a id="0x1_aptos_governance_GovernanceConfig"></a>

## Resource `GovernanceConfig`

Configurations of the AptosGovernance, set during Genesis and can be updated by the same process offered
by this AptosGovernance module.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a> <b>has</b> key<br /></code></pre>



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

<a id="0x1_aptos_governance_RecordKey"></a>

## Struct `RecordKey`



<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



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

<a id="0x1_aptos_governance_VotingRecords"></a>

## Resource `VotingRecords`

Records to track the proposals each stake pool has been used to vote on.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_RecordKey">aptos_governance::RecordKey</a>, bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_VotingRecordsV2"></a>

## Resource `VotingRecordsV2`

Records to track the voting power usage of each stake pool on each proposal.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_RecordKey">aptos_governance::RecordKey</a>, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_ApprovedExecutionHashes"></a>

## Resource `ApprovedExecutionHashes`

Used to track which execution script hashes have been approved by governance.
This is required to bypass cases where the execution scripts exceed the size limit imposed by mempool.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> <b>has</b> key<br /></code></pre>



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

<a id="0x1_aptos_governance_GovernanceEvents"></a>

## Resource `GovernanceEvents`

Events generated by interactions with the AptosGovernance module.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_CreateProposalEvent">aptos_governance::CreateProposalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_config_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_UpdateConfigEvent">aptos_governance::UpdateConfigEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VoteEvent">aptos_governance::VoteEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_CreateProposalEvent"></a>

## Struct `CreateProposalEvent`

Event emitted when a proposal is created.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_CreateProposalEvent">CreateProposalEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_aptos_governance_VoteEvent"></a>

## Struct `VoteEvent`

Event emitted when there&apos;s a vote on a proposa;


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_VoteEvent">VoteEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_aptos_governance_UpdateConfigEvent"></a>

## Struct `UpdateConfigEvent`

Event emitted when the governance configs are updated.


<pre><code><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_UpdateConfigEvent">UpdateConfigEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_aptos_governance_CreateProposal"></a>

## Struct `CreateProposal`

Event emitted when a proposal is created.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_CreateProposal">CreateProposal</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_aptos_governance_Vote"></a>

## Struct `Vote`

Event emitted when there&apos;s a vote on a proposa;


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_Vote">Vote</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_aptos_governance_UpdateConfig"></a>

## Struct `UpdateConfig`

Event emitted when the governance configs are updated.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="aptos_governance.md#0x1_aptos_governance_UpdateConfig">UpdateConfig</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aptos_governance_MAX_U64"></a>



<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_MAX_U64">MAX_U64</a>: u64 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_aptos_governance_PROPOSAL_STATE_SUCCEEDED"></a>

This matches the same enum const in voting. We have to duplicate it as Move doesn&apos;t have support for enums yet.


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_aptos_governance_EALREADY_VOTED"></a>

The specified stake pool has already been used to vote on the same proposal


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EALREADY_VOTED">EALREADY_VOTED</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_aptos_governance_EINSUFFICIENT_PROPOSER_STAKE"></a>

The specified stake pool does not have sufficient stake to create a proposal


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_aptos_governance_EINSUFFICIENT_STAKE_LOCKUP"></a>

The specified stake pool does not have long enough remaining lockup to create a proposal or vote


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EINSUFFICIENT_STAKE_LOCKUP">EINSUFFICIENT_STAKE_LOCKUP</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_aptos_governance_EMETADATA_HASH_TOO_LONG"></a>

Metadata hash cannot be longer than 256 chars


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EMETADATA_HASH_TOO_LONG">EMETADATA_HASH_TOO_LONG</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_aptos_governance_EMETADATA_LOCATION_TOO_LONG"></a>

Metadata location cannot be longer than 256 chars


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EMETADATA_LOCATION_TOO_LONG">EMETADATA_LOCATION_TOO_LONG</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_aptos_governance_ENOT_DELEGATED_VOTER"></a>

This account is not the designated voter of the specified stake pool


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_aptos_governance_ENOT_PARTIAL_VOTING_PROPOSAL"></a>

The proposal in the argument is not a partial voting proposal.


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_ENOT_PARTIAL_VOTING_PROPOSAL">ENOT_PARTIAL_VOTING_PROPOSAL</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x1_aptos_governance_ENO_VOTING_POWER"></a>

The specified stake pool must be part of the validator set


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_ENO_VOTING_POWER">ENO_VOTING_POWER</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_aptos_governance_EPARTIAL_VOTING_NOT_INITIALIZED"></a>

Partial voting feature hasn&apos;t been properly initialized.


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EPARTIAL_VOTING_NOT_INITIALIZED">EPARTIAL_VOTING_NOT_INITIALIZED</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x1_aptos_governance_EPROPOSAL_NOT_RESOLVABLE_YET"></a>

Proposal is not ready to be resolved. Waiting on time or votes


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EPROPOSAL_NOT_RESOLVABLE_YET">EPROPOSAL_NOT_RESOLVABLE_YET</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_aptos_governance_EPROPOSAL_NOT_RESOLVED_YET"></a>

The proposal has not been resolved yet


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EPROPOSAL_NOT_RESOLVED_YET">EPROPOSAL_NOT_RESOLVED_YET</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_aptos_governance_EUNAUTHORIZED"></a>

Account is not authorized to call this function.


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EUNAUTHORIZED">EUNAUTHORIZED</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_aptos_governance_EVOTING_POWER_OVERFLOW"></a>

The stake pool is using voting power more than it has.


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_EVOTING_POWER_OVERFLOW">EVOTING_POWER_OVERFLOW</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_aptos_governance_METADATA_HASH_KEY"></a>



<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_METADATA_HASH_KEY">METADATA_HASH_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [109, 101, 116, 97, 100, 97, 116, 97, 95, 104, 97, 115, 104];<br /></code></pre>



<a id="0x1_aptos_governance_METADATA_LOCATION_KEY"></a>

Proposal metadata attribute keys.


<pre><code><b>const</b> <a href="aptos_governance.md#0x1_aptos_governance_METADATA_LOCATION_KEY">METADATA_LOCATION_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [109, 101, 116, 97, 100, 97, 116, 97, 95, 108, 111, 99, 97, 116, 105, 111, 110];<br /></code></pre>



<a id="0x1_aptos_governance_store_signer_cap"></a>

## Function `store_signer_cap`

Can be called during genesis or by the governance itself.
Stores the signer capability for a given address.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_store_signer_cap">store_signer_cap</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>, signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_store_signer_cap">store_signer_cap</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    signer_address: <b>address</b>,<br />    signer_cap: SignerCapability,<br />) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved">system_addresses::assert_framework_reserved</a>(signer_address);<br /><br />    <b>if</b> (!<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework)) &#123;<br />        <b>move_to</b>(<br />            aptos_framework,<br />            <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a> &#123; signer_caps: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, SignerCapability&gt;() &#125;<br />        );<br />    &#125;;<br /><br />    <b>let</b> signer_caps &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework).signer_caps;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(signer_caps, signer_address, signer_cap);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_initialize"></a>

## Function `initialize`

Initializes the state for Aptos Governance. Can only be called during Genesis with a signer
for the aptos_framework (0x1) account.
This function is private because it&apos;s called directly from the vm.


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize">initialize</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    min_voting_threshold: u128,<br />    required_proposer_stake: u64,<br />    voting_duration_secs: u64,<br />) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <a href="voting.md#0x1_voting_register">voting::register</a>&lt;GovernanceProposal&gt;(aptos_framework);<br />    <b>move_to</b>(aptos_framework, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a> &#123;<br />        voting_duration_secs,<br />        min_voting_threshold,<br />        required_proposer_stake,<br />    &#125;);<br />    <b>move_to</b>(aptos_framework, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />        create_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_CreateProposalEvent">CreateProposalEvent</a>&gt;(aptos_framework),<br />        update_config_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_UpdateConfigEvent">UpdateConfigEvent</a>&gt;(aptos_framework),<br />        vote_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VoteEvent">VoteEvent</a>&gt;(aptos_framework),<br />    &#125;);<br />    <b>move_to</b>(aptos_framework, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a> &#123;<br />        votes: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),<br />    &#125;);<br />    <b>move_to</b>(aptos_framework, <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> &#123;<br />        hashes: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(),<br />    &#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_update_governance_config"></a>

## Function `update_governance_config`

Update the governance configurations. This can only be called as part of resolving a proposal in this same
AptosGovernance.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_update_governance_config">update_governance_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_update_governance_config">update_governance_config</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    min_voting_threshold: u128,<br />    required_proposer_stake: u64,<br />    voting_duration_secs: u64,<br />) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <b>let</b> governance_config &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework);<br />    governance_config.voting_duration_secs &#61; voting_duration_secs;<br />    governance_config.min_voting_threshold &#61; min_voting_threshold;<br />    governance_config.required_proposer_stake &#61; required_proposer_stake;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="aptos_governance.md#0x1_aptos_governance_UpdateConfig">UpdateConfig</a> &#123;<br />                min_voting_threshold,<br />                required_proposer_stake,<br />                voting_duration_secs<br />            &#125;,<br />        )<br />    &#125;;<br />    <b>let</b> events &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a>&gt;(@aptos_framework);<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_UpdateConfigEvent">UpdateConfigEvent</a>&gt;(<br />        &amp;<b>mut</b> events.update_config_events,<br />        <a href="aptos_governance.md#0x1_aptos_governance_UpdateConfigEvent">UpdateConfigEvent</a> &#123;<br />            min_voting_threshold,<br />            required_proposer_stake,<br />            voting_duration_secs<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_initialize_partial_voting"></a>

## Function `initialize_partial_voting`

Initializes the state for Aptos Governance partial voting. Can only be called through Aptos governance
proposals with a signer for the aptos_framework (0x1) account.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize_partial_voting">initialize_partial_voting</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize_partial_voting">initialize_partial_voting</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <b>move_to</b>(aptos_framework, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a> &#123;<br />        votes: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>(),<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_get_voting_duration_secs"></a>

## Function `get_voting_duration_secs`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_voting_duration_secs">get_voting_duration_secs</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_voting_duration_secs">get_voting_duration_secs</a>(): u64 <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework).voting_duration_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_get_min_voting_threshold"></a>

## Function `get_min_voting_threshold`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_min_voting_threshold">get_min_voting_threshold</a>(): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_min_voting_threshold">get_min_voting_threshold</a>(): u128 <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework).min_voting_threshold<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_get_required_proposer_stake"></a>

## Function `get_required_proposer_stake`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_required_proposer_stake">get_required_proposer_stake</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_required_proposer_stake">get_required_proposer_stake</a>(): u64 <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework).required_proposer_stake<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_has_entirely_voted"></a>

## Function `has_entirely_voted`

Return true if a stake pool has already voted on a proposal before partial governance voting is enabled.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_has_entirely_voted">has_entirely_voted</a>(stake_pool: <b>address</b>, proposal_id: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_has_entirely_voted">has_entirely_voted</a>(stake_pool: <b>address</b>, proposal_id: u64): bool <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a> &#123;<br />    <b>let</b> record_key &#61; <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a> &#123;<br />        stake_pool,<br />        proposal_id,<br />    &#125;;<br />    // If a <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> already voted on a proposal before partial governance <a href="voting.md#0x1_voting">voting</a> is enabled,<br />    // there is a record in <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>.<br />    <b>let</b> voting_records &#61; <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;voting_records.votes, record_key)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_get_remaining_voting_power"></a>

## Function `get_remaining_voting_power`

Return remaining voting power of a stake pool on a proposal.
Note: a stake pool&apos;s voting power on a proposal could increase over time(e.g. rewards/new stake).


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">get_remaining_voting_power</a>(stake_pool: <b>address</b>, proposal_id: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">get_remaining_voting_power</a>(<br />    stake_pool: <b>address</b>,<br />    proposal_id: u64<br />): u64 <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a> &#123;<br />    <a href="aptos_governance.md#0x1_aptos_governance_assert_voting_initialization">assert_voting_initialization</a>();<br /><br />    <b>let</b> proposal_expiration &#61; <a href="voting.md#0x1_voting_get_proposal_expiration_secs">voting::get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(<br />        @aptos_framework,<br />        proposal_id<br />    );<br />    <b>let</b> lockup_until &#61; <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(stake_pool);<br />    // The voter&apos;s <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be locked up at least <b>as</b> long <b>as</b> the proposal&apos;s expiration.<br />    // Also no one can vote on a expired proposal.<br />    <b>if</b> (proposal_expiration &gt; lockup_until &#124;&#124; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; proposal_expiration) &#123;<br />        <b>return</b> 0<br />    &#125;;<br /><br />    // If a <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> already voted on a proposal before partial governance <a href="voting.md#0x1_voting">voting</a> is enabled, the <a href="stake.md#0x1_stake">stake</a> pool<br />    // cannot vote on the proposal even after partial governance <a href="voting.md#0x1_voting">voting</a> is enabled.<br />    <b>if</b> (<a href="aptos_governance.md#0x1_aptos_governance_has_entirely_voted">has_entirely_voted</a>(stake_pool, proposal_id)) &#123;<br />        <b>return</b> 0<br />    &#125;;<br />    <b>let</b> record_key &#61; <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a> &#123;<br />        stake_pool,<br />        proposal_id,<br />    &#125;;<br />    <b>let</b> used_voting_power &#61; 0u64;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>()) &#123;<br />        <b>let</b> voting_records_v2 &#61; <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br />        used_voting_power &#61; &#42;<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_with_default">smart_table::borrow_with_default</a>(&amp;voting_records_v2.votes, record_key, &amp;0);<br />    &#125;;<br />    <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">get_voting_power</a>(stake_pool) &#45; used_voting_power<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal"></a>

## Function `create_proposal`

Create a single&#45;step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal">create_proposal</a>(proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal">create_proposal</a>(<br />    proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    stake_pool: <b>address</b>,<br />    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />    <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2">create_proposal_v2</a>(proposer, stake_pool, execution_hash, metadata_location, metadata_hash, <b>false</b>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal_v2"></a>

## Function `create_proposal_v2`

Create a single&#45;step or multi&#45;step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2">create_proposal_v2</a>(proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2">create_proposal_v2</a>(<br />    proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    stake_pool: <b>address</b>,<br />    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    is_multi_step_proposal: bool,<br />) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />    <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2_impl">create_proposal_v2_impl</a>(<br />        proposer,<br />        stake_pool,<br />        execution_hash,<br />        metadata_location,<br />        metadata_hash,<br />        is_multi_step_proposal<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal_v2_impl"></a>

## Function `create_proposal_v2_impl`

Create a single&#45;step or multi&#45;step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.
Return proposal_id when a proposal is successfully created.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2_impl">create_proposal_v2_impl</a>(proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2_impl">create_proposal_v2_impl</a>(<br />    proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    stake_pool: <b>address</b>,<br />    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    is_multi_step_proposal: bool,<br />): u64 <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />    <b>let</b> proposer_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(proposer);<br />    <b>assert</b>!(<br />        <a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(stake_pool) &#61;&#61; proposer_address,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>)<br />    );<br /><br />    // The proposer&apos;s <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be at least the required bond amount.<br />    <b>let</b> governance_config &#61; <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework);<br />    <b>let</b> stake_balance &#61; <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">get_voting_power</a>(stake_pool);<br />    <b>assert</b>!(<br />        stake_balance &gt;&#61; governance_config.required_proposer_stake,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EINSUFFICIENT_PROPOSER_STAKE">EINSUFFICIENT_PROPOSER_STAKE</a>),<br />    );<br /><br />    // The proposer&apos;s <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be locked up at least <b>as</b> long <b>as</b> the proposal&apos;s <a href="voting.md#0x1_voting">voting</a> period.<br />    <b>let</b> current_time &#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();<br />    <b>let</b> proposal_expiration &#61; current_time &#43; governance_config.voting_duration_secs;<br />    <b>assert</b>!(<br />        <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(stake_pool) &gt;&#61; proposal_expiration,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EINSUFFICIENT_STAKE_LOCKUP">EINSUFFICIENT_STAKE_LOCKUP</a>),<br />    );<br /><br />    // Create and validate proposal metadata.<br />    <b>let</b> proposal_metadata &#61; <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_metadata">create_proposal_metadata</a>(metadata_location, metadata_hash);<br /><br />    // We want <b>to</b> allow early resolution of proposals <b>if</b> more than 50% of the total supply of the network coins<br />    // <b>has</b> voted. This doesn&apos;t take into subsequent inflation/deflation (rewards are issued every epoch and gas fees<br />    // are burnt after every transaction), but inflation/delation is very unlikely <b>to</b> have a major impact on total<br />    // supply during the <a href="voting.md#0x1_voting">voting</a> period.<br />    <b>let</b> total_voting_token_supply &#61; <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;AptosCoin&gt;();<br />    <b>let</b> early_resolution_vote_threshold &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u128&gt;();<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;total_voting_token_supply)) &#123;<br />        <b>let</b> total_supply &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;total_voting_token_supply);<br />        // 50% &#43; 1 <b>to</b> avoid rounding errors.<br />        early_resolution_vote_threshold &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(total_supply / 2 &#43; 1);<br />    &#125;;<br /><br />    <b>let</b> proposal_id &#61; <a href="voting.md#0x1_voting_create_proposal_v2">voting::create_proposal_v2</a>(<br />        proposer_address,<br />        @aptos_framework,<br />        <a href="governance_proposal.md#0x1_governance_proposal_create_proposal">governance_proposal::create_proposal</a>(),<br />        execution_hash,<br />        governance_config.min_voting_threshold,<br />        proposal_expiration,<br />        early_resolution_vote_threshold,<br />        proposal_metadata,<br />        is_multi_step_proposal,<br />    );<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="aptos_governance.md#0x1_aptos_governance_CreateProposal">CreateProposal</a> &#123;<br />                proposal_id,<br />                proposer: proposer_address,<br />                stake_pool,<br />                execution_hash,<br />                proposal_metadata,<br />            &#125;,<br />        );<br />    &#125;;<br />    <b>let</b> events &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a>&gt;(@aptos_framework);<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_CreateProposalEvent">CreateProposalEvent</a>&gt;(<br />        &amp;<b>mut</b> events.create_proposal_events,<br />        <a href="aptos_governance.md#0x1_aptos_governance_CreateProposalEvent">CreateProposalEvent</a> &#123;<br />            proposal_id,<br />            proposer: proposer_address,<br />            stake_pool,<br />            execution_hash,<br />            proposal_metadata,<br />        &#125;,<br />    );<br />    proposal_id<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_vote"></a>

## Function `vote`

Vote on proposal with <code>proposal_id</code> and all voting power from <code>stake_pool</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_vote">vote</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, should_pass: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_vote">vote</a>(<br />    voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    stake_pool: <b>address</b>,<br />    proposal_id: u64,<br />    should_pass: bool,<br />) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />    <a href="aptos_governance.md#0x1_aptos_governance_vote_internal">vote_internal</a>(voter, stake_pool, proposal_id, <a href="aptos_governance.md#0x1_aptos_governance_MAX_U64">MAX_U64</a>, should_pass);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_partial_vote"></a>

## Function `partial_vote`

Vote on proposal with <code>proposal_id</code> and specified voting power from <code>stake_pool</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_partial_vote">partial_vote</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_partial_vote">partial_vote</a>(<br />    voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    stake_pool: <b>address</b>,<br />    proposal_id: u64,<br />    voting_power: u64,<br />    should_pass: bool,<br />) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />    <a href="aptos_governance.md#0x1_aptos_governance_vote_internal">vote_internal</a>(voter, stake_pool, proposal_id, voting_power, should_pass);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_vote_internal"></a>

## Function `vote_internal`

Vote on proposal with <code>proposal_id</code> and specified voting_power from <code>stake_pool</code>.
If voting_power is more than all the left voting power of <code>stake_pool</code>, use all the left voting power.
If a stake pool has already voted on a proposal before partial governance voting is enabled, the stake pool
cannot vote on the proposal even after partial governance voting is enabled.


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_vote_internal">vote_internal</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_vote_internal">vote_internal</a>(<br />    voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    stake_pool: <b>address</b>,<br />    proposal_id: u64,<br />    voting_power: u64,<br />    should_pass: bool,<br />) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>, <a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a> &#123;<br />    <b>let</b> voter_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);<br />    <b>assert</b>!(<a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(stake_pool) &#61;&#61; voter_address, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>));<br /><br />    // The voter&apos;s <a href="stake.md#0x1_stake">stake</a> needs <b>to</b> be locked up at least <b>as</b> long <b>as</b> the proposal&apos;s expiration.<br />    <b>let</b> proposal_expiration &#61; <a href="voting.md#0x1_voting_get_proposal_expiration_secs">voting::get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(<br />        @aptos_framework,<br />        proposal_id<br />    );<br />    <b>assert</b>!(<br />        <a href="stake.md#0x1_stake_get_lockup_secs">stake::get_lockup_secs</a>(stake_pool) &gt;&#61; proposal_expiration,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EINSUFFICIENT_STAKE_LOCKUP">EINSUFFICIENT_STAKE_LOCKUP</a>),<br />    );<br /><br />    // If a <a href="stake.md#0x1_stake">stake</a> pool <b>has</b> already voted on a proposal before partial governance <a href="voting.md#0x1_voting">voting</a> is enabled,<br />    // `get_remaining_voting_power` returns 0.<br />    <b>let</b> staking_pool_voting_power &#61; <a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">get_remaining_voting_power</a>(stake_pool, proposal_id);<br />    voting_power &#61; <b>min</b>(voting_power, staking_pool_voting_power);<br /><br />    // Short&#45;circuit <b>if</b> the voter <b>has</b> no <a href="voting.md#0x1_voting">voting</a> power.<br />    <b>assert</b>!(voting_power &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_ENO_VOTING_POWER">ENO_VOTING_POWER</a>));<br /><br />    <a href="voting.md#0x1_voting_vote">voting::vote</a>&lt;GovernanceProposal&gt;(<br />        &amp;<a href="governance_proposal.md#0x1_governance_proposal_create_empty_proposal">governance_proposal::create_empty_proposal</a>(),<br />        @aptos_framework,<br />        proposal_id,<br />        voting_power,<br />        should_pass,<br />    );<br /><br />    <b>let</b> record_key &#61; <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a> &#123;<br />        stake_pool,<br />        proposal_id,<br />    &#125;;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>()) &#123;<br />        <b>let</b> voting_records_v2 &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br />        <b>let</b> used_voting_power &#61; <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_mut_with_default">smart_table::borrow_mut_with_default</a>(&amp;<b>mut</b> voting_records_v2.votes, record_key, 0);<br />        // This calculation should never overflow because the used <a href="voting.md#0x1_voting">voting</a> cannot exceed the total <a href="voting.md#0x1_voting">voting</a> power of this <a href="stake.md#0x1_stake">stake</a> pool.<br />        &#42;used_voting_power &#61; &#42;used_voting_power &#43; voting_power;<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> voting_records &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br />        <b>assert</b>!(<br />            !<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;voting_records.votes, record_key),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EALREADY_VOTED">EALREADY_VOTED</a>));<br />        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> voting_records.votes, record_key, <b>true</b>);<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="aptos_governance.md#0x1_aptos_governance_Vote">Vote</a> &#123;<br />                proposal_id,<br />                voter: voter_address,<br />                stake_pool,<br />                num_votes: voting_power,<br />                should_pass,<br />            &#125;,<br />        );<br />    &#125;;<br />    <b>let</b> events &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a>&gt;(@aptos_framework);<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VoteEvent">VoteEvent</a>&gt;(<br />        &amp;<b>mut</b> events.vote_events,<br />        <a href="aptos_governance.md#0x1_aptos_governance_VoteEvent">VoteEvent</a> &#123;<br />            proposal_id,<br />            voter: voter_address,<br />            stake_pool,<br />            num_votes: voting_power,<br />            should_pass,<br />        &#125;,<br />    );<br /><br />    <b>let</b> proposal_state &#61; <a href="voting.md#0x1_voting_get_proposal_state">voting::get_proposal_state</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);<br />    <b>if</b> (proposal_state &#61;&#61; <a href="aptos_governance.md#0x1_aptos_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>) &#123;<br />        <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_add_approved_script_hash_script"></a>

## Function `add_approved_script_hash_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash_script">add_approved_script_hash_script</a>(proposal_id: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash_script">add_approved_script_hash_script</a>(proposal_id: u64) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> &#123;<br />    <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_add_approved_script_hash"></a>

## Function `add_approved_script_hash`

Add the execution script hash of a successful governance proposal to the approved list.
This is needed to bypass the mempool transaction size limit for approved governance proposal transactions that
are too large (e.g. module upgrades).


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id: u64) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> &#123;<br />    <b>let</b> approved_hashes &#61; <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework);<br /><br />    // Ensure the proposal can be resolved.<br />    <b>let</b> proposal_state &#61; <a href="voting.md#0x1_voting_get_proposal_state">voting::get_proposal_state</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);<br />    <b>assert</b>!(proposal_state &#61;&#61; <a href="aptos_governance.md#0x1_aptos_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EPROPOSAL_NOT_RESOLVABLE_YET">EPROPOSAL_NOT_RESOLVABLE_YET</a>));<br /><br />    <b>let</b> execution_hash &#61; <a href="voting.md#0x1_voting_get_execution_hash">voting::get_execution_hash</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);<br /><br />    // If this is a multi&#45;step proposal, the proposal id will already exist in the <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.<br />    // We will <b>update</b> execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> in <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> <b>to</b> be the next_execution_hash.<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;approved_hashes.hashes, &amp;proposal_id)) &#123;<br />        <b>let</b> current_execution_hash &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> approved_hashes.hashes, &amp;proposal_id);<br />        &#42;current_execution_hash &#61; execution_hash;<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> approved_hashes.hashes, proposal_id, execution_hash);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_resolve"></a>

## Function `resolve`

Resolve a successful single&#45;step proposal. This would fail if the proposal is not successful (not enough votes or more no
than yes).


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_resolve">resolve</a>(proposal_id: u64, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_resolve">resolve</a>(<br />    proposal_id: u64,<br />    signer_address: <b>address</b><br />): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>, <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a> &#123;<br />    <a href="voting.md#0x1_voting_resolve">voting::resolve</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);<br />    <a href="aptos_governance.md#0x1_aptos_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id);<br />    <a href="aptos_governance.md#0x1_aptos_governance_get_signer">get_signer</a>(signer_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_resolve_multi_step_proposal"></a>

## Function `resolve_multi_step_proposal`

Resolve a successful multi&#45;step proposal. This would fail if the proposal is not successful.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_resolve_multi_step_proposal">resolve_multi_step_proposal</a>(proposal_id: u64, signer_address: <b>address</b>, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_resolve_multi_step_proposal">resolve_multi_step_proposal</a>(<br />    proposal_id: u64,<br />    signer_address: <b>address</b>,<br />    next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>, <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> &#123;<br />    <a href="voting.md#0x1_voting_resolve_proposal_v2">voting::resolve_proposal_v2</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id, next_execution_hash);<br />    // If the current step is the last step of this multi&#45;step proposal,<br />    // we will remove the execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> from the <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;next_execution_hash) &#61;&#61; 0) &#123;<br />        <a href="aptos_governance.md#0x1_aptos_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id);<br />    &#125; <b>else</b> &#123;<br />        // If the current step is not the last step of this proposal,<br />        // we replace the current execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> <b>with</b> the next execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a><br />        // in the <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> map.<br />        <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id)<br />    &#125;;<br />    <a href="aptos_governance.md#0x1_aptos_governance_get_signer">get_signer</a>(signer_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_remove_approved_hash"></a>

## Function `remove_approved_hash`

Remove an approved proposal&apos;s execution script hash.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id: u64) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a> &#123;<br />    <b>assert</b>!(<br />        <a href="voting.md#0x1_voting_is_resolved">voting::is_resolved</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EPROPOSAL_NOT_RESOLVED_YET">EPROPOSAL_NOT_RESOLVED_YET</a>),<br />    );<br /><br />    <b>let</b> approved_hashes &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework).hashes;<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(approved_hashes, &amp;proposal_id)) &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(approved_hashes, &amp;proposal_id);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_reconfigure"></a>

## Function `reconfigure`

Manually reconfigure. Called at the end of a governance txn that alters on&#45;chain configs.

WARNING: this function always ensures a reconfiguration starts, but when the reconfiguration finishes depends.
&#45; If feature <code>RECONFIGURE_WITH_DKG</code> is disabled, it finishes immediately.
&#45; At the end of the calling transaction, we will be in a new epoch.
&#45; If feature <code>RECONFIGURE_WITH_DKG</code> is enabled, it starts DKG, and the new epoch will start in a block prologue after DKG finishes.

This behavior affects when an update of an on&#45;chain config (e.g. <code>ConsensusConfig</code>, <code>Features</code>) takes effect,
since such updates are applied whenever we enter an new epoch.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_reconfigure">reconfigure</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_reconfigure">reconfigure</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>if</b> (<a href="consensus_config.md#0x1_consensus_config_validator_txn_enabled">consensus_config::validator_txn_enabled</a>() &amp;&amp; <a href="randomness_config.md#0x1_randomness_config_enabled">randomness_config::enabled</a>()) &#123;<br />        <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_try_start">reconfiguration_with_dkg::try_start</a>();<br />    &#125; <b>else</b> &#123;<br />        <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">reconfiguration_with_dkg::finish</a>(aptos_framework);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_force_end_epoch"></a>

## Function `force_end_epoch`

Change epoch immediately.
If <code>RECONFIGURE_WITH_DKG</code> is enabled and we are in the middle of a DKG,
stop waiting for DKG and enter the new epoch without randomness.

WARNING: currently only used by tests. In most cases you should use <code><a href="aptos_governance.md#0x1_aptos_governance_reconfigure">reconfigure</a>()</code> instead.
TODO: migrate these tests to be aware of async reconfiguration.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_force_end_epoch">force_end_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_force_end_epoch">force_end_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">reconfiguration_with_dkg::finish</a>(aptos_framework);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_force_end_epoch_test_only"></a>

## Function `force_end_epoch_test_only`

<code><a href="aptos_governance.md#0x1_aptos_governance_force_end_epoch">force_end_epoch</a>()</code> equivalent but only called in testnet,
where the core resources account exists and has been granted power to mint Aptos coins.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_force_end_epoch_test_only">force_end_epoch_test_only</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_force_end_epoch_test_only">force_end_epoch_test_only</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a> &#123;<br />    <b>let</b> core_signer &#61; <a href="aptos_governance.md#0x1_aptos_governance_get_signer_testnet_only">get_signer_testnet_only</a>(aptos_framework, @0x1);<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(&amp;core_signer);<br />    <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">reconfiguration_with_dkg::finish</a>(&amp;core_signer);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_toggle_features"></a>

## Function `toggle_features`

Update feature flags and also trigger reconfiguration.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_toggle_features">toggle_features</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_toggle_features">toggle_features</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_change_feature_flags_for_next_epoch">features::change_feature_flags_for_next_epoch</a>(aptos_framework, enable, disable);<br />    <a href="aptos_governance.md#0x1_aptos_governance_reconfigure">reconfigure</a>(aptos_framework);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_get_signer_testnet_only"></a>

## Function `get_signer_testnet_only`

Only called in testnet where the core resources account exists and has been granted power to mint Aptos coins.


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_signer_testnet_only">get_signer_testnet_only</a>(core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_signer_testnet_only">get_signer_testnet_only</a>(<br />    core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_core_resource">system_addresses::assert_core_resource</a>(core_resources);<br />    // Core resources <a href="account.md#0x1_account">account</a> only <b>has</b> mint capability in tests/testnets.<br />    <b>assert</b>!(<a href="aptos_coin.md#0x1_aptos_coin_has_mint_capability">aptos_coin::has_mint_capability</a>(core_resources), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="aptos_governance.md#0x1_aptos_governance_EUNAUTHORIZED">EUNAUTHORIZED</a>));<br />    <a href="aptos_governance.md#0x1_aptos_governance_get_signer">get_signer</a>(signer_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_get_voting_power"></a>

## Function `get_voting_power`

Return the voting power a stake pool has with respect to governance proposals.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">get_voting_power</a>(pool_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">get_voting_power</a>(pool_address: <b>address</b>): u64 &#123;<br />    <b>let</b> allow_validator_set_change &#61; <a href="staking_config.md#0x1_staking_config_get_allow_validator_set_change">staking_config::get_allow_validator_set_change</a>(&amp;<a href="staking_config.md#0x1_staking_config_get">staking_config::get</a>());<br />    <b>if</b> (allow_validator_set_change) &#123;<br />        <b>let</b> (active, _, pending_active, pending_inactive) &#61; <a href="stake.md#0x1_stake_get_stake">stake::get_stake</a>(pool_address);<br />        // We calculate the <a href="voting.md#0x1_voting">voting</a> power <b>as</b> total non&#45;inactive stakes of the pool. Even <b>if</b> the validator is not in the<br />        // active validator set, <b>as</b> long <b>as</b> they have a lockup (separately checked in create_proposal and <a href="voting.md#0x1_voting">voting</a>), their<br />        // <a href="stake.md#0x1_stake">stake</a> would still count in their <a href="voting.md#0x1_voting">voting</a> power for governance proposals.<br />        active &#43; pending_active &#43; pending_inactive<br />    &#125; <b>else</b> &#123;<br />        <a href="stake.md#0x1_stake_get_current_epoch_voting_power">stake::get_current_epoch_voting_power</a>(pool_address)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_get_signer"></a>

## Function `get_signer`

Return a signer for making changes to 0x1 as part of on&#45;chain governance proposal process.


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_signer">get_signer</a>(signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_signer">get_signer</a>(signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a> &#123;<br />    <b>let</b> governance_responsibility &#61; <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework);<br />    <b>let</b> signer_cap &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;governance_responsibility.signer_caps, &amp;signer_address);<br />    create_signer_with_capability(signer_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal_metadata"></a>

## Function `create_proposal_metadata`



<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_metadata">create_proposal_metadata</a>(metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_metadata">create_proposal_metadata</a>(<br />    metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />): SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; &#123;<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;utf8(metadata_location)) &lt;&#61; 256, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EMETADATA_LOCATION_TOO_LONG">EMETADATA_LOCATION_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;utf8(metadata_hash)) &lt;&#61; 256, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="aptos_governance.md#0x1_aptos_governance_EMETADATA_HASH_TOO_LONG">EMETADATA_HASH_TOO_LONG</a>));<br /><br />    <b>let</b> metadata &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> metadata, utf8(<a href="aptos_governance.md#0x1_aptos_governance_METADATA_LOCATION_KEY">METADATA_LOCATION_KEY</a>), metadata_location);<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> metadata, utf8(<a href="aptos_governance.md#0x1_aptos_governance_METADATA_HASH_KEY">METADATA_HASH_KEY</a>), metadata_hash);<br />    metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_assert_voting_initialization"></a>

## Function `assert_voting_initialization`



<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_assert_voting_initialization">assert_voting_initialization</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_assert_voting_initialization">assert_voting_initialization</a>() &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_partial_governance_voting_enabled">features::partial_governance_voting_enabled</a>()) &#123;<br />        <b>assert</b>!(<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="aptos_governance.md#0x1_aptos_governance_EPARTIAL_VOTING_NOT_INITIALIZED">EPARTIAL_VOTING_NOT_INITIALIZED</a>));<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_aptos_governance_initialize_for_verification"></a>

## Function `initialize_for_verification`



<pre><code>&#35;[verify_only]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize_for_verification">initialize_for_verification</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize_for_verification">initialize_for_verification</a>(<br />    aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    min_voting_threshold: u128,<br />    required_proposer_stake: u64,<br />    voting_duration_secs: u64,<br />) &#123;<br />    <a href="aptos_governance.md#0x1_aptos_governance_initialize">initialize</a>(aptos_framework, min_voting_threshold, required_proposer_stake, voting_duration_secs);<br />&#125;<br /></code></pre>



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
<td>The create proposal function calls create proposal v2.</td>
<td>Low</td>
<td>The create_proposal function internally calls create_proposal_v2.</td>
<td>This is manually audited to ensure create_proposal_v2 is called in create_proposal.</td>
</tr>

<tr>
<td>2</td>
<td>The proposer must have a stake equal to or greater than the required bond amount.</td>
<td>High</td>
<td>The create_proposal_v2 function verifies that the stake balance equals or exceeds the required proposer stake amount.</td>
<td>Formally verified in <a href="#high-level-req-2">CreateProposalAbortsIf</a>.</td>
</tr>

<tr>
<td>3</td>
<td>The Approved execution hashes resources that exist when the vote function is called.</td>
<td>Low</td>
<td>The Vote function acquires the Approved execution hashes resources.</td>
<td>Formally verified in <a href="#high-level-req-3">VoteAbortIf</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The execution script hash of a successful governance proposal is added to the approved list if the proposal can be resolved.</td>
<td>Medium</td>
<td>The add_approved_script_hash function asserts that proposal_state &#61;&#61; PROPOSAL_STATE_SUCCEEDED.</td>
<td>Formally verified in <a href="#high-level-req-4">AddApprovedScriptHash</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_store_signer_cap"></a>

### Function `store_signer_cap`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_store_signer_cap">store_signer_cap</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>, signer_cap: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_framework_reserved_address">system_addresses::is_framework_reserved_address</a>(signer_address);<br /><b>let</b> signer_caps &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework).signer_caps;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(signer_caps, signer_address);<br /><b>ensures</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_signer_caps &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework).signer_caps;<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_signer_caps, signer_address);<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)<br /></code></pre>


Signer address must be @aptos_framework.
The signer does not allow these resources (GovernanceProposal, GovernanceConfig, GovernanceEvents, VotingRecords, ApprovedExecutionHashes) to exist.
The signer must have an Account.
Limit addition overflow.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>let</b> register_account &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> register_account.guid_creation_num &#43; 7 &gt; <a href="aptos_governance.md#0x1_aptos_governance_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> register_account.guid_creation_num &#43; 7 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;GovernanceProposal&gt;();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_InitializeAbortIf">InitializeAbortIf</a>;<br /><b>ensures</b> <b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;<a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>&gt;&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a>&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(addr);<br /></code></pre>



<a id="@Specification_1_update_governance_config"></a>

### Function `update_governance_config`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_update_governance_config">update_governance_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)<br /></code></pre>


Signer address must be @aptos_framework.
Address @aptos_framework must exist GovernanceConfig and GovernanceEvents.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>let</b> governance_config &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> new_governance_config &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a>&gt;(@aptos_framework);<br /><b>modifies</b> <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(addr);<br /><b>ensures</b> new_governance_config.voting_duration_secs &#61;&#61; voting_duration_secs;<br /><b>ensures</b> new_governance_config.min_voting_threshold &#61;&#61; min_voting_threshold;<br /><b>ensures</b> new_governance_config.required_proposer_stake &#61;&#61; required_proposer_stake;<br /></code></pre>



<a id="@Specification_1_initialize_partial_voting"></a>

### Function `initialize_partial_voting`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize_partial_voting">initialize_partial_voting</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


Signer address must be @aptos_framework.
Abort if structs have already been created.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br /></code></pre>




<a id="0x1_aptos_governance_InitializeAbortIf"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_InitializeAbortIf">InitializeAbortIf</a> &#123;<br />aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />min_voting_threshold: u128;<br />required_proposer_stake: u64;<br />voting_duration_secs: u64;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;<a href="governance_proposal.md#0x1_governance_proposal_GovernanceProposal">governance_proposal::GovernanceProposal</a>&gt;&gt;(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a>&gt;(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_voting_duration_secs"></a>

### Function `get_voting_duration_secs`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_voting_duration_secs">get_voting_duration_secs</a>(): u64<br /></code></pre>




<pre><code><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_AbortsIfNotGovernanceConfig">AbortsIfNotGovernanceConfig</a>;<br /></code></pre>



<a id="@Specification_1_get_min_voting_threshold"></a>

### Function `get_min_voting_threshold`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_min_voting_threshold">get_min_voting_threshold</a>(): u128<br /></code></pre>




<pre><code><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_AbortsIfNotGovernanceConfig">AbortsIfNotGovernanceConfig</a>;<br /></code></pre>



<a id="@Specification_1_get_required_proposer_stake"></a>

### Function `get_required_proposer_stake`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_required_proposer_stake">get_required_proposer_stake</a>(): u64<br /></code></pre>




<pre><code><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_AbortsIfNotGovernanceConfig">AbortsIfNotGovernanceConfig</a>;<br /></code></pre>




<a id="0x1_aptos_governance_AbortsIfNotGovernanceConfig"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_AbortsIfNotGovernanceConfig">AbortsIfNotGovernanceConfig</a> &#123;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceConfig">GovernanceConfig</a>&gt;(@aptos_framework);<br />&#125;<br /></code></pre>



<a id="@Specification_1_has_entirely_voted"></a>

### Function `has_entirely_voted`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_has_entirely_voted">has_entirely_voted</a>(stake_pool: <b>address</b>, proposal_id: u64): bool<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_get_remaining_voting_power"></a>

### Function `get_remaining_voting_power`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_remaining_voting_power">get_remaining_voting_power</a>(stake_pool: <b>address</b>, proposal_id: u64): u64<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>() &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br /><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">voting::AbortsIfNotContainProposalID</a>&lt;GovernanceProposal&gt; &#123;<br />    voting_forum_address: @aptos_framework<br />&#125;;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(stake_pool);<br /><b>aborts_if</b> spec_proposal_expiration &lt;&#61; locked_until &amp;&amp; !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>let</b> spec_proposal_expiration &#61; <a href="voting.md#0x1_voting_spec_get_proposal_expiration_secs">voting::spec_get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);<br /><b>let</b> locked_until &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(stake_pool).locked_until_secs;<br /><b>let</b> remain_zero_1_cond &#61; (spec_proposal_expiration &gt; locked_until &#124;&#124; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; spec_proposal_expiration);<br /><b>ensures</b> remain_zero_1_cond &#61;&#61;&gt; result &#61;&#61; 0;<br /><b>let</b> record_key &#61; <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a> &#123;<br />    stake_pool,<br />    proposal_id,<br />&#125;;<br /><b>let</b> entirely_voted &#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_has_entirely_voted">spec_has_entirely_voted</a>(stake_pool, proposal_id, record_key);<br /><b>aborts_if</b> !remain_zero_1_cond &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br /><b>include</b> !remain_zero_1_cond &amp;&amp; !entirely_voted &#61;&#61;&gt; <a href="aptos_governance.md#0x1_aptos_governance_GetVotingPowerAbortsIf">GetVotingPowerAbortsIf</a> &#123;<br />    pool_address: stake_pool<br />&#125;;<br /><b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> voting_power &#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_get_voting_power">spec_get_voting_power</a>(stake_pool, <a href="staking_config.md#0x1_staking_config">staking_config</a>);<br /><b>let</b> voting_records_v2 &#61; <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br /><b>let</b> used_voting_power &#61; <b>if</b> (<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(voting_records_v2.votes, record_key)) &#123;<br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_get">smart_table::spec_get</a>(voting_records_v2.votes, record_key)<br />&#125; <b>else</b> &#123;<br />    0<br />&#125;;<br /><b>aborts_if</b> !remain_zero_1_cond &amp;&amp; !entirely_voted &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>() &amp;&amp;<br />    used_voting_power &gt; 0 &amp;&amp; voting_power &lt; used_voting_power;<br /><b>ensures</b> result &#61;&#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_get_remaining_voting_power">spec_get_remaining_voting_power</a>(stake_pool, proposal_id);<br /></code></pre>




<a id="0x1_aptos_governance_spec_get_remaining_voting_power"></a>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_spec_get_remaining_voting_power">spec_get_remaining_voting_power</a>(stake_pool: <b>address</b>, proposal_id: u64): u64 &#123;<br />   <b>let</b> spec_proposal_expiration &#61; <a href="voting.md#0x1_voting_spec_get_proposal_expiration_secs">voting::spec_get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);<br />   <b>let</b> locked_until &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(stake_pool).locked_until_secs;<br />   <b>let</b> remain_zero_1_cond &#61; (spec_proposal_expiration &gt; locked_until &#124;&#124; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; spec_proposal_expiration);<br />   <b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br />   <b>let</b> voting_records_v2 &#61; <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br />   <b>let</b> record_key &#61; <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a> &#123;<br />       stake_pool,<br />       proposal_id,<br />   &#125;;<br />   <b>let</b> entirely_voted &#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_has_entirely_voted">spec_has_entirely_voted</a>(stake_pool, proposal_id, record_key);<br />   <b>let</b> voting_power &#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_get_voting_power">spec_get_voting_power</a>(stake_pool, <a href="staking_config.md#0x1_staking_config">staking_config</a>);<br />   <b>let</b> used_voting_power &#61; <b>if</b> (<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(voting_records_v2.votes, record_key)) &#123;<br />       <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_get">smart_table::spec_get</a>(voting_records_v2.votes, record_key)<br />   &#125; <b>else</b> &#123;<br />       0<br />   &#125;;<br />   <b>if</b> (remain_zero_1_cond) &#123;<br />       0<br />   &#125; <b>else</b> <b>if</b> (entirely_voted) &#123;<br />       0<br />   &#125; <b>else</b> <b>if</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>()) &#123;<br />       voting_power<br />   &#125; <b>else</b> &#123;<br />       voting_power &#45; used_voting_power<br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_aptos_governance_spec_has_entirely_voted"></a>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_spec_has_entirely_voted">spec_has_entirely_voted</a>(stake_pool: <b>address</b>, proposal_id: u64, record_key: <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a>): bool &#123;<br />   <b>let</b> voting_records &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br />   <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_records.votes, record_key)<br />&#125;<br /></code></pre>




<a id="0x1_aptos_governance_GetVotingPowerAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_GetVotingPowerAbortsIf">GetVotingPowerAbortsIf</a> &#123;<br />pool_address: <b>address</b>;<br /><b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> allow_validator_set_change &#61; <a href="staking_config.md#0x1_staking_config">staking_config</a>.allow_validator_set_change;<br /><b>let</b> stake_pool_res &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> allow_validator_set_change &amp;&amp; (stake_pool_res.active.value &#43; stake_pool_res.pending_active.value &#43; stake_pool_res.pending_inactive.value) &gt; <a href="aptos_governance.md#0x1_aptos_governance_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>aborts_if</b> !allow_validator_set_change &amp;&amp; !<b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !allow_validator_set_change &amp;&amp; <a href="stake.md#0x1_stake_spec_is_current_epoch_validator">stake::spec_is_current_epoch_validator</a>(pool_address) &amp;&amp; stake_pool_res.active.value &#43; stake_pool_res.pending_inactive.value &gt; <a href="aptos_governance.md#0x1_aptos_governance_MAX_U64">MAX_U64</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_proposal"></a>

### Function `create_proposal`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal">create_proposal</a>(proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>


The same as spec of <code><a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2">create_proposal_v2</a>()</code>.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 60;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_CreateProposalAbortsIf">CreateProposalAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_create_proposal_v2"></a>

### Function `create_proposal_v2`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2">create_proposal_v2</a>(proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 60;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_CreateProposalAbortsIf">CreateProposalAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_create_proposal_v2_impl"></a>

### Function `create_proposal_v2_impl`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_v2_impl">create_proposal_v2_impl</a>(proposer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_multi_step_proposal: bool): u64<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 60;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_CreateProposalAbortsIf">CreateProposalAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_vote">vote</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, should_pass: bool)<br /></code></pre>


stake_pool must exist StakePool.
The delegated voter under the resource StakePool of the stake_pool must be the voter address.
Address @aptos_framework must exist VotingRecords and GovernanceProposal.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 60;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_VoteAbortIf">VoteAbortIf</a>  &#123;<br />    voting_power: <a href="aptos_governance.md#0x1_aptos_governance_MAX_U64">MAX_U64</a><br />&#125;;<br /></code></pre>



<a id="@Specification_1_partial_vote"></a>

### Function `partial_vote`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_partial_vote">partial_vote</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)<br /></code></pre>


stake_pool must exist StakePool.
The delegated voter under the resource StakePool of the stake_pool must be the voter address.
Address @aptos_framework must exist VotingRecords and GovernanceProposal.
Address @aptos_framework must exist VotingRecordsV2 if partial_governance_voting flag is enabled.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 60;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_VoteAbortIf">VoteAbortIf</a>;<br /></code></pre>



<a id="@Specification_1_vote_internal"></a>

### Function `vote_internal`


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_vote_internal">vote_internal</a>(voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, stake_pool: <b>address</b>, proposal_id: u64, voting_power: u64, should_pass: bool)<br /></code></pre>


stake_pool must exist StakePool.
The delegated voter under the resource StakePool of the stake_pool must be the voter address.
Address @aptos_framework must exist VotingRecords and GovernanceProposal.
Address @aptos_framework must exist VotingRecordsV2 if partial_governance_voting flag is enabled.


<pre><code><b>pragma</b> verify_duration_estimate &#61; 60;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_VoteAbortIf">VoteAbortIf</a>;<br /></code></pre>




<a id="0x1_aptos_governance_VoteAbortIf"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_VoteAbortIf">VoteAbortIf</a> &#123;<br />voter: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />stake_pool: <b>address</b>;<br />proposal_id: u64;<br />should_pass: bool;<br />voting_power: u64;<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingGetDelegatedVoterAbortsIf">VotingGetDelegatedVoterAbortsIf</a> &#123; sign: voter &#125;;<br /><b>aborts_if</b> spec_proposal_expiration &lt;&#61; locked_until &amp;&amp; !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>let</b> spec_proposal_expiration &#61; <a href="voting.md#0x1_voting_spec_get_proposal_expiration_secs">voting::spec_get_proposal_expiration_secs</a>&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);<br /><b>let</b> locked_until &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(stake_pool).locked_until_secs;<br /><b>let</b> remain_zero_1_cond &#61; (spec_proposal_expiration &gt; locked_until &#124;&#124; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; spec_proposal_expiration);<br /><b>let</b> record_key &#61; <a href="aptos_governance.md#0x1_aptos_governance_RecordKey">RecordKey</a> &#123;<br />    stake_pool,<br />    proposal_id,<br />&#125;;<br /><b>let</b> entirely_voted &#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_has_entirely_voted">spec_has_entirely_voted</a>(stake_pool, proposal_id, record_key);<br /><b>aborts_if</b> !remain_zero_1_cond &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br /><b>include</b> !remain_zero_1_cond &amp;&amp; !entirely_voted &#61;&#61;&gt; <a href="aptos_governance.md#0x1_aptos_governance_GetVotingPowerAbortsIf">GetVotingPowerAbortsIf</a> &#123;<br />    pool_address: stake_pool<br />&#125;;<br /><b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> spec_voting_power &#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_get_voting_power">spec_get_voting_power</a>(stake_pool, <a href="staking_config.md#0x1_staking_config">staking_config</a>);<br /><b>let</b> voting_records_v2 &#61; <b>borrow_global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br /><b>let</b> used_voting_power &#61; <b>if</b> (<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_contains">smart_table::spec_contains</a>(voting_records_v2.votes, record_key)) &#123;<br />    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_spec_get">smart_table::spec_get</a>(voting_records_v2.votes, record_key)<br />&#125; <b>else</b> &#123;<br />    0<br />&#125;;<br /><b>aborts_if</b> !remain_zero_1_cond &amp;&amp; !entirely_voted &amp;&amp; <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>() &amp;&amp;<br />    used_voting_power &gt; 0 &amp;&amp; spec_voting_power &lt; used_voting_power;<br /><b>let</b> remaining_power &#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_get_remaining_voting_power">spec_get_remaining_voting_power</a>(stake_pool, proposal_id);<br /><b>let</b> real_voting_power &#61;  <b>min</b>(voting_power, remaining_power);<br /><b>aborts_if</b> !(real_voting_power &gt; 0);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br /><b>let</b> voting_records &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecords">VotingRecords</a>&gt;(@aptos_framework);<br /><b>let</b> allow_validator_set_change &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework).allow_validator_set_change;<br /><b>let</b> stake_pool_res &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(stake_pool);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> proposal_expiration &#61; proposal.expiration_secs;<br /><b>let</b> locked_until_secs &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(stake_pool).locked_until_secs;<br /><b>aborts_if</b> proposal_expiration &gt; locked_until_secs;<br /><b>aborts_if</b> <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; proposal_expiration;<br /><b>aborts_if</b> proposal.is_resolved;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>let</b> execution_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, execution_key) &amp;&amp;<br />          <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, execution_key) !&#61; std::bcs::to_bytes(<b>false</b>);<br /><b>aborts_if</b><br />    <b>if</b> (should_pass) &#123; proposal.yes_votes &#43; real_voting_power &gt; MAX_U128 &#125; <b>else</b> &#123; proposal.no_votes &#43; real_voting_power &gt; MAX_U128 &#125;;<br /><b>let</b> <b>post</b> post_voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">voting::RESOLVABLE_TIME_METADATA_KEY</a>);<br /><b>let</b> key &#61; utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">voting::RESOLVABLE_TIME_METADATA_KEY</a>);<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_proposal.metadata, key);<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, key) &#61;&#61; std::bcs::to_bytes(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>());<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>() &amp;&amp; used_voting_power &#43; real_voting_power &gt; <a href="aptos_governance.md#0x1_aptos_governance_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>() &amp;&amp; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_records.votes, record_key);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceEvents">GovernanceEvents</a>&gt;(@aptos_framework);<br /><b>let</b> early_resolution_threshold &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(proposal.early_resolution_vote_threshold);<br /><b>let</b> is_voting_period_over &#61; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; proposal_expiration;<br /><b>let</b> new_proposal_yes_votes_0 &#61; proposal.yes_votes &#43; real_voting_power;<br /><b>let</b> can_be_resolved_early_0 &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold) &amp;&amp;<br />                            (new_proposal_yes_votes_0 &gt;&#61; early_resolution_threshold &#124;&#124;<br />                             proposal.no_votes &gt;&#61; early_resolution_threshold);<br /><b>let</b> is_voting_closed_0 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_0;<br /><b>let</b> proposal_state_successed_0 &#61; is_voting_closed_0 &amp;&amp; new_proposal_yes_votes_0 &gt; proposal.no_votes &amp;&amp;<br />                                 new_proposal_yes_votes_0 &#43; proposal.no_votes &gt;&#61; proposal.min_vote_threshold;<br /><b>let</b> new_proposal_no_votes_0 &#61; proposal.no_votes &#43; real_voting_power;<br /><b>let</b> can_be_resolved_early_1 &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold) &amp;&amp;<br />                            (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;<br />                             new_proposal_no_votes_0 &gt;&#61; early_resolution_threshold);<br /><b>let</b> is_voting_closed_1 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_1;<br /><b>let</b> proposal_state_successed_1 &#61; is_voting_closed_1 &amp;&amp; proposal.yes_votes &gt; new_proposal_no_votes_0 &amp;&amp;<br />                                 proposal.yes_votes &#43; new_proposal_no_votes_0 &gt;&#61; proposal.min_vote_threshold;<br /><b>let</b> new_proposal_yes_votes_1 &#61; proposal.yes_votes &#43; real_voting_power;<br /><b>let</b> can_be_resolved_early_2 &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold) &amp;&amp;<br />                            (new_proposal_yes_votes_1 &gt;&#61; early_resolution_threshold &#124;&#124;<br />                             proposal.no_votes &gt;&#61; early_resolution_threshold);<br /><b>let</b> is_voting_closed_2 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_2;<br /><b>let</b> proposal_state_successed_2 &#61; is_voting_closed_2 &amp;&amp; new_proposal_yes_votes_1 &gt; proposal.no_votes &amp;&amp;<br />                                 new_proposal_yes_votes_1 &#43; proposal.no_votes &gt;&#61; proposal.min_vote_threshold;<br /><b>let</b> new_proposal_no_votes_1 &#61; proposal.no_votes &#43; real_voting_power;<br /><b>let</b> can_be_resolved_early_3 &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold) &amp;&amp;<br />                            (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;<br />                             new_proposal_no_votes_1 &gt;&#61; early_resolution_threshold);<br /><b>let</b> is_voting_closed_3 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_3;<br /><b>let</b> proposal_state_successed_3 &#61; is_voting_closed_3 &amp;&amp; proposal.yes_votes &gt; new_proposal_no_votes_1 &amp;&amp;<br />                                 proposal.yes_votes &#43; new_proposal_no_votes_1 &gt;&#61; proposal.min_vote_threshold;<br /><b>let</b> <b>post</b> can_be_resolved_early &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold) &amp;&amp;<br />                            (post_proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;<br />                             post_proposal.no_votes &gt;&#61; early_resolution_threshold);<br /><b>let</b> <b>post</b> is_voting_closed &#61; is_voting_period_over &#124;&#124; can_be_resolved_early;<br /><b>let</b> <b>post</b> proposal_state_successed &#61; is_voting_closed &amp;&amp; post_proposal.yes_votes &gt; post_proposal.no_votes &amp;&amp;<br />                                 post_proposal.yes_votes &#43; post_proposal.no_votes &gt;&#61; proposal.min_vote_threshold;<br /><b>let</b> execution_hash &#61; proposal.execution_hash;<br /><b>let</b> <b>post</b> post_approved_hashes &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
    <b>aborts_if</b><br />    <b>if</b> (should_pass) &#123;<br />        proposal_state_successed_0 &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework)<br />    &#125; <b>else</b> &#123;<br />        proposal_state_successed_1 &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework)<br />    &#125;;<br /><b>aborts_if</b><br />    <b>if</b> (should_pass) &#123;<br />        proposal_state_successed_2 &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework)<br />    &#125; <b>else</b> &#123;<br />        proposal_state_successed_3 &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework)<br />    &#125;;<br /><b>ensures</b> proposal_state_successed &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_approved_hashes.hashes, proposal_id) &amp;&amp;<br />                                     <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_approved_hashes.hashes, proposal_id) &#61;&#61; execution_hash;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>() &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br />&#125;<br /></code></pre>



<a id="@Specification_1_add_approved_script_hash_script"></a>

### Function `add_approved_script_hash_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash_script">add_approved_script_hash_script</a>(proposal_id: u64)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_AddApprovedScriptHash">AddApprovedScriptHash</a>;<br /></code></pre>




<a id="0x1_aptos_governance_AddApprovedScriptHash"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_AddApprovedScriptHash">AddApprovedScriptHash</a> &#123;<br />proposal_id: u64;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> early_resolution_threshold &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(proposal.early_resolution_vote_threshold);<br /><b>aborts_if</b> <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;&#61; proposal.expiration_secs &amp;&amp;<br />    (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(proposal.early_resolution_vote_threshold) &#124;&#124;<br />    proposal.yes_votes &lt; early_resolution_threshold &amp;&amp; proposal.no_votes &lt; early_resolution_threshold);<br /><b>aborts_if</b> (<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; proposal.expiration_secs &#124;&#124;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold) &amp;&amp; (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;<br />                                                                       proposal.no_votes &gt;&#61; early_resolution_threshold)) &amp;&amp;<br />    (proposal.yes_votes &lt;&#61; proposal.no_votes &#124;&#124; proposal.yes_votes &#43; proposal.no_votes &lt; proposal.min_vote_threshold);<br /><b>let</b> <b>post</b> post_approved_hashes &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
    <b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_approved_hashes.hashes, proposal_id) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_approved_hashes.hashes, proposal_id) &#61;&#61; proposal.execution_hash;<br />&#125;<br /></code></pre>



<a id="@Specification_1_add_approved_script_hash"></a>

### Function `add_approved_script_hash`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_add_approved_script_hash">add_approved_script_hash</a>(proposal_id: u64)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_AddApprovedScriptHash">AddApprovedScriptHash</a>;<br /></code></pre>



<a id="@Specification_1_resolve"></a>

### Function `resolve`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_resolve">resolve</a>(proposal_id: u64, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>


Address @aptos_framework must exist ApprovedExecutionHashes and GovernanceProposal and GovernanceResponsbility.


<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingIsProposalResolvableAbortsif">VotingIsProposalResolvableAbortsif</a>;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> multi_step_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">voting::IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>let</b> has_multi_step_key &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key);<br /><b>let</b> is_multi_step_proposal &#61; aptos_std::from_bcs::deserialize&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>aborts_if</b> has_multi_step_key &amp;&amp; !aptos_std::from_bcs::deserializable&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">voting::IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>aborts_if</b> has_multi_step_key &amp;&amp; is_multi_step_proposal;<br /><b>let</b> <b>post</b> post_voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);<br /><b>ensures</b> post_proposal.is_resolved &#61;&#61; <b>true</b> &amp;&amp; post_proposal.resolution_time_secs &#61;&#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(proposal.execution_content);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_approved_hashes &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework).hashes;<br /><b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_approved_hashes, proposal_id);<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_GetSignerAbortsIf">GetSignerAbortsIf</a>;<br /><b>let</b> governance_responsibility &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework);<br /><b>let</b> signer_cap &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(governance_responsibility.signer_caps, signer_address);<br /><b>let</b> addr &#61; signer_cap.<a href="account.md#0x1_account">account</a>;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) &#61;&#61; addr;<br /></code></pre>



<a id="@Specification_1_resolve_multi_step_proposal"></a>

### Function `resolve_multi_step_proposal`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_resolve_multi_step_proposal">resolve_multi_step_proposal</a>(proposal_id: u64, signer_address: <b>address</b>, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingIsProposalResolvableAbortsif">VotingIsProposalResolvableAbortsif</a>;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> <b>post</b> post_voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>let</b> multi_step_in_execution_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>let</b> <b>post</b> is_multi_step_proposal_in_execution_value &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, multi_step_in_execution_key);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">voting::IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>let</b> multi_step_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">voting::IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key) &amp;&amp;<br />    !aptos_std::from_bcs::deserializable&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>let</b> is_multi_step &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key) &amp;&amp;<br />                    aptos_std::from_bcs::deserialize&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>let</b> next_execution_hash_is_empty &#61; len(next_execution_hash) &#61;&#61; 0;<br /><b>aborts_if</b> !is_multi_step &amp;&amp; !next_execution_hash_is_empty;<br /><b>aborts_if</b> next_execution_hash_is_empty &amp;&amp; is_multi_step &amp;&amp; !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key);<br /><b>ensures</b> next_execution_hash_is_empty &#61;&#61;&gt; post_proposal.is_resolved &#61;&#61; <b>true</b> &amp;&amp; post_proposal.resolution_time_secs &#61;&#61; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &amp;&amp;<br />    <b>if</b> (is_multi_step) &#123;<br />        is_multi_step_proposal_in_execution_value &#61;&#61; std::bcs::serialize(<b>false</b>)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key) &#61;&#61;&gt;<br />            is_multi_step_proposal_in_execution_value &#61;&#61; std::bcs::serialize(<b>true</b>)<br />    &#125;;<br /><b>ensures</b> !next_execution_hash_is_empty &#61;&#61;&gt; post_proposal.execution_hash &#61;&#61; next_execution_hash;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_approved_hashes &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework).hashes;<br /><b>ensures</b> next_execution_hash_is_empty &#61;&#61;&gt; !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_approved_hashes, proposal_id);<br /><b>ensures</b> !next_execution_hash_is_empty &#61;&#61;&gt;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_approved_hashes, proposal_id) &#61;&#61; next_execution_hash;<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_GetSignerAbortsIf">GetSignerAbortsIf</a>;<br /><b>let</b> governance_responsibility &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework);<br /><b>let</b> signer_cap &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(governance_responsibility.signer_caps, signer_address);<br /><b>let</b> addr &#61; signer_cap.<a href="account.md#0x1_account">account</a>;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) &#61;&#61; addr;<br /></code></pre>




<a id="0x1_aptos_governance_VotingIsProposalResolvableAbortsif"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingIsProposalResolvableAbortsif">VotingIsProposalResolvableAbortsif</a> &#123;<br />proposal_id: u64;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> early_resolution_threshold &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(proposal.early_resolution_vote_threshold);<br /><b>let</b> voting_period_over &#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; proposal.expiration_secs;<br /><b>let</b> be_resolved_early &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold) &amp;&amp;<br />                            (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;<br />                             proposal.no_votes &gt;&#61; early_resolution_threshold);<br /><b>let</b> voting_closed &#61; voting_period_over &#124;&#124; be_resolved_early;<br /><b>aborts_if</b> voting_closed &amp;&amp; (proposal.yes_votes &lt;&#61; proposal.no_votes &#124;&#124; proposal.yes_votes &#43; proposal.no_votes &lt; proposal.min_vote_threshold);<br /><b>aborts_if</b> !voting_closed;<br /><b>aborts_if</b> proposal.is_resolved;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">voting::RESOLVABLE_TIME_METADATA_KEY</a>);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">voting::RESOLVABLE_TIME_METADATA_KEY</a>));<br /><b>let</b> resolvable_time &#61; aptos_std::from_bcs::deserialize&lt;u64&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">voting::RESOLVABLE_TIME_METADATA_KEY</a>)));<br /><b>aborts_if</b> !aptos_std::from_bcs::deserializable&lt;u64&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">voting::RESOLVABLE_TIME_METADATA_KEY</a>)));<br /><b>aborts_if</b> <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;&#61; resolvable_time;<br /><b>aborts_if</b> aptos_framework::transaction_context::spec_get_script_hash() !&#61; proposal.execution_hash;<br />&#125;<br /></code></pre>



<a id="@Specification_1_remove_approved_hash"></a>

### Function `remove_approved_hash`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_remove_approved_hash">remove_approved_hash</a>(proposal_id: u64)<br /></code></pre>


Address @aptos_framework must exist ApprovedExecutionHashes and GovernanceProposal.


<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework);<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">voting::VotingForum</a>&lt;GovernanceProposal&gt;&gt;(@aptos_framework);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !proposal.is_resolved;<br /><b>let</b> <b>post</b> approved_hashes &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_ApprovedExecutionHashes">ApprovedExecutionHashes</a>&gt;(@aptos_framework).hashes;<br /><b>ensures</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(approved_hashes, proposal_id);<br /></code></pre>



<a id="@Specification_1_reconfigure"></a>

### Function `reconfigure`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_reconfigure">reconfigure</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));<br /><b>include</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">reconfiguration_with_dkg::FinishRequirement</a> &#123;<br />    framework: aptos_framework<br />&#125;;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework);<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br /></code></pre>



<a id="@Specification_1_force_end_epoch"></a>

### Function `force_end_epoch`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_force_end_epoch">force_end_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> <b>address</b> &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>include</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">reconfiguration_with_dkg::FinishRequirement</a> &#123;<br />    framework: aptos_framework<br />&#125;;<br /></code></pre>




<a id="0x1_aptos_governance_VotingInitializationAbortIfs"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingInitializationAbortIfs">VotingInitializationAbortIfs</a> &#123;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_partial_governance_voting_enabled">features::spec_partial_governance_voting_enabled</a>() &amp;&amp; !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_VotingRecordsV2">VotingRecordsV2</a>&gt;(@aptos_framework);<br />&#125;<br /></code></pre>



<a id="@Specification_1_force_end_epoch_test_only"></a>

### Function `force_end_epoch_test_only`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_force_end_epoch_test_only">force_end_epoch_test_only</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_toggle_features"></a>

### Function `toggle_features`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_toggle_features">toggle_features</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)<br /></code></pre>


Signer address must be @aptos_framework.
Address @aptos_framework must exist GovernanceConfig and GovernanceEvents.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /><b>include</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_FinishRequirement">reconfiguration_with_dkg::FinishRequirement</a> &#123;<br />    framework: aptos_framework<br />&#125;;<br /><b>include</b> <a href="stake.md#0x1_stake_GetReconfigStartTimeRequirement">stake::GetReconfigStartTimeRequirement</a>;<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingRewardsConfig">staking_config::StakingRewardsConfig</a>&gt;(@aptos_framework);<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br /></code></pre>



<a id="@Specification_1_get_signer_testnet_only"></a>

### Function `get_signer_testnet_only`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_signer_testnet_only">get_signer_testnet_only</a>(core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>


Signer address must be @core_resources.
signer must exist in MintCapStore.
Address @aptos_framework must exist GovernanceResponsbility.


<pre><code><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(core_resources) !&#61; @core_resources;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_coin.md#0x1_aptos_coin_MintCapStore">aptos_coin::MintCapStore</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(core_resources));<br /><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_GetSignerAbortsIf">GetSignerAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_get_voting_power"></a>

### Function `get_voting_power`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">get_voting_power</a>(pool_address: <b>address</b>): u64<br /></code></pre>


Address @aptos_framework must exist StakingConfig.
limit addition overflow.
pool_address must exist in StakePool.


<pre><code><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_GetVotingPowerAbortsIf">GetVotingPowerAbortsIf</a>;<br /><b>let</b> <a href="staking_config.md#0x1_staking_config">staking_config</a> &#61; <b>global</b>&lt;<a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>&gt;(@aptos_framework);<br /><b>let</b> allow_validator_set_change &#61; <a href="staking_config.md#0x1_staking_config">staking_config</a>.allow_validator_set_change;<br /><b>let</b> stake_pool_res &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br /><b>ensures</b> allow_validator_set_change &#61;&#61;&gt; result &#61;&#61; stake_pool_res.active.value &#43; stake_pool_res.pending_active.value &#43; stake_pool_res.pending_inactive.value;<br /><b>ensures</b> !allow_validator_set_change &#61;&#61;&gt; <b>if</b> (<a href="stake.md#0x1_stake_spec_is_current_epoch_validator">stake::spec_is_current_epoch_validator</a>(pool_address)) &#123;<br />    result &#61;&#61; stake_pool_res.active.value &#43; stake_pool_res.pending_inactive.value<br />&#125; <b>else</b> &#123;<br />    result &#61;&#61; 0<br />&#125;;<br /><b>ensures</b> result &#61;&#61; <a href="aptos_governance.md#0x1_aptos_governance_spec_get_voting_power">spec_get_voting_power</a>(pool_address, <a href="staking_config.md#0x1_staking_config">staking_config</a>);<br /></code></pre>




<a id="0x1_aptos_governance_spec_get_voting_power"></a>


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_spec_get_voting_power">spec_get_voting_power</a>(pool_address: <b>address</b>, <a href="staking_config.md#0x1_staking_config">staking_config</a>: <a href="staking_config.md#0x1_staking_config_StakingConfig">staking_config::StakingConfig</a>): u64 &#123;<br />   <b>let</b> allow_validator_set_change &#61; <a href="staking_config.md#0x1_staking_config">staking_config</a>.allow_validator_set_change;<br />   <b>let</b> stake_pool_res &#61; <b>global</b>&lt;<a href="stake.md#0x1_stake_StakePool">stake::StakePool</a>&gt;(pool_address);<br />   <b>if</b> (allow_validator_set_change) &#123;<br />       stake_pool_res.active.value &#43; stake_pool_res.pending_active.value &#43; stake_pool_res.pending_inactive.value<br />   &#125; <b>else</b> <b>if</b> (!allow_validator_set_change &amp;&amp; (<a href="stake.md#0x1_stake_spec_is_current_epoch_validator">stake::spec_is_current_epoch_validator</a>(pool_address))) &#123;<br />       stake_pool_res.active.value &#43; stake_pool_res.pending_inactive.value<br />   &#125; <b>else</b> &#123;<br />       0<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_signer"></a>

### Function `get_signer`


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_get_signer">get_signer</a>(signer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>




<pre><code><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_GetSignerAbortsIf">GetSignerAbortsIf</a>;<br /></code></pre>




<a id="0x1_aptos_governance_GetSignerAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_GetSignerAbortsIf">GetSignerAbortsIf</a> &#123;<br />signer_address: <b>address</b>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework);<br /><b>let</b> cap_map &#61; <b>global</b>&lt;<a href="aptos_governance.md#0x1_aptos_governance_GovernanceResponsbility">GovernanceResponsbility</a>&gt;(@aptos_framework).signer_caps;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(cap_map, signer_address);<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_proposal_metadata"></a>

### Function `create_proposal_metadata`


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_create_proposal_metadata">create_proposal_metadata</a>(metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_CreateProposalMetadataAbortsIf">CreateProposalMetadataAbortsIf</a>;<br /></code></pre>




<a id="0x1_aptos_governance_CreateProposalMetadataAbortsIf"></a>


<pre><code><b>schema</b> <a href="aptos_governance.md#0x1_aptos_governance_CreateProposalMetadataAbortsIf">CreateProposalMetadataAbortsIf</a> &#123;<br />metadata_location: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />metadata_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(utf8(metadata_location)) &gt; 256;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(utf8(metadata_hash)) &gt; 256;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(metadata_location);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(metadata_hash);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="aptos_governance.md#0x1_aptos_governance_METADATA_LOCATION_KEY">METADATA_LOCATION_KEY</a>);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="aptos_governance.md#0x1_aptos_governance_METADATA_HASH_KEY">METADATA_HASH_KEY</a>);<br />&#125;<br /></code></pre>



<a id="@Specification_1_assert_voting_initialization"></a>

### Function `assert_voting_initialization`


<pre><code><b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_assert_voting_initialization">assert_voting_initialization</a>()<br /></code></pre>




<pre><code><b>include</b> <a href="aptos_governance.md#0x1_aptos_governance_VotingInitializationAbortIfs">VotingInitializationAbortIfs</a>;<br /></code></pre>



<a id="@Specification_1_initialize_for_verification"></a>

### Function `initialize_for_verification`


<pre><code>&#35;[verify_only]<br /><b>public</b> <b>fun</b> <a href="aptos_governance.md#0x1_aptos_governance_initialize_for_verification">initialize_for_verification</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)<br /></code></pre>


verify_only


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
