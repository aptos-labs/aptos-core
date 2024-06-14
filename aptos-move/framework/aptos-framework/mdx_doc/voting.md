
<a id="0x1_voting"></a>

# Module `0x1::voting`


This is the general Voting module that can be used as part of a DAO Governance. Voting is designed to be used by
standalone governance modules, who has full control over the voting flow and is responsible for voting power
calculation and including proper capabilities when creating the proposal so resolution can go through.
On&#45;chain governance of the Aptos network also uses Voting.

The voting flow:
1. The Voting module can be deployed at a known address (e.g. 0x1 for Aptos on&#45;chain governance)
2. The governance module, e.g. AptosGovernance, can be deployed later and define a GovernanceProposal resource type
that can also contain other information such as Capability resource for authorization.
3. The governance module&apos;s owner can then register the ProposalType with Voting. This also hosts the proposal list
(forum) on the calling account.
4. A proposer, through the governance module, can call Voting::create_proposal to create a proposal. create_proposal
cannot be called directly not through the governance module. A script hash of the resolution script that can later
be called to execute the proposal is required.
5. A voter, through the governance module, can call Voting::vote on a proposal. vote requires passing a &amp;ProposalType
and thus only the governance module that registers ProposalType can call vote.
6. Once the proposal&apos;s expiration time has passed and more than the defined threshold has voted yes on the proposal,
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;<br /></code></pre>



<a id="0x1_voting_Proposal"></a>

## Struct `Proposal`

Extra metadata (e.g. description, code url) can be part of the ProposalType struct.


<pre><code><b>struct</b> <a href="voting.md#0x1_voting_Proposal">Proposal</a>&lt;ProposalType: store&gt; <b>has</b> store<br /></code></pre>



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
 1. RESOLVABLE_TIME_METADATA_KEY: this is uesed to record the resolvable time to ensure that resolution has to be done non&#45;atomically.
 2. IS_MULTI_STEP_PROPOSAL_KEY: this is used to track if a proposal is single&#45;step or multi&#45;step.
 3. IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY: this attribute only applies to multi&#45;step proposals. A single&#45;step proposal will not have
 this field in its metadata map. The value is used to indicate if a multi&#45;step proposal is in execution. If yes, we will disable further
 voting for this multi&#45;step proposal.
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
<code>early_resolution_vote_threshold: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;</code>
</dt>
<dd>
 Optional. Early resolution threshold. If specified, the proposal can be resolved early if the total
 number of yes or no votes passes this threshold.
 For example, this can be set to 50% of the total supply of the voting token, so if &gt; 50% vote yes or no,
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



<pre><code><b>struct</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType: store&gt; <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposals: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="voting.md#0x1_voting_Proposal">voting::Proposal</a>&lt;ProposalType&gt;&gt;</code>
</dt>
<dd>
 Use Table for execution optimization instead of Vector for gas cost since Vector is read entirely into memory
 during execution while only relevant Table entries are.
</dd>
<dt>
<code>events: <a href="voting.md#0x1_voting_VotingEvents">voting::VotingEvents</a></code>
</dt>
<dd>

</dd>
<dt>
<code>next_proposal_id: u64</code>
</dt>
<dd>
 Unique identifier for a proposal. This allows for 2 &#42; 10&#42;&#42;19 proposals.
</dd>
</dl>


</details>

<a id="0x1_voting_VotingEvents"></a>

## Struct `VotingEvents`



<pre><code><b>struct</b> <a href="voting.md#0x1_voting_VotingEvents">VotingEvents</a> <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="voting.md#0x1_voting_CreateProposalEvent">voting::CreateProposalEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>register_forum_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="voting.md#0x1_voting_RegisterForumEvent">voting::RegisterForumEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>resolve_proposal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="voting.md#0x1_voting_ResolveProposal">voting::ResolveProposal</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="voting.md#0x1_voting_VoteEvent">voting::VoteEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_CreateProposal"></a>

## Struct `CreateProposal`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="voting.md#0x1_voting_CreateProposal">CreateProposal</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>early_resolution_vote_threshold: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;</code>
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
<code>min_vote_threshold: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_RegisterForum"></a>

## Struct `RegisterForum`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="voting.md#0x1_voting_RegisterForum">RegisterForum</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_voting_Vote"></a>

## Struct `Vote`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="voting.md#0x1_voting_Vote">Vote</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="voting.md#0x1_voting_CreateProposalEvent">CreateProposalEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>early_resolution_vote_threshold: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;</code>
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
<code>min_vote_threshold: u128</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_voting_RegisterForumEvent"></a>

## Struct `RegisterForumEvent`



<pre><code><b>struct</b> <a href="voting.md#0x1_voting_RegisterForumEvent">RegisterForumEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_voting_VoteEvent"></a>

## Struct `VoteEvent`



<pre><code><b>struct</b> <a href="voting.md#0x1_voting_VoteEvent">VoteEvent</a> <b>has</b> drop, store<br /></code></pre>



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


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EINVALID_MIN_VOTE_THRESHOLD">EINVALID_MIN_VOTE_THRESHOLD</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION"></a>

If a proposal is multi&#45;step, we need to use <code><a href="voting.md#0x1_voting_resolve_proposal_v2">resolve_proposal_v2</a>()</code> to resolve it.
If we use <code><a href="voting.md#0x1_voting_resolve">resolve</a>()</code> to resolve a multi&#45;step proposal, it will fail with EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION">EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION"></a>

Cannot vote if the specified multi&#45;step proposal is in execution.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION">EMULTI_STEP_PROPOSAL_IN_EXECUTION</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_voting_EPROPOSAL_ALREADY_RESOLVED"></a>

Proposal cannot be resolved more than once


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EPROPOSAL_ALREADY_RESOLVED">EPROPOSAL_ALREADY_RESOLVED</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_voting_EPROPOSAL_CANNOT_BE_RESOLVED"></a>

Proposal cannot be resolved. Either voting duration has not passed, not enough votes, or fewer yes than no votes


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EPROPOSAL_CANNOT_BE_RESOLVED">EPROPOSAL_CANNOT_BE_RESOLVED</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_voting_EPROPOSAL_EMPTY_EXECUTION_HASH"></a>

Proposal cannot contain an empty execution script hash


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EPROPOSAL_EMPTY_EXECUTION_HASH">EPROPOSAL_EMPTY_EXECUTION_HASH</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING"></a>

Current script&apos;s execution hash does not match the specified proposal&apos;s


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING">EPROPOSAL_EXECUTION_HASH_NOT_MATCHING</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_voting_EPROPOSAL_IS_SINGLE_STEP"></a>

Cannot call <code><a href="voting.md#0x1_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>()</code> on single&#45;step proposals.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EPROPOSAL_IS_SINGLE_STEP">EPROPOSAL_IS_SINGLE_STEP</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_voting_EPROPOSAL_VOTING_ALREADY_ENDED"></a>

Proposal&apos;s voting period has already ended.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EPROPOSAL_VOTING_ALREADY_ENDED">EPROPOSAL_VOTING_ALREADY_ENDED</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_voting_ERESOLUTION_CANNOT_BE_ATOMIC"></a>

Resolution of a proposal cannot happen atomically in the same transaction as the last vote.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_ERESOLUTION_CANNOT_BE_ATOMIC">ERESOLUTION_CANNOT_BE_ATOMIC</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH"></a>

If we call <code><a href="voting.md#0x1_voting_resolve_proposal_v2">resolve_proposal_v2</a>()</code> to resolve a single&#45;step proposal, the <code>next_execution_hash</code> parameter should be an empty vector.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH">ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_voting_EVOTING_FORUM_ALREADY_REGISTERED"></a>

Voting forum has already been registered.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_EVOTING_FORUM_ALREADY_REGISTERED">EVOTING_FORUM_ALREADY_REGISTERED</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY"></a>

Key used to track if the multi&#45;step proposal is in execution / resolving in progress.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 73, 78, 95, 69, 88, 69, 67, 85, 84, 73, 79, 78];<br /></code></pre>



<a id="0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY"></a>

Key used to track if the proposal is multi&#45;step


<pre><code><b>const</b> <a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 75, 69, 89];<br /></code></pre>



