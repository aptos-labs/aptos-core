
<a id="0x1_multisig_voting"></a>

# Module `0x1::multisig_voting`


This is the general Multisig Voting module that can be used as part of a DAO Governance. Multisig Voting is designed to be used by
calculation and including proper capabilities when creating the proposal so resolution can go through.
On-chain governance of the Supra network also uses Multisig Voting.

The Multisig voting flow:
1. The Multisig Voting module can be deployed at a known address (e.g. 0x1 for Supra on-chain governance)
2. The governance module, e.g. SupraGovernance, can be deployed later and define a GovernanceProposal resource type
that can also contain other information such as Capability resource for authorization.
3. The governance module's owner can then register the ProposalType with Multisig Voting. This also hosts the proposal list
(forum) on the calling account.
4. A proposer, through the governance module, can call multisig_voting::create_proposal to create a proposal. create_proposal
cannot be called directly not through the governance module. A script hash of the resolution script that can later
be called to execute the proposal is required.
5. A voter, through the governance module, can call multisig_voting::vote on a proposal. vote requires passing a &ProposalType
and thus only the governance module that registers ProposalType can call vote.
6. Once the proposal's expiration time has passed and more than the defined threshold has voted yes on the proposal,
anyone can call resolve which returns the content of the proposal (of type ProposalType) that can be used to execute.
7. Only the resolution script with the same script hash specified in the proposal can call multisig_voting::resolve as part of
the resolution process.


