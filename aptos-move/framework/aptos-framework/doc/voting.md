
<a id="0x1_voting"></a>

# Module `0x1::voting`


This is the general Voting module that can be used as part of a DAO Governance. Voting is designed to be used by
standalone governance modules, who has full control over the voting flow and is responsible for voting power
calculation and including proper capabilities when creating the proposal so resolution can go through.
On-chain governance of the Aptos network also uses Voting.

The voting flow:
1. The Voting module can be deployed at a known address (e.g. 0x1 for Aptos on-chain governance)
2. The governance module, e.g. AptosGovernance, can be deployed later and define a GovernanceProposal resource type
that can also contain other information such as Capability resource for authorization.
3. The governance module's owner can then register the ProposalType with Voting. This also hosts the proposal list
(forum) on the calling account.
4. A proposer, through the governance module, can call Voting::create_proposal to create a proposal. create_proposal
cannot be called directly not through the governance module. A script hash of the resolution script that can later
be called to execute the proposal is required.
5. A voter, through the governance module, can call Voting::vote on a proposal. vote requires passing a &ProposalType
and thus only the governance module that registers ProposalType can call vote.
6. Once the proposal's expiration time has passed and more than the defined threshold has voted yes on the proposal,
anyone can call resolve which returns the content of the proposal (of type ProposalType) that can be used to execute.
7. Only the resolution script with the same script hash specified in the proposal can call Voting::resolve as part of
the resolution process.