<a id="0x1_voting_PROPOSAL_STATE_FAILED"></a>

Proposal has failed because either the min vote threshold is not met or majority voted no.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_PROPOSAL_STATE_FAILED">PROPOSAL_STATE_FAILED</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_voting_PROPOSAL_STATE_PENDING"></a>

ProposalStateEnum representing proposal state.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_PROPOSAL_STATE_PENDING">PROPOSAL_STATE_PENDING</a>: u64 &#61; 0;<br /></code></pre>



<a id="0x1_voting_PROPOSAL_STATE_SUCCEEDED"></a>



<pre><code><b>const</b> <a href="voting.md#0x1_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_voting_RESOLVABLE_TIME_METADATA_KEY"></a>

Key used to track the resolvable time in the proposal&apos;s metadata.


<pre><code><b>const</b> <a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [82, 69, 83, 79, 76, 86, 65, 66, 76, 69, 95, 84, 73, 77, 69, 95, 77, 69, 84, 65, 68, 65, 84, 65, 95, 75, 69, 89];<br /></code></pre>



<a id="0x1_voting_register"></a>

## Function `register`



<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_register">register</a>&lt;ProposalType: store&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_register">register</a>&lt;ProposalType: store&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(!<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="voting.md#0x1_voting_EVOTING_FORUM_ALREADY_REGISTERED">EVOTING_FORUM_ALREADY_REGISTERED</a>));<br /><br />    <b>let</b> voting_forum &#61; <a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt; &#123;<br />        next_proposal_id: 0,<br />        proposals: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;u64, <a href="voting.md#0x1_voting_Proposal">Proposal</a>&lt;ProposalType&gt;&gt;(),<br />        events: <a href="voting.md#0x1_voting_VotingEvents">VotingEvents</a> &#123;<br />            create_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="voting.md#0x1_voting_CreateProposalEvent">CreateProposalEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),<br />            register_forum_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="voting.md#0x1_voting_RegisterForumEvent">RegisterForumEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),<br />            resolve_proposal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a>&gt;(<a href="account.md#0x1_account">account</a>),<br />            vote_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="voting.md#0x1_voting_VoteEvent">VoteEvent</a>&gt;(<a href="account.md#0x1_account">account</a>),<br />        &#125;<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="voting.md#0x1_voting_RegisterForum">RegisterForum</a> &#123;<br />                hosting_account: addr,<br />                proposal_type_info: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;ProposalType&gt;(),<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="voting.md#0x1_voting_RegisterForumEvent">RegisterForumEvent</a>&gt;(<br />        &amp;<b>mut</b> voting_forum.events.register_forum_events,<br />        <a href="voting.md#0x1_voting_RegisterForumEvent">RegisterForumEvent</a> &#123;<br />            hosting_account: addr,<br />            proposal_type_info: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;ProposalType&gt;(),<br />        &#125;,<br />    );<br /><br />    <b>move_to</b>(<a href="account.md#0x1_account">account</a>, voting_forum);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_create_proposal"></a>

## Function `create_proposal`

Create a single&#45;step proposal with the given parameters