-  [Struct `Proposal`](#0x1_multisig_voting_Proposal)
-  [Resource `VotingForum`](#0x1_multisig_voting_VotingForum)
-  [Struct `VotingEvents`](#0x1_multisig_voting_VotingEvents)
-  [Struct `CreateProposal`](#0x1_multisig_voting_CreateProposal)
-  [Struct `RegisterForum`](#0x1_multisig_voting_RegisterForum)
-  [Struct `Vote`](#0x1_multisig_voting_Vote)
-  [Struct `ResolveProposal`](#0x1_multisig_voting_ResolveProposal)
-  [Struct `CreateProposalEvent`](#0x1_multisig_voting_CreateProposalEvent)
-  [Struct `RegisterForumEvent`](#0x1_multisig_voting_RegisterForumEvent)
-  [Struct `VoteEvent`](#0x1_multisig_voting_VoteEvent)
-  [Constants](#@Constants_0)
-  [Function `register`](#0x1_multisig_voting_register)
-  [Function `create_proposal`](#0x1_multisig_voting_create_proposal)
-  [Function `create_proposal_v2`](#0x1_multisig_voting_create_proposal_v2)
-  [Function `vote`](#0x1_multisig_voting_vote)
-  [Function `is_proposal_resolvable`](#0x1_multisig_voting_is_proposal_resolvable)
-  [Function `resolve`](#0x1_multisig_voting_resolve)
-  [Function `resolve_proposal_v2`](#0x1_multisig_voting_resolve_proposal_v2)
-  [Function `next_proposal_id`](#0x1_multisig_voting_next_proposal_id)
-  [Function `get_proposer`](#0x1_multisig_voting_get_proposer)
-  [Function `is_voting_closed`](#0x1_multisig_voting_is_voting_closed)
-  [Function `can_be_resolved_early`](#0x1_multisig_voting_can_be_resolved_early)
-  [Function `get_proposal_metadata`](#0x1_multisig_voting_get_proposal_metadata)
-  [Function `get_proposal_metadata_value`](#0x1_multisig_voting_get_proposal_metadata_value)
-  [Function `get_proposal_state`](#0x1_multisig_voting_get_proposal_state)
-  [Function `get_proposal_creation_secs`](#0x1_multisig_voting_get_proposal_creation_secs)
-  [Function `get_proposal_expiration_secs`](#0x1_multisig_voting_get_proposal_expiration_secs)
-  [Function `get_execution_hash`](#0x1_multisig_voting_get_execution_hash)
-  [Function `get_min_vote_threshold`](#0x1_multisig_voting_get_min_vote_threshold)
-  [Function `get_votes`](#0x1_multisig_voting_get_votes)
-  [Function `is_resolved`](#0x1_multisig_voting_is_resolved)
-  [Function `get_resolution_time_secs`](#0x1_multisig_voting_get_resolution_time_secs)
-  [Function `is_multi_step_proposal_in_execution`](#0x1_multisig_voting_is_multi_step_proposal_in_execution)
-  [Function `is_voting_period_over`](#0x1_multisig_voting_is_voting_period_over)
-  [Function `get_proposal`](#0x1_multisig_voting_get_proposal)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `register`](#@Specification_1_register)
    -  [Function `create_proposal`](#@Specification_1_create_proposal)
    -  [Function `create_proposal_v2`](#@Specification_1_create_proposal_v2)
    -  [Function `vote`](#@Specification_1_vote)
    -  [Function `is_proposal_resolvable`](#@Specification_1_is_proposal_resolvable)
    -  [Function `resolve`](#@Specification_1_resolve)
    -  [Function `resolve_proposal_v2`](#@Specification_1_resolve_proposal_v2)
    -  [Function `next_proposal_id`](#@Specification_1_next_proposal_id)
    -  [Function `get_proposer`](#@Specification_1_get_proposer)
    -  [Function `is_voting_closed`](#@Specification_1_is_voting_closed)
    -  [Function `can_be_resolved_early`](#@Specification_1_can_be_resolved_early)
    -  [Function `get_proposal_metadata`](#@Specification_1_get_proposal_metadata)
    -  [Function `get_proposal_metadata_value`](#@Specification_1_get_proposal_metadata_value)
    -  [Function `get_proposal_state`](#@Specification_1_get_proposal_state)
    -  [Function `get_proposal_creation_secs`](#@Specification_1_get_proposal_creation_secs)
    -  [Function `get_proposal_expiration_secs`](#@Specification_1_get_proposal_expiration_secs)
    -  [Function `get_execution_hash`](#@Specification_1_get_execution_hash)
    -  [Function `get_min_vote_threshold`](#@Specification_1_get_min_vote_threshold)
    -  [Function `get_votes`](#@Specification_1_get_votes)
    -  [Function `is_resolved`](#@Specification_1_is_resolved)
    -  [Function `get_resolution_time_secs`](#@Specification_1_get_resolution_time_secs)
    -  [Function `is_multi_step_proposal_in_execution`](#@Specification_1_is_multi_step_proposal_in_execution)
    -  [Function `is_voting_period_over`](#@Specification_1_is_voting_period_over)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_multisig_voting_Proposal"></a>

## Struct `Proposal`

Extra metadata (e.g. description, code url) can be part of the ProposalType struct.


<pre><code><b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_Proposal">Proposal</a>&lt;ProposalType: store&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>
 Required. The address of the proposer.
</dd>
<dt>
<code>execution_content: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;ProposalType&gt;</code>
</dt>
<dd>
 Required. Should contain enough information to execute later, for example the required capability.
 This is stored as an option so we can return it to governance when the proposal is resolved.
</dd>
<dt>
<code>metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Optional. Value is serialized value of an attribute.
 Currently, we have three attributes that are used by the voting flow.
 1. RESOLVABLE_TIME_METADATA_KEY: this is used to record the resolvable time to ensure that resolution has to be done non-atomically.
 2. IS_MULTI_STEP_PROPOSAL_KEY: this is used to track if a proposal is single-step or multi-step.
 3. IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY: this attribute only applies to multi-step proposals. A single-step proposal will not have
 this field in its metadata map. The value is used to indicate if a multi-step proposal is in execution. If yes, we will disable further
 voting for this multi-step proposal.
</dd>
<dt>
<code>creation_time_secs: u64</code>
</dt>
<dd>
 Timestamp when the proposal was created.
</dd>
<dt>
<code>execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 Required. The hash for the execution script module. Only the same exact script module can resolve this
 proposal.
</dd>
<dt>
<code>min_vote_threshold: u64</code>
</dt>
<dd>
 A proposal is only resolved if expiration has passed and the number of votes is above threshold.
</dd>
<dt>
<code>expiration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>yes_votes: u64</code>
</dt>
<dd>
 Number of votes for each outcome.
</dd>
<dt>
<code>no_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>
 list of voters
</dd>
<dt>
<code>voted_records: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<b>address</b>, bool&gt;</code>
</dt>
<dd>
 list of voters who completed their vote
</dd>
<dt>
<code>is_resolved: bool</code>
</dt>
<dd>
 Whether the proposal has been resolved.
</dd>
<dt>
<code>resolution_time_secs: u64</code>
</dt>
<dd>
 Resolution timestamp if the proposal has been resolved. 0 otherwise.
</dd>
</dl>


</details>

<a id="0x1_multisig_voting_VotingForum"></a>

## Resource `VotingForum`



<pre><code><b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType: store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposals: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="multisig_voting.md#0x1_multisig_voting_Proposal">multisig_voting::Proposal</a>&lt;ProposalType&gt;&gt;</code>
</dt>
<dd>
 Use Table for execution optimization instead of Vector for gas cost since Vector is read entirely into memory
 during execution while only relevant Table entries are.
</dd>
<dt>
<code>events: <a href="multisig_voting.md#0x1_multisig_voting_VotingEvents">multisig_voting::VotingEvents</a></code>
</dt>
<dd>

</dd>
<dt>
<code>next_proposal_id: u64</code>
</dt>
<dd>
 Unique identifier for a proposal. This allows for 2 * 10**19 proposals.
</dd>
</dl>


</details>

<a id="0x1_multisig_voting_VotingEvents"></a>

## Struct `VotingEvents`



<pre><code><b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingEvents">VotingEvents</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_CreateProposalEvent">multisig_voting::CreateProposalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>register_forum_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_RegisterForumEvent">multisig_voting::RegisterForumEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>resolve_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">multisig_voting::ResolveProposal</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VoteEvent">multisig_voting::VoteEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_voting_CreateProposal"></a>

## Struct `CreateProposal`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_CreateProposal">CreateProposal</a> <b>has</b> drop, store
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
<code>execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>min_vote_threshold: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_voting_RegisterForum"></a>

## Struct `RegisterForum`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_RegisterForum">RegisterForum</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hosting_account: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_type_info: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_voting_Vote"></a>

## Struct `Vote`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_Vote">Vote</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>is_vote_yes: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_voting_ResolveProposal"></a>

## Struct `ResolveProposal`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a> <b>has</b> drop, store
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
<code>yes_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>no_votes: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>resolved_early: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_voting_CreateProposalEvent"></a>

## Struct `CreateProposalEvent`



<pre><code><b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_CreateProposalEvent">CreateProposalEvent</a> <b>has</b> drop, store
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
<code>execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>min_vote_threshold: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_voting_RegisterForumEvent"></a>

## Struct `RegisterForumEvent`



<pre><code><b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_RegisterForumEvent">RegisterForumEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hosting_account: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_type_info: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_voting_VoteEvent"></a>

## Struct `VoteEvent`



<pre><code><b>struct</b> <a href="multisig_voting.md#0x1_multisig_voting_VoteEvent">VoteEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>is_vote_yes: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_multisig_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION"></a>

If a proposal is multi-step, we need to use <code><a href="multisig_voting.md#0x1_multisig_voting_resolve_proposal_v2">resolve_proposal_v2</a>()</code> to resolve it.
If we use <code><a href="multisig_voting.md#0x1_multisig_voting_resolve">resolve</a>()</code> to resolve a multi-step proposal, it will fail with EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION">EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION</a>: u64 = 10;
</code></pre>



<a id="0x1_multisig_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION"></a>

Cannot vote if the specified multi-step proposal is in execution.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION">EMULTI_STEP_PROPOSAL_IN_EXECUTION</a>: u64 = 9;
</code></pre>



<a id="0x1_multisig_voting_ENOT_MULTISIG_OWNER"></a>

Not a multisig owner authorized to vote.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_ENOT_MULTISIG_OWNER">ENOT_MULTISIG_OWNER</a>: u64 = 16;
</code></pre>



<a id="0x1_multisig_voting_EPROPOSAL_ALREADY_RESOLVED"></a>

Proposal cannot be resolved more than once


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_ALREADY_RESOLVED">EPROPOSAL_ALREADY_RESOLVED</a>: u64 = 3;
</code></pre>



<a id="0x1_multisig_voting_EPROPOSAL_CANNOT_BE_RESOLVED"></a>

Proposal cannot be resolved. Either voting duration has not passed, not enough votes, or fewer yes than no votes


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_CANNOT_BE_RESOLVED">EPROPOSAL_CANNOT_BE_RESOLVED</a>: u64 = 2;
</code></pre>



<a id="0x1_multisig_voting_EPROPOSAL_EMPTY_EXECUTION_HASH"></a>

Proposal cannot contain an empty execution script hash


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_EMPTY_EXECUTION_HASH">EPROPOSAL_EMPTY_EXECUTION_HASH</a>: u64 = 4;
</code></pre>



<a id="0x1_multisig_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING"></a>

Current script's execution hash does not match the specified proposal's


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING">EPROPOSAL_EXECUTION_HASH_NOT_MATCHING</a>: u64 = 1;
</code></pre>



<a id="0x1_multisig_voting_EPROPOSAL_IS_SINGLE_STEP"></a>

Cannot call <code><a href="multisig_voting.md#0x1_multisig_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>()</code> on single-step proposals.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_IS_SINGLE_STEP">EPROPOSAL_IS_SINGLE_STEP</a>: u64 = 12;
</code></pre>



<a id="0x1_multisig_voting_EPROPOSAL_VOTING_ALREADY_ENDED"></a>

Proposal's voting period has already ended.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_VOTING_ALREADY_ENDED">EPROPOSAL_VOTING_ALREADY_ENDED</a>: u64 = 5;
</code></pre>



<a id="0x1_multisig_voting_ERESOLUTION_CANNOT_BE_ATOMIC"></a>

Resolution of a proposal cannot happen atomically in the same transaction as the last vote.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_ERESOLUTION_CANNOT_BE_ATOMIC">ERESOLUTION_CANNOT_BE_ATOMIC</a>: u64 = 8;
</code></pre>



<a id="0x1_multisig_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH"></a>

If we call <code><a href="multisig_voting.md#0x1_multisig_voting_resolve_proposal_v2">resolve_proposal_v2</a>()</code> to resolve a single-step proposal, the <code>next_execution_hash</code> parameter should be an empty vector.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH">ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH</a>: u64 = 11;
</code></pre>



<a id="0x1_multisig_voting_ETHRESHOLD_EXCEEDS_VOTERS"></a>

Threshold should not exceeds voters


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_ETHRESHOLD_EXCEEDS_VOTERS">ETHRESHOLD_EXCEEDS_VOTERS</a>: u64 = 15;
</code></pre>



<a id="0x1_multisig_voting_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE"></a>

Threshold value must be greater than 1


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE">ETHRESHOLD_MUST_BE_GREATER_THAN_ONE</a>: u64 = 17;
</code></pre>



<a id="0x1_multisig_voting_EVOTE_DUPLICATE_VOTE"></a>

Cannot vote duplicate


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EVOTE_DUPLICATE_VOTE">EVOTE_DUPLICATE_VOTE</a>: u64 = 13;
</code></pre>



<a id="0x1_multisig_voting_EVOTE_OVERFLOW"></a>

Vote is overflow


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EVOTE_OVERFLOW">EVOTE_OVERFLOW</a>: u64 = 14;
</code></pre>



<a id="0x1_multisig_voting_EVOTING_FORUM_ALREADY_REGISTERED"></a>

Voting forum has already been registered.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_EVOTING_FORUM_ALREADY_REGISTERED">EVOTING_FORUM_ALREADY_REGISTERED</a>: u64 = 6;
</code></pre>



<a id="0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY"></a>

Key used to track if the multi-step proposal is in execution / resolving in progress.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 73, 78, 95, 69, 88, 69, 67, 85, 84, 73, 79, 78];
</code></pre>



<a id="0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY"></a>

Key used to track if the proposal is multi-step


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 75, 69, 89];
</code></pre>



<a id="0x1_multisig_voting_PROPOSAL_STATE_FAILED"></a>

Proposal has failed because either the min vote threshold is not met or majority voted no.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_FAILED">PROPOSAL_STATE_FAILED</a>: u64 = 3;
</code></pre>



<a id="0x1_multisig_voting_PROPOSAL_STATE_PENDING"></a>

ProposalStateEnum representing proposal state.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_PENDING">PROPOSAL_STATE_PENDING</a>: u64 = 0;
</code></pre>



<a id="0x1_multisig_voting_PROPOSAL_STATE_SUCCEEDED"></a>



<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>: u64 = 1;
</code></pre>



<a id="0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY"></a>

Key used to track the resolvable time in the proposal's metadata.


<pre><code><b>const</b> <a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [82, 69, 83, 79, 76, 86, 65, 66, 76, 69, 95, 84, 73, 77, 69, 95, 77, 69, 84, 65, 68, 65, 84, 65, 95, 75, 69, 89];
</code></pre>



<a id="0x1_multisig_voting_register"></a>

## Function `register`



<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_register">register</a>&lt;ProposalType: store&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_register">register</a>&lt;ProposalType: store&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(!<b>exists</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="multisig_voting.md#0x1_multisig_voting_EVOTING_FORUM_ALREADY_REGISTERED">EVOTING_FORUM_ALREADY_REGISTERED</a>));

    <b>let</b> voting_forum = <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt; {
        next_proposal_id: 0,
        proposals: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;u64, <a href="multisig_voting.md#0x1_multisig_voting_Proposal">Proposal</a>&lt;ProposalType&gt;&gt;(),
        events: <a href="multisig_voting.md#0x1_multisig_voting_VotingEvents">VotingEvents</a> {
            create_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_CreateProposalEvent">CreateProposalEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),
            register_forum_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_RegisterForumEvent">RegisterForumEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),
            resolve_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a>&gt;(<a href="account.md#0x1_account">account</a>),
            vote_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VoteEvent">VoteEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),
        }
    };

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="multisig_voting.md#0x1_multisig_voting_RegisterForum">RegisterForum</a> {
                hosting_account: addr,
                proposal_type_info: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;ProposalType&gt;(),
            },
        );
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_RegisterForumEvent">RegisterForumEvent</a>&gt;(
        &<b>mut</b> voting_forum.events.register_forum_events,
        <a href="multisig_voting.md#0x1_multisig_voting_RegisterForumEvent">RegisterForumEvent</a> {
            hosting_account: addr,
            proposal_type_info: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;ProposalType&gt;(),
        },
    );

    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, voting_forum);
}
</code></pre>



</details>

<a id="0x1_multisig_voting_create_proposal"></a>

## Function `create_proposal`

Create a single-step proposal with the given parameters

@param voting_forum_address The forum's address where the proposal will be stored.
@param execution_content The execution content that will be given back at resolution time. This can contain
data such as a capability resource used to scope the execution.
@param execution_hash The hash for the execution script module. Only the same exact script module can resolve
this proposal.
@param min_vote_threshold The minimum number of votes needed to consider this proposal successful.
@param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.
@param metadata A simple_map that stores information about this proposal.
@return The proposal id.


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_create_proposal">create_proposal</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, expiration_secs: u64, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_create_proposal">create_proposal</a>&lt;ProposalType: store&gt;(
    proposer: <b>address</b>,
    voting_forum_address: <b>address</b>,
    execution_content: ProposalType,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    min_vote_threshold: u64,
    voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    expiration_secs: u64,
    metadata: SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <a href="multisig_voting.md#0x1_multisig_voting_create_proposal_v2">create_proposal_v2</a>(
        proposer,
        voting_forum_address,
        execution_content,
        execution_hash,
        min_vote_threshold,
        voters,
        expiration_secs,
        metadata,
        <b>false</b>
    )
}
</code></pre>



</details>

<a id="0x1_multisig_voting_create_proposal_v2"></a>

## Function `create_proposal_v2`

Create a single-step or a multi-step proposal with the given parameters

@param voting_forum_address The forum's address where the proposal will be stored.
@param execution_content The execution content that will be given back at resolution time. This can contain
data such as a capability resource used to scope the execution.
@param execution_hash The sha-256 hash for the execution script module. Only the same exact script module can
resolve this proposal.
@param min_vote_threshold The minimum number of votes needed to consider this proposal successful.
@param voters The list of voters who are eligible to vote.".
@param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.
@param metadata A simple_map that stores information about this proposal.
@param is_multi_step_proposal A bool value that indicates if the proposal is single-step or multi-step.
@return The proposal id.


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_create_proposal_v2">create_proposal_v2</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, expiration_secs: u64, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_create_proposal_v2">create_proposal_v2</a>&lt;ProposalType: store&gt;(
    proposer: <b>address</b>,
    voting_forum_address: <b>address</b>,
    execution_content: ProposalType,
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    min_vote_threshold: u64,
    voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    expiration_secs: u64,
    metadata: SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    is_multi_step_proposal: bool,
): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {

    // Make sure the execution <b>script</b>'s <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> is not empty.
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&execution_hash) != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_EMPTY_EXECUTION_HASH">EPROPOSAL_EMPTY_EXECUTION_HASH</a>));

    <b>assert</b>!(min_vote_threshold &gt; 1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_voting.md#0x1_multisig_voting_ETHRESHOLD_MUST_BE_GREATER_THAN_ONE">ETHRESHOLD_MUST_BE_GREATER_THAN_ONE</a>));

    // Make sure voters length must greater or equal  <b>to</b> vote threshold
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&voters) &gt;= min_vote_threshold, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_voting.md#0x1_multisig_voting_ETHRESHOLD_EXCEEDS_VOTERS">ETHRESHOLD_EXCEEDS_VOTERS</a>));

    <b>let</b> voting_forum = <b>borrow_global_mut</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal_id = voting_forum.next_proposal_id;
    voting_forum.next_proposal_id = voting_forum.next_proposal_id + 1;

    // Add a flag <b>to</b> indicate <b>if</b> this proposal is single-step or multi-step.
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> metadata, utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>), to_bytes(&is_multi_step_proposal));

    <b>let</b> is_multi_step_in_execution_key = utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
    <b>if</b> (is_multi_step_proposal) {
        // If the given proposal is a multi-step proposal, we will add a flag <b>to</b> indicate <b>if</b> this multi-step proposal is in execution.
        // This value is by default <b>false</b>. We turn this value <b>to</b> <b>true</b> when we start executing the multi-step proposal. This value
        // will be used <b>to</b> disable further <a href="voting.md#0x1_voting">voting</a> after we started executing the multi-step proposal.
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> metadata, is_multi_step_in_execution_key, to_bytes(&<b>false</b>));
        // If the proposal is a single-step proposal, we check <b>if</b> the metadata passed by the client <b>has</b> the <a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a> key.
        // If they have the key, we will remove it, because a single-step proposal that doesn't need this key.
    } <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&<b>mut</b> metadata, &is_multi_step_in_execution_key)) {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> metadata, &is_multi_step_in_execution_key);
    };

    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> voting_forum.proposals, proposal_id, <a href="multisig_voting.md#0x1_multisig_voting_Proposal">Proposal</a> {
        proposer,
        creation_time_secs: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),
        execution_content: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>&lt;ProposalType&gt;(execution_content),
        execution_hash,
        metadata,
        min_vote_threshold,
        voters,
        expiration_secs,
        yes_votes: 0,
        no_votes: 0,
        voted_records: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        is_resolved: <b>false</b>,
        resolution_time_secs: 0,
    });

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="multisig_voting.md#0x1_multisig_voting_CreateProposal">CreateProposal</a> {
                proposal_id,
                execution_hash,
                expiration_secs,
                metadata,
                min_vote_threshold,
            },
        );
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_CreateProposalEvent">CreateProposalEvent</a>&gt;(
        &<b>mut</b> voting_forum.events.create_proposal_events,
        <a href="multisig_voting.md#0x1_multisig_voting_CreateProposalEvent">CreateProposalEvent</a> {
            proposal_id,
            execution_hash,
            expiration_secs,
            metadata,
            min_vote_threshold,
        },
    );

    proposal_id
}
</code></pre>



</details>

<a id="0x1_multisig_voting_vote"></a>

## Function `vote`

Vote on the given proposal.

@param _proof Required so only the governance module that defines ProposalType can initiate voting.
@param voter signer of the voter.
@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@param should_pass Whether the votes are for yes or no.


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_vote">vote</a>&lt;ProposalType: store&gt;(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _proof: &ProposalType, voting_forum_address: <b>address</b>, proposal_id: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_vote">vote</a>&lt;ProposalType: store&gt;(
    voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _proof: &ProposalType,
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
    should_pass: bool,
) <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> voter_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);
    <b>let</b> voting_forum = <b>borrow_global_mut</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> voting_forum.proposals, proposal_id);

    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&proposal.voters, &voter_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="multisig_voting.md#0x1_multisig_voting_ENOT_MULTISIG_OWNER">ENOT_MULTISIG_OWNER</a>));

    // Voting might still be possible after the proposal <b>has</b> enough yes votes <b>to</b> be resolved early. This would only
    // lead <b>to</b> possible proposal resolution failure <b>if</b> the resolve early threshold is not definitive (e.g. &lt; 50% + 1
    // of the total <a href="voting.md#0x1_voting">voting</a> token's supply). In this case, more <a href="voting.md#0x1_voting">voting</a> might actually still be desirable.
    // Governance mechanisms built on this <a href="voting.md#0x1_voting">voting</a> <b>module</b> can <b>apply</b> additional rules on when <a href="voting.md#0x1_voting">voting</a> is closed <b>as</b>
    // appropriate.
    <b>assert</b>!(!<a href="multisig_voting.md#0x1_multisig_voting_is_voting_period_over">is_voting_period_over</a>(proposal), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_VOTING_ALREADY_ENDED">EPROPOSAL_VOTING_ALREADY_ENDED</a>));
    <b>assert</b>!(!proposal.is_resolved, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_ALREADY_RESOLVED">EPROPOSAL_ALREADY_RESOLVED</a>));
    // Assert this proposal is single-step, or <b>if</b> the proposal is multi-step, it is not in execution yet.
    <b>assert</b>!(!<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&proposal.metadata, &utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>))
        || *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&proposal.metadata, &utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>)) == to_bytes(
        &<b>false</b>
    ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_voting.md#0x1_multisig_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION">EMULTI_STEP_PROPOSAL_IN_EXECUTION</a>));

    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&proposal.voted_records, voter_address)) {
        <b>let</b> voted_record = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> proposal.voted_records, voter_address);
        <b>assert</b>!(voted_record != &should_pass, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="multisig_voting.md#0x1_multisig_voting_EVOTE_DUPLICATE_VOTE">EVOTE_DUPLICATE_VOTE</a>));
        *voted_record = should_pass;

        <b>if</b> (should_pass) {
            proposal.yes_votes = proposal.yes_votes + 1;
            // Since the entry existed in `voted_records`, the vote flip should be reflected by this subtraction
            proposal.no_votes = proposal.no_votes - 1;
        } <b>else</b> {
            // Since the voter already voted and flipping the vote here, this subtraction is needed.
            proposal.yes_votes = proposal.yes_votes - 1;
            proposal.no_votes = proposal.no_votes + 1;
        };
    } <b>else</b> {
        // vote cannot be more than `max_vote_number`
        <b>assert</b>!(proposal.yes_votes + proposal.no_votes &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&proposal.voters), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_voting.md#0x1_multisig_voting_EVOTE_OVERFLOW">EVOTE_OVERFLOW</a>));

        <b>if</b> (should_pass) {
            proposal.yes_votes = proposal.yes_votes + 1;
        } <b>else</b> {
            proposal.no_votes = proposal.no_votes + 1;
        };
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> proposal.voted_records, voter_address, should_pass);
    };

    // Record the resolvable time <b>to</b> ensure that resolution <b>has</b> <b>to</b> be done non-atomically.
    <b>let</b> timestamp_secs_bytes = to_bytes(&<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>());
    <b>let</b> key = utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&<b>mut</b> proposal.metadata, key, timestamp_secs_bytes);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="multisig_voting.md#0x1_multisig_voting_Vote">Vote</a> { proposal_id, is_vote_yes: should_pass, voter: voter_address });
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VoteEvent">VoteEvent</a>&gt;(
        &<b>mut</b> voting_forum.events.vote_events,
        <a href="multisig_voting.md#0x1_multisig_voting_VoteEvent">VoteEvent</a> { proposal_id, is_vote_yes: should_pass, voter: voter_address },
    );
}
</code></pre>



</details>

<a id="0x1_multisig_voting_is_proposal_resolvable"></a>

## Function `is_proposal_resolvable`

Common checks on if a proposal is resolvable, regardless if the proposal is single-step or multi-step.


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
) <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal_state = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    <b>assert</b>!(proposal_state == <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_CANNOT_BE_RESOLVED">EPROPOSAL_CANNOT_BE_RESOLVED</a>));

    <b>let</b> voting_forum = <b>borrow_global_mut</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> voting_forum.proposals, proposal_id);
    <b>assert</b>!(!proposal.is_resolved, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_ALREADY_RESOLVED">EPROPOSAL_ALREADY_RESOLVED</a>));

    // We need <b>to</b> make sure that the resolution is happening in
    // a separate transaction from the last vote <b>to</b> guard against <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> potential flashloan attacks.
    <b>let</b> resolvable_time = to_u64(*<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&proposal.metadata, &utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>)));
    <b>assert</b>!(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; resolvable_time, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_voting.md#0x1_multisig_voting_ERESOLUTION_CANNOT_BE_ATOMIC">ERESOLUTION_CANNOT_BE_ATOMIC</a>));

    <b>assert</b>!(
        <a href="transaction_context.md#0x1_transaction_context_get_script_hash">transaction_context::get_script_hash</a>() == proposal.execution_hash,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING">EPROPOSAL_EXECUTION_HASH_NOT_MATCHING</a>),
    );
}
</code></pre>



</details>

<a id="0x1_multisig_voting_resolve"></a>

## Function `resolve`

Resolve a single-step proposal with given id. Can only be done if there are at least as many votes as min required and
there are more yes votes than no. If either of these conditions is not met, this will revert.

@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_resolve">resolve</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): ProposalType
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_resolve">resolve</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): ProposalType <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <a href="multisig_voting.md#0x1_multisig_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);

    <b>let</b> voting_forum = <b>borrow_global_mut</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> voting_forum.proposals, proposal_id);

    // Assert that the specified proposal is not a multi-step proposal.
    <b>let</b> multi_step_key = utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
    <b>let</b> has_multi_step_key = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&proposal.metadata, &multi_step_key);
    <b>if</b> (has_multi_step_key) {
        <b>let</b> is_multi_step_proposal = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(*<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&proposal.metadata, &multi_step_key));
        <b>assert</b>!(
            !is_multi_step_proposal,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="multisig_voting.md#0x1_multisig_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION">EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION</a>)
        );
    };

    <b>let</b> resolved_early = <a href="multisig_voting.md#0x1_multisig_voting_can_be_resolved_early">can_be_resolved_early</a>(proposal);
    proposal.is_resolved = <b>true</b>;
    proposal.resolution_time_secs = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a> {
                proposal_id,
                yes_votes: proposal.yes_votes,
                no_votes: proposal.no_votes,
                resolved_early,
            },
        );
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a>&gt;(
        &<b>mut</b> voting_forum.events.resolve_proposal_events,
        <a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a> {
            proposal_id,
            yes_votes: proposal.yes_votes,
            no_votes: proposal.no_votes,
            resolved_early,
        },
    );

    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> proposal.execution_content)
}
</code></pre>



</details>

<a id="0x1_multisig_voting_resolve_proposal_v2"></a>

## Function `resolve_proposal_v2`

Resolve a single-step or a multi-step proposal with the given id.
Can only be done if there are at least as many votes as min required and
there are more yes votes than no. If either of these conditions is not met, this will revert.


@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@param next_execution_hash The next execution hash if the given proposal is multi-step.


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_resolve_proposal_v2">resolve_proposal_v2</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_resolve_proposal_v2">resolve_proposal_v2</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
    next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <a href="multisig_voting.md#0x1_multisig_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);

    <b>let</b> voting_forum = <b>borrow_global_mut</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> voting_forum.proposals, proposal_id);

    // Update the <a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a> key <b>to</b> indicate that the multi-step proposal is in execution.
    <b>let</b> multi_step_in_execution_key = utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&proposal.metadata, &multi_step_in_execution_key)) {
        <b>let</b> is_multi_step_proposal_in_execution_value = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(
            &<b>mut</b> proposal.metadata,
            &multi_step_in_execution_key
        );
        *is_multi_step_proposal_in_execution_value = to_bytes(&<b>true</b>);
    };

    <b>let</b> multi_step_key = utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
    <b>let</b> is_multi_step = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&proposal.metadata, &multi_step_key) && <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(
        *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&proposal.metadata, &multi_step_key)
    );
    <b>let</b> next_execution_hash_is_empty = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&next_execution_hash) == 0;

    // Assert that <b>if</b> this proposal is single-step, the `next_execution_hash` parameter is empty.
    <b>assert</b>!(
        is_multi_step || next_execution_hash_is_empty,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_voting.md#0x1_multisig_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH">ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH</a>)
    );

    // If the `next_execution_hash` parameter is empty, it means that either
    // - this proposal is a single-step proposal, or
    // - this proposal is multi-step and we're currently resolving the last step in the multi-step proposal.
    // We can mark that this proposal is resolved.
    <b>if</b> (next_execution_hash_is_empty) {
        proposal.is_resolved = <b>true</b>;
        proposal.resolution_time_secs = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();

        // Set the `<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>` value <b>to</b> <b>false</b> upon successful resolution of the last step of a multi-step proposal.
        <b>if</b> (is_multi_step) {
            <b>let</b> is_multi_step_proposal_in_execution_value = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(
                &<b>mut</b> proposal.metadata,
                &multi_step_in_execution_key
            );
            *is_multi_step_proposal_in_execution_value = to_bytes(&<b>false</b>);
        };
    } <b>else</b> {
        // If the current step is not the last step,
        // <b>update</b> the proposal's execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> on-chain <b>to</b> the execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> of the next step.
        proposal.execution_hash = next_execution_hash;
    };

    // For single-step proposals, we emit one `<a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a>` <a href="event.md#0x1_event">event</a> per proposal.
    // For multi-step proposals, we emit one `<a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a>` <a href="event.md#0x1_event">event</a> per step in the multi-step proposal. This means
    // that we emit multiple `<a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a>` events for the same multi-step proposal.
    <b>let</b> resolved_early = <a href="multisig_voting.md#0x1_multisig_voting_can_be_resolved_early">can_be_resolved_early</a>(proposal);
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a> {
                proposal_id,
                yes_votes: proposal.yes_votes,
                no_votes: proposal.no_votes,
                resolved_early,
            },
        );
    };
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> voting_forum.events.resolve_proposal_events,
        <a href="multisig_voting.md#0x1_multisig_voting_ResolveProposal">ResolveProposal</a> {
            proposal_id,
            yes_votes: proposal.yes_votes,
            no_votes: proposal.no_votes,
            resolved_early,
        },
    );

}
</code></pre>



</details>

<a id="0x1_multisig_voting_next_proposal_id"></a>

## Function `next_proposal_id`

Return the next unassigned proposal id


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_next_proposal_id">next_proposal_id</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_next_proposal_id">next_proposal_id</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, ): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> voting_forum = <b>borrow_global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    voting_forum.next_proposal_id
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_proposer"></a>

## Function `get_proposer`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposer">get_proposer</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposer">get_proposer</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64
): <b>address</b> <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.proposer
}
</code></pre>



</details>

<a id="0x1_multisig_voting_is_voting_closed"></a>

## Function `is_voting_closed`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64
): bool <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    <a href="multisig_voting.md#0x1_multisig_voting_can_be_resolved_early">can_be_resolved_early</a>(proposal) || <a href="multisig_voting.md#0x1_multisig_voting_is_voting_period_over">is_voting_period_over</a>(proposal)
}
</code></pre>



</details>

<a id="0x1_multisig_voting_can_be_resolved_early"></a>

## Function `can_be_resolved_early`

Return true if the proposal has reached early resolution threshold (if specified).


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_can_be_resolved_early">can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">multisig_voting::Proposal</a>&lt;ProposalType&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_can_be_resolved_early">can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">Proposal</a>&lt;ProposalType&gt;): bool {
    <b>if</b> (proposal.yes_votes &gt;= proposal.min_vote_threshold || proposal.no_votes &gt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&proposal.voters) - proposal.min_vote_threshold + 1) {
        <b>return</b> <b>true</b>
    };
    <b>false</b>
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_proposal_metadata"></a>

## Function `get_proposal_metadata`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_metadata">get_proposal_metadata</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_metadata">get_proposal_metadata</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.metadata
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_proposal_metadata_value"></a>

## Function `get_proposal_metadata_value`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_metadata_value">get_proposal_metadata_value</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, metadata_key: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_metadata_value">get_proposal_metadata_value</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
    metadata_key: String,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&proposal.metadata, &metadata_key)
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_proposal_state"></a>

