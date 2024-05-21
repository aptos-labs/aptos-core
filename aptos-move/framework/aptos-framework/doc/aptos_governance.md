
<a id="0x1_aptos_governance"></a>

# Module `0x1::aptos_governance`


AptosGovernance represents the on-chain governance of the Aptos network. Voting power is calculated based on the
current epoch's voting power of the proposer or voter's backing stake pool. In addition, for it to count,
the stake pool's lockup needs to be at least as long as the proposal's duration.

It provides the following flow:
1. Proposers can create a proposal by calling AptosGovernance::create_proposal. The proposer's backing stake pool
needs to have the minimum proposer stake required. Off-chain components can subscribe to CreateProposalEvent to
track proposal creation and proposal ids.
2. Voters can vote on a proposal. Their voting power is derived from the backing stake pool. A stake pool can vote
on a proposal multiple times as long as the total voting power of these votes doesn't exceed its total voting power.


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


<pre><code>use 0x1::account;
use 0x1::aptos_coin;
use 0x1::coin;
use 0x1::consensus_config;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::governance_proposal;
use 0x1::math64;
use 0x1::option;
use 0x1::randomness_config;
use 0x1::reconfiguration_with_dkg;
use 0x1::signer;
use 0x1::simple_map;
use 0x1::smart_table;
use 0x1::stake;
use 0x1::staking_config;
use 0x1::string;
use 0x1::system_addresses;
use 0x1::table;
use 0x1::timestamp;
use 0x1::voting;
</code></pre>



<a id="0x1_aptos_governance_GovernanceResponsbility"></a>

## Resource `GovernanceResponsbility`

Store the SignerCapabilities of accounts under the on-chain governance's control.


<pre><code>struct GovernanceResponsbility has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>signer_caps: simple_map::SimpleMap&lt;address, account::SignerCapability&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_GovernanceConfig"></a>

## Resource `GovernanceConfig`

Configurations of the AptosGovernance, set during Genesis and can be updated by the same process offered
by this AptosGovernance module.


<pre><code>struct GovernanceConfig has key
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

<a id="0x1_aptos_governance_RecordKey"></a>

## Struct `RecordKey`



<pre><code>struct RecordKey has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>stake_pool: address</code>
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


<pre><code>struct VotingRecords has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: table::Table&lt;aptos_governance::RecordKey, bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_VotingRecordsV2"></a>

## Resource `VotingRecordsV2`

Records to track the voting power usage of each stake pool on each proposal.


<pre><code>struct VotingRecordsV2 has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>votes: smart_table::SmartTable&lt;aptos_governance::RecordKey, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_ApprovedExecutionHashes"></a>

## Resource `ApprovedExecutionHashes`

Used to track which execution script hashes have been approved by governance.
This is required to bypass cases where the execution scripts exceed the size limit imposed by mempool.


<pre><code>struct ApprovedExecutionHashes has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hashes: simple_map::SimpleMap&lt;u64, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_GovernanceEvents"></a>

## Resource `GovernanceEvents`

Events generated by interactions with the AptosGovernance module.


<pre><code>struct GovernanceEvents has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create_proposal_events: event::EventHandle&lt;aptos_governance::CreateProposalEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_config_events: event::EventHandle&lt;aptos_governance::UpdateConfigEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: event::EventHandle&lt;aptos_governance::VoteEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_CreateProposalEvent"></a>

## Struct `CreateProposalEvent`

Event emitted when a proposal is created.


<pre><code>struct CreateProposalEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: address</code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: address</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_VoteEvent"></a>

## Struct `VoteEvent`

Event emitted when there's a vote on a proposa;


<pre><code>struct VoteEvent has drop, store
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
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: address</code>
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


<pre><code>struct UpdateConfigEvent has drop, store
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

<a id="0x1_aptos_governance_CreateProposal"></a>

## Struct `CreateProposal`

Event emitted when a proposal is created.


<pre><code>&#35;[event]
struct CreateProposal has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: address</code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: address</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_aptos_governance_Vote"></a>

## Struct `Vote`

Event emitted when there's a vote on a proposa;


<pre><code>&#35;[event]
struct Vote has drop, store
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
<code>voter: address</code>
</dt>
<dd>

</dd>
<dt>
<code>stake_pool: address</code>
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


<pre><code>&#35;[event]
struct UpdateConfig has drop, store
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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_aptos_governance_MAX_U64"></a>



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_aptos_governance_PROPOSAL_STATE_SUCCEEDED"></a>

This matches the same enum const in voting. We have to duplicate it as Move doesn't have support for enums yet.


<pre><code>const PROPOSAL_STATE_SUCCEEDED: u64 &#61; 1;
</code></pre>



<a id="0x1_aptos_governance_EALREADY_VOTED"></a>

The specified stake pool has already been used to vote on the same proposal


<pre><code>const EALREADY_VOTED: u64 &#61; 4;
</code></pre>



<a id="0x1_aptos_governance_EINSUFFICIENT_PROPOSER_STAKE"></a>

The specified stake pool does not have sufficient stake to create a proposal


<pre><code>const EINSUFFICIENT_PROPOSER_STAKE: u64 &#61; 1;
</code></pre>



<a id="0x1_aptos_governance_EINSUFFICIENT_STAKE_LOCKUP"></a>

The specified stake pool does not have long enough remaining lockup to create a proposal or vote


<pre><code>const EINSUFFICIENT_STAKE_LOCKUP: u64 &#61; 3;
</code></pre>



<a id="0x1_aptos_governance_EMETADATA_HASH_TOO_LONG"></a>

Metadata hash cannot be longer than 256 chars


<pre><code>const EMETADATA_HASH_TOO_LONG: u64 &#61; 10;
</code></pre>



<a id="0x1_aptos_governance_EMETADATA_LOCATION_TOO_LONG"></a>

Metadata location cannot be longer than 256 chars


<pre><code>const EMETADATA_LOCATION_TOO_LONG: u64 &#61; 9;
</code></pre>



<a id="0x1_aptos_governance_ENOT_DELEGATED_VOTER"></a>

This account is not the designated voter of the specified stake pool


<pre><code>const ENOT_DELEGATED_VOTER: u64 &#61; 2;
</code></pre>



<a id="0x1_aptos_governance_ENOT_PARTIAL_VOTING_PROPOSAL"></a>

The proposal in the argument is not a partial voting proposal.


<pre><code>const ENOT_PARTIAL_VOTING_PROPOSAL: u64 &#61; 14;
</code></pre>



<a id="0x1_aptos_governance_ENO_VOTING_POWER"></a>

The specified stake pool must be part of the validator set


<pre><code>const ENO_VOTING_POWER: u64 &#61; 5;
</code></pre>



<a id="0x1_aptos_governance_EPARTIAL_VOTING_NOT_INITIALIZED"></a>

Partial voting feature hasn't been properly initialized.


<pre><code>const EPARTIAL_VOTING_NOT_INITIALIZED: u64 &#61; 13;
</code></pre>



<a id="0x1_aptos_governance_EPROPOSAL_NOT_RESOLVABLE_YET"></a>

Proposal is not ready to be resolved. Waiting on time or votes


<pre><code>const EPROPOSAL_NOT_RESOLVABLE_YET: u64 &#61; 6;
</code></pre>



<a id="0x1_aptos_governance_EPROPOSAL_NOT_RESOLVED_YET"></a>

The proposal has not been resolved yet


<pre><code>const EPROPOSAL_NOT_RESOLVED_YET: u64 &#61; 8;
</code></pre>



<a id="0x1_aptos_governance_EUNAUTHORIZED"></a>

Account is not authorized to call this function.


<pre><code>const EUNAUTHORIZED: u64 &#61; 11;
</code></pre>



<a id="0x1_aptos_governance_EVOTING_POWER_OVERFLOW"></a>

The stake pool is using voting power more than it has.


<pre><code>const EVOTING_POWER_OVERFLOW: u64 &#61; 12;
</code></pre>



<a id="0x1_aptos_governance_METADATA_HASH_KEY"></a>



<pre><code>const METADATA_HASH_KEY: vector&lt;u8&gt; &#61; [109, 101, 116, 97, 100, 97, 116, 97, 95, 104, 97, 115, 104];
</code></pre>



<a id="0x1_aptos_governance_METADATA_LOCATION_KEY"></a>

Proposal metadata attribute keys.


<pre><code>const METADATA_LOCATION_KEY: vector&lt;u8&gt; &#61; [109, 101, 116, 97, 100, 97, 116, 97, 95, 108, 111, 99, 97, 116, 105, 111, 110];
</code></pre>



<a id="0x1_aptos_governance_store_signer_cap"></a>

## Function `store_signer_cap`

Can be called during genesis or by the governance itself.
Stores the signer capability for a given address.