@param voting_forum_address The forum&apos;s address where the proposal will be stored.
@param execution_content The execution content that will be given back at resolution time. This can contain
data such as a capability resource used to scope the execution.
@param execution_hash The hash for the execution script module. Only the same exact script module can resolve
this proposal.
@param min_vote_threshold The minimum number of votes needed to consider this proposal successful.
@param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.
@param early_resolution_vote_threshold The vote threshold for early resolution of this proposal.
@param metadata A simple_map that stores information about this proposal.
@return The proposal id.


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_create_proposal">create_proposal</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_create_proposal">create_proposal</a>&lt;ProposalType: store&gt;(<br />    proposer: <b>address</b>,<br />    voting_forum_address: <b>address</b>,<br />    execution_content: ProposalType,<br />    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    min_vote_threshold: u128,<br />    expiration_secs: u64,<br />    early_resolution_vote_threshold: Option&lt;u128&gt;,<br />    metadata: SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />): u64 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <a href="voting.md#0x1_voting_create_proposal_v2">create_proposal_v2</a>(<br />        proposer,<br />        voting_forum_address,<br />        execution_content,<br />        execution_hash,<br />        min_vote_threshold,<br />        expiration_secs,<br />        early_resolution_vote_threshold,<br />        metadata,<br />        <b>false</b><br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_create_proposal_v2"></a>

## Function `create_proposal_v2`

Create a single&#45;step or a multi&#45;step proposal with the given parameters

@param voting_forum_address The forum&apos;s address where the proposal will be stored.
@param execution_content The execution content that will be given back at resolution time. This can contain
data such as a capability resource used to scope the execution.
@param execution_hash The sha&#45;256 hash for the execution script module. Only the same exact script module can
resolve this proposal.
@param min_vote_threshold The minimum number of votes needed to consider this proposal successful.
@param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.
@param early_resolution_vote_threshold The vote threshold for early resolution of this proposal.
@param metadata A simple_map that stores information about this proposal.
@param is_multi_step_proposal A bool value that indicates if the proposal is single&#45;step or multi&#45;step.
@return The proposal id.


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_create_proposal_v2">create_proposal_v2</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_create_proposal_v2">create_proposal_v2</a>&lt;ProposalType: store&gt;(<br />    proposer: <b>address</b>,<br />    voting_forum_address: <b>address</b>,<br />    execution_content: ProposalType,<br />    execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    min_vote_threshold: u128,<br />    expiration_secs: u64,<br />    early_resolution_vote_threshold: Option&lt;u128&gt;,<br />    metadata: SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    is_multi_step_proposal: bool,<br />): u64 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;early_resolution_vote_threshold)) &#123;<br />        <b>assert</b>!(<br />            min_vote_threshold &lt;&#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;early_resolution_vote_threshold),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="voting.md#0x1_voting_EINVALID_MIN_VOTE_THRESHOLD">EINVALID_MIN_VOTE_THRESHOLD</a>),<br />        );<br />    &#125;;<br />    // Make sure the execution <b>script</b>&apos;s <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> is not empty.<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;execution_hash) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="voting.md#0x1_voting_EPROPOSAL_EMPTY_EXECUTION_HASH">EPROPOSAL_EMPTY_EXECUTION_HASH</a>));<br /><br />    <b>let</b> voting_forum &#61; <b>borrow_global_mut</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    <b>let</b> proposal_id &#61; voting_forum.next_proposal_id;<br />    voting_forum.next_proposal_id &#61; voting_forum.next_proposal_id &#43; 1;<br /><br />    // Add a flag <b>to</b> indicate <b>if</b> this proposal is single&#45;step or multi&#45;step.<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> metadata, utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>), to_bytes(&amp;is_multi_step_proposal));<br /><br />    <b>let</b> is_multi_step_in_execution_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br />    <b>if</b> (is_multi_step_proposal) &#123;<br />        // If the given proposal is a multi&#45;step proposal, we will add a flag <b>to</b> indicate <b>if</b> this multi&#45;step proposal is in execution.<br />        // This value is by default <b>false</b>. We turn this value <b>to</b> <b>true</b> when we start executing the multi&#45;step proposal. This value<br />        // will be used <b>to</b> disable further <a href="voting.md#0x1_voting">voting</a> after we started executing the multi&#45;step proposal.<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> metadata, is_multi_step_in_execution_key, to_bytes(&amp;<b>false</b>));<br />        // If the proposal is a single&#45;step proposal, we check <b>if</b> the metadata passed by the client <b>has</b> the <a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a> key.<br />        // If they have the key, we will remove it, because a single&#45;step proposal that doesn&apos;t need this key.<br />    &#125; <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;<b>mut</b> metadata, &amp;is_multi_step_in_execution_key)) &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&amp;<b>mut</b> metadata, &amp;is_multi_step_in_execution_key);<br />    &#125;;<br /><br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> voting_forum.proposals, proposal_id, <a href="voting.md#0x1_voting_Proposal">Proposal</a> &#123;<br />        proposer,<br />        creation_time_secs: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),<br />        execution_content: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>&lt;ProposalType&gt;(execution_content),<br />        execution_hash,<br />        metadata,<br />        min_vote_threshold,<br />        expiration_secs,<br />        early_resolution_vote_threshold,<br />        yes_votes: 0,<br />        no_votes: 0,<br />        is_resolved: <b>false</b>,<br />        resolution_time_secs: 0,<br />    &#125;);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="voting.md#0x1_voting_CreateProposal">CreateProposal</a> &#123;<br />                proposal_id,<br />                early_resolution_vote_threshold,<br />                execution_hash,<br />                expiration_secs,<br />                metadata,<br />                min_vote_threshold,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="voting.md#0x1_voting_CreateProposalEvent">CreateProposalEvent</a>&gt;(<br />        &amp;<b>mut</b> voting_forum.events.create_proposal_events,<br />        <a href="voting.md#0x1_voting_CreateProposalEvent">CreateProposalEvent</a> &#123;<br />            proposal_id,<br />            early_resolution_vote_threshold,<br />            execution_hash,<br />            expiration_secs,<br />            metadata,<br />            min_vote_threshold,<br />        &#125;,<br />    );<br /><br />    proposal_id<br />&#125;<br /></code></pre>



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


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_vote">vote</a>&lt;ProposalType: store&gt;(_proof: &amp;ProposalType, voting_forum_address: <b>address</b>, proposal_id: u64, num_votes: u64, should_pass: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_vote">vote</a>&lt;ProposalType: store&gt;(<br />    _proof: &amp;ProposalType,<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />    num_votes: u64,<br />    should_pass: bool,<br />) <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> voting_forum &#61; <b>borrow_global_mut</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> voting_forum.proposals, proposal_id);<br />    // Voting might still be possible after the proposal <b>has</b> enough yes votes <b>to</b> be resolved early. This would only<br />    // lead <b>to</b> possible proposal resolution failure <b>if</b> the resolve early threshold is not definitive (e.g. &lt; 50% &#43; 1<br />    // of the total <a href="voting.md#0x1_voting">voting</a> token&apos;s supply). In this case, more <a href="voting.md#0x1_voting">voting</a> might actually still be desirable.<br />    // Governance mechanisms built on this <a href="voting.md#0x1_voting">voting</a> <b>module</b> can <b>apply</b> additional rules on when <a href="voting.md#0x1_voting">voting</a> is closed <b>as</b><br />    // appropriate.<br />    <b>assert</b>!(!<a href="voting.md#0x1_voting_is_voting_period_over">is_voting_period_over</a>(proposal), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="voting.md#0x1_voting_EPROPOSAL_VOTING_ALREADY_ENDED">EPROPOSAL_VOTING_ALREADY_ENDED</a>));<br />    <b>assert</b>!(!proposal.is_resolved, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="voting.md#0x1_voting_EPROPOSAL_ALREADY_RESOLVED">EPROPOSAL_ALREADY_RESOLVED</a>));<br />    // Assert this proposal is single&#45;step, or <b>if</b> the proposal is multi&#45;step, it is not in execution yet.<br />    <b>assert</b>!(!<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;proposal.metadata, &amp;utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>))<br />        &#124;&#124; &#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;proposal.metadata, &amp;utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>)) &#61;&#61; to_bytes(<br />        &amp;<b>false</b><br />    ),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="voting.md#0x1_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION">EMULTI_STEP_PROPOSAL_IN_EXECUTION</a>));<br /><br />    <b>if</b> (should_pass) &#123;<br />        proposal.yes_votes &#61; proposal.yes_votes &#43; (num_votes <b>as</b> u128);<br />    &#125; <b>else</b> &#123;<br />        proposal.no_votes &#61; proposal.no_votes &#43; (num_votes <b>as</b> u128);<br />    &#125;;<br /><br />    // Record the resolvable time <b>to</b> ensure that resolution <b>has</b> <b>to</b> be done non&#45;atomically.<br />    <b>let</b> timestamp_secs_bytes &#61; to_bytes(&amp;<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>());<br />    <b>let</b> key &#61; utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;proposal.metadata, &amp;key)) &#123;<br />        &#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&amp;<b>mut</b> proposal.metadata, &amp;key) &#61; timestamp_secs_bytes;<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&amp;<b>mut</b> proposal.metadata, key, timestamp_secs_bytes);<br />    &#125;;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="voting.md#0x1_voting_Vote">Vote</a> &#123; proposal_id, num_votes &#125;);<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="voting.md#0x1_voting_VoteEvent">VoteEvent</a>&gt;(<br />        &amp;<b>mut</b> voting_forum.events.vote_events,<br />        <a href="voting.md#0x1_voting_VoteEvent">VoteEvent</a> &#123; proposal_id, num_votes &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_is_proposal_resolvable"></a>