## Function `get_proposal_state`

Return the state of the proposal with given id.

@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@return Proposal state as an enum value.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>if</b> (<a href="multisig_voting.md#0x1_multisig_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id)) {
        <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
        <b>if</b> (proposal.yes_votes &gt;= proposal.min_vote_threshold) {
            <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>
        } <b>else</b> {
            <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_FAILED">PROPOSAL_STATE_FAILED</a>
        }
    } <b>else</b> {
        <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_PENDING">PROPOSAL_STATE_PENDING</a>
    }
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_proposal_creation_secs"></a>

## Function `get_proposal_creation_secs`

Return the proposal's creation time.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_creation_secs">get_proposal_creation_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_creation_secs">get_proposal_creation_secs</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.creation_time_secs
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_proposal_expiration_secs"></a>

## Function `get_proposal_expiration_secs`

Return the proposal's expiration time.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_expiration_secs">get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_expiration_secs">get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.expiration_secs
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_execution_hash"></a>

## Function `get_execution_hash`

Return the proposal's execution hash.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_execution_hash">get_execution_hash</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_execution_hash">get_execution_hash</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.execution_hash
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_min_vote_threshold"></a>

## Function `get_min_vote_threshold`

Return the proposal's minimum vote threshold


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_min_vote_threshold">get_min_vote_threshold</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_min_vote_threshold">get_min_vote_threshold</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.min_vote_threshold
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_votes"></a>