-  [Struct `Proposal`](#0x1_voting_Proposal)
-  [Resource `VotingForum`](#0x1_voting_VotingForum)
-  [Struct `VotingEvents`](#0x1_voting_VotingEvents)
-  [Struct `CreateProposal`](#0x1_voting_CreateProposal)
-  [Struct `RegisterForum`](#0x1_voting_RegisterForum)
-  [Struct `Vote`](#0x1_voting_Vote)
-  [Struct `ResolveProposal`](#0x1_voting_ResolveProposal)
-  [Struct `CreateProposalEvent`](#0x1_voting_CreateProposalEvent)
-  [Struct `RegisterForumEvent`](#0x1_voting_RegisterForumEvent)
-  [Struct `VoteEvent`](#0x1_voting_VoteEvent)
-  [Constants](#@Constants_0)
-  [Function `register`](#0x1_voting_register)
-  [Function `create_proposal`](#0x1_voting_create_proposal)
-  [Function `create_proposal_v2`](#0x1_voting_create_proposal_v2)
-  [Function `vote`](#0x1_voting_vote)
-  [Function `is_proposal_resolvable`](#0x1_voting_is_proposal_resolvable)
-  [Function `resolve`](#0x1_voting_resolve)
-  [Function `resolve_proposal_v2`](#0x1_voting_resolve_proposal_v2)
-  [Function `next_proposal_id`](#0x1_voting_next_proposal_id)
-  [Function `get_proposer`](#0x1_voting_get_proposer)
-  [Function `is_voting_closed`](#0x1_voting_is_voting_closed)
-  [Function `can_be_resolved_early`](#0x1_voting_can_be_resolved_early)
-  [Function `get_proposal_metadata`](#0x1_voting_get_proposal_metadata)
-  [Function `get_proposal_metadata_value`](#0x1_voting_get_proposal_metadata_value)
-  [Function `get_proposal_state`](#0x1_voting_get_proposal_state)
-  [Function `get_proposal_creation_secs`](#0x1_voting_get_proposal_creation_secs)
-  [Function `get_proposal_expiration_secs`](#0x1_voting_get_proposal_expiration_secs)
-  [Function `get_execution_hash`](#0x1_voting_get_execution_hash)
-  [Function `get_min_vote_threshold`](#0x1_voting_get_min_vote_threshold)
-  [Function `get_early_resolution_vote_threshold`](#0x1_voting_get_early_resolution_vote_threshold)
-  [Function `get_votes`](#0x1_voting_get_votes)
-  [Function `is_resolved`](#0x1_voting_is_resolved)
-  [Function `get_resolution_time_secs`](#0x1_voting_get_resolution_time_secs)
-  [Function `is_multi_step_proposal_in_execution`](#0x1_voting_is_multi_step_proposal_in_execution)
-  [Function `is_voting_period_over`](#0x1_voting_is_voting_period_over)
-  [Function `get_proposal`](#0x1_voting_get_proposal)
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
    -  [Function `get_early_resolution_vote_threshold`](#@Specification_1_get_early_resolution_vote_threshold)
    -  [Function `get_votes`](#@Specification_1_get_votes)
    -  [Function `is_resolved`](#@Specification_1_is_resolved)
    -  [Function `get_resolution_time_secs`](#@Specification_1_get_resolution_time_secs)
    -  [Function `is_multi_step_proposal_in_execution`](#@Specification_1_is_multi_step_proposal_in_execution)
    -  [Function `is_voting_period_over`](#@Specification_1_is_voting_period_over)


<pre><code>use 0x1::account;
use 0x1::bcs;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::from_bcs;
use 0x1::option;
use 0x1::signer;
use 0x1::simple_map;
use 0x1::string;
use 0x1::table;
use 0x1::timestamp;
use 0x1::transaction_context;
use 0x1::type_info;
</code></pre>



<a id="0x1_voting_Proposal"></a>

## Struct `Proposal`

Extra metadata (e.g. description, code url) can be part of the ProposalType struct.


<pre><code>struct Proposal&lt;ProposalType: store&gt; has store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposer: address</code>
</dt>
<dd>
 Required. The address of the proposer.
</dd>
<dt>
<code>execution_content: option::Option&lt;ProposalType&gt;</code>
</dt>
<dd>
 Required. Should contain enough information to execute later, for example the required capability.
 This is stored as an option so we can return it to governance when the proposal is resolved.
</dd>
<dt>
<code>metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Optional. Value is serialized value of an attribute.
 Currently, we have three attributes that are used by the voting flow.
 1. RESOLVABLE_TIME_METADATA_KEY: this is uesed to record the resolvable time to ensure that resolution has to be done non-atomically.
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
<code>execution_hash: vector&lt;u8&gt;</code>
</dt>
<dd>
 Required. The hash for the execution script module. Only the same exact script module can resolve this
 proposal.
</dd>
<dt>
<code>min_vote_threshold: u128</code>
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
<code>early_resolution_vote_threshold: option::Option&lt;u128&gt;</code>
</dt>
<dd>
 Optional. Early resolution threshold. If specified, the proposal can be resolved early if the total
 number of yes or no votes passes this threshold.
 For example, this can be set to 50% of the total supply of the voting token, so if > 50% vote yes or no,
 the proposal can be resolved before expiration.
</dd>
<dt>
<code>yes_votes: u128</code>
</dt>
<dd>
 Number of votes for each outcome.
 u128 since the voting power is already u64 and can add up to more than u64 can hold.
</dd>
<dt>
<code>no_votes: u128</code>
</dt>
<dd>

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

<a id="0x1_voting_VotingForum"></a>

## Resource `VotingForum`



<pre><code>struct VotingForum&lt;ProposalType: store&gt; has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposals: table::Table&lt;u64, voting::Proposal&lt;ProposalType&gt;&gt;</code>
</dt>
<dd>
 Use Table for execution optimization instead of Vector for gas cost since Vector is read entirely into memory
 during execution while only relevant Table entries are.
</dd>
<dt>
<code>events: voting::VotingEvents</code>
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

<a id="0x1_voting_VotingEvents"></a>

## Struct `VotingEvents`



<pre><code>struct VotingEvents has store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create_proposal_events: event::EventHandle&lt;voting::CreateProposalEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>register_forum_events: event::EventHandle&lt;voting::RegisterForumEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>resolve_proposal_events: event::EventHandle&lt;voting::ResolveProposal&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: event::EventHandle&lt;voting::VoteEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_CreateProposal"></a>

## Struct `CreateProposal`



<pre><code>&#35;[event]
struct CreateProposal has drop, store
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
<code>early_resolution_vote_threshold: option::Option&lt;u128&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>min_vote_threshold: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_RegisterForum"></a>

## Struct `RegisterForum`



<pre><code>&#35;[event]
struct RegisterForum has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hosting_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_type_info: type_info::TypeInfo</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_Vote"></a>

## Struct `Vote`



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
<code>num_votes: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_ResolveProposal"></a>

## Struct `ResolveProposal`



<pre><code>&#35;[event]
struct ResolveProposal has drop, store
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
<code>yes_votes: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>no_votes: u128</code>
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

<a id="0x1_voting_CreateProposalEvent"></a>

## Struct `CreateProposalEvent`



<pre><code>struct CreateProposalEvent has drop, store
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
<code>early_resolution_vote_threshold: option::Option&lt;u128&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_hash: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>min_vote_threshold: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_RegisterForumEvent"></a>

## Struct `RegisterForumEvent`



<pre><code>struct RegisterForumEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>hosting_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_type_info: type_info::TypeInfo</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_VoteEvent"></a>

## Struct `VoteEvent`



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
<code>num_votes: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_voting_EINVALID_MIN_VOTE_THRESHOLD"></a>

Minimum vote threshold cannot be higher than early resolution threshold.


<pre><code>const EINVALID_MIN_VOTE_THRESHOLD: u64 &#61; 7;
</code></pre>



<a id="0x1_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION"></a>

If a proposal is multi-step, we need to use <code>resolve_proposal_v2()</code> to resolve it.
If we use <code>resolve()</code> to resolve a multi-step proposal, it will fail with EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION.


<pre><code>const EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION: u64 &#61; 10;
</code></pre>



<a id="0x1_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION"></a>

Cannot vote if the specified multi-step proposal is in execution.


<pre><code>const EMULTI_STEP_PROPOSAL_IN_EXECUTION: u64 &#61; 9;
</code></pre>



<a id="0x1_voting_EPROPOSAL_ALREADY_RESOLVED"></a>

Proposal cannot be resolved more than once


<pre><code>const EPROPOSAL_ALREADY_RESOLVED: u64 &#61; 3;
</code></pre>



<a id="0x1_voting_EPROPOSAL_CANNOT_BE_RESOLVED"></a>

Proposal cannot be resolved. Either voting duration has not passed, not enough votes, or fewer yes than no votes


<pre><code>const EPROPOSAL_CANNOT_BE_RESOLVED: u64 &#61; 2;
</code></pre>



<a id="0x1_voting_EPROPOSAL_EMPTY_EXECUTION_HASH"></a>

Proposal cannot contain an empty execution script hash


<pre><code>const EPROPOSAL_EMPTY_EXECUTION_HASH: u64 &#61; 4;
</code></pre>



<a id="0x1_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING"></a>

Current script's execution hash does not match the specified proposal's


<pre><code>const EPROPOSAL_EXECUTION_HASH_NOT_MATCHING: u64 &#61; 1;
</code></pre>



<a id="0x1_voting_EPROPOSAL_IS_SINGLE_STEP"></a>

Cannot call <code>is_multi_step_proposal_in_execution()</code> on single-step proposals.


<pre><code>const EPROPOSAL_IS_SINGLE_STEP: u64 &#61; 12;
</code></pre>



<a id="0x1_voting_EPROPOSAL_VOTING_ALREADY_ENDED"></a>

Proposal's voting period has already ended.


<pre><code>const EPROPOSAL_VOTING_ALREADY_ENDED: u64 &#61; 5;
</code></pre>



<a id="0x1_voting_ERESOLUTION_CANNOT_BE_ATOMIC"></a>

Resolution of a proposal cannot happen atomically in the same transaction as the last vote.


<pre><code>const ERESOLUTION_CANNOT_BE_ATOMIC: u64 &#61; 8;
</code></pre>



<a id="0x1_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH"></a>

If we call <code>resolve_proposal_v2()</code> to resolve a single-step proposal, the <code>next_execution_hash</code> parameter should be an empty vector.


<pre><code>const ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH: u64 &#61; 11;
</code></pre>



<a id="0x1_voting_EVOTING_FORUM_ALREADY_REGISTERED"></a>

Voting forum has already been registered.


<pre><code>const EVOTING_FORUM_ALREADY_REGISTERED: u64 &#61; 6;
</code></pre>



<a id="0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY"></a>

Key used to track if the multi-step proposal is in execution / resolving in progress.


<pre><code>const IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY: vector&lt;u8&gt; &#61; [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 73, 78, 95, 69, 88, 69, 67, 85, 84, 73, 79, 78];
</code></pre>



<a id="0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY"></a>

Key used to track if the proposal is multi-step


<pre><code>const IS_MULTI_STEP_PROPOSAL_KEY: vector&lt;u8&gt; &#61; [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 75, 69, 89];
</code></pre>



<a id="0x1_voting_PROPOSAL_STATE_FAILED"></a>

Proposal has failed because either the min vote threshold is not met or majority voted no.


<pre><code>const PROPOSAL_STATE_FAILED: u64 &#61; 3;
</code></pre>



<a id="0x1_voting_PROPOSAL_STATE_PENDING"></a>

ProposalStateEnum representing proposal state.


<pre><code>const PROPOSAL_STATE_PENDING: u64 &#61; 0;
</code></pre>



<a id="0x1_voting_PROPOSAL_STATE_SUCCEEDED"></a>



<pre><code>const PROPOSAL_STATE_SUCCEEDED: u64 &#61; 1;
</code></pre>



<a id="0x1_voting_RESOLVABLE_TIME_METADATA_KEY"></a>

Key used to track the resolvable time in the proposal's metadata.


<pre><code>const RESOLVABLE_TIME_METADATA_KEY: vector&lt;u8&gt; &#61; [82, 69, 83, 79, 76, 86, 65, 66, 76, 69, 95, 84, 73, 77, 69, 95, 77, 69, 84, 65, 68, 65, 84, 65, 95, 75, 69, 89];
</code></pre>



<a id="0x1_voting_register"></a>

## Function `register`



<pre><code>public fun register&lt;ProposalType: store&gt;(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun register&lt;ProposalType: store&gt;(account: &amp;signer) &#123;
    let addr &#61; signer::address_of(account);
    assert!(!exists&lt;VotingForum&lt;ProposalType&gt;&gt;(addr), error::already_exists(EVOTING_FORUM_ALREADY_REGISTERED));

    let voting_forum &#61; VotingForum&lt;ProposalType&gt; &#123;
        next_proposal_id: 0,
        proposals: table::new&lt;u64, Proposal&lt;ProposalType&gt;&gt;(),
        events: VotingEvents &#123;
            create_proposal_events: account::new_event_handle&lt;CreateProposalEvent&gt;(account),
            register_forum_events: account::new_event_handle&lt;RegisterForumEvent&gt;(account),
            resolve_proposal_events: account::new_event_handle&lt;ResolveProposal&gt;(account),
            vote_events: account::new_event_handle&lt;VoteEvent&gt;(account),
        &#125;
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            RegisterForum &#123;
                hosting_account: addr,
                proposal_type_info: type_info::type_of&lt;ProposalType&gt;(),
            &#125;,
        );
    &#125;;
    event::emit_event&lt;RegisterForumEvent&gt;(
        &amp;mut voting_forum.events.register_forum_events,
        RegisterForumEvent &#123;
            hosting_account: addr,
            proposal_type_info: type_info::type_of&lt;ProposalType&gt;(),
        &#125;,
    );

    move_to(account, voting_forum);
&#125;
</code></pre>



</details>

<a id="0x1_voting_create_proposal"></a>

## Function `create_proposal`

Create a single-step proposal with the given parameters

@param voting_forum_address The forum's address where the proposal will be stored.
@param execution_content The execution content that will be given back at resolution time. This can contain
data such as a capability resource used to scope the execution.
@param execution_hash The hash for the execution script module. Only the same exact script module can resolve
this proposal.
@param min_vote_threshold The minimum number of votes needed to consider this proposal successful.
@param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.
@param early_resolution_vote_threshold The vote threshold for early resolution of this proposal.
@param metadata A simple_map that stores information about this proposal.
@return The proposal id.


<pre><code>public fun create_proposal&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_proposal&lt;ProposalType: store&gt;(
    proposer: address,
    voting_forum_address: address,
    execution_content: ProposalType,
    execution_hash: vector&lt;u8&gt;,
    min_vote_threshold: u128,
    expiration_secs: u64,
    early_resolution_vote_threshold: Option&lt;u128&gt;,
    metadata: SimpleMap&lt;String, vector&lt;u8&gt;&gt;,
): u64 acquires VotingForum &#123;
    create_proposal_v2(
        proposer,
        voting_forum_address,
        execution_content,
        execution_hash,
        min_vote_threshold,
        expiration_secs,
        early_resolution_vote_threshold,
        metadata,
        false
    )
&#125;
</code></pre>



</details>

<a id="0x1_voting_create_proposal_v2"></a>

## Function `create_proposal_v2`

Create a single-step or a multi-step proposal with the given parameters

@param voting_forum_address The forum's address where the proposal will be stored.
@param execution_content The execution content that will be given back at resolution time. This can contain
data such as a capability resource used to scope the execution.
@param execution_hash The sha-256 hash for the execution script module. Only the same exact script module can
resolve this proposal.
@param min_vote_threshold The minimum number of votes needed to consider this proposal successful.
@param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.
@param early_resolution_vote_threshold The vote threshold for early resolution of this proposal.
@param metadata A simple_map that stores information about this proposal.
@param is_multi_step_proposal A bool value that indicates if the proposal is single-step or multi-step.
@return The proposal id.


<pre><code>public fun create_proposal_v2&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_proposal_v2&lt;ProposalType: store&gt;(
    proposer: address,
    voting_forum_address: address,
    execution_content: ProposalType,
    execution_hash: vector&lt;u8&gt;,
    min_vote_threshold: u128,
    expiration_secs: u64,
    early_resolution_vote_threshold: Option&lt;u128&gt;,
    metadata: SimpleMap&lt;String, vector&lt;u8&gt;&gt;,
    is_multi_step_proposal: bool,
): u64 acquires VotingForum &#123;
    if (option::is_some(&amp;early_resolution_vote_threshold)) &#123;
        assert!(
            min_vote_threshold &lt;&#61; &#42;option::borrow(&amp;early_resolution_vote_threshold),
            error::invalid_argument(EINVALID_MIN_VOTE_THRESHOLD),
        );
    &#125;;
    // Make sure the execution script&apos;s hash is not empty.
    assert!(vector::length(&amp;execution_hash) &gt; 0, error::invalid_argument(EPROPOSAL_EMPTY_EXECUTION_HASH));

    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal_id &#61; voting_forum.next_proposal_id;
    voting_forum.next_proposal_id &#61; voting_forum.next_proposal_id &#43; 1;

    // Add a flag to indicate if this proposal is single&#45;step or multi&#45;step.
    simple_map::add(&amp;mut metadata, utf8(IS_MULTI_STEP_PROPOSAL_KEY), to_bytes(&amp;is_multi_step_proposal));

    let is_multi_step_in_execution_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
    if (is_multi_step_proposal) &#123;
        // If the given proposal is a multi&#45;step proposal, we will add a flag to indicate if this multi&#45;step proposal is in execution.
        // This value is by default false. We turn this value to true when we start executing the multi&#45;step proposal. This value
        // will be used to disable further voting after we started executing the multi&#45;step proposal.
        simple_map::add(&amp;mut metadata, is_multi_step_in_execution_key, to_bytes(&amp;false));
        // If the proposal is a single&#45;step proposal, we check if the metadata passed by the client has the IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY key.
        // If they have the key, we will remove it, because a single&#45;step proposal that doesn&apos;t need this key.
    &#125; else if (simple_map::contains_key(&amp;mut metadata, &amp;is_multi_step_in_execution_key)) &#123;
        simple_map::remove(&amp;mut metadata, &amp;is_multi_step_in_execution_key);
    &#125;;

    table::add(&amp;mut voting_forum.proposals, proposal_id, Proposal &#123;
        proposer,
        creation_time_secs: timestamp::now_seconds(),
        execution_content: option::some&lt;ProposalType&gt;(execution_content),
        execution_hash,
        metadata,
        min_vote_threshold,
        expiration_secs,
        early_resolution_vote_threshold,
        yes_votes: 0,
        no_votes: 0,
        is_resolved: false,
        resolution_time_secs: 0,
    &#125;);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            CreateProposal &#123;
                proposal_id,
                early_resolution_vote_threshold,
                execution_hash,
                expiration_secs,
                metadata,
                min_vote_threshold,
            &#125;,
        );
    &#125;;
    event::emit_event&lt;CreateProposalEvent&gt;(
        &amp;mut voting_forum.events.create_proposal_events,
        CreateProposalEvent &#123;
            proposal_id,
            early_resolution_vote_threshold,
            execution_hash,
            expiration_secs,
            metadata,
            min_vote_threshold,
        &#125;,
    );

    proposal_id
&#125;
</code></pre>



</details>

<a id="0x1_voting_vote"></a>

## Function `vote`

Vote on the given proposal.

@param _proof Required so only the governance module that defines ProposalType can initiate voting.
This guarantees that voting eligibility and voting power are controlled by the right governance.
@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@param num_votes Number of votes. Voting power should be calculated by governance.
@param should_pass Whether the votes are for yes or no.


<pre><code>public fun vote&lt;ProposalType: store&gt;(_proof: &amp;ProposalType, voting_forum_address: address, proposal_id: u64, num_votes: u64, should_pass: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vote&lt;ProposalType: store&gt;(
    _proof: &amp;ProposalType,
    voting_forum_address: address,
    proposal_id: u64,
    num_votes: u64,
    should_pass: bool,
) acquires VotingForum &#123;
    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);
    // Voting might still be possible after the proposal has enough yes votes to be resolved early. This would only
    // lead to possible proposal resolution failure if the resolve early threshold is not definitive (e.g. &lt; 50% &#43; 1
    // of the total voting token&apos;s supply). In this case, more voting might actually still be desirable.
    // Governance mechanisms built on this voting module can apply additional rules on when voting is closed as
    // appropriate.
    assert!(!is_voting_period_over(proposal), error::invalid_state(EPROPOSAL_VOTING_ALREADY_ENDED));
    assert!(!proposal.is_resolved, error::invalid_state(EPROPOSAL_ALREADY_RESOLVED));
    // Assert this proposal is single&#45;step, or if the proposal is multi&#45;step, it is not in execution yet.
    assert!(!simple_map::contains_key(&amp;proposal.metadata, &amp;utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY))
        &#124;&#124; &#42;simple_map::borrow(&amp;proposal.metadata, &amp;utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY)) &#61;&#61; to_bytes(
        &amp;false
    ),
        error::invalid_state(EMULTI_STEP_PROPOSAL_IN_EXECUTION));

    if (should_pass) &#123;
        proposal.yes_votes &#61; proposal.yes_votes &#43; (num_votes as u128);
    &#125; else &#123;
        proposal.no_votes &#61; proposal.no_votes &#43; (num_votes as u128);
    &#125;;

    // Record the resolvable time to ensure that resolution has to be done non&#45;atomically.
    let timestamp_secs_bytes &#61; to_bytes(&amp;timestamp::now_seconds());
    let key &#61; utf8(RESOLVABLE_TIME_METADATA_KEY);
    if (simple_map::contains_key(&amp;proposal.metadata, &amp;key)) &#123;
        &#42;simple_map::borrow_mut(&amp;mut proposal.metadata, &amp;key) &#61; timestamp_secs_bytes;
    &#125; else &#123;
        simple_map::add(&amp;mut proposal.metadata, key, timestamp_secs_bytes);
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(Vote &#123; proposal_id, num_votes &#125;);
    &#125;;
    event::emit_event&lt;VoteEvent&gt;(
        &amp;mut voting_forum.events.vote_events,
        VoteEvent &#123; proposal_id, num_votes &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_voting_is_proposal_resolvable"></a>

## Function `is_proposal_resolvable`

Common checks on if a proposal is resolvable, regardless if the proposal is single-step or multi-step.


<pre><code>fun is_proposal_resolvable&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_proposal_resolvable&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
) acquires VotingForum &#123;
    let proposal_state &#61; get_proposal_state&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    assert!(proposal_state &#61;&#61; PROPOSAL_STATE_SUCCEEDED, error::invalid_state(EPROPOSAL_CANNOT_BE_RESOLVED));

    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);
    assert!(!proposal.is_resolved, error::invalid_state(EPROPOSAL_ALREADY_RESOLVED));

    // We need to make sure that the resolution is happening in
    // a separate transaction from the last vote to guard against any potential flashloan attacks.
    let resolvable_time &#61; to_u64(&#42;simple_map::borrow(&amp;proposal.metadata, &amp;utf8(RESOLVABLE_TIME_METADATA_KEY)));
    assert!(timestamp::now_seconds() &gt; resolvable_time, error::invalid_state(ERESOLUTION_CANNOT_BE_ATOMIC));

    assert!(
        transaction_context::get_script_hash() &#61;&#61; proposal.execution_hash,
        error::invalid_argument(EPROPOSAL_EXECUTION_HASH_NOT_MATCHING),
    );
&#125;
</code></pre>



</details>

<a id="0x1_voting_resolve"></a>

## Function `resolve`

Resolve a single-step proposal with given id. Can only be done if there are at least as many votes as min required and
there are more yes votes than no. If either of these conditions is not met, this will revert.

@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.


<pre><code>public fun resolve&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): ProposalType
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resolve&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): ProposalType acquires VotingForum &#123;
    is_proposal_resolvable&lt;ProposalType&gt;(voting_forum_address, proposal_id);

    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);

    // Assert that the specified proposal is not a multi&#45;step proposal.
    let multi_step_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_KEY);
    let has_multi_step_key &#61; simple_map::contains_key(&amp;proposal.metadata, &amp;multi_step_key);
    if (has_multi_step_key) &#123;
        let is_multi_step_proposal &#61; from_bcs::to_bool(&#42;simple_map::borrow(&amp;proposal.metadata, &amp;multi_step_key));
        assert!(
            !is_multi_step_proposal,
            error::permission_denied(EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION)
        );
    &#125;;

    let resolved_early &#61; can_be_resolved_early(proposal);
    proposal.is_resolved &#61; true;
    proposal.resolution_time_secs &#61; timestamp::now_seconds();

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            ResolveProposal &#123;
                proposal_id,
                yes_votes: proposal.yes_votes,
                no_votes: proposal.no_votes,
                resolved_early,
            &#125;,
        );
    &#125;;
    event::emit_event&lt;ResolveProposal&gt;(
        &amp;mut voting_forum.events.resolve_proposal_events,
        ResolveProposal &#123;
            proposal_id,
            yes_votes: proposal.yes_votes,
            no_votes: proposal.no_votes,
            resolved_early,
        &#125;,
    );

    option::extract(&amp;mut proposal.execution_content)
&#125;
</code></pre>



</details>

<a id="0x1_voting_resolve_proposal_v2"></a>

## Function `resolve_proposal_v2`

Resolve a single-step or a multi-step proposal with the given id.
Can only be done if there are at least as many votes as min required and
there are more yes votes than no. If either of these conditions is not met, this will revert.


@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@param next_execution_hash The next execution hash if the given proposal is multi-step.


<pre><code>public fun resolve_proposal_v2&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, next_execution_hash: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resolve_proposal_v2&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
    next_execution_hash: vector&lt;u8&gt;,
) acquires VotingForum &#123;
    is_proposal_resolvable&lt;ProposalType&gt;(voting_forum_address, proposal_id);

    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);

    // Update the IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY key to indicate that the multi&#45;step proposal is in execution.
    let multi_step_in_execution_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
    if (simple_map::contains_key(&amp;proposal.metadata, &amp;multi_step_in_execution_key)) &#123;
        let is_multi_step_proposal_in_execution_value &#61; simple_map::borrow_mut(
            &amp;mut proposal.metadata,
            &amp;multi_step_in_execution_key
        );
        &#42;is_multi_step_proposal_in_execution_value &#61; to_bytes(&amp;true);
    &#125;;

    let multi_step_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_KEY);
    let is_multi_step &#61; simple_map::contains_key(&amp;proposal.metadata, &amp;multi_step_key) &amp;&amp; from_bcs::to_bool(
        &#42;simple_map::borrow(&amp;proposal.metadata, &amp;multi_step_key)
    );
    let next_execution_hash_is_empty &#61; vector::length(&amp;next_execution_hash) &#61;&#61; 0;

    // Assert that if this proposal is single&#45;step, the `next_execution_hash` parameter is empty.
    assert!(
        is_multi_step &#124;&#124; next_execution_hash_is_empty,
        error::invalid_argument(ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH)
    );

    // If the `next_execution_hash` parameter is empty, it means that either
    // &#45; this proposal is a single&#45;step proposal, or
    // &#45; this proposal is multi&#45;step and we&apos;re currently resolving the last step in the multi&#45;step proposal.
    // We can mark that this proposal is resolved.
    if (next_execution_hash_is_empty) &#123;
        proposal.is_resolved &#61; true;
        proposal.resolution_time_secs &#61; timestamp::now_seconds();

        // Set the `IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY` value to false upon successful resolution of the last step of a multi&#45;step proposal.
        if (is_multi_step) &#123;
            let is_multi_step_proposal_in_execution_value &#61; simple_map::borrow_mut(
                &amp;mut proposal.metadata,
                &amp;multi_step_in_execution_key
            );
            &#42;is_multi_step_proposal_in_execution_value &#61; to_bytes(&amp;false);
        &#125;;
    &#125; else &#123;
        // If the current step is not the last step,
        // update the proposal&apos;s execution hash on&#45;chain to the execution hash of the next step.
        proposal.execution_hash &#61; next_execution_hash;
    &#125;;

    // For single&#45;step proposals, we emit one `ResolveProposal` event per proposal.
    // For multi&#45;step proposals, we emit one `ResolveProposal` event per step in the multi&#45;step proposal. This means
    // that we emit multiple `ResolveProposal` events for the same multi&#45;step proposal.
    let resolved_early &#61; can_be_resolved_early(proposal);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            ResolveProposal &#123;
                proposal_id,
                yes_votes: proposal.yes_votes,
                no_votes: proposal.no_votes,
                resolved_early,
            &#125;,
        );
    &#125;;
    event::emit_event(
        &amp;mut voting_forum.events.resolve_proposal_events,
        ResolveProposal &#123;
            proposal_id,
            yes_votes: proposal.yes_votes,
            no_votes: proposal.no_votes,
            resolved_early,
        &#125;,
    );

&#125;
</code></pre>



</details>

<a id="0x1_voting_next_proposal_id"></a>

## Function `next_proposal_id`

Return the next unassigned proposal id


<pre><code>&#35;[view]
public fun next_proposal_id&lt;ProposalType: store&gt;(voting_forum_address: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun next_proposal_id&lt;ProposalType: store&gt;(voting_forum_address: address, ): u64 acquires VotingForum &#123;
    let voting_forum &#61; borrow_global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    voting_forum.next_proposal_id
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_proposer"></a>

## Function `get_proposer`



<pre><code>&#35;[view]
public fun get_proposer&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposer&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64
): address acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.proposer
&#125;
</code></pre>



</details>

<a id="0x1_voting_is_voting_closed"></a>

## Function `is_voting_closed`



<pre><code>&#35;[view]
public fun is_voting_closed&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_voting_closed&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64
): bool acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    can_be_resolved_early(proposal) &#124;&#124; is_voting_period_over(proposal)
&#125;
</code></pre>



</details>

<a id="0x1_voting_can_be_resolved_early"></a>

## Function `can_be_resolved_early`

Return true if the proposal has reached early resolution threshold (if specified).


<pre><code>public fun can_be_resolved_early&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_be_resolved_early&lt;ProposalType: store&gt;(proposal: &amp;Proposal&lt;ProposalType&gt;): bool &#123;
    if (option::is_some(&amp;proposal.early_resolution_vote_threshold)) &#123;
        let early_resolution_threshold &#61; &#42;option::borrow(&amp;proposal.early_resolution_vote_threshold);
        if (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124; proposal.no_votes &gt;&#61; early_resolution_threshold) &#123;
            return true
        &#125;;
    &#125;;
    false
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_proposal_metadata"></a>

## Function `get_proposal_metadata`



<pre><code>&#35;[view]
public fun get_proposal_metadata&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_metadata&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): SimpleMap&lt;String, vector&lt;u8&gt;&gt; acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.metadata
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_proposal_metadata_value"></a>

## Function `get_proposal_metadata_value`



<pre><code>&#35;[view]
public fun get_proposal_metadata_value&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, metadata_key: string::String): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_metadata_value&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
    metadata_key: String,
): vector&lt;u8&gt; acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    &#42;simple_map::borrow(&amp;proposal.metadata, &amp;metadata_key)
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_proposal_state"></a>

## Function `get_proposal_state`

Return the state of the proposal with given id.

@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@return Proposal state as an enum value.


<pre><code>&#35;[view]
public fun get_proposal_state&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_state&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): u64 acquires VotingForum &#123;
    if (is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id)) &#123;
        let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
        let yes_votes &#61; proposal.yes_votes;
        let no_votes &#61; proposal.no_votes;

        if (yes_votes &gt; no_votes &amp;&amp; yes_votes &#43; no_votes &gt;&#61; proposal.min_vote_threshold) &#123;
            PROPOSAL_STATE_SUCCEEDED
        &#125; else &#123;
            PROPOSAL_STATE_FAILED
        &#125;
    &#125; else &#123;
        PROPOSAL_STATE_PENDING
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_proposal_creation_secs"></a>

## Function `get_proposal_creation_secs`

Return the proposal's creation time.


<pre><code>&#35;[view]
public fun get_proposal_creation_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_creation_secs&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): u64 acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.creation_time_secs
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_proposal_expiration_secs"></a>

## Function `get_proposal_expiration_secs`

Return the proposal's expiration time.


<pre><code>&#35;[view]
public fun get_proposal_expiration_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_expiration_secs&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): u64 acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.expiration_secs
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_execution_hash"></a>

## Function `get_execution_hash`

Return the proposal's execution hash.


<pre><code>&#35;[view]
public fun get_execution_hash&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_execution_hash&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): vector&lt;u8&gt; acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.execution_hash
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_min_vote_threshold"></a>

## Function `get_min_vote_threshold`

Return the proposal's minimum vote threshold


<pre><code>&#35;[view]
public fun get_min_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_min_vote_threshold&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): u128 acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.min_vote_threshold
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_early_resolution_vote_threshold"></a>

## Function `get_early_resolution_vote_threshold`

Return the proposal's early resolution minimum vote threshold (optionally set)


<pre><code>&#35;[view]
public fun get_early_resolution_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): option::Option&lt;u128&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_early_resolution_vote_threshold&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): Option&lt;u128&gt; acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.early_resolution_vote_threshold
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_votes"></a>

## Function `get_votes`

Return the proposal's current vote count (yes_votes, no_votes)


<pre><code>&#35;[view]
public fun get_votes&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): (u128, u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_votes&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): (u128, u128) acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    (proposal.yes_votes, proposal.no_votes)
&#125;
</code></pre>



</details>

<a id="0x1_voting_is_resolved"></a>

## Function `is_resolved`

Return true if the governance proposal has already been resolved.


<pre><code>&#35;[view]
public fun is_resolved&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_resolved&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): bool acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.is_resolved
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_resolution_time_secs"></a>

## Function `get_resolution_time_secs`



<pre><code>&#35;[view]
public fun get_resolution_time_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_resolution_time_secs&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): u64 acquires VotingForum &#123;
    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    proposal.resolution_time_secs
&#125;
</code></pre>



</details>

<a id="0x1_voting_is_multi_step_proposal_in_execution"></a>

## Function `is_multi_step_proposal_in_execution`

Return true if the multi-step governance proposal is in execution.


<pre><code>&#35;[view]
public fun is_multi_step_proposal_in_execution&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_multi_step_proposal_in_execution&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): bool acquires VotingForum &#123;
    let voting_forum &#61; borrow_global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal &#61; table::borrow(&amp;voting_forum.proposals, proposal_id);
    let is_multi_step_in_execution_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
    assert!(
        simple_map::contains_key(&amp;proposal.metadata, &amp;is_multi_step_in_execution_key),
        error::invalid_argument(EPROPOSAL_IS_SINGLE_STEP)
    );
    from_bcs::to_bool(&#42;simple_map::borrow(&amp;proposal.metadata, &amp;is_multi_step_in_execution_key))
&#125;
</code></pre>



</details>

<a id="0x1_voting_is_voting_period_over"></a>

## Function `is_voting_period_over`

Return true if the voting period of the given proposal has already ended.


<pre><code>fun is_voting_period_over&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_voting_period_over&lt;ProposalType: store&gt;(proposal: &amp;Proposal&lt;ProposalType&gt;): bool &#123;
    timestamp::now_seconds() &gt; proposal.expiration_secs
&#125;
</code></pre>



</details>

<a id="0x1_voting_get_proposal"></a>

## Function `get_proposal`



<pre><code>fun get_proposal&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): &amp;voting::Proposal&lt;ProposalType&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun get_proposal&lt;ProposalType: store&gt;(
    voting_forum_address: address,
    proposal_id: u64,
): &amp;Proposal&lt;ProposalType&gt; acquires VotingForum &#123;
    let voting_forum &#61; borrow_global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    table::borrow(&amp;voting_forum.proposals, proposal_id)
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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code>public fun register&lt;ProposalType: store&gt;(account: &amp;signer)
</code></pre>




<pre><code>let addr &#61; signer::address_of(account);
aborts_if exists&lt;VotingForum&lt;ProposalType&gt;&gt;(addr);
aborts_if !exists&lt;account::Account&gt;(addr);
let register_account &#61; global&lt;account::Account&gt;(addr);
aborts_if register_account.guid_creation_num &#43; 4 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if register_account.guid_creation_num &#43; 4 &gt; MAX_U64;
aborts_if !type_info::spec_is_struct&lt;ProposalType&gt;();
ensures exists&lt;VotingForum&lt;ProposalType&gt;&gt;(addr);
</code></pre>



<a id="@Specification_1_create_proposal"></a>

### Function `create_proposal`


<pre><code>public fun create_proposal&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;): u64
</code></pre>




<pre><code>requires chain_status::is_operating();
include CreateProposalAbortsIfAndEnsures&lt;ProposalType&gt;&#123;is_multi_step_proposal: false&#125;;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures result &#61;&#61; old(global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;
</code></pre>



<a id="@Specification_1_create_proposal_v2"></a>

### Function `create_proposal_v2`


<pre><code>public fun create_proposal_v2&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64
</code></pre>




<pre><code>requires chain_status::is_operating();
include CreateProposalAbortsIfAndEnsures&lt;ProposalType&gt;;
ensures result &#61;&#61; old(global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;
</code></pre>




<a id="0x1_voting_CreateProposalAbortsIfAndEnsures"></a>


<pre><code>schema CreateProposalAbortsIfAndEnsures&lt;ProposalType&gt; &#123;
    voting_forum_address: address;
    execution_hash: vector&lt;u8&gt;;
    min_vote_threshold: u128;
    early_resolution_vote_threshold: Option&lt;u128&gt;;
    metadata: SimpleMap&lt;String, vector&lt;u8&gt;&gt;;
    is_multi_step_proposal: bool;
    let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal_id &#61; voting_forum.next_proposal_id;
    aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    aborts_if table::spec_contains(voting_forum.proposals,proposal_id);
    aborts_if len(early_resolution_vote_threshold.vec) !&#61; 0 &amp;&amp; min_vote_threshold &gt; early_resolution_vote_threshold.vec[0];
    aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_KEY);
    aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
    aborts_if len(execution_hash) &#61;&#61; 0;
    let execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_KEY);
    aborts_if simple_map::spec_contains_key(metadata, execution_key);
    aborts_if voting_forum.next_proposal_id &#43; 1 &gt; MAX_U64;
    let is_multi_step_in_execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
    aborts_if is_multi_step_proposal &amp;&amp; simple_map::spec_contains_key(metadata, is_multi_step_in_execution_key);
    let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let post post_metadata &#61; table::spec_get(post_voting_forum.proposals, proposal_id).metadata;
    ensures post_voting_forum.next_proposal_id &#61;&#61; voting_forum.next_proposal_id &#43; 1;
    ensures table::spec_contains(post_voting_forum.proposals, proposal_id);
    ensures if (is_multi_step_proposal) &#123;
        simple_map::spec_get(post_metadata, is_multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(false)
    &#125; else &#123;
        !simple_map::spec_contains_key(post_metadata, is_multi_step_in_execution_key)
    &#125;;
&#125;
</code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code>public fun vote&lt;ProposalType: store&gt;(_proof: &amp;ProposalType, voting_forum_address: address, proposal_id: u64, num_votes: u64, should_pass: bool)
</code></pre>




<pre><code>requires chain_status::is_operating();
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
aborts_if is_voting_period_over(proposal);
aborts_if proposal.is_resolved;
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
let execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
aborts_if simple_map::spec_contains_key(proposal.metadata, execution_key) &amp;&amp;
          simple_map::spec_get(proposal.metadata, execution_key) !&#61; std::bcs::serialize(false);
aborts_if if (should_pass) &#123; proposal.yes_votes &#43; num_votes &gt; MAX_U128 &#125; else &#123; proposal.no_votes &#43; num_votes &gt; MAX_U128 &#125;;
aborts_if !std::string::spec_internal_check_utf8(RESOLVABLE_TIME_METADATA_KEY);
let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);
ensures if (should_pass) &#123;
    post_proposal.yes_votes &#61;&#61; proposal.yes_votes &#43; num_votes
&#125; else &#123;
    post_proposal.no_votes &#61;&#61; proposal.no_votes &#43; num_votes
&#125;;
let timestamp_secs_bytes &#61; std::bcs::serialize(timestamp::spec_now_seconds());
let key &#61; std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY);
ensures simple_map::spec_get(post_proposal.metadata, key) &#61;&#61; timestamp_secs_bytes;
</code></pre>



<a id="@Specification_1_is_proposal_resolvable"></a>

### Function `is_proposal_resolvable`


<pre><code>fun is_proposal_resolvable&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64)
</code></pre>




<pre><code>requires chain_status::is_operating();
include IsProposalResolvableAbortsIf&lt;ProposalType&gt;;
</code></pre>




<a id="0x1_voting_IsProposalResolvableAbortsIf"></a>


<pre><code>schema IsProposalResolvableAbortsIf&lt;ProposalType&gt; &#123;
    voting_forum_address: address;
    proposal_id: u64;
    include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
    let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
    let voting_closed &#61; spec_is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id);
    aborts_if voting_closed &amp;&amp; (proposal.yes_votes &lt;&#61; proposal.no_votes &#124;&#124; proposal.yes_votes &#43; proposal.no_votes &lt; proposal.min_vote_threshold);
    aborts_if !voting_closed;
    aborts_if proposal.is_resolved;
    aborts_if !std::string::spec_internal_check_utf8(RESOLVABLE_TIME_METADATA_KEY);
    aborts_if !simple_map::spec_contains_key(proposal.metadata, std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY));
    aborts_if !from_bcs::deserializable&lt;u64&gt;(simple_map::spec_get(proposal.metadata, std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY)));
    aborts_if timestamp::spec_now_seconds() &lt;&#61; from_bcs::deserialize&lt;u64&gt;(simple_map::spec_get(proposal.metadata, std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY)));
    aborts_if transaction_context::spec_get_script_hash() !&#61; proposal.execution_hash;
&#125;
</code></pre>



<a id="@Specification_1_resolve"></a>

### Function `resolve`


<pre><code>public fun resolve&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): ProposalType
</code></pre>




<pre><code>requires chain_status::is_operating();
include IsProposalResolvableAbortsIf&lt;ProposalType&gt;;
aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_KEY);
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
let multi_step_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_KEY);
let has_multi_step_key &#61; simple_map::spec_contains_key(proposal.metadata, multi_step_key);
aborts_if has_multi_step_key &amp;&amp; !from_bcs::deserializable&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));
aborts_if has_multi_step_key &amp;&amp; from_bcs::deserialize&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));
let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures post_proposal.is_resolved &#61;&#61; true;
ensures post_proposal.resolution_time_secs &#61;&#61; timestamp::spec_now_seconds();
aborts_if option::spec_is_none(proposal.execution_content);
ensures result &#61;&#61; option::spec_borrow(proposal.execution_content);
ensures option::spec_is_none(post_proposal.execution_content);
</code></pre>



<a id="@Specification_1_resolve_proposal_v2"></a>

### Function `resolve_proposal_v2`


<pre><code>public fun resolve_proposal_v2&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, next_execution_hash: vector&lt;u8&gt;)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;
requires chain_status::is_operating();
include IsProposalResolvableAbortsIf&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);
let multi_step_in_execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_KEY);
ensures (simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key) &amp;&amp; len(next_execution_hash) !&#61; 0) &#61;&#61;&gt;
    simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(true);