## Function `is_proposal_resolvable`

Common checks on if a proposal is resolvable, regardless if the proposal is single&#45;step or multi&#45;step.


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />) <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal_state &#61; <a href="voting.md#0x1_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    <b>assert</b>!(proposal_state &#61;&#61; <a href="voting.md#0x1_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="voting.md#0x1_voting_EPROPOSAL_CANNOT_BE_RESOLVED">EPROPOSAL_CANNOT_BE_RESOLVED</a>));<br /><br />    <b>let</b> voting_forum &#61; <b>borrow_global_mut</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> voting_forum.proposals, proposal_id);<br />    <b>assert</b>!(!proposal.is_resolved, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="voting.md#0x1_voting_EPROPOSAL_ALREADY_RESOLVED">EPROPOSAL_ALREADY_RESOLVED</a>));<br /><br />    // We need <b>to</b> make sure that the resolution is happening in<br />    // a separate transaction from the last vote <b>to</b> guard against <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> potential flashloan attacks.<br />    <b>let</b> resolvable_time &#61; to_u64(&#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;proposal.metadata, &amp;utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>)));<br />    <b>assert</b>!(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; resolvable_time, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="voting.md#0x1_voting_ERESOLUTION_CANNOT_BE_ATOMIC">ERESOLUTION_CANNOT_BE_ATOMIC</a>));<br /><br />    <b>assert</b>!(<br />        <a href="transaction_context.md#0x1_transaction_context_get_script_hash">transaction_context::get_script_hash</a>() &#61;&#61; proposal.execution_hash,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="voting.md#0x1_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING">EPROPOSAL_EXECUTION_HASH_NOT_MATCHING</a>),<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_resolve"></a>

## Function `resolve`

Resolve a single&#45;step proposal with given id. Can only be done if there are at least as many votes as min required and
there are more yes votes than no. If either of these conditions is not met, this will revert.

@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_resolve">resolve</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): ProposalType<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_resolve">resolve</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): ProposalType <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <a href="voting.md#0x1_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br /><br />    <b>let</b> voting_forum &#61; <b>borrow_global_mut</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> voting_forum.proposals, proposal_id);<br /><br />    // Assert that the specified proposal is not a multi&#45;step proposal.<br />    <b>let</b> multi_step_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br />    <b>let</b> has_multi_step_key &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;proposal.metadata, &amp;multi_step_key);<br />    <b>if</b> (has_multi_step_key) &#123;<br />        <b>let</b> is_multi_step_proposal &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(&#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;proposal.metadata, &amp;multi_step_key));<br />        <b>assert</b>!(<br />            !is_multi_step_proposal,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="voting.md#0x1_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION">EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION</a>)<br />        );<br />    &#125;;<br /><br />    <b>let</b> resolved_early &#61; <a href="voting.md#0x1_voting_can_be_resolved_early">can_be_resolved_early</a>(proposal);<br />    proposal.is_resolved &#61; <b>true</b>;<br />    proposal.resolution_time_secs &#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a> &#123;<br />                proposal_id,<br />                yes_votes: proposal.yes_votes,<br />                no_votes: proposal.no_votes,<br />                resolved_early,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a>&gt;(<br />        &amp;<b>mut</b> voting_forum.events.resolve_proposal_events,<br />        <a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a> &#123;<br />            proposal_id,<br />            yes_votes: proposal.yes_votes,<br />            no_votes: proposal.no_votes,<br />            resolved_early,<br />        &#125;,<br />    );<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> proposal.execution_content)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_resolve_proposal_v2"></a>

## Function `resolve_proposal_v2`

Resolve a single&#45;step or a multi&#45;step proposal with the given id.
Can only be done if there are at least as many votes as min required and
there are more yes votes than no. If either of these conditions is not met, this will revert.


@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@param next_execution_hash The next execution hash if the given proposal is multi&#45;step.


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_resolve_proposal_v2">resolve_proposal_v2</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_resolve_proposal_v2">resolve_proposal_v2</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />    next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <a href="voting.md#0x1_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br /><br />    <b>let</b> voting_forum &#61; <b>borrow_global_mut</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> voting_forum.proposals, proposal_id);<br /><br />    // Update the <a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a> key <b>to</b> indicate that the multi&#45;step proposal is in execution.<br />    <b>let</b> multi_step_in_execution_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;proposal.metadata, &amp;multi_step_in_execution_key)) &#123;<br />        <b>let</b> is_multi_step_proposal_in_execution_value &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(<br />            &amp;<b>mut</b> proposal.metadata,<br />            &amp;multi_step_in_execution_key<br />        );<br />        &#42;is_multi_step_proposal_in_execution_value &#61; to_bytes(&amp;<b>true</b>);<br />    &#125;;<br /><br />    <b>let</b> multi_step_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br />    <b>let</b> is_multi_step &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;proposal.metadata, &amp;multi_step_key) &amp;&amp; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(<br />        &#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;proposal.metadata, &amp;multi_step_key)<br />    );<br />    <b>let</b> next_execution_hash_is_empty &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;next_execution_hash) &#61;&#61; 0;<br /><br />    // Assert that <b>if</b> this proposal is single&#45;step, the `next_execution_hash` parameter is empty.<br />    <b>assert</b>!(<br />        is_multi_step &#124;&#124; next_execution_hash_is_empty,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="voting.md#0x1_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH">ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH</a>)<br />    );<br /><br />    // If the `next_execution_hash` parameter is empty, it means that either<br />    // &#45; this proposal is a single&#45;step proposal, or<br />    // &#45; this proposal is multi&#45;step and we&apos;re currently resolving the last step in the multi&#45;step proposal.<br />    // We can mark that this proposal is resolved.<br />    <b>if</b> (next_execution_hash_is_empty) &#123;<br />        proposal.is_resolved &#61; <b>true</b>;<br />        proposal.resolution_time_secs &#61; <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();<br /><br />        // Set the `<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>` value <b>to</b> <b>false</b> upon successful resolution of the last step of a multi&#45;step proposal.<br />        <b>if</b> (is_multi_step) &#123;<br />            <b>let</b> is_multi_step_proposal_in_execution_value &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(<br />                &amp;<b>mut</b> proposal.metadata,<br />                &amp;multi_step_in_execution_key<br />            );<br />            &#42;is_multi_step_proposal_in_execution_value &#61; to_bytes(&amp;<b>false</b>);<br />        &#125;;<br />    &#125; <b>else</b> &#123;<br />        // If the current step is not the last step,<br />        // <b>update</b> the proposal&apos;s execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> on&#45;chain <b>to</b> the execution <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> of the next step.<br />        proposal.execution_hash &#61; next_execution_hash;<br />    &#125;;<br /><br />    // For single&#45;step proposals, we emit one `<a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a>` <a href="event.md#0x1_event">event</a> per proposal.<br />    // For multi&#45;step proposals, we emit one `<a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a>` <a href="event.md#0x1_event">event</a> per step in the multi&#45;step proposal. This means<br />    // that we emit multiple `<a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a>` events for the same multi&#45;step proposal.<br />    <b>let</b> resolved_early &#61; <a href="voting.md#0x1_voting_can_be_resolved_early">can_be_resolved_early</a>(proposal);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a> &#123;<br />                proposal_id,<br />                yes_votes: proposal.yes_votes,<br />                no_votes: proposal.no_votes,<br />                resolved_early,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(<br />        &amp;<b>mut</b> voting_forum.events.resolve_proposal_events,<br />        <a href="voting.md#0x1_voting_ResolveProposal">ResolveProposal</a> &#123;<br />            proposal_id,<br />            yes_votes: proposal.yes_votes,<br />            no_votes: proposal.no_votes,<br />            resolved_early,<br />        &#125;,<br />    );<br /><br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_next_proposal_id"></a>