## Function `get_votes`

Return the proposal's current vote count (yes_votes, no_votes)


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_votes">get_votes</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_votes">get_votes</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): (u64, u64) <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    (proposal.yes_votes, proposal.no_votes)
}
</code></pre>



</details>

<a id="0x1_multisig_voting_is_resolved"></a>

## Function `is_resolved`

Return true if the governance proposal has already been resolved.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_resolved">is_resolved</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_resolved">is_resolved</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): bool <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.is_resolved
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_resolution_time_secs"></a>

## Function `get_resolution_time_secs`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_resolution_time_secs">get_resolution_time_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_resolution_time_secs">get_resolution_time_secs</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): u64 <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> proposal = <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.resolution_time_secs
}
</code></pre>



</details>

<a id="0x1_multisig_voting_is_multi_step_proposal_in_execution"></a>

## Function `is_multi_step_proposal_in_execution`

Return true if the multi-step governance proposal is in execution.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): bool <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> voting_forum = <b>borrow_global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&voting_forum.proposals, proposal_id);
    <b>let</b> is_multi_step_in_execution_key = utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&proposal.metadata, &is_multi_step_in_execution_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_voting.md#0x1_multisig_voting_EPROPOSAL_IS_SINGLE_STEP">EPROPOSAL_IS_SINGLE_STEP</a>)
    );
    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(*<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&proposal.metadata, &is_multi_step_in_execution_key))
}
</code></pre>