ensures (simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key) &amp;&amp;
    (len(next_execution_hash) &#61;&#61; 0 &amp;&amp; !is_multi_step)) &#61;&#61;&gt;
    simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(true);
let multi_step_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_KEY);
aborts_if simple_map::spec_contains_key(proposal.metadata, multi_step_key) &amp;&amp;
    !from_bcs::deserializable&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));
let is_multi_step &#61; simple_map::spec_contains_key(proposal.metadata, multi_step_key) &amp;&amp;
    from_bcs::deserialize(simple_map::spec_get(proposal.metadata, multi_step_key));
aborts_if !is_multi_step &amp;&amp; len(next_execution_hash) !&#61; 0;
aborts_if len(next_execution_hash) &#61;&#61; 0 &amp;&amp; !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
aborts_if len(next_execution_hash) &#61;&#61; 0 &amp;&amp; is_multi_step &amp;&amp; !simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key);
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
ensures len(next_execution_hash) &#61;&#61; 0 &#61;&#61;&gt; post_proposal.resolution_time_secs &#61;&#61; timestamp::spec_now_seconds();
ensures len(next_execution_hash) &#61;&#61; 0 &#61;&#61;&gt; post_proposal.is_resolved &#61;&#61; true;
ensures (len(next_execution_hash) &#61;&#61; 0 &amp;&amp; is_multi_step) &#61;&#61;&gt; simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(false);
ensures len(next_execution_hash) !&#61; 0 &#61;&#61;&gt; post_proposal.execution_hash &#61;&#61; next_execution_hash;
</code></pre>



<a id="@Specification_1_next_proposal_id"></a>

### Function `next_proposal_id`


<pre><code>&#35;[view]
public fun next_proposal_id&lt;ProposalType: store&gt;(voting_forum_address: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
ensures result &#61;&#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address).next_proposal_id;
</code></pre>



<a id="@Specification_1_get_proposer"></a>

### Function `get_proposer`


<pre><code>&#35;[view]
public fun get_proposer&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): address
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.proposer;
</code></pre>



<a id="@Specification_1_is_voting_closed"></a>

### Function `is_voting_closed`


<pre><code>&#35;[view]
public fun is_voting_closed&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool
</code></pre>




<pre><code>requires chain_status::is_operating();
include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
ensures result &#61;&#61; spec_is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id);
</code></pre>




<a id="0x1_voting_spec_is_voting_closed"></a>


<pre><code>fun spec_is_voting_closed&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool &#123;
   let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
   let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
   spec_can_be_resolved_early&lt;ProposalType&gt;(proposal) &#124;&#124; is_voting_period_over(proposal)
&#125;
</code></pre>



<a id="@Specification_1_can_be_resolved_early"></a>

### Function `can_be_resolved_early`


<pre><code>public fun can_be_resolved_early&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_can_be_resolved_early&lt;ProposalType&gt;(proposal);
</code></pre>




<a id="0x1_voting_spec_can_be_resolved_early"></a>


<pre><code>fun spec_can_be_resolved_early&lt;ProposalType: store&gt;(proposal: Proposal&lt;ProposalType&gt;): bool &#123;
   if (option::spec_is_some(proposal.early_resolution_vote_threshold)) &#123;
       let early_resolution_threshold &#61; option::spec_borrow(proposal.early_resolution_vote_threshold);
       if (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124; proposal.no_votes &gt;&#61; early_resolution_threshold) &#123;
           true
       &#125; else&#123;
           false
       &#125;
   &#125; else &#123;
       false
   &#125;
&#125;
</code></pre>




<a id="0x1_voting_spec_get_proposal_state"></a>


<pre><code>fun spec_get_proposal_state&lt;ProposalType&gt;(
   voting_forum_address: address,
   proposal_id: u64,
   voting_forum: VotingForum&lt;ProposalType&gt;
): u64 &#123;
   let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
   let voting_closed &#61; spec_is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id);
   let proposal_vote_cond &#61; (proposal.yes_votes &gt; proposal.no_votes &amp;&amp; proposal.yes_votes &#43; proposal.no_votes &gt;&#61; proposal.min_vote_threshold);
   if (voting_closed &amp;&amp; proposal_vote_cond) &#123;
       PROPOSAL_STATE_SUCCEEDED
   &#125; else if (voting_closed &amp;&amp; !proposal_vote_cond) &#123;
       PROPOSAL_STATE_FAILED
   &#125; else &#123;
       PROPOSAL_STATE_PENDING
   &#125;
&#125;
</code></pre>




<a id="0x1_voting_spec_get_proposal_expiration_secs"></a>


<pre><code>fun spec_get_proposal_expiration_secs&lt;ProposalType: store&gt;(
   voting_forum_address: address,
   proposal_id: u64,
): u64 &#123;
   let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
   let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
   proposal.expiration_secs
&#125;
</code></pre>



<a id="@Specification_1_get_proposal_metadata"></a>

### Function `get_proposal_metadata`


<pre><code>&#35;[view]
public fun get_proposal_metadata&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.metadata;
</code></pre>



<a id="@Specification_1_get_proposal_metadata_value"></a>

### Function `get_proposal_metadata_value`


<pre><code>&#35;[view]
public fun get_proposal_metadata_value&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, metadata_key: string::String): vector&lt;u8&gt;
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
aborts_if !simple_map::spec_contains_key(proposal.metadata, metadata_key);
ensures result &#61;&#61; simple_map::spec_get(proposal.metadata, metadata_key);
</code></pre>



<a id="@Specification_1_get_proposal_state"></a>

### Function `get_proposal_state`


<pre><code>&#35;[view]
public fun get_proposal_state&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>




<pre><code>pragma addition_overflow_unchecked;
requires chain_status::is_operating();
include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
ensures result &#61;&#61; spec_get_proposal_state(voting_forum_address, proposal_id, voting_forum);
</code></pre>



<a id="@Specification_1_get_proposal_creation_secs"></a>

### Function `get_proposal_creation_secs`


<pre><code>&#35;[view]
public fun get_proposal_creation_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.creation_time_secs;
</code></pre>



<a id="@Specification_1_get_proposal_expiration_secs"></a>

### Function `get_proposal_expiration_secs`


<pre><code>&#35;[view]
public fun get_proposal_expiration_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
ensures result &#61;&#61; spec_get_proposal_expiration_secs&lt;ProposalType&gt;(voting_forum_address, proposal_id);
</code></pre>



<a id="@Specification_1_get_execution_hash"></a>

### Function `get_execution_hash`


<pre><code>&#35;[view]
public fun get_execution_hash&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): vector&lt;u8&gt;
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.execution_hash;
</code></pre>