## Function `next_proposal_id`

Return the next unassigned proposal id


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_next_proposal_id">next_proposal_id</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_next_proposal_id">next_proposal_id</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, ): u64 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> voting_forum &#61; <b>borrow_global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    voting_forum.next_proposal_id<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_proposer"></a>

## Function `get_proposer`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposer">get_proposer</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposer">get_proposer</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64<br />): <b>address</b> <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.proposer<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_is_voting_closed"></a>

## Function `is_voting_closed`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64<br />): bool <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    <a href="voting.md#0x1_voting_can_be_resolved_early">can_be_resolved_early</a>(proposal) &#124;&#124; <a href="voting.md#0x1_voting_is_voting_period_over">is_voting_period_over</a>(proposal)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_can_be_resolved_early"></a>

## Function `can_be_resolved_early`

Return true if the proposal has reached early resolution threshold (if specified).


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_can_be_resolved_early">can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: &amp;<a href="voting.md#0x1_voting_Proposal">voting::Proposal</a>&lt;ProposalType&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_can_be_resolved_early">can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: &amp;<a href="voting.md#0x1_voting_Proposal">Proposal</a>&lt;ProposalType&gt;): bool &#123;<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;proposal.early_resolution_vote_threshold)) &#123;<br />        <b>let</b> early_resolution_threshold &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;proposal.early_resolution_vote_threshold);<br />        <b>if</b> (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124; proposal.no_votes &gt;&#61; early_resolution_threshold) &#123;<br />            <b>return</b> <b>true</b><br />        &#125;;<br />    &#125;;<br />    <b>false</b><br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_proposal_metadata"></a>

## Function `get_proposal_metadata`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_metadata">get_proposal_metadata</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_metadata">get_proposal_metadata</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.metadata<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_proposal_metadata_value"></a>

## Function `get_proposal_metadata_value`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_metadata_value">get_proposal_metadata_value</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, metadata_key: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_metadata_value">get_proposal_metadata_value</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />    metadata_key: String,<br />): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    &#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;proposal.metadata, &amp;metadata_key)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_proposal_state"></a>

## Function `get_proposal_state`

Return the state of the proposal with given id.

@param voting_forum_address The address of the forum where the proposals are stored.
@param proposal_id The proposal id.
@return Proposal state as an enum value.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): u64 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>if</b> (<a href="voting.md#0x1_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id)) &#123;<br />        <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />        <b>let</b> yes_votes &#61; proposal.yes_votes;<br />        <b>let</b> no_votes &#61; proposal.no_votes;<br /><br />        <b>if</b> (yes_votes &gt; no_votes &amp;&amp; yes_votes &#43; no_votes &gt;&#61; proposal.min_vote_threshold) &#123;<br />            <a href="voting.md#0x1_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a><br />        &#125; <b>else</b> &#123;<br />            <a href="voting.md#0x1_voting_PROPOSAL_STATE_FAILED">PROPOSAL_STATE_FAILED</a><br />        &#125;<br />    &#125; <b>else</b> &#123;<br />        <a href="voting.md#0x1_voting_PROPOSAL_STATE_PENDING">PROPOSAL_STATE_PENDING</a><br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_proposal_creation_secs"></a>

## Function `get_proposal_creation_secs`

Return the proposal&apos;s creation time.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_creation_secs">get_proposal_creation_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_creation_secs">get_proposal_creation_secs</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): u64 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.creation_time_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_proposal_expiration_secs"></a>

## Function `get_proposal_expiration_secs`

Return the proposal&apos;s expiration time.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_expiration_secs">get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_expiration_secs">get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): u64 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.expiration_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_execution_hash"></a>

## Function `get_execution_hash`

Return the proposal&apos;s execution hash.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_execution_hash">get_execution_hash</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_execution_hash">get_execution_hash</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.execution_hash<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_min_vote_threshold"></a>

## Function `get_min_vote_threshold`

Return the proposal&apos;s minimum vote threshold


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_min_vote_threshold">get_min_vote_threshold</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_min_vote_threshold">get_min_vote_threshold</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): u128 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.min_vote_threshold<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_early_resolution_vote_threshold"></a>

## Function `get_early_resolution_vote_threshold`

Return the proposal&apos;s early resolution minimum vote threshold (optionally set)


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_early_resolution_vote_threshold">get_early_resolution_vote_threshold</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_early_resolution_vote_threshold">get_early_resolution_vote_threshold</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): Option&lt;u128&gt; <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.early_resolution_vote_threshold<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_votes"></a>

## Function `get_votes`

Return the proposal&apos;s current vote count (yes_votes, no_votes)


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_votes">get_votes</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): (u128, u128)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_votes">get_votes</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): (u128, u128) <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    (proposal.yes_votes, proposal.no_votes)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_is_resolved"></a>

## Function `is_resolved`

Return true if the governance proposal has already been resolved.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_resolved">is_resolved</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_resolved">is_resolved</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): bool <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.is_resolved<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_resolution_time_secs"></a>

## Function `get_resolution_time_secs`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_resolution_time_secs">get_resolution_time_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_resolution_time_secs">get_resolution_time_secs</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): u64 <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> proposal &#61; <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />    proposal.resolution_time_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_is_multi_step_proposal_in_execution"></a>

## Function `is_multi_step_proposal_in_execution`

Return true if the multi&#45;step governance proposal is in execution.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): bool <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> voting_forum &#61; <b>borrow_global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;voting_forum.proposals, proposal_id);<br />    <b>let</b> is_multi_step_in_execution_key &#61; utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&amp;proposal.metadata, &amp;is_multi_step_in_execution_key),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="voting.md#0x1_voting_EPROPOSAL_IS_SINGLE_STEP">EPROPOSAL_IS_SINGLE_STEP</a>)<br />    );<br />    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_bool">from_bcs::to_bool</a>(&#42;<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&amp;proposal.metadata, &amp;is_multi_step_in_execution_key))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_is_voting_period_over"></a>