</details>

<a id="0x1_multisig_voting_is_voting_period_over"></a>

## Function `is_voting_period_over`

Return true if the voting period of the given proposal has already ended.


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_voting_period_over">is_voting_period_over</a>&lt;ProposalType: store&gt;(proposal: &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">multisig_voting::Proposal</a>&lt;ProposalType&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_voting_period_over">is_voting_period_over</a>&lt;ProposalType: store&gt;(proposal: &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">Proposal</a>&lt;ProposalType&gt;): bool {
    <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; proposal.expiration_secs
}
</code></pre>



</details>

<a id="0x1_multisig_voting_get_proposal"></a>

## Function `get_proposal`



<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">multisig_voting::Proposal</a>&lt;ProposalType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal">get_proposal</a>&lt;ProposalType: store&gt;(
    voting_forum_address: <b>address</b>,
    proposal_id: u64,
): &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">Proposal</a>&lt;ProposalType&gt; <b>acquires</b> <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a> {
    <b>let</b> voting_forum = <b>borrow_global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&voting_forum.proposals, proposal_id)
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
<td>The proposal ID in a voting forum is unique and always increases monotonically with each new proposal created for that voting forum.</td>
<td>High</td>
<td>The create_proposal and create_proposal_v2 create a new proposal with a unique ID derived from the voting_forum's next_proposal_id incrementally.</td>
<td>Formally verified via <a href="#high-level-req-1">create_proposal</a>.</td>
</tr>