<a id="@Specification_1_get_min_vote_threshold"></a>

### Function `get_min_vote_threshold`


<pre><code>&#35;[view]
public fun get_min_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u128
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.min_vote_threshold;
</code></pre>



<a id="@Specification_1_get_early_resolution_vote_threshold"></a>

### Function `get_early_resolution_vote_threshold`


<pre><code>&#35;[view]
public fun get_early_resolution_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): option::Option&lt;u128&gt;
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.early_resolution_vote_threshold;
</code></pre>



<a id="@Specification_1_get_votes"></a>

### Function `get_votes`


<pre><code>&#35;[view]
public fun get_votes&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): (u128, u128)
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result_1 &#61;&#61; proposal.yes_votes;
ensures result_2 &#61;&#61; proposal.no_votes;
</code></pre>



<a id="@Specification_1_is_resolved"></a>

### Function `is_resolved`


<pre><code>&#35;[view]
public fun is_resolved&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.is_resolved;
</code></pre>




<a id="0x1_voting_AbortsIfNotContainProposalID"></a>


<pre><code>schema AbortsIfNotContainProposalID&lt;ProposalType&gt; &#123;
    proposal_id: u64;
    voting_forum_address: address;
    let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
    aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);
    aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
&#125;
</code></pre>



<a id="@Specification_1_get_resolution_time_secs"></a>