## Function `is_voting_period_over`

Return true if the voting period of the given proposal has already ended.


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_is_voting_period_over">is_voting_period_over</a>&lt;ProposalType: store&gt;(proposal: &amp;<a href="voting.md#0x1_voting_Proposal">voting::Proposal</a>&lt;ProposalType&gt;): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_is_voting_period_over">is_voting_period_over</a>&lt;ProposalType: store&gt;(proposal: &amp;<a href="voting.md#0x1_voting_Proposal">Proposal</a>&lt;ProposalType&gt;): bool &#123;<br />    <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &gt; proposal.expiration_secs<br />&#125;<br /></code></pre>



</details>

<a id="0x1_voting_get_proposal"></a>

## Function `get_proposal`



<pre><code><b>fun</b> <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): &amp;<a href="voting.md#0x1_voting_Proposal">voting::Proposal</a>&lt;ProposalType&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="voting.md#0x1_voting_get_proposal">get_proposal</a>&lt;ProposalType: store&gt;(<br />    voting_forum_address: <b>address</b>,<br />    proposal_id: u64,<br />): &amp;<a href="voting.md#0x1_voting_Proposal">Proposal</a>&lt;ProposalType&gt; <b>acquires</b> <a href="voting.md#0x1_voting_VotingForum">VotingForum</a> &#123;<br />    <b>let</b> voting_forum &#61; <b>borrow_global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;voting_forum.proposals, proposal_id)<br />&#125;<br /></code></pre>



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
<td>The create_proposal and create_proposal_v2 create a new proposal with a unique ID derived from the voting_forum&apos;s next_proposal_id incrementally.</td>
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
<td>After resolving a single&#45;step proposal, the corresponding proposal is guaranteed to be marked as successfully resolved.</td>
<td>High</td>
<td>Upon invoking the resolve function on a proposal, it undergoes a series of checks to ensure its validity. These include verifying if the proposal exists, is a single&#45;step proposal, and meets the criteria for resolution. If the checks pass, the proposal&apos;s is_resolved flag becomes true, indicating a successful resolution.</td>
<td>Formally verified via <a href="#high-level-req-3">resolve</a>.</td>
</tr>

<tr>
<td>4</td>
<td>In the context of v2 proposal resolving, both single&#45;step and multi&#45;step proposals are accurately handled. It ensures that for single&#45;step proposals, the next execution hash is empty and resolves the proposal, while for multi&#45;step proposals, it guarantees that the next execution hash corresponds to the hash of the next step, maintaining the integrity of the proposal execution sequence.</td>
<td>Medium</td>
<td>The function resolve_proposal_v2 correctly handles both single&#45;step and multi&#45;step proposals. For single&#45;step proposals, it ensures that the next_execution_hash parameter is empty and resolves the proposal. For multi&#45;step proposals, it ensures that the next_execution_hash parameter contains the hash of the next step.</td>
<td>Formally verified via <a href="#high-level-req-4">resolve_proposal_v2</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_register">register</a>&lt;ProposalType: store&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> register_account &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> register_account.guid_creation_num &#43; 4 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> register_account.guid_creation_num &#43; 4 &gt; MAX_U64;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;ProposalType&gt;();<br /><b>ensures</b> <b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(addr);<br /></code></pre>



<a id="@Specification_1_create_proposal"></a>

### Function `create_proposal`


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_create_proposal">create_proposal</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): u64<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="voting.md#0x1_voting_CreateProposalAbortsIfAndEnsures">CreateProposalAbortsIfAndEnsures</a>&lt;ProposalType&gt;&#123;is_multi_step_proposal: <b>false</b>&#125;;<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> result &#61;&#61; <b>old</b>(<b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;<br /></code></pre>



<a id="@Specification_1_create_proposal_v2"></a>

### Function `create_proposal_v2`


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_create_proposal_v2">create_proposal_v2</a>&lt;ProposalType: store&gt;(proposer: <b>address</b>, voting_forum_address: <b>address</b>, execution_content: ProposalType, execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;, metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="voting.md#0x1_voting_CreateProposalAbortsIfAndEnsures">CreateProposalAbortsIfAndEnsures</a>&lt;ProposalType&gt;;<br /><b>ensures</b> result &#61;&#61; <b>old</b>(<b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;<br /></code></pre>




<a id="0x1_voting_CreateProposalAbortsIfAndEnsures"></a>


<pre><code><b>schema</b> <a href="voting.md#0x1_voting_CreateProposalAbortsIfAndEnsures">CreateProposalAbortsIfAndEnsures</a>&lt;ProposalType&gt; &#123;<br />voting_forum_address: <b>address</b>;<br />execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />min_vote_threshold: u128;<br />early_resolution_vote_threshold: Option&lt;u128&gt;;<br />metadata: SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;;<br />is_multi_step_proposal: bool;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal_id &#61; voting_forum.next_proposal_id;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals,proposal_id);<br /><b>aborts_if</b> len(early_resolution_vote_threshold.vec) !&#61; 0 &amp;&amp; min_vote_threshold &gt; early_resolution_vote_threshold.vec[0];<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>aborts_if</b> len(execution_hash) &#61;&#61; 0;<br /><b>let</b> execution_key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(metadata, execution_key);<br /><b>aborts_if</b> voting_forum.next_proposal_id &#43; 1 &gt; MAX_U64;<br /><b>let</b> is_multi_step_in_execution_key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>aborts_if</b> is_multi_step_proposal &amp;&amp; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(metadata, is_multi_step_in_execution_key);<br /><b>let</b> <b>post</b> post_voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> <b>post</b> post_metadata &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id).metadata;<br /><b>ensures</b> post_voting_forum.next_proposal_id &#61;&#61; voting_forum.next_proposal_id &#43; 1;<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(post_voting_forum.proposals, proposal_id);<br /><b>ensures</b> <b>if</b> (is_multi_step_proposal) &#123;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_metadata, is_multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(<b>false</b>)<br />&#125; <b>else</b> &#123;<br />    !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(post_metadata, is_multi_step_in_execution_key)<br />&#125;;<br />&#125;<br /></code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_vote">vote</a>&lt;ProposalType: store&gt;(_proof: &amp;ProposalType, voting_forum_address: <b>address</b>, proposal_id: u64, num_votes: u64, should_pass: bool)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> <a href="voting.md#0x1_voting_is_voting_period_over">is_voting_period_over</a>(proposal);<br /><b>aborts_if</b> proposal.is_resolved;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>let</b> execution_key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, execution_key) &amp;&amp;<br />          <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, execution_key) !&#61; std::bcs::serialize(<b>false</b>);<br /><b>aborts_if</b> <b>if</b> (should_pass) &#123; proposal.yes_votes &#43; num_votes &gt; MAX_U128 &#125; <b>else</b> &#123; proposal.no_votes &#43; num_votes &gt; MAX_U128 &#125;;<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);<br /><b>let</b> <b>post</b> post_voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> <b>post</b> post_proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);<br /><b>ensures</b> <b>if</b> (should_pass) &#123;<br />    post_proposal.yes_votes &#61;&#61; proposal.yes_votes &#43; num_votes<br />&#125; <b>else</b> &#123;<br />    post_proposal.no_votes &#61;&#61; proposal.no_votes &#43; num_votes<br />&#125;;<br /><b>let</b> timestamp_secs_bytes &#61; std::bcs::serialize(<a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>());<br /><b>let</b> key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, key) &#61;&#61; timestamp_secs_bytes;<br /></code></pre>