<tr>
<td>2</td>
<td>While voting, it ensures that only the governance module that defines ProposalType may initiate voting and that the proposal under vote exists in the specified voting forum.</td>
<td>Critical</td>
<td>The vote function verifies the eligibility and validity of a proposal before allowing voting. It ensures that only the correct governance module initiates voting. The function checks if the proposal is currently eligible for voting by confirming it has not resolved and the voting period has not ended.</td>
<td>Formally verified via <a href="#high-level-req-2">vote</a>.</td>
</tr>

<tr>
<td>3</td>
<td>After resolving a single-step proposal, the corresponding proposal is guaranteed to be marked as successfully resolved.</td>
<td>High</td>
<td>Upon invoking the resolve function on a proposal, it undergoes a series of checks to ensure its validity. These include verifying if the proposal exists, is a single-step proposal, and meets the criteria for resolution. If the checks pass, the proposal's is_resolved flag becomes true, indicating a successful resolution.</td>
<td>Formally verified via <a href="#high-level-req-3">resolve</a>.</td>
</tr>

<tr>
<td>4</td>
<td>In the context of v2 proposal resolving, both single-step and multi-step proposals are accurately handled. It ensures that for single-step proposals, the next execution hash is empty and resolves the proposal, while for multi-step proposals, it guarantees that the next execution hash corresponds to the hash of the next step, maintaining the integrity of the proposal execution sequence.</td>
<td>Medium</td>
<td>The function resolve_proposal_v2 correctly handles both single-step and multi-step proposals. For single-step proposals, it ensures that the next_execution_hash parameter is empty and resolves the proposal. For multi-step proposals, it ensures that the next_execution_hash parameter contains the hash of the next step.</td>
<td>Formally verified via <a href="#high-level-req-4">resolve_proposal_v2</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_register">register</a>&lt;ProposalType: store&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>aborts_if</b> <b>exists</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> register_account = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>aborts_if</b> register_account.guid_creation_num + 4 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;
<b>aborts_if</b> register_account.guid_creation_num + 4 &gt; MAX_U64;
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;ProposalType&gt;();
<b>ensures</b> <b>exists</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(addr);
</code></pre>