### Function `get_resolution_time_secs`


<pre><code>&#35;[view]
public fun get_resolution_time_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);
ensures result &#61;&#61; proposal.resolution_time_secs;
</code></pre>



<a id="@Specification_1_is_multi_step_proposal_in_execution"></a>

### Function `is_multi_step_proposal_in_execution`


<pre><code>&#35;[view]
public fun is_multi_step_proposal_in_execution&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool
</code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;
let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);
let proposal &#61; table::spec_get(voting_forum.proposals,proposal_id);
aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
let execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);
aborts_if !simple_map::spec_contains_key(proposal.metadata,execution_key);
let is_multi_step_in_execution_key &#61; simple_map::spec_get(proposal.metadata,execution_key);
aborts_if !from_bcs::deserializable&lt;bool&gt;(is_multi_step_in_execution_key);
ensures result &#61;&#61; from_bcs::deserialize&lt;bool&gt;(is_multi_step_in_execution_key);
</code></pre>



<a id="@Specification_1_is_voting_period_over"></a>

### Function `is_voting_period_over`


<pre><code>fun is_voting_period_over&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool
</code></pre>




<pre><code>requires chain_status::is_operating();
aborts_if false;
ensures result &#61;&#61; (timestamp::spec_now_seconds() &gt; proposal.expiration_secs);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