<a id="@Specification_1_is_proposal_resolvable"></a>

### Function `is_proposal_resolvable`


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_is_proposal_resolvable">is_proposal_resolvable</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64)<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="voting.md#0x1_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt;;<br /></code></pre>




<a id="0x1_voting_IsProposalResolvableAbortsIf"></a>


<pre><code><b>schema</b> <a href="voting.md#0x1_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt; &#123;<br />voting_forum_address: <b>address</b>;<br />proposal_id: u64;<br /><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> voting_closed &#61; <a href="voting.md#0x1_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br /><b>aborts_if</b> voting_closed &amp;&amp; (proposal.yes_votes &lt;&#61; proposal.no_votes &#124;&#124; proposal.yes_votes &#43; proposal.no_votes &lt; proposal.min_vote_threshold);<br /><b>aborts_if</b> !voting_closed;<br /><b>aborts_if</b> proposal.is_resolved;<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, std::string::spec_utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>));<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;u64&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, std::string::spec_utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>)));<br /><b>aborts_if</b> <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &lt;&#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;u64&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, std::string::spec_utf8(<a href="voting.md#0x1_voting_RESOLVABLE_TIME_METADATA_KEY">RESOLVABLE_TIME_METADATA_KEY</a>)));<br /><b>aborts_if</b> <a href="transaction_context.md#0x1_transaction_context_spec_get_script_hash">transaction_context::spec_get_script_hash</a>() !&#61; proposal.execution_hash;<br />&#125;<br /></code></pre>



<a id="@Specification_1_resolve"></a>

### Function `resolve`


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_resolve">resolve</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): ProposalType<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="voting.md#0x1_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt;;<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> multi_step_key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>let</b> has_multi_step_key &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key);<br /><b>aborts_if</b> has_multi_step_key &amp;&amp; !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>aborts_if</b> has_multi_step_key &amp;&amp; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>let</b> <b>post</b> post_voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> <b>post</b> post_proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>ensures</b> post_proposal.is_resolved &#61;&#61; <b>true</b>;<br /><b>ensures</b> post_proposal.resolution_time_secs &#61;&#61; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(proposal.execution_content);<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(proposal.execution_content);<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(post_proposal.execution_content);<br /></code></pre>



<a id="@Specification_1_resolve_proposal_v2"></a>

### Function `resolve_proposal_v2`


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_resolve_proposal_v2">resolve_proposal_v2</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, next_execution_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 300;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="voting.md#0x1_voting_IsProposalResolvableAbortsIf">IsProposalResolvableAbortsIf</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>let</b> <b>post</b> post_voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> <b>post</b> post_proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_voting_forum.proposals, proposal_id);<br /><b>let</b> multi_step_in_execution_key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>ensures</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key) &amp;&amp; len(next_execution_hash) !&#61; 0) &#61;&#61;&gt;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(<b>true</b>);<br /><b>ensures</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key) &amp;&amp;<br />    (len(next_execution_hash) &#61;&#61; 0 &amp;&amp; !is_multi_step)) &#61;&#61;&gt;<br />    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(<b>true</b>);<br /><b>let</b> multi_step_key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY">IS_MULTI_STEP_PROPOSAL_KEY</a>);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key) &amp;&amp;<br />    !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;bool&gt;(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>let</b> is_multi_step &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_key) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>(<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, multi_step_key));<br /><b>aborts_if</b> !is_multi_step &amp;&amp; len(next_execution_hash) !&#61; 0;<br /><b>aborts_if</b> len(next_execution_hash) &#61;&#61; 0 &amp;&amp; !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> len(next_execution_hash) &#61;&#61; 0 &amp;&amp; is_multi_step &amp;&amp; !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, multi_step_in_execution_key);<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>ensures</b> len(next_execution_hash) &#61;&#61; 0 &#61;&#61;&gt; post_proposal.resolution_time_secs &#61;&#61; <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>();<br /><b>ensures</b> len(next_execution_hash) &#61;&#61; 0 &#61;&#61;&gt; post_proposal.is_resolved &#61;&#61; <b>true</b>;<br /><b>ensures</b> (len(next_execution_hash) &#61;&#61; 0 &amp;&amp; is_multi_step) &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(<b>false</b>);<br /><b>ensures</b> len(next_execution_hash) !&#61; 0 &#61;&#61;&gt; post_proposal.execution_hash &#61;&#61; next_execution_hash;<br /></code></pre>



<a id="@Specification_1_next_proposal_id"></a>

### Function `next_proposal_id`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_next_proposal_id">next_proposal_id</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>ensures</b> result &#61;&#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address).next_proposal_id;<br /></code></pre>



<a id="@Specification_1_get_proposer"></a>

### Function `get_proposer`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposer">get_proposer</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <b>address</b><br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.proposer;<br /></code></pre>



<a id="@Specification_1_is_voting_closed"></a>

### Function `is_voting_closed`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_voting_closed">is_voting_closed</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>ensures</b> result &#61;&#61; <a href="voting.md#0x1_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br /></code></pre>




<a id="0x1_voting_spec_is_voting_closed"></a>


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool &#123;<br />   <b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />   <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br />   <a href="voting.md#0x1_voting_spec_can_be_resolved_early">spec_can_be_resolved_early</a>&lt;ProposalType&gt;(proposal) &#124;&#124; <a href="voting.md#0x1_voting_is_voting_period_over">is_voting_period_over</a>(proposal)<br />&#125;<br /></code></pre>



<a id="@Specification_1_can_be_resolved_early"></a>

### Function `can_be_resolved_early`


<pre><code><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_can_be_resolved_early">can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: &amp;<a href="voting.md#0x1_voting_Proposal">voting::Proposal</a>&lt;ProposalType&gt;): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="voting.md#0x1_voting_spec_can_be_resolved_early">spec_can_be_resolved_early</a>&lt;ProposalType&gt;(proposal);<br /></code></pre>