<pre><code>public fun store_signer_cap(aptos_framework: &amp;signer, signer_address: address, signer_cap: account::SignerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun store_signer_cap(
    aptos_framework: &amp;signer,
    signer_address: address,
    signer_cap: SignerCapability,
) acquires GovernanceResponsbility &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    system_addresses::assert_framework_reserved(signer_address);

    if (!exists&lt;GovernanceResponsbility&gt;(@aptos_framework)) &#123;
        move_to(
            aptos_framework,
            GovernanceResponsbility &#123; signer_caps: simple_map::create&lt;address, SignerCapability&gt;() &#125;
        );
    &#125;;

    let signer_caps &#61; &amp;mut borrow_global_mut&lt;GovernanceResponsbility&gt;(@aptos_framework).signer_caps;
    simple_map::add(signer_caps, signer_address, signer_cap);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_initialize"></a>

## Function `initialize`

Initializes the state for Aptos Governance. Can only be called during Genesis with a signer
for the aptos_framework (0x1) account.
This function is private because it's called directly from the vm.


<pre><code>fun initialize(aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize(
    aptos_framework: &amp;signer,
    min_voting_threshold: u128,
    required_proposer_stake: u64,
    voting_duration_secs: u64,
) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);

    voting::register&lt;GovernanceProposal&gt;(aptos_framework);
    move_to(aptos_framework, GovernanceConfig &#123;
        voting_duration_secs,
        min_voting_threshold,
        required_proposer_stake,
    &#125;);
    move_to(aptos_framework, GovernanceEvents &#123;
        create_proposal_events: account::new_event_handle&lt;CreateProposalEvent&gt;(aptos_framework),
        update_config_events: account::new_event_handle&lt;UpdateConfigEvent&gt;(aptos_framework),
        vote_events: account::new_event_handle&lt;VoteEvent&gt;(aptos_framework),
    &#125;);
    move_to(aptos_framework, VotingRecords &#123;
        votes: table::new(),
    &#125;);
    move_to(aptos_framework, ApprovedExecutionHashes &#123;
        hashes: simple_map::create&lt;u64, vector&lt;u8&gt;&gt;(),
    &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_update_governance_config"></a>

## Function `update_governance_config`

Update the governance configurations. This can only be called as part of resolving a proposal in this same
AptosGovernance.


<pre><code>public fun update_governance_config(aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_governance_config(
    aptos_framework: &amp;signer,
    min_voting_threshold: u128,
    required_proposer_stake: u64,
    voting_duration_secs: u64,
) acquires GovernanceConfig, GovernanceEvents &#123;
    system_addresses::assert_aptos_framework(aptos_framework);

    let governance_config &#61; borrow_global_mut&lt;GovernanceConfig&gt;(@aptos_framework);
    governance_config.voting_duration_secs &#61; voting_duration_secs;
    governance_config.min_voting_threshold &#61; min_voting_threshold;
    governance_config.required_proposer_stake &#61; required_proposer_stake;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            UpdateConfig &#123;
                min_voting_threshold,
                required_proposer_stake,
                voting_duration_secs
            &#125;,
        )
    &#125;;
    let events &#61; borrow_global_mut&lt;GovernanceEvents&gt;(@aptos_framework);
    event::emit_event&lt;UpdateConfigEvent&gt;(
        &amp;mut events.update_config_events,
        UpdateConfigEvent &#123;
            min_voting_threshold,
            required_proposer_stake,
            voting_duration_secs
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_initialize_partial_voting"></a>

## Function `initialize_partial_voting`

Initializes the state for Aptos Governance partial voting. Can only be called through Aptos governance
proposals with a signer for the aptos_framework (0x1) account.


<pre><code>public fun initialize_partial_voting(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_partial_voting(
    aptos_framework: &amp;signer,
) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);

    move_to(aptos_framework, VotingRecordsV2 &#123;
        votes: smart_table::new(),
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_get_voting_duration_secs"></a>

## Function `get_voting_duration_secs`



<pre><code>&#35;[view]
public fun get_voting_duration_secs(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_voting_duration_secs(): u64 acquires GovernanceConfig &#123;
    borrow_global&lt;GovernanceConfig&gt;(@aptos_framework).voting_duration_secs
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_get_min_voting_threshold"></a>

## Function `get_min_voting_threshold`



<pre><code>&#35;[view]
public fun get_min_voting_threshold(): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_min_voting_threshold(): u128 acquires GovernanceConfig &#123;
    borrow_global&lt;GovernanceConfig&gt;(@aptos_framework).min_voting_threshold
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_get_required_proposer_stake"></a>

## Function `get_required_proposer_stake`



<pre><code>&#35;[view]
public fun get_required_proposer_stake(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_required_proposer_stake(): u64 acquires GovernanceConfig &#123;
    borrow_global&lt;GovernanceConfig&gt;(@aptos_framework).required_proposer_stake
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_has_entirely_voted"></a>

## Function `has_entirely_voted`

Return true if a stake pool has already voted on a proposal before partial governance voting is enabled.


<pre><code>&#35;[view]
public fun has_entirely_voted(stake_pool: address, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun has_entirely_voted(stake_pool: address, proposal_id: u64): bool acquires VotingRecords &#123;
    let record_key &#61; RecordKey &#123;
        stake_pool,
        proposal_id,
    &#125;;
    // If a stake pool has already voted on a proposal before partial governance voting is enabled,
    // there is a record in VotingRecords.
    let voting_records &#61; borrow_global&lt;VotingRecords&gt;(@aptos_framework);
    table::contains(&amp;voting_records.votes, record_key)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_get_remaining_voting_power"></a>

## Function `get_remaining_voting_power`

Return remaining voting power of a stake pool on a proposal.
Note: a stake pool's voting power on a proposal could increase over time(e.g. rewards/new stake).


<pre><code>&#35;[view]
public fun get_remaining_voting_power(stake_pool: address, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_remaining_voting_power(
    stake_pool: address,
    proposal_id: u64
): u64 acquires VotingRecords, VotingRecordsV2 &#123;
    assert_voting_initialization();

    let proposal_expiration &#61; voting::get_proposal_expiration_secs&lt;GovernanceProposal&gt;(
        @aptos_framework,
        proposal_id
    );
    let lockup_until &#61; stake::get_lockup_secs(stake_pool);
    // The voter&apos;s stake needs to be locked up at least as long as the proposal&apos;s expiration.
    // Also no one can vote on a expired proposal.
    if (proposal_expiration &gt; lockup_until &#124;&#124; timestamp::now_seconds() &gt; proposal_expiration) &#123;
        return 0
    &#125;;

    // If a stake pool has already voted on a proposal before partial governance voting is enabled, the stake pool
    // cannot vote on the proposal even after partial governance voting is enabled.
    if (has_entirely_voted(stake_pool, proposal_id)) &#123;
        return 0
    &#125;;
    let record_key &#61; RecordKey &#123;
        stake_pool,
        proposal_id,
    &#125;;
    let used_voting_power &#61; 0u64;
    if (features::partial_governance_voting_enabled()) &#123;
        let voting_records_v2 &#61; borrow_global&lt;VotingRecordsV2&gt;(@aptos_framework);
        used_voting_power &#61; &#42;smart_table::borrow_with_default(&amp;voting_records_v2.votes, record_key, &amp;0);
    &#125;;
    get_voting_power(stake_pool) &#45; used_voting_power
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal"></a>

## Function `create_proposal`

Create a single-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code>public entry fun create_proposal(proposer: &amp;signer, stake_pool: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_proposal(
    proposer: &amp;signer,
    stake_pool: address,
    execution_hash: vector&lt;u8&gt;,
    metadata_location: vector&lt;u8&gt;,
    metadata_hash: vector&lt;u8&gt;,
) acquires GovernanceConfig, GovernanceEvents &#123;
    create_proposal_v2(proposer, stake_pool, execution_hash, metadata_location, metadata_hash, false);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal_v2"></a>

## Function `create_proposal_v2`

Create a single-step or multi-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.


<pre><code>public entry fun create_proposal_v2(proposer: &amp;signer, stake_pool: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;, is_multi_step_proposal: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_proposal_v2(
    proposer: &amp;signer,
    stake_pool: address,
    execution_hash: vector&lt;u8&gt;,
    metadata_location: vector&lt;u8&gt;,
    metadata_hash: vector&lt;u8&gt;,
    is_multi_step_proposal: bool,
) acquires GovernanceConfig, GovernanceEvents &#123;
    create_proposal_v2_impl(
        proposer,
        stake_pool,
        execution_hash,
        metadata_location,
        metadata_hash,
        is_multi_step_proposal
    );
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal_v2_impl"></a>

## Function `create_proposal_v2_impl`

Create a single-step or multi-step proposal with the backing <code>stake_pool</code>.
@param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
only the exact script with matching hash can be successfully executed.
Return proposal_id when a proposal is successfully created.


<pre><code>public fun create_proposal_v2_impl(proposer: &amp;signer, stake_pool: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;, is_multi_step_proposal: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_proposal_v2_impl(
    proposer: &amp;signer,
    stake_pool: address,
    execution_hash: vector&lt;u8&gt;,
    metadata_location: vector&lt;u8&gt;,
    metadata_hash: vector&lt;u8&gt;,
    is_multi_step_proposal: bool,
): u64 acquires GovernanceConfig, GovernanceEvents &#123;
    let proposer_address &#61; signer::address_of(proposer);
    assert!(
        stake::get_delegated_voter(stake_pool) &#61;&#61; proposer_address,
        error::invalid_argument(ENOT_DELEGATED_VOTER)
    );

    // The proposer&apos;s stake needs to be at least the required bond amount.
    let governance_config &#61; borrow_global&lt;GovernanceConfig&gt;(@aptos_framework);
    let stake_balance &#61; get_voting_power(stake_pool);
    assert!(
        stake_balance &gt;&#61; governance_config.required_proposer_stake,
        error::invalid_argument(EINSUFFICIENT_PROPOSER_STAKE),
    );

    // The proposer&apos;s stake needs to be locked up at least as long as the proposal&apos;s voting period.
    let current_time &#61; timestamp::now_seconds();
    let proposal_expiration &#61; current_time &#43; governance_config.voting_duration_secs;
    assert!(
        stake::get_lockup_secs(stake_pool) &gt;&#61; proposal_expiration,
        error::invalid_argument(EINSUFFICIENT_STAKE_LOCKUP),
    );

    // Create and validate proposal metadata.
    let proposal_metadata &#61; create_proposal_metadata(metadata_location, metadata_hash);

    // We want to allow early resolution of proposals if more than 50% of the total supply of the network coins
    // has voted. This doesn&apos;t take into subsequent inflation/deflation (rewards are issued every epoch and gas fees
    // are burnt after every transaction), but inflation/delation is very unlikely to have a major impact on total
    // supply during the voting period.
    let total_voting_token_supply &#61; coin::supply&lt;AptosCoin&gt;();
    let early_resolution_vote_threshold &#61; option::none&lt;u128&gt;();
    if (option::is_some(&amp;total_voting_token_supply)) &#123;
        let total_supply &#61; &#42;option::borrow(&amp;total_voting_token_supply);
        // 50% &#43; 1 to avoid rounding errors.
        early_resolution_vote_threshold &#61; option::some(total_supply / 2 &#43; 1);
    &#125;;

    let proposal_id &#61; voting::create_proposal_v2(
        proposer_address,
        @aptos_framework,
        governance_proposal::create_proposal(),
        execution_hash,
        governance_config.min_voting_threshold,
        proposal_expiration,
        early_resolution_vote_threshold,
        proposal_metadata,
        is_multi_step_proposal,
    );

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            CreateProposal &#123;
                proposal_id,
                proposer: proposer_address,
                stake_pool,
                execution_hash,
                proposal_metadata,
            &#125;,
        );
    &#125;;
    let events &#61; borrow_global_mut&lt;GovernanceEvents&gt;(@aptos_framework);
    event::emit_event&lt;CreateProposalEvent&gt;(
        &amp;mut events.create_proposal_events,
        CreateProposalEvent &#123;
            proposal_id,
            proposer: proposer_address,
            stake_pool,
            execution_hash,
            proposal_metadata,
        &#125;,
    );
    proposal_id
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_vote"></a>

## Function `vote`

Vote on proposal with <code>proposal_id</code> and all voting power from <code>stake_pool</code>.


<pre><code>public entry fun vote(voter: &amp;signer, stake_pool: address, proposal_id: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote(
    voter: &amp;signer,
    stake_pool: address,
    proposal_id: u64,
    should_pass: bool,
) acquires ApprovedExecutionHashes, VotingRecords, VotingRecordsV2, GovernanceEvents &#123;
    vote_internal(voter, stake_pool, proposal_id, MAX_U64, should_pass);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_partial_vote"></a>

## Function `partial_vote`

Vote on proposal with <code>proposal_id</code> and specified voting power from <code>stake_pool</code>.


<pre><code>public entry fun partial_vote(voter: &amp;signer, stake_pool: address, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun partial_vote(
    voter: &amp;signer,
    stake_pool: address,
    proposal_id: u64,
    voting_power: u64,
    should_pass: bool,
) acquires ApprovedExecutionHashes, VotingRecords, VotingRecordsV2, GovernanceEvents &#123;
    vote_internal(voter, stake_pool, proposal_id, voting_power, should_pass);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_vote_internal"></a>

## Function `vote_internal`

Vote on proposal with <code>proposal_id</code> and specified voting_power from <code>stake_pool</code>.
If voting_power is more than all the left voting power of <code>stake_pool</code>, use all the left voting power.
If a stake pool has already voted on a proposal before partial governance voting is enabled, the stake pool
cannot vote on the proposal even after partial governance voting is enabled.


<pre><code>fun vote_internal(voter: &amp;signer, stake_pool: address, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun vote_internal(
    voter: &amp;signer,
    stake_pool: address,
    proposal_id: u64,
    voting_power: u64,
    should_pass: bool,
) acquires ApprovedExecutionHashes, VotingRecords, VotingRecordsV2, GovernanceEvents &#123;
    let voter_address &#61; signer::address_of(voter);
    assert!(stake::get_delegated_voter(stake_pool) &#61;&#61; voter_address, error::invalid_argument(ENOT_DELEGATED_VOTER));

    // The voter&apos;s stake needs to be locked up at least as long as the proposal&apos;s expiration.
    let proposal_expiration &#61; voting::get_proposal_expiration_secs&lt;GovernanceProposal&gt;(
        @aptos_framework,
        proposal_id
    );
    assert!(
        stake::get_lockup_secs(stake_pool) &gt;&#61; proposal_expiration,
        error::invalid_argument(EINSUFFICIENT_STAKE_LOCKUP),
    );

    // If a stake pool has already voted on a proposal before partial governance voting is enabled,
    // `get_remaining_voting_power` returns 0.
    let staking_pool_voting_power &#61; get_remaining_voting_power(stake_pool, proposal_id);
    voting_power &#61; min(voting_power, staking_pool_voting_power);

    // Short&#45;circuit if the voter has no voting power.
    assert!(voting_power &gt; 0, error::invalid_argument(ENO_VOTING_POWER));

    voting::vote&lt;GovernanceProposal&gt;(
        &amp;governance_proposal::create_empty_proposal(),
        @aptos_framework,
        proposal_id,
        voting_power,
        should_pass,
    );

    let record_key &#61; RecordKey &#123;
        stake_pool,
        proposal_id,
    &#125;;
    if (features::partial_governance_voting_enabled()) &#123;
        let voting_records_v2 &#61; borrow_global_mut&lt;VotingRecordsV2&gt;(@aptos_framework);
        let used_voting_power &#61; smart_table::borrow_mut_with_default(&amp;mut voting_records_v2.votes, record_key, 0);
        // This calculation should never overflow because the used voting cannot exceed the total voting power of this stake pool.
        &#42;used_voting_power &#61; &#42;used_voting_power &#43; voting_power;
    &#125; else &#123;
        let voting_records &#61; borrow_global_mut&lt;VotingRecords&gt;(@aptos_framework);
        assert!(
            !table::contains(&amp;voting_records.votes, record_key),
            error::invalid_argument(EALREADY_VOTED));
        table::add(&amp;mut voting_records.votes, record_key, true);
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            Vote &#123;
                proposal_id,
                voter: voter_address,
                stake_pool,
                num_votes: voting_power,
                should_pass,
            &#125;,
        );
    &#125;;
    let events &#61; borrow_global_mut&lt;GovernanceEvents&gt;(@aptos_framework);
    event::emit_event&lt;VoteEvent&gt;(
        &amp;mut events.vote_events,
        VoteEvent &#123;
            proposal_id,
            voter: voter_address,
            stake_pool,
            num_votes: voting_power,
            should_pass,
        &#125;,
    );

    let proposal_state &#61; voting::get_proposal_state&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);
    if (proposal_state &#61;&#61; PROPOSAL_STATE_SUCCEEDED) &#123;
        add_approved_script_hash(proposal_id);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_add_approved_script_hash_script"></a>

## Function `add_approved_script_hash_script`



<pre><code>public entry fun add_approved_script_hash_script(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun add_approved_script_hash_script(proposal_id: u64) acquires ApprovedExecutionHashes &#123;
    add_approved_script_hash(proposal_id)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_add_approved_script_hash"></a>

## Function `add_approved_script_hash`

Add the execution script hash of a successful governance proposal to the approved list.
This is needed to bypass the mempool transaction size limit for approved governance proposal transactions that
are too large (e.g. module upgrades).


<pre><code>public fun add_approved_script_hash(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_approved_script_hash(proposal_id: u64) acquires ApprovedExecutionHashes &#123;
    let approved_hashes &#61; borrow_global_mut&lt;ApprovedExecutionHashes&gt;(@aptos_framework);

    // Ensure the proposal can be resolved.
    let proposal_state &#61; voting::get_proposal_state&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);
    assert!(proposal_state &#61;&#61; PROPOSAL_STATE_SUCCEEDED, error::invalid_argument(EPROPOSAL_NOT_RESOLVABLE_YET));

    let execution_hash &#61; voting::get_execution_hash&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);

    // If this is a multi&#45;step proposal, the proposal id will already exist in the ApprovedExecutionHashes map.
    // We will update execution hash in ApprovedExecutionHashes to be the next_execution_hash.
    if (simple_map::contains_key(&amp;approved_hashes.hashes, &amp;proposal_id)) &#123;
        let current_execution_hash &#61; simple_map::borrow_mut(&amp;mut approved_hashes.hashes, &amp;proposal_id);
        &#42;current_execution_hash &#61; execution_hash;
    &#125; else &#123;
        simple_map::add(&amp;mut approved_hashes.hashes, proposal_id, execution_hash);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_resolve"></a>

## Function `resolve`

Resolve a successful single-step proposal. This would fail if the proposal is not successful (not enough votes or more no
than yes).


<pre><code>public fun resolve(proposal_id: u64, signer_address: address): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resolve(
    proposal_id: u64,
    signer_address: address
): signer acquires ApprovedExecutionHashes, GovernanceResponsbility &#123;
    voting::resolve&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);
    remove_approved_hash(proposal_id);
    get_signer(signer_address)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_resolve_multi_step_proposal"></a>

## Function `resolve_multi_step_proposal`

Resolve a successful multi-step proposal. This would fail if the proposal is not successful.


<pre><code>public fun resolve_multi_step_proposal(proposal_id: u64, signer_address: address, next_execution_hash: vector&lt;u8&gt;): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resolve_multi_step_proposal(
    proposal_id: u64,
    signer_address: address,
    next_execution_hash: vector&lt;u8&gt;
): signer acquires GovernanceResponsbility, ApprovedExecutionHashes &#123;
    voting::resolve_proposal_v2&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id, next_execution_hash);
    // If the current step is the last step of this multi&#45;step proposal,
    // we will remove the execution hash from the ApprovedExecutionHashes map.
    if (vector::length(&amp;next_execution_hash) &#61;&#61; 0) &#123;
        remove_approved_hash(proposal_id);
    &#125; else &#123;
        // If the current step is not the last step of this proposal,
        // we replace the current execution hash with the next execution hash
        // in the ApprovedExecutionHashes map.
        add_approved_script_hash(proposal_id)
    &#125;;
    get_signer(signer_address)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_remove_approved_hash"></a>

## Function `remove_approved_hash`

Remove an approved proposal's execution script hash.


<pre><code>public fun remove_approved_hash(proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_approved_hash(proposal_id: u64) acquires ApprovedExecutionHashes &#123;
    assert!(
        voting::is_resolved&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id),
        error::invalid_argument(EPROPOSAL_NOT_RESOLVED_YET),
    );

    let approved_hashes &#61; &amp;mut borrow_global_mut&lt;ApprovedExecutionHashes&gt;(@aptos_framework).hashes;
    if (simple_map::contains_key(approved_hashes, &amp;proposal_id)) &#123;
        simple_map::remove(approved_hashes, &amp;proposal_id);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_reconfigure"></a>

## Function `reconfigure`

Manually reconfigure. Called at the end of a governance txn that alters on-chain configs.

WARNING: this function always ensures a reconfiguration starts, but when the reconfiguration finishes depends.
- If feature <code>RECONFIGURE_WITH_DKG</code> is disabled, it finishes immediately.
- At the end of the calling transaction, we will be in a new epoch.
- If feature <code>RECONFIGURE_WITH_DKG</code> is enabled, it starts DKG, and the new epoch will start in a block prologue after DKG finishes.

This behavior affects when an update of an on-chain config (e.g. <code>ConsensusConfig</code>, <code>Features</code>) takes effect,
since such updates are applied whenever we enter an new epoch.


<pre><code>public entry fun reconfigure(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reconfigure(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    if (consensus_config::validator_txn_enabled() &amp;&amp; randomness_config::enabled()) &#123;
        reconfiguration_with_dkg::try_start();
    &#125; else &#123;
        reconfiguration_with_dkg::finish(aptos_framework);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_force_end_epoch"></a>

## Function `force_end_epoch`

Change epoch immediately.
If <code>RECONFIGURE_WITH_DKG</code> is enabled and we are in the middle of a DKG,
stop waiting for DKG and enter the new epoch without randomness.

WARNING: currently only used by tests. In most cases you should use <code>reconfigure()</code> instead.
TODO: migrate these tests to be aware of async reconfiguration.


<pre><code>public entry fun force_end_epoch(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun force_end_epoch(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    reconfiguration_with_dkg::finish(aptos_framework);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_force_end_epoch_test_only"></a>

## Function `force_end_epoch_test_only`

<code>force_end_epoch()</code> equivalent but only called in testnet,
where the core resources account exists and has been granted power to mint Aptos coins.


<pre><code>public entry fun force_end_epoch_test_only(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun force_end_epoch_test_only(aptos_framework: &amp;signer) acquires GovernanceResponsbility &#123;
    let core_signer &#61; get_signer_testnet_only(aptos_framework, @0x1);
    system_addresses::assert_aptos_framework(&amp;core_signer);
    reconfiguration_with_dkg::finish(&amp;core_signer);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_toggle_features"></a>

## Function `toggle_features`

Update feature flags and also trigger reconfiguration.


<pre><code>public fun toggle_features(aptos_framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun toggle_features(aptos_framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    features::change_feature_flags_for_next_epoch(aptos_framework, enable, disable);
    reconfigure(aptos_framework);
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_get_signer_testnet_only"></a>

## Function `get_signer_testnet_only`

Only called in testnet where the core resources account exists and has been granted power to mint Aptos coins.


<pre><code>public fun get_signer_testnet_only(core_resources: &amp;signer, signer_address: address): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_signer_testnet_only(
    core_resources: &amp;signer, signer_address: address): signer acquires GovernanceResponsbility &#123;
    system_addresses::assert_core_resource(core_resources);
    // Core resources account only has mint capability in tests/testnets.
    assert!(aptos_coin::has_mint_capability(core_resources), error::unauthenticated(EUNAUTHORIZED));
    get_signer(signer_address)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_get_voting_power"></a>

## Function `get_voting_power`

Return the voting power a stake pool has with respect to governance proposals.


<pre><code>&#35;[view]
public fun get_voting_power(pool_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_voting_power(pool_address: address): u64 &#123;
    let allow_validator_set_change &#61; staking_config::get_allow_validator_set_change(&amp;staking_config::get());
    if (allow_validator_set_change) &#123;
        let (active, _, pending_active, pending_inactive) &#61; stake::get_stake(pool_address);
        // We calculate the voting power as total non&#45;inactive stakes of the pool. Even if the validator is not in the
        // active validator set, as long as they have a lockup (separately checked in create_proposal and voting), their
        // stake would still count in their voting power for governance proposals.
        active &#43; pending_active &#43; pending_inactive
    &#125; else &#123;
        stake::get_current_epoch_voting_power(pool_address)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_get_signer"></a>

## Function `get_signer`

Return a signer for making changes to 0x1 as part of on-chain governance proposal process.


<pre><code>fun get_signer(signer_address: address): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_signer(signer_address: address): signer acquires GovernanceResponsbility &#123;
    let governance_responsibility &#61; borrow_global&lt;GovernanceResponsbility&gt;(@aptos_framework);
    let signer_cap &#61; simple_map::borrow(&amp;governance_responsibility.signer_caps, &amp;signer_address);
    create_signer_with_capability(signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_create_proposal_metadata"></a>

## Function `create_proposal_metadata`



<pre><code>fun create_proposal_metadata(metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_proposal_metadata(
    metadata_location: vector&lt;u8&gt;,
    metadata_hash: vector&lt;u8&gt;
): SimpleMap&lt;String, vector&lt;u8&gt;&gt; &#123;
    assert!(string::length(&amp;utf8(metadata_location)) &lt;&#61; 256, error::invalid_argument(EMETADATA_LOCATION_TOO_LONG));
    assert!(string::length(&amp;utf8(metadata_hash)) &lt;&#61; 256, error::invalid_argument(EMETADATA_HASH_TOO_LONG));

    let metadata &#61; simple_map::create&lt;String, vector&lt;u8&gt;&gt;();
    simple_map::add(&amp;mut metadata, utf8(METADATA_LOCATION_KEY), metadata_location);
    simple_map::add(&amp;mut metadata, utf8(METADATA_HASH_KEY), metadata_hash);
    metadata
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_assert_voting_initialization"></a>

## Function `assert_voting_initialization`



<pre><code>fun assert_voting_initialization()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_voting_initialization() &#123;
    if (features::partial_governance_voting_enabled()) &#123;
        assert!(exists&lt;VotingRecordsV2&gt;(@aptos_framework), error::invalid_state(EPARTIAL_VOTING_NOT_INITIALIZED));
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_aptos_governance_initialize_for_verification"></a>

## Function `initialize_for_verification`



<pre><code>&#35;[verify_only]
public fun initialize_for_verification(aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_for_verification(
    aptos_framework: &amp;signer,
    min_voting_threshold: u128,
    required_proposer_stake: u64,
    voting_duration_secs: u64,
) &#123;
    initialize(aptos_framework, min_voting_threshold, required_proposer_stake, voting_duration_secs);
&#125;
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
<td>The add_approved_script_hash function asserts that proposal_state == PROPOSAL_STATE_SUCCEEDED.</td>
<td>Formally verified in <a href="#high-level-req-4">AddApprovedScriptHash</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_store_signer_cap"></a>

### Function `store_signer_cap`


<pre><code>public fun store_signer_cap(aptos_framework: &amp;signer, signer_address: address, signer_cap: account::SignerCapability)
</code></pre>




<pre><code>aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
aborts_if !system_addresses::is_framework_reserved_address(signer_address);
let signer_caps &#61; global&lt;GovernanceResponsbility&gt;(@aptos_framework).signer_caps;
aborts_if exists&lt;GovernanceResponsbility&gt;(@aptos_framework) &amp;&amp;
    simple_map::spec_contains_key(signer_caps, signer_address);
ensures exists&lt;GovernanceResponsbility&gt;(@aptos_framework);
let post post_signer_caps &#61; global&lt;GovernanceResponsbility&gt;(@aptos_framework).signer_caps;
ensures simple_map::spec_contains_key(post_signer_caps, signer_address);
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>fun initialize(aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)
</code></pre>


Signer address must be @aptos_framework.
The signer does not allow these resources (GovernanceProposal, GovernanceConfig, GovernanceEvents, VotingRecords, ApprovedExecutionHashes) to exist.
The signer must have an Account.
Limit addition overflow.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
let register_account &#61; global&lt;account::Account&gt;(addr);
aborts_if exists&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(addr);
aborts_if !exists&lt;account::Account&gt;(addr);
aborts_if register_account.guid_creation_num &#43; 7 &gt; MAX_U64;
aborts_if register_account.guid_creation_num &#43; 7 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !type_info::spec_is_struct&lt;GovernanceProposal&gt;();
include InitializeAbortIf;
ensures exists&lt;voting::VotingForum&lt;governance_proposal::GovernanceProposal&gt;&gt;(addr);
ensures exists&lt;GovernanceConfig&gt;(addr);
ensures exists&lt;GovernanceEvents&gt;(addr);
ensures exists&lt;VotingRecords&gt;(addr);
ensures exists&lt;ApprovedExecutionHashes&gt;(addr);
</code></pre>



<a id="@Specification_1_update_governance_config"></a>

### Function `update_governance_config`


<pre><code>public fun update_governance_config(aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)
</code></pre>


Signer address must be @aptos_framework.
Address @aptos_framework must exist GovernanceConfig and GovernanceEvents.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
let governance_config &#61; global&lt;GovernanceConfig&gt;(@aptos_framework);
let post new_governance_config &#61; global&lt;GovernanceConfig&gt;(@aptos_framework);
aborts_if addr !&#61; @aptos_framework;
aborts_if !exists&lt;GovernanceConfig&gt;(@aptos_framework);
aborts_if !exists&lt;GovernanceEvents&gt;(@aptos_framework);
modifies global&lt;GovernanceConfig&gt;(addr);
ensures new_governance_config.voting_duration_secs &#61;&#61; voting_duration_secs;
ensures new_governance_config.min_voting_threshold &#61;&#61; min_voting_threshold;
ensures new_governance_config.required_proposer_stake &#61;&#61; required_proposer_stake;
</code></pre>



<a id="@Specification_1_initialize_partial_voting"></a>

### Function `initialize_partial_voting`


<pre><code>public fun initialize_partial_voting(aptos_framework: &amp;signer)
</code></pre>


Signer address must be @aptos_framework.
Abort if structs have already been created.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
aborts_if addr !&#61; @aptos_framework;
aborts_if exists&lt;VotingRecordsV2&gt;(@aptos_framework);
ensures exists&lt;VotingRecordsV2&gt;(@aptos_framework);
</code></pre>




<a id="0x1_aptos_governance_InitializeAbortIf"></a>


<pre><code>schema InitializeAbortIf &#123;
    aptos_framework: &amp;signer;
    min_voting_threshold: u128;
    required_proposer_stake: u64;
    voting_duration_secs: u64;
    let addr &#61; signer::address_of(aptos_framework);
    let account &#61; global&lt;account::Account&gt;(addr);
    aborts_if addr !&#61; @aptos_framework;
    aborts_if exists&lt;voting::VotingForum&lt;governance_proposal::GovernanceProposal&gt;&gt;(addr);
    aborts_if exists&lt;GovernanceConfig&gt;(addr);
    aborts_if exists&lt;GovernanceEvents&gt;(addr);
    aborts_if exists&lt;VotingRecords&gt;(addr);
    aborts_if exists&lt;ApprovedExecutionHashes&gt;(addr);
    aborts_if !exists&lt;account::Account&gt;(addr);
&#125;
</code></pre>



<a id="@Specification_1_get_voting_duration_secs"></a>

### Function `get_voting_duration_secs`


<pre><code>&#35;[view]
public fun get_voting_duration_secs(): u64
</code></pre>




<pre><code>include AbortsIfNotGovernanceConfig;
</code></pre>



<a id="@Specification_1_get_min_voting_threshold"></a>

### Function `get_min_voting_threshold`


<pre><code>&#35;[view]
public fun get_min_voting_threshold(): u128
</code></pre>




<pre><code>include AbortsIfNotGovernanceConfig;
</code></pre>



<a id="@Specification_1_get_required_proposer_stake"></a>

### Function `get_required_proposer_stake`


<pre><code>&#35;[view]
public fun get_required_proposer_stake(): u64
</code></pre>




<pre><code>include AbortsIfNotGovernanceConfig;
</code></pre>




<a id="0x1_aptos_governance_AbortsIfNotGovernanceConfig"></a>


<pre><code>schema AbortsIfNotGovernanceConfig &#123;
    aborts_if !exists&lt;GovernanceConfig&gt;(@aptos_framework);
&#125;
</code></pre>



<a id="@Specification_1_has_entirely_voted"></a>

### Function `has_entirely_voted`


<pre><code>&#35;[view]
public fun has_entirely_voted(stake_pool: address, proposal_id: u64): bool
</code></pre>




<pre><code>aborts_if !exists&lt;VotingRecords&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_get_remaining_voting_power"></a>

### Function `get_remaining_voting_power`


<pre><code>&#35;[view]
public fun get_remaining_voting_power(stake_pool: address, proposal_id: u64): u64
</code></pre>




<pre><code>aborts_if features::spec_partial_governance_voting_enabled() &amp;&amp; !exists&lt;VotingRecordsV2&gt;(@aptos_framework);
include voting::AbortsIfNotContainProposalID&lt;GovernanceProposal&gt; &#123;
    voting_forum_address: @aptos_framework
&#125;;
aborts_if !exists&lt;stake::StakePool&gt;(stake_pool);
aborts_if spec_proposal_expiration &lt;&#61; locked_until &amp;&amp; !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
let spec_proposal_expiration &#61; voting::spec_get_proposal_expiration_secs&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);
let locked_until &#61; global&lt;stake::StakePool&gt;(stake_pool).locked_until_secs;
let remain_zero_1_cond &#61; (spec_proposal_expiration &gt; locked_until &#124;&#124; timestamp::spec_now_seconds() &gt; spec_proposal_expiration);
ensures remain_zero_1_cond &#61;&#61;&gt; result &#61;&#61; 0;
let record_key &#61; RecordKey &#123;
    stake_pool,
    proposal_id,
&#125;;
let entirely_voted &#61; spec_has_entirely_voted(stake_pool, proposal_id, record_key);
aborts_if !remain_zero_1_cond &amp;&amp; !exists&lt;VotingRecords&gt;(@aptos_framework);
include !remain_zero_1_cond &amp;&amp; !entirely_voted &#61;&#61;&gt; GetVotingPowerAbortsIf &#123;
    pool_address: stake_pool
&#125;;
let staking_config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
let voting_power &#61; spec_get_voting_power(stake_pool, staking_config);
let voting_records_v2 &#61; borrow_global&lt;VotingRecordsV2&gt;(@aptos_framework);
let used_voting_power &#61; if (smart_table::spec_contains(voting_records_v2.votes, record_key)) &#123;
    smart_table::spec_get(voting_records_v2.votes, record_key)
&#125; else &#123;
    0
&#125;;
aborts_if !remain_zero_1_cond &amp;&amp; !entirely_voted &amp;&amp; features::spec_partial_governance_voting_enabled() &amp;&amp;
    used_voting_power &gt; 0 &amp;&amp; voting_power &lt; used_voting_power;
ensures result &#61;&#61; spec_get_remaining_voting_power(stake_pool, proposal_id);
</code></pre>




<a id="0x1_aptos_governance_spec_get_remaining_voting_power"></a>


<pre><code>fun spec_get_remaining_voting_power(stake_pool: address, proposal_id: u64): u64 &#123;
   let spec_proposal_expiration &#61; voting::spec_get_proposal_expiration_secs&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);
   let locked_until &#61; global&lt;stake::StakePool&gt;(stake_pool).locked_until_secs;
   let remain_zero_1_cond &#61; (spec_proposal_expiration &gt; locked_until &#124;&#124; timestamp::spec_now_seconds() &gt; spec_proposal_expiration);
   let staking_config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
   let voting_records_v2 &#61; borrow_global&lt;VotingRecordsV2&gt;(@aptos_framework);
   let record_key &#61; RecordKey &#123;
       stake_pool,
       proposal_id,
   &#125;;
   let entirely_voted &#61; spec_has_entirely_voted(stake_pool, proposal_id, record_key);
   let voting_power &#61; spec_get_voting_power(stake_pool, staking_config);
   let used_voting_power &#61; if (smart_table::spec_contains(voting_records_v2.votes, record_key)) &#123;
       smart_table::spec_get(voting_records_v2.votes, record_key)
   &#125; else &#123;
       0
   &#125;;
   if (remain_zero_1_cond) &#123;
       0
   &#125; else if (entirely_voted) &#123;
       0
   &#125; else if (!features::spec_partial_governance_voting_enabled()) &#123;
       voting_power
   &#125; else &#123;
       voting_power &#45; used_voting_power
   &#125;
&#125;
</code></pre>




<a id="0x1_aptos_governance_spec_has_entirely_voted"></a>


<pre><code>fun spec_has_entirely_voted(stake_pool: address, proposal_id: u64, record_key: RecordKey): bool &#123;
   let voting_records &#61; global&lt;VotingRecords&gt;(@aptos_framework);
   table::spec_contains(voting_records.votes, record_key)
&#125;
</code></pre>




<a id="0x1_aptos_governance_GetVotingPowerAbortsIf"></a>


<pre><code>schema GetVotingPowerAbortsIf &#123;
    pool_address: address;
    let staking_config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    aborts_if !exists&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    let allow_validator_set_change &#61; staking_config.allow_validator_set_change;
    let stake_pool_res &#61; global&lt;stake::StakePool&gt;(pool_address);
    aborts_if allow_validator_set_change &amp;&amp; (stake_pool_res.active.value &#43; stake_pool_res.pending_active.value &#43; stake_pool_res.pending_inactive.value) &gt; MAX_U64;
    aborts_if !exists&lt;stake::StakePool&gt;(pool_address);
    aborts_if !allow_validator_set_change &amp;&amp; !exists&lt;stake::ValidatorSet&gt;(@aptos_framework);
    aborts_if !allow_validator_set_change &amp;&amp; stake::spec_is_current_epoch_validator(pool_address) &amp;&amp; stake_pool_res.active.value &#43; stake_pool_res.pending_inactive.value &gt; MAX_U64;
&#125;
</code></pre>



<a id="@Specification_1_create_proposal"></a>

### Function `create_proposal`


<pre><code>public entry fun create_proposal(proposer: &amp;signer, stake_pool: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;)
</code></pre>


The same as spec of <code>create_proposal_v2()</code>.


<pre><code>pragma verify_duration_estimate &#61; 60;
requires chain_status::is_operating();
include CreateProposalAbortsIf;
</code></pre>



<a id="@Specification_1_create_proposal_v2"></a>

### Function `create_proposal_v2`


<pre><code>public entry fun create_proposal_v2(proposer: &amp;signer, stake_pool: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;, is_multi_step_proposal: bool)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 60;
requires chain_status::is_operating();
include CreateProposalAbortsIf;
</code></pre>



<a id="@Specification_1_create_proposal_v2_impl"></a>

### Function `create_proposal_v2_impl`


<pre><code>public fun create_proposal_v2_impl(proposer: &amp;signer, stake_pool: address, execution_hash: vector&lt;u8&gt;, metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;, is_multi_step_proposal: bool): u64
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 60;
requires chain_status::is_operating();
include CreateProposalAbortsIf;
</code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code>public entry fun vote(voter: &amp;signer, stake_pool: address, proposal_id: u64, should_pass: bool)
</code></pre>


stake_pool must exist StakePool.
The delegated voter under the resource StakePool of the stake_pool must be the voter address.
Address @aptos_framework must exist VotingRecords and GovernanceProposal.


<pre><code>pragma verify_duration_estimate &#61; 60;
requires chain_status::is_operating();
include VoteAbortIf  &#123;
    voting_power: MAX_U64
&#125;;
</code></pre>



<a id="@Specification_1_partial_vote"></a>

### Function `partial_vote`


<pre><code>public entry fun partial_vote(voter: &amp;signer, stake_pool: address, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>


stake_pool must exist StakePool.
The delegated voter under the resource StakePool of the stake_pool must be the voter address.
Address @aptos_framework must exist VotingRecords and GovernanceProposal.
Address @aptos_framework must exist VotingRecordsV2 if partial_governance_voting flag is enabled.


<pre><code>pragma verify_duration_estimate &#61; 60;
requires chain_status::is_operating();
include VoteAbortIf;
</code></pre>



<a id="@Specification_1_vote_internal"></a>

### Function `vote_internal`


<pre><code>fun vote_internal(voter: &amp;signer, stake_pool: address, proposal_id: u64, voting_power: u64, should_pass: bool)
</code></pre>


stake_pool must exist StakePool.
The delegated voter under the resource StakePool of the stake_pool must be the voter address.
Address @aptos_framework must exist VotingRecords and GovernanceProposal.
Address @aptos_framework must exist VotingRecordsV2 if partial_governance_voting flag is enabled.


<pre><code>pragma verify_duration_estimate &#61; 60;
requires chain_status::is_operating();
include VoteAbortIf;
</code></pre>




<a id="0x1_aptos_governance_VoteAbortIf"></a>


<pre><code>schema VoteAbortIf &#123;
    voter: &amp;signer;
    stake_pool: address;
    proposal_id: u64;
    should_pass: bool;
    voting_power: u64;
    include VotingGetDelegatedVoterAbortsIf &#123; sign: voter &#125;;
    aborts_if spec_proposal_expiration &lt;&#61; locked_until &amp;&amp; !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
    let spec_proposal_expiration &#61; voting::spec_get_proposal_expiration_secs&lt;GovernanceProposal&gt;(@aptos_framework, proposal_id);
    let locked_until &#61; global&lt;stake::StakePool&gt;(stake_pool).locked_until_secs;
    let remain_zero_1_cond &#61; (spec_proposal_expiration &gt; locked_until &#124;&#124; timestamp::spec_now_seconds() &gt; spec_proposal_expiration);
    let record_key &#61; RecordKey &#123;
        stake_pool,
        proposal_id,
    &#125;;
    let entirely_voted &#61; spec_has_entirely_voted(stake_pool, proposal_id, record_key);
    aborts_if !remain_zero_1_cond &amp;&amp; !exists&lt;VotingRecords&gt;(@aptos_framework);
    include !remain_zero_1_cond &amp;&amp; !entirely_voted &#61;&#61;&gt; GetVotingPowerAbortsIf &#123;
        pool_address: stake_pool
    &#125;;
    let staking_config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
    let spec_voting_power &#61; spec_get_voting_power(stake_pool, staking_config);
    let voting_records_v2 &#61; borrow_global&lt;VotingRecordsV2&gt;(@aptos_framework);
    let used_voting_power &#61; if (smart_table::spec_contains(voting_records_v2.votes, record_key)) &#123;
        smart_table::spec_get(voting_records_v2.votes, record_key)
    &#125; else &#123;
        0
    &#125;;
    aborts_if !remain_zero_1_cond &amp;&amp; !entirely_voted &amp;&amp; features::spec_partial_governance_voting_enabled() &amp;&amp;
        used_voting_power &gt; 0 &amp;&amp; spec_voting_power &lt; used_voting_power;
    let remaining_power &#61; spec_get_remaining_voting_power(stake_pool, proposal_id);
    let real_voting_power &#61;  min(voting_power, remaining_power);
    aborts_if !(real_voting_power &gt; 0);
    aborts_if !exists&lt;VotingRecords&gt;(@aptos_framework);
    let voting_records &#61; global&lt;VotingRecords&gt;(@aptos_framework);
    let allow_validator_set_change &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework).allow_validator_set_change;
    let stake_pool_res &#61; global&lt;stake::StakePool&gt;(stake_pool);
    aborts_if !exists&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
    let voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
    let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
    aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
    let proposal_expiration &#61; proposal.expiration_secs;
    let locked_until_secs &#61; global&lt;stake::StakePool&gt;(stake_pool).locked_until_secs;
    aborts_if proposal_expiration &gt; locked_until_secs;
    aborts_if timestamp::now_seconds() &gt; proposal_expiration;
    aborts_if proposal.is_resolved;
    aborts_if !string::spec_internal_check_utf8(voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
    let execution_key &#61; utf8(voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
    aborts_if simple_map::spec_contains_key(proposal.metadata, execution_key) &amp;&amp;
              simple_map::spec_get(proposal.metadata, execution_key) !&#61; std::bcs::to_bytes(false);
    aborts_if
        if (should_pass) &#123; proposal.yes_votes &#43; real_voting_power &gt; MAX_U128 &#125; else &#123; proposal.no_votes &#43; real_voting_power &gt; MAX_U128 &#125;;
    let post post_voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
    let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);
    aborts_if !string::spec_internal_check_utf8(voting::RESOLVABLE_TIME_METADATA_KEY);
    let key &#61; utf8(voting::RESOLVABLE_TIME_METADATA_KEY);
    ensures simple_map::spec_contains_key(post_proposal.metadata, key);
    ensures simple_map::spec_get(post_proposal.metadata, key) &#61;&#61; std::bcs::to_bytes(timestamp::now_seconds());
    aborts_if features::spec_partial_governance_voting_enabled() &amp;&amp; used_voting_power &#43; real_voting_power &gt; MAX_U64;
    aborts_if !features::spec_partial_governance_voting_enabled() &amp;&amp; table::spec_contains(voting_records.votes, record_key);
    aborts_if !exists&lt;GovernanceEvents&gt;(@aptos_framework);
    let early_resolution_threshold &#61; option::spec_borrow(proposal.early_resolution_vote_threshold);
    let is_voting_period_over &#61; timestamp::spec_now_seconds() &gt; proposal_expiration;
    let new_proposal_yes_votes_0 &#61; proposal.yes_votes &#43; real_voting_power;
    let can_be_resolved_early_0 &#61; option::spec_is_some(proposal.early_resolution_vote_threshold) &amp;&amp;
                                (new_proposal_yes_votes_0 &gt;&#61; early_resolution_threshold &#124;&#124;
                                 proposal.no_votes &gt;&#61; early_resolution_threshold);
    let is_voting_closed_0 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_0;
    let proposal_state_successed_0 &#61; is_voting_closed_0 &amp;&amp; new_proposal_yes_votes_0 &gt; proposal.no_votes &amp;&amp;
                                     new_proposal_yes_votes_0 &#43; proposal.no_votes &gt;&#61; proposal.min_vote_threshold;
    let new_proposal_no_votes_0 &#61; proposal.no_votes &#43; real_voting_power;
    let can_be_resolved_early_1 &#61; option::spec_is_some(proposal.early_resolution_vote_threshold) &amp;&amp;
                                (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;
                                 new_proposal_no_votes_0 &gt;&#61; early_resolution_threshold);
    let is_voting_closed_1 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_1;
    let proposal_state_successed_1 &#61; is_voting_closed_1 &amp;&amp; proposal.yes_votes &gt; new_proposal_no_votes_0 &amp;&amp;
                                     proposal.yes_votes &#43; new_proposal_no_votes_0 &gt;&#61; proposal.min_vote_threshold;
    let new_proposal_yes_votes_1 &#61; proposal.yes_votes &#43; real_voting_power;
    let can_be_resolved_early_2 &#61; option::spec_is_some(proposal.early_resolution_vote_threshold) &amp;&amp;
                                (new_proposal_yes_votes_1 &gt;&#61; early_resolution_threshold &#124;&#124;
                                 proposal.no_votes &gt;&#61; early_resolution_threshold);
    let is_voting_closed_2 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_2;
    let proposal_state_successed_2 &#61; is_voting_closed_2 &amp;&amp; new_proposal_yes_votes_1 &gt; proposal.no_votes &amp;&amp;
                                     new_proposal_yes_votes_1 &#43; proposal.no_votes &gt;&#61; proposal.min_vote_threshold;
    let new_proposal_no_votes_1 &#61; proposal.no_votes &#43; real_voting_power;
    let can_be_resolved_early_3 &#61; option::spec_is_some(proposal.early_resolution_vote_threshold) &amp;&amp;
                                (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;
                                 new_proposal_no_votes_1 &gt;&#61; early_resolution_threshold);
    let is_voting_closed_3 &#61; is_voting_period_over &#124;&#124; can_be_resolved_early_3;
    let proposal_state_successed_3 &#61; is_voting_closed_3 &amp;&amp; proposal.yes_votes &gt; new_proposal_no_votes_1 &amp;&amp;
                                     proposal.yes_votes &#43; new_proposal_no_votes_1 &gt;&#61; proposal.min_vote_threshold;
    let post can_be_resolved_early &#61; option::spec_is_some(proposal.early_resolution_vote_threshold) &amp;&amp;
                                (post_proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;
                                 post_proposal.no_votes &gt;&#61; early_resolution_threshold);
    let post is_voting_closed &#61; is_voting_period_over &#124;&#124; can_be_resolved_early;
    let post proposal_state_successed &#61; is_voting_closed &amp;&amp; post_proposal.yes_votes &gt; post_proposal.no_votes &amp;&amp;
                                     post_proposal.yes_votes &#43; post_proposal.no_votes &gt;&#61; proposal.min_vote_threshold;
    let execution_hash &#61; proposal.execution_hash;
    let post post_approved_hashes &#61; global&lt;ApprovedExecutionHashes&gt;(@aptos_framework);
    // This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    aborts_if
        if (should_pass) &#123;
            proposal_state_successed_0 &amp;&amp; !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework)
        &#125; else &#123;
            proposal_state_successed_1 &amp;&amp; !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework)
        &#125;;
    aborts_if
        if (should_pass) &#123;
            proposal_state_successed_2 &amp;&amp; !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework)
        &#125; else &#123;
            proposal_state_successed_3 &amp;&amp; !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework)
        &#125;;
    ensures proposal_state_successed &#61;&#61;&gt; simple_map::spec_contains_key(post_approved_hashes.hashes, proposal_id) &amp;&amp;
                                         simple_map::spec_get(post_approved_hashes.hashes, proposal_id) &#61;&#61; execution_hash;
    aborts_if features::spec_partial_governance_voting_enabled() &amp;&amp; !exists&lt;VotingRecordsV2&gt;(@aptos_framework);
&#125;
</code></pre>



<a id="@Specification_1_add_approved_script_hash_script"></a>

### Function `add_approved_script_hash_script`


<pre><code>public entry fun add_approved_script_hash_script(proposal_id: u64)
</code></pre>




<pre><code>requires chain_status::is_operating();
include AddApprovedScriptHash;
</code></pre>




<a id="0x1_aptos_governance_AddApprovedScriptHash"></a>


<pre><code>schema AddApprovedScriptHash &#123;
    proposal_id: u64;
    aborts_if !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework);
    aborts_if !exists&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
    let voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
    let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
    aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
    let early_resolution_threshold &#61; option::spec_borrow(proposal.early_resolution_vote_threshold);
    aborts_if timestamp::now_seconds() &lt;&#61; proposal.expiration_secs &amp;&amp;
        (option::spec_is_none(proposal.early_resolution_vote_threshold) &#124;&#124;
        proposal.yes_votes &lt; early_resolution_threshold &amp;&amp; proposal.no_votes &lt; early_resolution_threshold);
    aborts_if (timestamp::now_seconds() &gt; proposal.expiration_secs &#124;&#124;
        option::spec_is_some(proposal.early_resolution_vote_threshold) &amp;&amp; (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;
                                                                           proposal.no_votes &gt;&#61; early_resolution_threshold)) &amp;&amp;
        (proposal.yes_votes &lt;&#61; proposal.no_votes &#124;&#124; proposal.yes_votes &#43; proposal.no_votes &lt; proposal.min_vote_threshold);
    let post post_approved_hashes &#61; global&lt;ApprovedExecutionHashes&gt;(@aptos_framework);
    // This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
    ensures simple_map::spec_contains_key(post_approved_hashes.hashes, proposal_id) &amp;&amp;
        simple_map::spec_get(post_approved_hashes.hashes, proposal_id) &#61;&#61; proposal.execution_hash;
&#125;
</code></pre>



<a id="@Specification_1_add_approved_script_hash"></a>

### Function `add_approved_script_hash`


<pre><code>public fun add_approved_script_hash(proposal_id: u64)
</code></pre>




<pre><code>requires chain_status::is_operating();
include AddApprovedScriptHash;
</code></pre>



<a id="@Specification_1_resolve"></a>

### Function `resolve`


<pre><code>public fun resolve(proposal_id: u64, signer_address: address): signer
</code></pre>


Address @aptos_framework must exist ApprovedExecutionHashes and GovernanceProposal and GovernanceResponsbility.


<pre><code>requires chain_status::is_operating();
include VotingIsProposalResolvableAbortsif;
let voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
let multi_step_key &#61; utf8(voting::IS_MULTI_STEP_PROPOSAL_KEY);
let has_multi_step_key &#61; simple_map::spec_contains_key(proposal.metadata, multi_step_key);
let is_multi_step_proposal &#61; aptos_std::from_bcs::deserialize&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));
aborts_if has_multi_step_key &amp;&amp; !aptos_std::from_bcs::deserializable&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));
aborts_if !string::spec_internal_check_utf8(voting::IS_MULTI_STEP_PROPOSAL_KEY);
aborts_if has_multi_step_key &amp;&amp; is_multi_step_proposal;
let post post_voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);
ensures post_proposal.is_resolved &#61;&#61; true &amp;&amp; post_proposal.resolution_time_secs &#61;&#61; timestamp::now_seconds();
aborts_if option::spec_is_none(proposal.execution_content);
aborts_if !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework);
let post post_approved_hashes &#61; global&lt;ApprovedExecutionHashes&gt;(@aptos_framework).hashes;
ensures !simple_map::spec_contains_key(post_approved_hashes, proposal_id);
include GetSignerAbortsIf;
let governance_responsibility &#61; global&lt;GovernanceResponsbility&gt;(@aptos_framework);
let signer_cap &#61; simple_map::spec_get(governance_responsibility.signer_caps, signer_address);
let addr &#61; signer_cap.account;
ensures signer::address_of(result) &#61;&#61; addr;
</code></pre>



<a id="@Specification_1_resolve_multi_step_proposal"></a>

### Function `resolve_multi_step_proposal`


<pre><code>public fun resolve_multi_step_proposal(proposal_id: u64, signer_address: address, next_execution_hash: vector&lt;u8&gt;): signer
</code></pre>




<pre><code>requires chain_status::is_operating();
pragma verify_duration_estimate &#61; 120;
include VotingIsProposalResolvableAbortsif;
let voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
let post post_voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);
aborts_if !string::spec_internal_check_utf8(voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
let multi_step_in_execution_key &#61; utf8(voting::IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
let post is_multi_step_proposal_in_execution_value &#61; simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key);
aborts_if !string::spec_internal_check_utf8(voting::IS_MULTI_STEP_PROPOSAL_KEY);
let multi_step_key &#61; utf8(voting::IS_MULTI_STEP_PROPOSAL_KEY);
aborts_if simple_map::spec_contains_key(proposal.metadata, multi_step_key) &amp;&amp;
    !aptos_std::from_bcs::deserializable&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));
let is_multi_step &#61; simple_map::spec_contains_key(proposal.metadata, multi_step_key) &amp;&amp;
                    aptos_std::from_bcs::deserialize&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));
let next_execution_hash_is_empty &#61; len(next_execution_hash) &#61;&#61; 0;
aborts_if !is_multi_step &amp;&amp; !next_execution_hash_is_empty;
aborts_if next_execution_hash_is_empty &amp;&amp; is_multi_step &amp;&amp; !simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key);
ensures next_execution_hash_is_empty &#61;&#61;&gt; post_proposal.is_resolved &#61;&#61; true &amp;&amp; post_proposal.resolution_time_secs &#61;&#61; timestamp::spec_now_seconds() &amp;&amp;
    if (is_multi_step) &#123;
        is_multi_step_proposal_in_execution_value &#61;&#61; std::bcs::serialize(false)
    &#125; else &#123;
        simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key) &#61;&#61;&gt;
            is_multi_step_proposal_in_execution_value &#61;&#61; std::bcs::serialize(true)
    &#125;;
ensures !next_execution_hash_is_empty &#61;&#61;&gt; post_proposal.execution_hash &#61;&#61; next_execution_hash;
aborts_if !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework);
let post post_approved_hashes &#61; global&lt;ApprovedExecutionHashes&gt;(@aptos_framework).hashes;
ensures next_execution_hash_is_empty &#61;&#61;&gt; !simple_map::spec_contains_key(post_approved_hashes, proposal_id);
ensures !next_execution_hash_is_empty &#61;&#61;&gt;
    simple_map::spec_get(post_approved_hashes, proposal_id) &#61;&#61; next_execution_hash;
include GetSignerAbortsIf;
let governance_responsibility &#61; global&lt;GovernanceResponsbility&gt;(@aptos_framework);
let signer_cap &#61; simple_map::spec_get(governance_responsibility.signer_caps, signer_address);
let addr &#61; signer_cap.account;
ensures signer::address_of(result) &#61;&#61; addr;
</code></pre>




<a id="0x1_aptos_governance_VotingIsProposalResolvableAbortsif"></a>


<pre><code>schema VotingIsProposalResolvableAbortsif &#123;
    proposal_id: u64;
    aborts_if !exists&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
    let voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
    let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
    aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
    let early_resolution_threshold &#61; option::spec_borrow(proposal.early_resolution_vote_threshold);
    let voting_period_over &#61; timestamp::now_seconds() &gt; proposal.expiration_secs;
    let be_resolved_early &#61; option::spec_is_some(proposal.early_resolution_vote_threshold) &amp;&amp;
                                (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124;
                                 proposal.no_votes &gt;&#61; early_resolution_threshold);
    let voting_closed &#61; voting_period_over &#124;&#124; be_resolved_early;
    aborts_if voting_closed &amp;&amp; (proposal.yes_votes &lt;&#61; proposal.no_votes &#124;&#124; proposal.yes_votes &#43; proposal.no_votes &lt; proposal.min_vote_threshold);
    aborts_if !voting_closed;
    aborts_if proposal.is_resolved;
    aborts_if !string::spec_internal_check_utf8(voting::RESOLVABLE_TIME_METADATA_KEY);
    aborts_if !simple_map::spec_contains_key(proposal.metadata, utf8(voting::RESOLVABLE_TIME_METADATA_KEY));
    let resolvable_time &#61; aptos_std::from_bcs::deserialize&lt;u64&gt;(simple_map::spec_get(proposal.metadata, utf8(voting::RESOLVABLE_TIME_METADATA_KEY)));
    aborts_if !aptos_std::from_bcs::deserializable&lt;u64&gt;(simple_map::spec_get(proposal.metadata, utf8(voting::RESOLVABLE_TIME_METADATA_KEY)));
    aborts_if timestamp::now_seconds() &lt;&#61; resolvable_time;
    aborts_if aptos_framework::transaction_context::spec_get_script_hash() !&#61; proposal.execution_hash;
&#125;
</code></pre>



<a id="@Specification_1_remove_approved_hash"></a>

### Function `remove_approved_hash`


<pre><code>public fun remove_approved_hash(proposal_id: u64)
</code></pre>


Address @aptos_framework must exist ApprovedExecutionHashes and GovernanceProposal.


<pre><code>aborts_if !exists&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
aborts_if !exists&lt;ApprovedExecutionHashes&gt;(@aptos_framework);
let voting_forum &#61; global&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
aborts_if !exists&lt;voting::VotingForum&lt;GovernanceProposal&gt;&gt;(@aptos_framework);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
aborts_if !proposal.is_resolved;
let post approved_hashes &#61; global&lt;ApprovedExecutionHashes&gt;(@aptos_framework).hashes;
ensures !simple_map::spec_contains_key(approved_hashes, proposal_id);
</code></pre>



<a id="@Specification_1_reconfigure"></a>

### Function `reconfigure`


<pre><code>public entry fun reconfigure(aptos_framework: &amp;signer)
</code></pre>




<pre><code>pragma verify &#61; false;
aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
include reconfiguration_with_dkg::FinishRequirement &#123;
    framework: aptos_framework
&#125;;
include stake::GetReconfigStartTimeRequirement;
include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
requires chain_status::is_operating();
requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
requires exists&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);
include staking_config::StakingRewardsConfigRequirement;
</code></pre>



<a id="@Specification_1_force_end_epoch"></a>

### Function `force_end_epoch`


<pre><code>public entry fun force_end_epoch(aptos_framework: &amp;signer)
</code></pre>




<pre><code>pragma verify &#61; false;
let address &#61; signer::address_of(aptos_framework);
include reconfiguration_with_dkg::FinishRequirement &#123;
    framework: aptos_framework
&#125;;
</code></pre>




<a id="0x1_aptos_governance_VotingInitializationAbortIfs"></a>


<pre><code>schema VotingInitializationAbortIfs &#123;
    aborts_if features::spec_partial_governance_voting_enabled() &amp;&amp; !exists&lt;VotingRecordsV2&gt;(@aptos_framework);
&#125;
</code></pre>



<a id="@Specification_1_force_end_epoch_test_only"></a>

### Function `force_end_epoch_test_only`


<pre><code>public entry fun force_end_epoch_test_only(aptos_framework: &amp;signer)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_toggle_features"></a>

### Function `toggle_features`


<pre><code>public fun toggle_features(aptos_framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)
</code></pre>


Signer address must be @aptos_framework.
Address @aptos_framework must exist GovernanceConfig and GovernanceEvents.


<pre><code>pragma verify &#61; false;
let addr &#61; signer::address_of(aptos_framework);
aborts_if addr !&#61; @aptos_framework;
include reconfiguration_with_dkg::FinishRequirement &#123;
    framework: aptos_framework
&#125;;
include stake::GetReconfigStartTimeRequirement;
include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
requires chain_status::is_operating();
requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
requires exists&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);
include staking_config::StakingRewardsConfigRequirement;
</code></pre>



<a id="@Specification_1_get_signer_testnet_only"></a>

### Function `get_signer_testnet_only`


<pre><code>public fun get_signer_testnet_only(core_resources: &amp;signer, signer_address: address): signer
</code></pre>


Signer address must be @core_resources.
signer must exist in MintCapStore.
Address @aptos_framework must exist GovernanceResponsbility.


<pre><code>aborts_if signer::address_of(core_resources) !&#61; @core_resources;
aborts_if !exists&lt;aptos_coin::MintCapStore&gt;(signer::address_of(core_resources));
include GetSignerAbortsIf;
</code></pre>



<a id="@Specification_1_get_voting_power"></a>

### Function `get_voting_power`


<pre><code>&#35;[view]
public fun get_voting_power(pool_address: address): u64
</code></pre>


Address @aptos_framework must exist StakingConfig.
limit addition overflow.
pool_address must exist in StakePool.


<pre><code>include GetVotingPowerAbortsIf;
let staking_config &#61; global&lt;staking_config::StakingConfig&gt;(@aptos_framework);
let allow_validator_set_change &#61; staking_config.allow_validator_set_change;
let stake_pool_res &#61; global&lt;stake::StakePool&gt;(pool_address);
ensures allow_validator_set_change &#61;&#61;&gt; result &#61;&#61; stake_pool_res.active.value &#43; stake_pool_res.pending_active.value &#43; stake_pool_res.pending_inactive.value;
ensures !allow_validator_set_change &#61;&#61;&gt; if (stake::spec_is_current_epoch_validator(pool_address)) &#123;
    result &#61;&#61; stake_pool_res.active.value &#43; stake_pool_res.pending_inactive.value
&#125; else &#123;
    result &#61;&#61; 0
&#125;;
ensures result &#61;&#61; spec_get_voting_power(pool_address, staking_config);
</code></pre>




<a id="0x1_aptos_governance_spec_get_voting_power"></a>


<pre><code>fun spec_get_voting_power(pool_address: address, staking_config: staking_config::StakingConfig): u64 &#123;
   let allow_validator_set_change &#61; staking_config.allow_validator_set_change;
   let stake_pool_res &#61; global&lt;stake::StakePool&gt;(pool_address);
   if (allow_validator_set_change) &#123;
       stake_pool_res.active.value &#43; stake_pool_res.pending_active.value &#43; stake_pool_res.pending_inactive.value
   &#125; else if (!allow_validator_set_change &amp;&amp; (stake::spec_is_current_epoch_validator(pool_address))) &#123;
       stake_pool_res.active.value &#43; stake_pool_res.pending_inactive.value
   &#125; else &#123;
       0
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_get_signer"></a>

### Function `get_signer`


<pre><code>fun get_signer(signer_address: address): signer
</code></pre>




<pre><code>include GetSignerAbortsIf;
</code></pre>




<a id="0x1_aptos_governance_GetSignerAbortsIf"></a>


<pre><code>schema GetSignerAbortsIf &#123;
    signer_address: address;
    aborts_if !exists&lt;GovernanceResponsbility&gt;(@aptos_framework);
    let cap_map &#61; global&lt;GovernanceResponsbility&gt;(@aptos_framework).signer_caps;
    aborts_if !simple_map::spec_contains_key(cap_map, signer_address);
&#125;
</code></pre>



<a id="@Specification_1_create_proposal_metadata"></a>

### Function `create_proposal_metadata`


<pre><code>fun create_proposal_metadata(metadata_location: vector&lt;u8&gt;, metadata_hash: vector&lt;u8&gt;): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;
</code></pre>




<pre><code>include CreateProposalMetadataAbortsIf;
</code></pre>




<a id="0x1_aptos_governance_CreateProposalMetadataAbortsIf"></a>


<pre><code>schema CreateProposalMetadataAbortsIf &#123;
    metadata_location: vector&lt;u8&gt;;
    metadata_hash: vector&lt;u8&gt;;
    aborts_if string::length(utf8(metadata_location)) &gt; 256;
    aborts_if string::length(utf8(metadata_hash)) &gt; 256;
    aborts_if !string::spec_internal_check_utf8(metadata_location);
    aborts_if !string::spec_internal_check_utf8(metadata_hash);
    aborts_if !string::spec_internal_check_utf8(METADATA_LOCATION_KEY);
    aborts_if !string::spec_internal_check_utf8(METADATA_HASH_KEY);
&#125;
</code></pre>



<a id="@Specification_1_assert_voting_initialization"></a>

### Function `assert_voting_initialization`


<pre><code>fun assert_voting_initialization()
</code></pre>




<pre><code>include VotingInitializationAbortIfs;
</code></pre>



<a id="@Specification_1_initialize_for_verification"></a>

### Function `initialize_for_verification`


<pre><code>&#35;[verify_only]
public fun initialize_for_verification(aptos_framework: &amp;signer, min_voting_threshold: u128, required_proposer_stake: u64, voting_duration_secs: u64)
</code></pre>


verify_only


<pre><code>pragma verify &#61; false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
