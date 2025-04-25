
<a id="0x1_supra_governance"></a>

# Module `0x1::supra_governance`


SupraGovernance represents the on-chain governance of the Supra network. Voting power is calculated based on the
current epoch's voting power of the proposer or voter's backing stake pool. In addition, for it to count,
the stake pool's lockup needs to be at least as long as the proposal's duration.

It provides the following flow:
1. Proposers can create a proposal by calling SupraGovernance::create_proposal. The proposer's backing stake pool
needs to have the minimum proposer stake required. Off-chain components can subscribe to CreateProposalEvent to
track proposal creation and proposal ids.
2. Voters can vote on a proposal. Their voting power is derived from the backing stake pool. A stake pool can vote
on a proposal multiple times as long as the total voting power of these votes doesn't exceed its total voting power.


-  [Resource `GovernanceResponsbility`](#0x1_supra_governance_GovernanceResponsbility)
-  [Resource `ApprovedExecutionHashes`](#0x1_supra_governance_ApprovedExecutionHashes)
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
-  [Function `initialize`](#0x1_supra_governance_initialize)
-  [Function `update_supra_governance_config`](#0x1_supra_governance_update_supra_governance_config)
-  [Function `get_voting_duration_secs`](#0x1_supra_governance_get_voting_duration_secs)
-  [Function `get_min_voting_threshold`](#0x1_supra_governance_get_min_voting_threshold)
-  [Function `get_voters_list`](#0x1_supra_governance_get_voters_list)
-  [Function `supra_create_proposal`](#0x1_supra_governance_supra_create_proposal)
-  [Function `supra_create_proposal_v2`](#0x1_supra_governance_supra_create_proposal_v2)
-  [Function `supra_create_proposal_v2_impl`](#0x1_supra_governance_supra_create_proposal_v2_impl)
-  [Function `supra_vote`](#0x1_supra_governance_supra_vote)
-  [Function `supra_vote_internal`](#0x1_supra_governance_supra_vote_internal)
-  [Function `add_supra_approved_script_hash_script`](#0x1_supra_governance_add_supra_approved_script_hash_script)
-  [Function `add_supra_approved_script_hash`](#0x1_supra_governance_add_supra_approved_script_hash)
-  [Function `supra_resolve`](#0x1_supra_governance_supra_resolve)
-  [Function `resolve_supra_multi_step_proposal`](#0x1_supra_governance_resolve_supra_multi_step_proposal)
-  [Function `remove_supra_approved_hash`](#0x1_supra_governance_remove_supra_approved_hash)
-  [Function `reconfigure`](#0x1_supra_governance_reconfigure)
-  [Function `force_end_epoch`](#0x1_supra_governance_force_end_epoch)
-  [Function `force_end_epoch_test_only`](#0x1_supra_governance_force_end_epoch_test_only)
-  [Function `toggle_features`](#0x1_supra_governance_toggle_features)
-  [Function `get_signer_testnet_only`](#0x1_supra_governance_get_signer_testnet_only)
-  [Function `get_signer`](#0x1_supra_governance_get_signer)
-  [Function `create_proposal_metadata`](#0x1_supra_governance_create_proposal_metadata)
-  [Function `initialize_for_verification`](#0x1_supra_governance_initialize_for_verification)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="consensus_config.md#0x1_consensus_config">0x1::consensus_config</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="governance_proposal.md#0x1_governance_proposal">0x1::governance_proposal</a>;
<b>use</b> <a href="multisig_voting.md#0x1_multisig_voting">0x1::multisig_voting</a>;
<b>use</b> <a href="randomness_config.md#0x1_randomness_config">0x1::randomness_config</a>;
<b>use</b> <a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg">0x1::reconfiguration_with_dkg</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
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


<a id="0x1_supra_governance_ETHRESHOLD_EXCEEDS_VOTERS"></a>

Threshold should not exceeds voters


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_EXCEEDS_VOTERS">ETHRESHOLD_EXCEEDS_VOTERS</a>: u64 = 17;
</code></pre>



<a id="0x1_supra_governance_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE"></a>

Threshold value must be greater than 1


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE">ETHRESHOLD_MUST_BE_GREATER_THAN_ONE</a>: u64 = 18;
</code></pre>



<a id="0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED"></a>

This matches the same enum const in voting. We have to duplicate it as Move doesn't have support for enums yet.


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>: u64 = 1;
</code></pre>



<a id="0x1_supra_governance_EACCOUNT_NOT_AUTHORIZED"></a>

The account does not have permission to propose or vote


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EACCOUNT_NOT_AUTHORIZED">EACCOUNT_NOT_AUTHORIZED</a>: u64 = 15;
</code></pre>



<a id="0x1_supra_governance_EMETADATA_HASH_TOO_LONG"></a>

Metadata hash cannot be longer than 256 chars


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EMETADATA_HASH_TOO_LONG">EMETADATA_HASH_TOO_LONG</a>: u64 = 10;
</code></pre>



<a id="0x1_supra_governance_EMETADATA_LOCATION_TOO_LONG"></a>

Metadata location cannot be longer than 256 chars


<pre><code><b>const</b> <a href="supra_governance.md#0x1_supra_governance_EMETADATA_LOCATION_TOO_LONG">EMETADATA_LOCATION_TOO_LONG</a>: u64 = 9;
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

<a id="0x1_supra_governance_initialize"></a>

## Function `initialize`

Initializes the state for Supra Governance. Can only be called during Genesis with a signer
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
SupraGovernance.


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
where the core resources account exists and has been granted power to mint Supra coins.


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

Only called in testnet where the core resources account exists and has been granted power to mint Supra coins.


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
    <a href="supra_governance.md#0x1_supra_governance_initialize">initialize</a>(supra_framework, voting_duration_secs, supra_min_voting_threshold, voters);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