<a id="@Specification_1_create_proposal"></a>

### Function `create_proposal`


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_create_proposal">create_proposal</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, expiration_secs: u64, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): u64
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_CreateProposalAbortsIfAndEnsures">CreateProposalAbortsIfAndEnsures</a>&lt;ProposalType&gt;{is_multi_step_proposal: <b>false</b>};
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> result == <b>old</b>(<b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;
</code></pre>



<a id="@Specification_1_create_proposal_v2"></a>

### Function `create_proposal_v2`


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_create_proposal_v2">create_proposal_v2</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u64, voters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, expiration_secs: u64, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_CreateProposalAbortsIfAndEnsures">CreateProposalAbortsIfAndEnsures</a>&lt;ProposalType&gt;;
<b>ensures</b> result == <b>old</b>(<b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;
</code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_vote">vote</a>&lt;ProposalType: store&gt;(voter: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _proof: &ProposalType, voting_forum_address: <b>address</b>, proposal_id: u64, should_pass: bool)
</code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> voter_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(voter);
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>let</b> <b>post</b> post_voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> <b>post</b> post_proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);
<b>aborts_if</b> <a href="multisig_voting.md#0x1_multisig_voting_is_voting_period_over">is_voting_period_over</a>(proposal);
<b>aborts_if</b> proposal.is_resolved;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@supra_framework);
<b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
<b>let</b> execution_key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, execution_key) &&
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, execution_key) != std::bcs::serialize(<b>false</b>);
<b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);
<b>let</b> timestamp_secs_bytes = std::bcs::serialize(<a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>());
<b>let</b> key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);
<b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, key) == timestamp_secs_bytes;
<b>ensures</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(proposal.voted_records, voter_address) && should_pass) ==&gt; post_proposal.yes_votes == proposal.yes_votes + 1 && post_proposal.no_votes == proposal.no_votes - 1;
<b>ensures</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(proposal.voted_records, voter_address) && !should_pass) ==&gt; post_proposal.no_votes == proposal.no_votes + 1 && post_proposal.yes_votes == proposal.yes_votes - 1;
<b>ensures</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(proposal.voted_records, voter_address) && should_pass) ==&gt; post_proposal.yes_votes == proposal.yes_votes + 1;
<b>ensures</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(proposal.voted_records, voter_address) && !should_pass) ==&gt; post_proposal.no_votes == proposal.no_votes + 1;
</code></pre>



<a id="@Specification_1_is_proposal_resolvable"></a>

### Function `is_proposal_resolvable`


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64)
</code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt;;
</code></pre>



<a id="@Specification_1_resolve"></a>

### Function `resolve`


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_resolve">resolve</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): ProposalType
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt;;
<b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>let</b> multi_step_key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
<b>let</b> has_multi_step_key = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key);
<b>aborts_if</b> has_multi_step_key && !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));
<b>let</b> <b>post</b> post_voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> <b>post</b> post_proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@supra_framework);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>ensures</b> post_proposal.is_resolved == <b>true</b>;
<b>ensures</b> post_proposal.resolution_time_secs == <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(proposal.execution_content);
<b>ensures</b> result == <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(proposal.execution_content);
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(post_proposal.execution_content);
</code></pre>



<a id="@Specification_1_resolve_proposal_v2"></a>

### Function `resolve_proposal_v2`


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_resolve_proposal_v2">resolve_proposal_v2</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify_duration_estimate = 300;
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>let</b> <b>post</b> post_voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> <b>post</b> post_proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);
<b>let</b> multi_step_in_execution_key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
<b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
<b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
<b>ensures</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key) && len(next_execution_hash) != 0) ==&gt;
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, multi_step_in_execution_key) == std::bcs::serialize(<b>true</b>);
<b>ensures</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key) &&
    (len(next_execution_hash) == 0 && !is_multi_step)) ==&gt;
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, multi_step_in_execution_key) == std::bcs::serialize(<b>true</b>);
<b>let</b> multi_step_key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key) &&
    !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));
<b>let</b> is_multi_step = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key) &&
    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));
<b>aborts_if</b> !is_multi_step && len(next_execution_hash) != 0;
<b>aborts_if</b> len(next_execution_hash) == 0 && !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@supra_framework);
<b>aborts_if</b> len(next_execution_hash) == 0 && is_multi_step && !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key);
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>ensures</b> len(next_execution_hash) == 0 ==&gt; post_proposal.resolution_time_secs == <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();
<b>ensures</b> len(next_execution_hash) == 0 ==&gt; post_proposal.is_resolved == <b>true</b>;
<b>ensures</b> (len(next_execution_hash) == 0 && is_multi_step) ==&gt; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, multi_step_in_execution_key) == std::bcs::serialize(<b>false</b>);
<b>ensures</b> len(next_execution_hash) != 0 ==&gt; post_proposal.execution_hash == next_execution_hash;
</code></pre>



<a id="@Specification_1_next_proposal_id"></a>

### Function `next_proposal_id`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_next_proposal_id">next_proposal_id</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>ensures</b> result == <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address).next_proposal_id;
</code></pre>



<a id="@Specification_1_get_proposer"></a>

### Function `get_proposer`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposer">get_proposer</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <b>address</b>
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result == proposal.proposer;
</code></pre>



<a id="@Specification_1_is_voting_closed"></a>

### Function `is_voting_closed`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool
</code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>ensures</b> result == <a href="multisig_voting.md#0x1_multisig_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
</code></pre>



<a id="@Specification_1_can_be_resolved_early"></a>

### Function `can_be_resolved_early`


<pre><code><b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_can_be_resolved_early">can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">multisig_voting::Proposal</a>&lt;ProposalType&gt;): bool
</code></pre>




<pre><code><b>ensures</b> result == <a href="multisig_voting.md#0x1_multisig_voting_spec_can_be_resolved_early">spec_can_be_resolved_early</a>&lt;ProposalType&gt;(proposal);
</code></pre>



<a id="@Specification_1_get_proposal_metadata"></a>

### Function `get_proposal_metadata`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_metadata">get_proposal_metadata</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result == proposal.metadata;
</code></pre>



<a id="@Specification_1_get_proposal_metadata_value"></a>

### Function `get_proposal_metadata_value`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_metadata_value">get_proposal_metadata_value</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, metadata_key: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, metadata_key);
<b>ensures</b> result == <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, metadata_key);
</code></pre>



<a id="@Specification_1_get_proposal_state"></a>

### Function `get_proposal_state`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>




<pre><code><b>pragma</b> addition_overflow_unchecked;
<b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();
<b>pragma</b> aborts_if_is_partial = <b>true</b>;
<b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>ensures</b> result == <a href="multisig_voting.md#0x1_multisig_voting_spec_get_proposal_state">spec_get_proposal_state</a>(voting_forum_address, proposal_id, voting_forum);
</code></pre>



<a id="@Specification_1_get_proposal_creation_secs"></a>

### Function `get_proposal_creation_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_creation_secs">get_proposal_creation_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result == proposal.creation_time_secs;
</code></pre>



<a id="@Specification_1_get_proposal_expiration_secs"></a>

### Function `get_proposal_expiration_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_proposal_expiration_secs">get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>ensures</b> result == <a href="multisig_voting.md#0x1_multisig_voting_spec_get_proposal_expiration_secs">spec_get_proposal_expiration_secs</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
</code></pre>