<a id="0x1_voting_spec_can_be_resolved_early"></a>


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_spec_can_be_resolved_early">spec_can_be_resolved_early</a>&lt;ProposalType: store&gt;(proposal: <a href="voting.md#0x1_voting_Proposal">Proposal</a>&lt;ProposalType&gt;): bool &#123;<br />   <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(proposal.early_resolution_vote_threshold)) &#123;<br />       <b>let</b> early_resolution_threshold &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(proposal.early_resolution_vote_threshold);<br />       <b>if</b> (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124; proposal.no_votes &gt;&#61; early_resolution_threshold) &#123;<br />           <b>true</b><br />       &#125; <b>else</b>&#123;<br />           <b>false</b><br />       &#125;<br />   &#125; <b>else</b> &#123;<br />       <b>false</b><br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_voting_spec_get_proposal_state"></a>


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_spec_get_proposal_state">spec_get_proposal_state</a>&lt;ProposalType&gt;(<br />   voting_forum_address: <b>address</b>,<br />   proposal_id: u64,<br />   voting_forum: <a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;<br />): u64 &#123;<br />   <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br />   <b>let</b> voting_closed &#61; <a href="voting.md#0x1_voting_spec_is_voting_closed">spec_is_voting_closed</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br />   <b>let</b> proposal_vote_cond &#61; (proposal.yes_votes &gt; proposal.no_votes &amp;&amp; proposal.yes_votes &#43; proposal.no_votes &gt;&#61; proposal.min_vote_threshold);<br />   <b>if</b> (voting_closed &amp;&amp; proposal_vote_cond) &#123;<br />       <a href="voting.md#0x1_voting_PROPOSAL_STATE_SUCCEEDED">PROPOSAL_STATE_SUCCEEDED</a><br />   &#125; <b>else</b> <b>if</b> (voting_closed &amp;&amp; !proposal_vote_cond) &#123;<br />       <a href="voting.md#0x1_voting_PROPOSAL_STATE_FAILED">PROPOSAL_STATE_FAILED</a><br />   &#125; <b>else</b> &#123;<br />       <a href="voting.md#0x1_voting_PROPOSAL_STATE_PENDING">PROPOSAL_STATE_PENDING</a><br />   &#125;<br />&#125;<br /></code></pre>




<a id="0x1_voting_spec_get_proposal_expiration_secs"></a>


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_spec_get_proposal_expiration_secs">spec_get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(<br />   voting_forum_address: <b>address</b>,<br />   proposal_id: u64,<br />): u64 &#123;<br />   <b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />   <b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br />   proposal.expiration_secs<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_proposal_metadata"></a>

### Function `get_proposal_metadata`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_metadata">get_proposal_metadata</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.metadata;<br /></code></pre>



<a id="@Specification_1_get_proposal_metadata_value"></a>

### Function `get_proposal_metadata_value`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_metadata_value">get_proposal_metadata_value</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64, metadata_key: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata, metadata_key);<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata, metadata_key);<br /></code></pre>



<a id="@Specification_1_get_proposal_state"></a>

### Function `get_proposal_state`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_state">get_proposal_state</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>




<pre><code><b>pragma</b> addition_overflow_unchecked;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>ensures</b> result &#61;&#61; <a href="voting.md#0x1_voting_spec_get_proposal_state">spec_get_proposal_state</a>(voting_forum_address, proposal_id, voting_forum);<br /></code></pre>



<a id="@Specification_1_get_proposal_creation_secs"></a>

### Function `get_proposal_creation_secs`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_creation_secs">get_proposal_creation_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.creation_time_secs;<br /></code></pre>



<a id="@Specification_1_get_proposal_expiration_secs"></a>

### Function `get_proposal_expiration_secs`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_proposal_expiration_secs">get_proposal_expiration_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>ensures</b> result &#61;&#61; <a href="voting.md#0x1_voting_spec_get_proposal_expiration_secs">spec_get_proposal_expiration_secs</a>&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br /></code></pre>



<a id="@Specification_1_get_execution_hash"></a>

### Function `get_execution_hash`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_execution_hash">get_execution_hash</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.execution_hash;<br /></code></pre>



<a id="@Specification_1_get_min_vote_threshold"></a>

### Function `get_min_vote_threshold`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_min_vote_threshold">get_min_vote_threshold</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u128<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.min_vote_threshold;<br /></code></pre>



<a id="@Specification_1_get_early_resolution_vote_threshold"></a>

### Function `get_early_resolution_vote_threshold`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_early_resolution_vote_threshold">get_early_resolution_vote_threshold</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.early_resolution_vote_threshold;<br /></code></pre>



<a id="@Specification_1_get_votes"></a>

### Function `get_votes`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_votes">get_votes</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): (u128, u128)<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result_1 &#61;&#61; proposal.yes_votes;<br /><b>ensures</b> result_2 &#61;&#61; proposal.no_votes;<br /></code></pre>



<a id="@Specification_1_is_resolved"></a>

### Function `is_resolved`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_resolved">is_resolved</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.is_resolved;<br /></code></pre>




<a id="0x1_voting_AbortsIfNotContainProposalID"></a>


<pre><code><b>schema</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt; &#123;<br />proposal_id: u64;<br />voting_forum_address: <b>address</b>;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(voting_forum.proposals, proposal_id);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_resolution_time_secs"></a>

### Function `get_resolution_time_secs`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_get_resolution_time_secs">get_resolution_time_secs</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): u64<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals, proposal_id);<br /><b>ensures</b> result &#61;&#61; proposal.resolution_time_secs;<br /></code></pre>



<a id="@Specification_1_is_multi_step_proposal_in_execution"></a>

### Function `is_multi_step_proposal_in_execution`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="voting.md#0x1_voting_is_multi_step_proposal_in_execution">is_multi_step_proposal_in_execution</a>&lt;ProposalType: store&gt;(voting_forum_address: <b>address</b>, proposal_id: u64): bool<br /></code></pre>




<pre><code><b>include</b> <a href="voting.md#0x1_voting_AbortsIfNotContainProposalID">AbortsIfNotContainProposalID</a>&lt;ProposalType&gt;;<br /><b>let</b> voting_forum &#61; <b>global</b>&lt;<a href="voting.md#0x1_voting_VotingForum">VotingForum</a>&lt;ProposalType&gt;&gt;(voting_forum_address);<br /><b>let</b> proposal &#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(voting_forum.proposals,proposal_id);<br /><b>aborts_if</b> !std::string::spec_internal_check_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>let</b> execution_key &#61; std::string::spec_utf8(<a href="voting.md#0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY">IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY</a>);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(proposal.metadata,execution_key);<br /><b>let</b> is_multi_step_in_execution_key &#61; <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(proposal.metadata,execution_key);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;bool&gt;(is_multi_step_in_execution_key);<br /><b>ensures</b> result &#61;&#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;bool&gt;(is_multi_step_in_execution_key);<br /></code></pre>



<a id="@Specification_1_is_voting_period_over"></a>

### Function `is_voting_period_over`


<pre><code><b>fun</b> <a href="voting.md#0x1_voting_is_voting_period_over">is_voting_period_over</a>&lt;ProposalType: store&gt;(proposal: &amp;<a href="voting.md#0x1_voting_Proposal">voting::Proposal</a>&lt;ProposalType&gt;): bool<br /></code></pre>




<pre><code><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>();<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; (<a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>() &gt; proposal.expiration_secs);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