<a id="@Specification_1_get_execution_hash"></a>

### Function `get_execution_hash`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_execution_hash">get_execution_hash</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result == proposal.execution_hash;
</code></pre>



<a id="@Specification_1_get_min_vote_threshold"></a>

### Function `get_min_vote_threshold`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_min_vote_threshold">get_min_vote_threshold</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result == proposal.min_vote_threshold;
</code></pre>



<a id="@Specification_1_get_votes"></a>

### Function `get_votes`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_votes">get_votes</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): (u64, u64)
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result_1 == proposal.yes_votes;
<b>ensures</b> result_2 == proposal.no_votes;
</code></pre>



<a id="@Specification_1_is_resolved"></a>

### Function `is_resolved`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_resolved">is_resolved</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result == proposal.is_resolved;
</code></pre>




<a id="0x1_multisig_voting_AbortsIfNotContainProposalID"></a>


<pre><code><b>schema</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt; {
    proposal_id: u64;
    voting_forum_address: <b>address</b>;
    <b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
}
</code></pre>




<a id="0x1_multisig_voting_IsProposalResolvableAbortsIf"></a>


<pre><code><b>schema</b> <a href="multisig_voting.md#0x1_multisig_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt; {
    voting_forum_address: <b>address</b>;
    proposal_id: u64;
    <b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
    <b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
    <b>let</b> voting_closed = <a href="multisig_voting.md#0x1_multisig_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    <b>aborts_if</b> voting_closed && !(proposal.yes_votes &gt;= proposal.min_vote_threshold);
    <b>aborts_if</b> !voting_closed;
    <b>aborts_if</b> proposal.is_resolved;
    <b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);
    <b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>));
    <b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;u64&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>)));
    <b>aborts_if</b> <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &lt;= <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;u64&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>)));
    <b>aborts_if</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">transaction_context::spec_get_script_hash</a>() != proposal.execution_hash;
}
</code></pre>




<a id="0x1_multisig_voting_CreateProposalAbortsIfAndEnsures"></a>


<pre><code><b>schema</b> <a href="multisig_voting.md#0x1_multisig_voting_CreateProposalAbortsIfAndEnsures">CreateProposalAbortsIfAndEnsures</a>&lt;ProposalType&gt; {
    voting_forum_address: <b>address</b>;
    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    min_vote_threshold: u128;
    metadata: SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;;
    is_multi_step_proposal: bool;
    <b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> proposal_id = voting_forum.next_proposal_id;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals,proposal_id);
    <b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
    <b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
    <b>aborts_if</b> len(execution_hash) == 0;
    <b>let</b> execution_key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);
    <b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(metadata, execution_key);
    <b>aborts_if</b> voting_forum.next_proposal_id + 1 &gt; MAX_U64;
    <b>let</b> is_multi_step_in_execution_key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
    <b>aborts_if</b> is_multi_step_proposal && <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(metadata, is_multi_step_in_execution_key);
    <b>let</b> <b>post</b> post_voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
    <b>let</b> <b>post</b> post_metadata = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id).metadata;
    <b>ensures</b> post_voting_forum.next_proposal_id == voting_forum.next_proposal_id + 1;
    <b>ensures</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(post_voting_forum.proposals, proposal_id);
    <b>ensures</b> <b>if</b> (is_multi_step_proposal) {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_metadata, is_multi_step_in_execution_key) == std::bcs::serialize(<b>false</b>)
    } <b>else</b> {
        !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_metadata, is_multi_step_in_execution_key)
    };
}
</code></pre>



<a id="@Specification_1_get_resolution_time_secs"></a>

### Function `get_resolution_time_secs`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_get_resolution_time_secs">get_resolution_time_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
<b>ensures</b> result == proposal.resolution_time_secs;
</code></pre>



<a id="@Specification_1_is_multi_step_proposal_in_execution"></a>

### Function `is_multi_step_proposal_in_execution`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool
</code></pre>




<pre><code><b>include</b> <a href="multisig_voting.md#0x1_multisig_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;
<b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
<b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals,proposal_id);
<b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
<b>let</b> execution_key = std::string::spec_utf8(<a href="multisig_voting.md#0x1_multisig_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata,execution_key);
<b>let</b> is_multi_step_in_execution_key = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata,execution_key);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;bool&gt;(is_multi_step_in_execution_key);
<b>ensures</b> result == <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;bool&gt;(is_multi_step_in_execution_key);
</code></pre>



<a id="@Specification_1_is_voting_period_over"></a>

### Function `is_voting_period_over`


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_is_voting_period_over">is_voting_period_over</a>&lt;ProposalType: store&gt;(proposal: &<a href="multisig_voting.md#0x1_multisig_voting_Proposal">multisig_voting::Proposal</a>&lt;ProposalType&gt;): bool
</code></pre>




<pre><code><b>ensures</b> result == (<a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; proposal.expiration_secs);
</code></pre>




<a id="0x1_multisig_voting_spec_get_proposal_expiration_secs"></a>


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_spec_get_proposal_expiration_secs">spec_get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(
   voting_forum_address: <b>address</b>,
   proposal_id: u64,
): u64 {
   <b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
   <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
   proposal.expiration_secs
}
</code></pre>




<a id="0x1_multisig_voting_spec_get_proposal_state"></a>


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_spec_get_proposal_state">spec_get_proposal_state</a>&lt;ProposalType&gt;(
   voting_forum_address: <b>address</b>,
   proposal_id: u64,
   voting_forum: <a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;
): u64 {
   <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
   <b>let</b> voting_closed = <a href="multisig_voting.md#0x1_multisig_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);
   <b>let</b> proposal_vote_cond = (proposal.yes_votes &gt; proposal.no_votes && proposal.yes_votes + proposal.no_votes &gt;= proposal.min_vote_threshold);
   <b>if</b> (voting_closed) {
       <b>if</b> (proposal.yes_votes &gt;= proposal.min_vote_threshold) {
           <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>
       } <b>else</b> {
           <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_FAILED">PROPOSAL_STATE_FAILED</a>
       }
   } <b>else</b> {
       <a href="multisig_voting.md#0x1_multisig_voting_PROPOSAL_STATE_PENDING">PROPOSAL_STATE_PENDING</a>
   }
}
</code></pre>




<a id="0x1_multisig_voting_spec_is_voting_closed"></a>


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool {
   <b>let</b> voting_forum = <b>global</b>&lt;<a href="multisig_voting.md#0x1_multisig_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);
   <b>let</b> proposal = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);
   <a href="multisig_voting.md#0x1_multisig_voting_spec_can_be_resolved_early">spec_can_be_resolved_early</a>(proposal)||<a href="multisig_voting.md#0x1_multisig_voting_is_voting_period_over">is_voting_period_over</a>(proposal)
}
</code></pre>




<a id="0x1_multisig_voting_spec_can_be_resolved_early"></a>


<pre><code><b>fun</b> <a href="multisig_voting.md#0x1_multisig_voting_spec_can_be_resolved_early">spec_can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: <a href="multisig_voting.md#0x1_multisig_voting_Proposal">Proposal</a>&lt;ProposalType&gt;): bool {
   <b>if</b> (proposal.yes_votes &gt;= proposal.min_vote_threshold || proposal.no_votes &gt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(proposal.voters) - proposal.min_vote_threshold + 1) {
       <b>true</b>
   } <b>else</b> {
       <b>false</b>
   }
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
