
<a id="0x1_voting"></a>

# Module `0x1::voting`

<br/> This is the general Voting module that can be used as part of a DAO Governance. Voting is designed to be used by<br/> standalone governance modules, who has full control over the voting flow and is responsible for voting power<br/> calculation and including proper capabilities when creating the proposal so resolution can go through.<br/> On&#45;chain governance of the Aptos network also uses Voting.<br/><br/> The voting flow:<br/> 1. The Voting module can be deployed at a known address (e.g. 0x1 for Aptos on&#45;chain governance)<br/> 2. The governance module, e.g. AptosGovernance, can be deployed later and define a GovernanceProposal resource type<br/> that can also contain other information such as Capability resource for authorization.<br/> 3. The governance module&apos;s owner can then register the ProposalType with Voting. This also hosts the proposal list
(forum) on the calling account.<br/> 4. A proposer, through the governance module, can call Voting::create_proposal to create a proposal. create_proposal<br/> cannot be called directly not through the governance module. A script hash of the resolution script that can later<br/> be called to execute the proposal is required.<br/> 5. A voter, through the governance module, can call Voting::vote on a proposal. vote requires passing a &amp;ProposalType<br/> and thus only the governance module that registers ProposalType can call vote.<br/> 6. Once the proposal&apos;s expiration time has passed and more than the defined threshold has voted yes on the proposal,<br/> anyone can call resolve which returns the content of the proposal (of type ProposalType) that can be used to execute.<br/> 7. Only the resolution script with the same script hash specified in the proposal can call Voting::resolve as part of<br/> the resolution process.


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


<pre><code>use 0x1::account;<br/>use 0x1::bcs;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::from_bcs;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::simple_map;<br/>use 0x1::string;<br/>use 0x1::table;<br/>use 0x1::timestamp;<br/>use 0x1::transaction_context;<br/>use 0x1::type_info;<br/></code></pre>



<a id="0x1_voting_Proposal"></a>

## Struct `Proposal`

Extra metadata (e.g. description, code url) can be part of the ProposalType struct.


<pre><code>struct Proposal&lt;ProposalType: store&gt; has store<br/></code></pre>



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
 Required. Should contain enough information to execute later, for example the required capability.<br/> This is stored as an option so we can return it to governance when the proposal is resolved.
</dd>
<dt>
<code>metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Optional. Value is serialized value of an attribute.<br/> Currently, we have three attributes that are used by the voting flow.<br/> 1. RESOLVABLE_TIME_METADATA_KEY: this is uesed to record the resolvable time to ensure that resolution has to be done non&#45;atomically.<br/> 2. IS_MULTI_STEP_PROPOSAL_KEY: this is used to track if a proposal is single&#45;step or multi&#45;step.<br/> 3. IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY: this attribute only applies to multi&#45;step proposals. A single&#45;step proposal will not have<br/> this field in its metadata map. The value is used to indicate if a multi&#45;step proposal is in execution. If yes, we will disable further<br/> voting for this multi&#45;step proposal.
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
 Required. The hash for the execution script module. Only the same exact script module can resolve this<br/> proposal.
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
 Optional. Early resolution threshold. If specified, the proposal can be resolved early if the total<br/> number of yes or no votes passes this threshold.<br/> For example, this can be set to 50% of the total supply of the voting token, so if &gt; 50% vote yes or no,<br/> the proposal can be resolved before expiration.
</dd>
<dt>
<code>yes_votes: u128</code>
</dt>
<dd>
 Number of votes for each outcome.<br/> u128 since the voting power is already u64 and can add up to more than u64 can hold.
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



<pre><code>struct VotingForum&lt;ProposalType: store&gt; has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposals: table::Table&lt;u64, voting::Proposal&lt;ProposalType&gt;&gt;</code>
</dt>
<dd>
 Use Table for execution optimization instead of Vector for gas cost since Vector is read entirely into memory<br/> during execution while only relevant Table entries are.
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
 Unique identifier for a proposal. This allows for 2 &#42; 10&#42;&#42;19 proposals.
</dd>
</dl>


</details>

<a id="0x1_voting_VotingEvents"></a>

## Struct `VotingEvents`



<pre><code>struct VotingEvents has store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct CreateProposal has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct RegisterForum has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct Vote has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct ResolveProposal has drop, store<br/></code></pre>



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



<pre><code>struct CreateProposalEvent has drop, store<br/></code></pre>



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



<pre><code>struct RegisterForumEvent has drop, store<br/></code></pre>



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



<pre><code>struct VoteEvent has drop, store<br/></code></pre>



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


<pre><code>const EINVALID_MIN_VOTE_THRESHOLD: u64 &#61; 7;<br/></code></pre>



<a id="0x1_voting_EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION"></a>

If a proposal is multi&#45;step, we need to use <code>resolve_proposal_v2()</code> to resolve it.<br/> If we use <code>resolve()</code> to resolve a multi&#45;step proposal, it will fail with EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION.


<pre><code>const EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION: u64 &#61; 10;<br/></code></pre>



<a id="0x1_voting_EMULTI_STEP_PROPOSAL_IN_EXECUTION"></a>

Cannot vote if the specified multi&#45;step proposal is in execution.


<pre><code>const EMULTI_STEP_PROPOSAL_IN_EXECUTION: u64 &#61; 9;<br/></code></pre>



<a id="0x1_voting_EPROPOSAL_ALREADY_RESOLVED"></a>

Proposal cannot be resolved more than once


<pre><code>const EPROPOSAL_ALREADY_RESOLVED: u64 &#61; 3;<br/></code></pre>



<a id="0x1_voting_EPROPOSAL_CANNOT_BE_RESOLVED"></a>

Proposal cannot be resolved. Either voting duration has not passed, not enough votes, or fewer yes than no votes


<pre><code>const EPROPOSAL_CANNOT_BE_RESOLVED: u64 &#61; 2;<br/></code></pre>



<a id="0x1_voting_EPROPOSAL_EMPTY_EXECUTION_HASH"></a>

Proposal cannot contain an empty execution script hash


<pre><code>const EPROPOSAL_EMPTY_EXECUTION_HASH: u64 &#61; 4;<br/></code></pre>



<a id="0x1_voting_EPROPOSAL_EXECUTION_HASH_NOT_MATCHING"></a>

Current script&apos;s execution hash does not match the specified proposal&apos;s


<pre><code>const EPROPOSAL_EXECUTION_HASH_NOT_MATCHING: u64 &#61; 1;<br/></code></pre>



<a id="0x1_voting_EPROPOSAL_IS_SINGLE_STEP"></a>

Cannot call <code>is_multi_step_proposal_in_execution()</code> on single&#45;step proposals.


<pre><code>const EPROPOSAL_IS_SINGLE_STEP: u64 &#61; 12;<br/></code></pre>



<a id="0x1_voting_EPROPOSAL_VOTING_ALREADY_ENDED"></a>

Proposal&apos;s voting period has already ended.


<pre><code>const EPROPOSAL_VOTING_ALREADY_ENDED: u64 &#61; 5;<br/></code></pre>



<a id="0x1_voting_ERESOLUTION_CANNOT_BE_ATOMIC"></a>

Resolution of a proposal cannot happen atomically in the same transaction as the last vote.


<pre><code>const ERESOLUTION_CANNOT_BE_ATOMIC: u64 &#61; 8;<br/></code></pre>



<a id="0x1_voting_ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH"></a>

If we call <code>resolve_proposal_v2()</code> to resolve a single&#45;step proposal, the <code>next_execution_hash</code> parameter should be an empty vector.


<pre><code>const ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH: u64 &#61; 11;<br/></code></pre>



<a id="0x1_voting_EVOTING_FORUM_ALREADY_REGISTERED"></a>

Voting forum has already been registered.


<pre><code>const EVOTING_FORUM_ALREADY_REGISTERED: u64 &#61; 6;<br/></code></pre>



<a id="0x1_voting_IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY"></a>

Key used to track if the multi&#45;step proposal is in execution / resolving in progress.


<pre><code>const IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY: vector&lt;u8&gt; &#61; [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 73, 78, 95, 69, 88, 69, 67, 85, 84, 73, 79, 78];<br/></code></pre>



<a id="0x1_voting_IS_MULTI_STEP_PROPOSAL_KEY"></a>

Key used to track if the proposal is multi&#45;step


<pre><code>const IS_MULTI_STEP_PROPOSAL_KEY: vector&lt;u8&gt; &#61; [73, 83, 95, 77, 85, 76, 84, 73, 95, 83, 84, 69, 80, 95, 80, 82, 79, 80, 79, 83, 65, 76, 95, 75, 69, 89];<br/></code></pre>



<a id="0x1_voting_PROPOSAL_STATE_FAILED"></a>

Proposal has failed because either the min vote threshold is not met or majority voted no.


<pre><code>const PROPOSAL_STATE_FAILED: u64 &#61; 3;<br/></code></pre>



<a id="0x1_voting_PROPOSAL_STATE_PENDING"></a>

ProposalStateEnum representing proposal state.


<pre><code>const PROPOSAL_STATE_PENDING: u64 &#61; 0;<br/></code></pre>



<a id="0x1_voting_PROPOSAL_STATE_SUCCEEDED"></a>



<pre><code>const PROPOSAL_STATE_SUCCEEDED: u64 &#61; 1;<br/></code></pre>



<a id="0x1_voting_RESOLVABLE_TIME_METADATA_KEY"></a>

Key used to track the resolvable time in the proposal&apos;s metadata.


<pre><code>const RESOLVABLE_TIME_METADATA_KEY: vector&lt;u8&gt; &#61; [82, 69, 83, 79, 76, 86, 65, 66, 76, 69, 95, 84, 73, 77, 69, 95, 77, 69, 84, 65, 68, 65, 84, 65, 95, 75, 69, 89];<br/></code></pre>



<a id="0x1_voting_register"></a>

## Function `register`



<pre><code>public fun register&lt;ProposalType: store&gt;(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun register&lt;ProposalType: store&gt;(account: &amp;signer) &#123;<br/>    let addr &#61; signer::address_of(account);<br/>    assert!(!exists&lt;VotingForum&lt;ProposalType&gt;&gt;(addr), error::already_exists(EVOTING_FORUM_ALREADY_REGISTERED));<br/><br/>    let voting_forum &#61; VotingForum&lt;ProposalType&gt; &#123;<br/>        next_proposal_id: 0,<br/>        proposals: table::new&lt;u64, Proposal&lt;ProposalType&gt;&gt;(),<br/>        events: VotingEvents &#123;<br/>            create_proposal_events: account::new_event_handle&lt;CreateProposalEvent&gt;(account),<br/>            register_forum_events: account::new_event_handle&lt;RegisterForumEvent&gt;(account),<br/>            resolve_proposal_events: account::new_event_handle&lt;ResolveProposal&gt;(account),<br/>            vote_events: account::new_event_handle&lt;VoteEvent&gt;(account),<br/>        &#125;<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            RegisterForum &#123;<br/>                hosting_account: addr,<br/>                proposal_type_info: type_info::type_of&lt;ProposalType&gt;(),<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;RegisterForumEvent&gt;(<br/>        &amp;mut voting_forum.events.register_forum_events,<br/>        RegisterForumEvent &#123;<br/>            hosting_account: addr,<br/>            proposal_type_info: type_info::type_of&lt;ProposalType&gt;(),<br/>        &#125;,<br/>    );<br/><br/>    move_to(account, voting_forum);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_create_proposal"></a>

## Function `create_proposal`

Create a single&#45;step proposal with the given parameters<br/><br/> @param voting_forum_address The forum&apos;s address where the proposal will be stored.<br/> @param execution_content The execution content that will be given back at resolution time. This can contain<br/> data such as a capability resource used to scope the execution.<br/> @param execution_hash The hash for the execution script module. Only the same exact script module can resolve<br/> this proposal.<br/> @param min_vote_threshold The minimum number of votes needed to consider this proposal successful.<br/> @param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.<br/> @param early_resolution_vote_threshold The vote threshold for early resolution of this proposal.<br/> @param metadata A simple_map that stores information about this proposal.<br/> @return The proposal id.


<pre><code>public fun create_proposal&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_proposal&lt;ProposalType: store&gt;(<br/>    proposer: address,<br/>    voting_forum_address: address,<br/>    execution_content: ProposalType,<br/>    execution_hash: vector&lt;u8&gt;,<br/>    min_vote_threshold: u128,<br/>    expiration_secs: u64,<br/>    early_resolution_vote_threshold: Option&lt;u128&gt;,<br/>    metadata: SimpleMap&lt;String, vector&lt;u8&gt;&gt;,<br/>): u64 acquires VotingForum &#123;<br/>    create_proposal_v2(<br/>        proposer,<br/>        voting_forum_address,<br/>        execution_content,<br/>        execution_hash,<br/>        min_vote_threshold,<br/>        expiration_secs,<br/>        early_resolution_vote_threshold,<br/>        metadata,<br/>        false<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_create_proposal_v2"></a>

## Function `create_proposal_v2`

Create a single&#45;step or a multi&#45;step proposal with the given parameters<br/><br/> @param voting_forum_address The forum&apos;s address where the proposal will be stored.<br/> @param execution_content The execution content that will be given back at resolution time. This can contain<br/> data such as a capability resource used to scope the execution.<br/> @param execution_hash The sha&#45;256 hash for the execution script module. Only the same exact script module can<br/> resolve this proposal.<br/> @param min_vote_threshold The minimum number of votes needed to consider this proposal successful.<br/> @param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.<br/> @param early_resolution_vote_threshold The vote threshold for early resolution of this proposal.<br/> @param metadata A simple_map that stores information about this proposal.<br/> @param is_multi_step_proposal A bool value that indicates if the proposal is single&#45;step or multi&#45;step.<br/> @return The proposal id.


<pre><code>public fun create_proposal_v2&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_proposal_v2&lt;ProposalType: store&gt;(<br/>    proposer: address,<br/>    voting_forum_address: address,<br/>    execution_content: ProposalType,<br/>    execution_hash: vector&lt;u8&gt;,<br/>    min_vote_threshold: u128,<br/>    expiration_secs: u64,<br/>    early_resolution_vote_threshold: Option&lt;u128&gt;,<br/>    metadata: SimpleMap&lt;String, vector&lt;u8&gt;&gt;,<br/>    is_multi_step_proposal: bool,<br/>): u64 acquires VotingForum &#123;<br/>    if (option::is_some(&amp;early_resolution_vote_threshold)) &#123;<br/>        assert!(<br/>            min_vote_threshold &lt;&#61; &#42;option::borrow(&amp;early_resolution_vote_threshold),<br/>            error::invalid_argument(EINVALID_MIN_VOTE_THRESHOLD),<br/>        );<br/>    &#125;;<br/>    // Make sure the execution script&apos;s hash is not empty.<br/>    assert!(vector::length(&amp;execution_hash) &gt; 0, error::invalid_argument(EPROPOSAL_EMPTY_EXECUTION_HASH));<br/><br/>    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    let proposal_id &#61; voting_forum.next_proposal_id;<br/>    voting_forum.next_proposal_id &#61; voting_forum.next_proposal_id &#43; 1;<br/><br/>    // Add a flag to indicate if this proposal is single&#45;step or multi&#45;step.<br/>    simple_map::add(&amp;mut metadata, utf8(IS_MULTI_STEP_PROPOSAL_KEY), to_bytes(&amp;is_multi_step_proposal));<br/><br/>    let is_multi_step_in_execution_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>    if (is_multi_step_proposal) &#123;<br/>        // If the given proposal is a multi&#45;step proposal, we will add a flag to indicate if this multi&#45;step proposal is in execution.<br/>        // This value is by default false. We turn this value to true when we start executing the multi&#45;step proposal. This value<br/>        // will be used to disable further voting after we started executing the multi&#45;step proposal.<br/>        simple_map::add(&amp;mut metadata, is_multi_step_in_execution_key, to_bytes(&amp;false));<br/>        // If the proposal is a single&#45;step proposal, we check if the metadata passed by the client has the IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY key.<br/>        // If they have the key, we will remove it, because a single&#45;step proposal that doesn&apos;t need this key.<br/>    &#125; else if (simple_map::contains_key(&amp;mut metadata, &amp;is_multi_step_in_execution_key)) &#123;<br/>        simple_map::remove(&amp;mut metadata, &amp;is_multi_step_in_execution_key);<br/>    &#125;;<br/><br/>    table::add(&amp;mut voting_forum.proposals, proposal_id, Proposal &#123;<br/>        proposer,<br/>        creation_time_secs: timestamp::now_seconds(),<br/>        execution_content: option::some&lt;ProposalType&gt;(execution_content),<br/>        execution_hash,<br/>        metadata,<br/>        min_vote_threshold,<br/>        expiration_secs,<br/>        early_resolution_vote_threshold,<br/>        yes_votes: 0,<br/>        no_votes: 0,<br/>        is_resolved: false,<br/>        resolution_time_secs: 0,<br/>    &#125;);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            CreateProposal &#123;<br/>                proposal_id,<br/>                early_resolution_vote_threshold,<br/>                execution_hash,<br/>                expiration_secs,<br/>                metadata,<br/>                min_vote_threshold,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;CreateProposalEvent&gt;(<br/>        &amp;mut voting_forum.events.create_proposal_events,<br/>        CreateProposalEvent &#123;<br/>            proposal_id,<br/>            early_resolution_vote_threshold,<br/>            execution_hash,<br/>            expiration_secs,<br/>            metadata,<br/>            min_vote_threshold,<br/>        &#125;,<br/>    );<br/><br/>    proposal_id<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_vote"></a>

## Function `vote`

Vote on the given proposal.<br/><br/> @param _proof Required so only the governance module that defines ProposalType can initiate voting.<br/>               This guarantees that voting eligibility and voting power are controlled by the right governance.<br/> @param voting_forum_address The address of the forum where the proposals are stored.<br/> @param proposal_id The proposal id.<br/> @param num_votes Number of votes. Voting power should be calculated by governance.<br/> @param should_pass Whether the votes are for yes or no.


<pre><code>public fun vote&lt;ProposalType: store&gt;(_proof: &amp;ProposalType, voting_forum_address: address, proposal_id: u64, num_votes: u64, should_pass: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vote&lt;ProposalType: store&gt;(<br/>    _proof: &amp;ProposalType,<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>    num_votes: u64,<br/>    should_pass: bool,<br/>) acquires VotingForum &#123;<br/>    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);<br/>    // Voting might still be possible after the proposal has enough yes votes to be resolved early. This would only<br/>    // lead to possible proposal resolution failure if the resolve early threshold is not definitive (e.g. &lt; 50% &#43; 1<br/>    // of the total voting token&apos;s supply). In this case, more voting might actually still be desirable.<br/>    // Governance mechanisms built on this voting module can apply additional rules on when voting is closed as<br/>    // appropriate.<br/>    assert!(!is_voting_period_over(proposal), error::invalid_state(EPROPOSAL_VOTING_ALREADY_ENDED));<br/>    assert!(!proposal.is_resolved, error::invalid_state(EPROPOSAL_ALREADY_RESOLVED));<br/>    // Assert this proposal is single&#45;step, or if the proposal is multi&#45;step, it is not in execution yet.<br/>    assert!(!simple_map::contains_key(&amp;proposal.metadata, &amp;utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY))<br/>        &#124;&#124; &#42;simple_map::borrow(&amp;proposal.metadata, &amp;utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY)) &#61;&#61; to_bytes(<br/>        &amp;false<br/>    ),<br/>        error::invalid_state(EMULTI_STEP_PROPOSAL_IN_EXECUTION));<br/><br/>    if (should_pass) &#123;<br/>        proposal.yes_votes &#61; proposal.yes_votes &#43; (num_votes as u128);<br/>    &#125; else &#123;<br/>        proposal.no_votes &#61; proposal.no_votes &#43; (num_votes as u128);<br/>    &#125;;<br/><br/>    // Record the resolvable time to ensure that resolution has to be done non&#45;atomically.<br/>    let timestamp_secs_bytes &#61; to_bytes(&amp;timestamp::now_seconds());<br/>    let key &#61; utf8(RESOLVABLE_TIME_METADATA_KEY);<br/>    if (simple_map::contains_key(&amp;proposal.metadata, &amp;key)) &#123;<br/>        &#42;simple_map::borrow_mut(&amp;mut proposal.metadata, &amp;key) &#61; timestamp_secs_bytes;<br/>    &#125; else &#123;<br/>        simple_map::add(&amp;mut proposal.metadata, key, timestamp_secs_bytes);<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Vote &#123; proposal_id, num_votes &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;VoteEvent&gt;(<br/>        &amp;mut voting_forum.events.vote_events,<br/>        VoteEvent &#123; proposal_id, num_votes &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_is_proposal_resolvable"></a>

## Function `is_proposal_resolvable`

Common checks on if a proposal is resolvable, regardless if the proposal is single&#45;step or multi&#45;step.


<pre><code>fun is_proposal_resolvable&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_proposal_resolvable&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>) acquires VotingForum &#123;<br/>    let proposal_state &#61; get_proposal_state&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    assert!(proposal_state &#61;&#61; PROPOSAL_STATE_SUCCEEDED, error::invalid_state(EPROPOSAL_CANNOT_BE_RESOLVED));<br/><br/>    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);<br/>    assert!(!proposal.is_resolved, error::invalid_state(EPROPOSAL_ALREADY_RESOLVED));<br/><br/>    // We need to make sure that the resolution is happening in<br/>    // a separate transaction from the last vote to guard against any potential flashloan attacks.<br/>    let resolvable_time &#61; to_u64(&#42;simple_map::borrow(&amp;proposal.metadata, &amp;utf8(RESOLVABLE_TIME_METADATA_KEY)));<br/>    assert!(timestamp::now_seconds() &gt; resolvable_time, error::invalid_state(ERESOLUTION_CANNOT_BE_ATOMIC));<br/><br/>    assert!(<br/>        transaction_context::get_script_hash() &#61;&#61; proposal.execution_hash,<br/>        error::invalid_argument(EPROPOSAL_EXECUTION_HASH_NOT_MATCHING),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_resolve"></a>

## Function `resolve`

Resolve a single&#45;step proposal with given id. Can only be done if there are at least as many votes as min required and<br/> there are more yes votes than no. If either of these conditions is not met, this will revert.<br/><br/> @param voting_forum_address The address of the forum where the proposals are stored.<br/> @param proposal_id The proposal id.


<pre><code>public fun resolve&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): ProposalType<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resolve&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): ProposalType acquires VotingForum &#123;<br/>    is_proposal_resolvable&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/><br/>    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);<br/><br/>    // Assert that the specified proposal is not a multi&#45;step proposal.<br/>    let multi_step_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>    let has_multi_step_key &#61; simple_map::contains_key(&amp;proposal.metadata, &amp;multi_step_key);<br/>    if (has_multi_step_key) &#123;<br/>        let is_multi_step_proposal &#61; from_bcs::to_bool(&#42;simple_map::borrow(&amp;proposal.metadata, &amp;multi_step_key));<br/>        assert!(<br/>            !is_multi_step_proposal,<br/>            error::permission_denied(EMULTI_STEP_PROPOSAL_CANNOT_USE_SINGLE_STEP_RESOLVE_FUNCTION)<br/>        );<br/>    &#125;;<br/><br/>    let resolved_early &#61; can_be_resolved_early(proposal);<br/>    proposal.is_resolved &#61; true;<br/>    proposal.resolution_time_secs &#61; timestamp::now_seconds();<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            ResolveProposal &#123;<br/>                proposal_id,<br/>                yes_votes: proposal.yes_votes,<br/>                no_votes: proposal.no_votes,<br/>                resolved_early,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;ResolveProposal&gt;(<br/>        &amp;mut voting_forum.events.resolve_proposal_events,<br/>        ResolveProposal &#123;<br/>            proposal_id,<br/>            yes_votes: proposal.yes_votes,<br/>            no_votes: proposal.no_votes,<br/>            resolved_early,<br/>        &#125;,<br/>    );<br/><br/>    option::extract(&amp;mut proposal.execution_content)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_resolve_proposal_v2"></a>

## Function `resolve_proposal_v2`

Resolve a single&#45;step or a multi&#45;step proposal with the given id.<br/> Can only be done if there are at least as many votes as min required and<br/> there are more yes votes than no. If either of these conditions is not met, this will revert.<br/><br/><br/> @param voting_forum_address The address of the forum where the proposals are stored.<br/> @param proposal_id The proposal id.<br/> @param next_execution_hash The next execution hash if the given proposal is multi&#45;step.


<pre><code>public fun resolve_proposal_v2&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, next_execution_hash: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resolve_proposal_v2&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>    next_execution_hash: vector&lt;u8&gt;,<br/>) acquires VotingForum &#123;<br/>    is_proposal_resolvable&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/><br/>    let voting_forum &#61; borrow_global_mut&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    let proposal &#61; table::borrow_mut(&amp;mut voting_forum.proposals, proposal_id);<br/><br/>    // Update the IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY key to indicate that the multi&#45;step proposal is in execution.<br/>    let multi_step_in_execution_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>    if (simple_map::contains_key(&amp;proposal.metadata, &amp;multi_step_in_execution_key)) &#123;<br/>        let is_multi_step_proposal_in_execution_value &#61; simple_map::borrow_mut(<br/>            &amp;mut proposal.metadata,<br/>            &amp;multi_step_in_execution_key<br/>        );<br/>        &#42;is_multi_step_proposal_in_execution_value &#61; to_bytes(&amp;true);<br/>    &#125;;<br/><br/>    let multi_step_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>    let is_multi_step &#61; simple_map::contains_key(&amp;proposal.metadata, &amp;multi_step_key) &amp;&amp; from_bcs::to_bool(<br/>        &#42;simple_map::borrow(&amp;proposal.metadata, &amp;multi_step_key)<br/>    );<br/>    let next_execution_hash_is_empty &#61; vector::length(&amp;next_execution_hash) &#61;&#61; 0;<br/><br/>    // Assert that if this proposal is single&#45;step, the `next_execution_hash` parameter is empty.<br/>    assert!(<br/>        is_multi_step &#124;&#124; next_execution_hash_is_empty,<br/>        error::invalid_argument(ESINGLE_STEP_PROPOSAL_CANNOT_HAVE_NEXT_EXECUTION_HASH)<br/>    );<br/><br/>    // If the `next_execution_hash` parameter is empty, it means that either<br/>    // &#45; this proposal is a single&#45;step proposal, or<br/>    // &#45; this proposal is multi&#45;step and we&apos;re currently resolving the last step in the multi&#45;step proposal.<br/>    // We can mark that this proposal is resolved.<br/>    if (next_execution_hash_is_empty) &#123;<br/>        proposal.is_resolved &#61; true;<br/>        proposal.resolution_time_secs &#61; timestamp::now_seconds();<br/><br/>        // Set the `IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY` value to false upon successful resolution of the last step of a multi&#45;step proposal.<br/>        if (is_multi_step) &#123;<br/>            let is_multi_step_proposal_in_execution_value &#61; simple_map::borrow_mut(<br/>                &amp;mut proposal.metadata,<br/>                &amp;multi_step_in_execution_key<br/>            );<br/>            &#42;is_multi_step_proposal_in_execution_value &#61; to_bytes(&amp;false);<br/>        &#125;;<br/>    &#125; else &#123;<br/>        // If the current step is not the last step,<br/>        // update the proposal&apos;s execution hash on&#45;chain to the execution hash of the next step.<br/>        proposal.execution_hash &#61; next_execution_hash;<br/>    &#125;;<br/><br/>    // For single&#45;step proposals, we emit one `ResolveProposal` event per proposal.<br/>    // For multi&#45;step proposals, we emit one `ResolveProposal` event per step in the multi&#45;step proposal. This means<br/>    // that we emit multiple `ResolveProposal` events for the same multi&#45;step proposal.<br/>    let resolved_early &#61; can_be_resolved_early(proposal);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            ResolveProposal &#123;<br/>                proposal_id,<br/>                yes_votes: proposal.yes_votes,<br/>                no_votes: proposal.no_votes,<br/>                resolved_early,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event(<br/>        &amp;mut voting_forum.events.resolve_proposal_events,<br/>        ResolveProposal &#123;<br/>            proposal_id,<br/>            yes_votes: proposal.yes_votes,<br/>            no_votes: proposal.no_votes,<br/>            resolved_early,<br/>        &#125;,<br/>    );<br/><br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_next_proposal_id"></a>

## Function `next_proposal_id`

Return the next unassigned proposal id


<pre><code>&#35;[view]<br/>public fun next_proposal_id&lt;ProposalType: store&gt;(voting_forum_address: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun next_proposal_id&lt;ProposalType: store&gt;(voting_forum_address: address, ): u64 acquires VotingForum &#123;<br/>    let voting_forum &#61; borrow_global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    voting_forum.next_proposal_id<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_proposer"></a>

## Function `get_proposer`



<pre><code>&#35;[view]<br/>public fun get_proposer&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposer&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64<br/>): address acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.proposer<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_is_voting_closed"></a>

## Function `is_voting_closed`



<pre><code>&#35;[view]<br/>public fun is_voting_closed&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_voting_closed&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64<br/>): bool acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    can_be_resolved_early(proposal) &#124;&#124; is_voting_period_over(proposal)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_can_be_resolved_early"></a>

## Function `can_be_resolved_early`

Return true if the proposal has reached early resolution threshold (if specified).


<pre><code>public fun can_be_resolved_early&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_be_resolved_early&lt;ProposalType: store&gt;(proposal: &amp;Proposal&lt;ProposalType&gt;): bool &#123;<br/>    if (option::is_some(&amp;proposal.early_resolution_vote_threshold)) &#123;<br/>        let early_resolution_threshold &#61; &#42;option::borrow(&amp;proposal.early_resolution_vote_threshold);<br/>        if (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124; proposal.no_votes &gt;&#61; early_resolution_threshold) &#123;<br/>            return true<br/>        &#125;;<br/>    &#125;;<br/>    false<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_proposal_metadata"></a>

## Function `get_proposal_metadata`



<pre><code>&#35;[view]<br/>public fun get_proposal_metadata&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_metadata&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): SimpleMap&lt;String, vector&lt;u8&gt;&gt; acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_proposal_metadata_value"></a>

## Function `get_proposal_metadata_value`



<pre><code>&#35;[view]<br/>public fun get_proposal_metadata_value&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, metadata_key: string::String): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_metadata_value&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>    metadata_key: String,<br/>): vector&lt;u8&gt; acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    &#42;simple_map::borrow(&amp;proposal.metadata, &amp;metadata_key)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_proposal_state"></a>

## Function `get_proposal_state`

Return the state of the proposal with given id.<br/><br/> @param voting_forum_address The address of the forum where the proposals are stored.<br/> @param proposal_id The proposal id.<br/> @return Proposal state as an enum value.


<pre><code>&#35;[view]<br/>public fun get_proposal_state&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_state&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): u64 acquires VotingForum &#123;<br/>    if (is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id)) &#123;<br/>        let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>        let yes_votes &#61; proposal.yes_votes;<br/>        let no_votes &#61; proposal.no_votes;<br/><br/>        if (yes_votes &gt; no_votes &amp;&amp; yes_votes &#43; no_votes &gt;&#61; proposal.min_vote_threshold) &#123;<br/>            PROPOSAL_STATE_SUCCEEDED<br/>        &#125; else &#123;<br/>            PROPOSAL_STATE_FAILED<br/>        &#125;<br/>    &#125; else &#123;<br/>        PROPOSAL_STATE_PENDING<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_proposal_creation_secs"></a>

## Function `get_proposal_creation_secs`

Return the proposal&apos;s creation time.


<pre><code>&#35;[view]<br/>public fun get_proposal_creation_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_creation_secs&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): u64 acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.creation_time_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_proposal_expiration_secs"></a>

## Function `get_proposal_expiration_secs`

Return the proposal&apos;s expiration time.


<pre><code>&#35;[view]<br/>public fun get_proposal_expiration_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_proposal_expiration_secs&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): u64 acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.expiration_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_execution_hash"></a>

## Function `get_execution_hash`

Return the proposal&apos;s execution hash.


<pre><code>&#35;[view]<br/>public fun get_execution_hash&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_execution_hash&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): vector&lt;u8&gt; acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.execution_hash<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_min_vote_threshold"></a>

## Function `get_min_vote_threshold`

Return the proposal&apos;s minimum vote threshold


<pre><code>&#35;[view]<br/>public fun get_min_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u128<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_min_vote_threshold&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): u128 acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.min_vote_threshold<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_early_resolution_vote_threshold"></a>

## Function `get_early_resolution_vote_threshold`

Return the proposal&apos;s early resolution minimum vote threshold (optionally set)


<pre><code>&#35;[view]<br/>public fun get_early_resolution_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): option::Option&lt;u128&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_early_resolution_vote_threshold&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): Option&lt;u128&gt; acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.early_resolution_vote_threshold<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_votes"></a>

## Function `get_votes`

Return the proposal&apos;s current vote count (yes_votes, no_votes)


<pre><code>&#35;[view]<br/>public fun get_votes&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): (u128, u128)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_votes&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): (u128, u128) acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    (proposal.yes_votes, proposal.no_votes)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_is_resolved"></a>

## Function `is_resolved`

Return true if the governance proposal has already been resolved.


<pre><code>&#35;[view]<br/>public fun is_resolved&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_resolved&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): bool acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.is_resolved<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_resolution_time_secs"></a>

## Function `get_resolution_time_secs`



<pre><code>&#35;[view]<br/>public fun get_resolution_time_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_resolution_time_secs&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): u64 acquires VotingForum &#123;<br/>    let proposal &#61; get_proposal&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>    proposal.resolution_time_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_is_multi_step_proposal_in_execution"></a>

## Function `is_multi_step_proposal_in_execution`

Return true if the multi&#45;step governance proposal is in execution.


<pre><code>&#35;[view]<br/>public fun is_multi_step_proposal_in_execution&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_multi_step_proposal_in_execution&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): bool acquires VotingForum &#123;<br/>    let voting_forum &#61; borrow_global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    let proposal &#61; table::borrow(&amp;voting_forum.proposals, proposal_id);<br/>    let is_multi_step_in_execution_key &#61; utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>    assert!(<br/>        simple_map::contains_key(&amp;proposal.metadata, &amp;is_multi_step_in_execution_key),<br/>        error::invalid_argument(EPROPOSAL_IS_SINGLE_STEP)<br/>    );<br/>    from_bcs::to_bool(&#42;simple_map::borrow(&amp;proposal.metadata, &amp;is_multi_step_in_execution_key))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_is_voting_period_over"></a>

## Function `is_voting_period_over`

Return true if the voting period of the given proposal has already ended.


<pre><code>fun is_voting_period_over&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_voting_period_over&lt;ProposalType: store&gt;(proposal: &amp;Proposal&lt;ProposalType&gt;): bool &#123;<br/>    timestamp::now_seconds() &gt; proposal.expiration_secs<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_voting_get_proposal"></a>

## Function `get_proposal`



<pre><code>fun get_proposal&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): &amp;voting::Proposal&lt;ProposalType&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun get_proposal&lt;ProposalType: store&gt;(<br/>    voting_forum_address: address,<br/>    proposal_id: u64,<br/>): &amp;Proposal&lt;ProposalType&gt; acquires VotingForum &#123;<br/>    let voting_forum &#61; borrow_global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>    table::borrow(&amp;voting_forum.proposals, proposal_id)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The proposal ID in a voting forum is unique and always increases monotonically with each new proposal created for that voting forum.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The create_proposal and create_proposal_v2 create a new proposal with a unique ID derived from the voting_forum&apos;s next_proposal_id incrementally.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;create_proposal&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;While voting, it ensures that only the governance module that defines ProposalType may initiate voting and that the proposal under vote exists in the specified voting forum.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The vote function verifies the eligibility and validity of a proposal before allowing voting. It ensures that only the correct governance module initiates voting. The function checks if the proposal is currently eligible for voting by confirming it has not resolved and the voting period has not ended.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;vote&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;After resolving a single&#45;step proposal, the corresponding proposal is guaranteed to be marked as successfully resolved.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;Upon invoking the resolve function on a proposal, it undergoes a series of checks to ensure its validity. These include verifying if the proposal exists, is a single&#45;step proposal, and meets the criteria for resolution. If the checks pass, the proposal&apos;s is_resolved flag becomes true, indicating a successful resolution.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;resolve&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;In the context of v2 proposal resolving, both single&#45;step and multi&#45;step proposals are accurately handled. It ensures that for single&#45;step proposals, the next execution hash is empty and resolves the proposal, while for multi&#45;step proposals, it guarantees that the next execution hash corresponds to the hash of the next step, maintaining the integrity of the proposal execution sequence.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The function resolve_proposal_v2 correctly handles both single&#45;step and multi&#45;step proposals. For single&#45;step proposals, it ensures that the next_execution_hash parameter is empty and resolves the proposal. For multi&#45;step proposals, it ensures that the next_execution_hash parameter contains the hash of the next step.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;resolve_proposal_v2&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_register"></a>

### Function `register`


<pre><code>public fun register&lt;ProposalType: store&gt;(account: &amp;signer)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(account);<br/>aborts_if exists&lt;VotingForum&lt;ProposalType&gt;&gt;(addr);<br/>aborts_if !exists&lt;account::Account&gt;(addr);<br/>let register_account &#61; global&lt;account::Account&gt;(addr);<br/>aborts_if register_account.guid_creation_num &#43; 4 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if register_account.guid_creation_num &#43; 4 &gt; MAX_U64;<br/>aborts_if !type_info::spec_is_struct&lt;ProposalType&gt;();<br/>ensures exists&lt;VotingForum&lt;ProposalType&gt;&gt;(addr);<br/></code></pre>



<a id="@Specification_1_create_proposal"></a>

### Function `create_proposal`


<pre><code>public fun create_proposal&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;): u64<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>include CreateProposalAbortsIfAndEnsures&lt;ProposalType&gt;&#123;is_multi_step_proposal: false&#125;;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures result &#61;&#61; old(global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;<br/></code></pre>



<a id="@Specification_1_create_proposal_v2"></a>

### Function `create_proposal_v2`


<pre><code>public fun create_proposal_v2&lt;ProposalType: store&gt;(proposer: address, voting_forum_address: address, execution_content: ProposalType, execution_hash: vector&lt;u8&gt;, min_vote_threshold: u128, expiration_secs: u64, early_resolution_vote_threshold: option::Option&lt;u128&gt;, metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;, is_multi_step_proposal: bool): u64<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>include CreateProposalAbortsIfAndEnsures&lt;ProposalType&gt;;<br/>ensures result &#61;&#61; old(global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address)).next_proposal_id;<br/></code></pre>




<a id="0x1_voting_CreateProposalAbortsIfAndEnsures"></a>


<pre><code>schema CreateProposalAbortsIfAndEnsures&lt;ProposalType&gt; &#123;<br/>voting_forum_address: address;<br/>execution_hash: vector&lt;u8&gt;;<br/>min_vote_threshold: u128;<br/>early_resolution_vote_threshold: Option&lt;u128&gt;;<br/>metadata: SimpleMap&lt;String, vector&lt;u8&gt;&gt;;<br/>is_multi_step_proposal: bool;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal_id &#61; voting_forum.next_proposal_id;<br/>aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>aborts_if table::spec_contains(voting_forum.proposals,proposal_id);<br/>aborts_if len(early_resolution_vote_threshold.vec) !&#61; 0 &amp;&amp; min_vote_threshold &gt; early_resolution_vote_threshold.vec[0];<br/>aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>aborts_if len(execution_hash) &#61;&#61; 0;<br/>let execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>aborts_if simple_map::spec_contains_key(metadata, execution_key);<br/>aborts_if voting_forum.next_proposal_id &#43; 1 &gt; MAX_U64;<br/>let is_multi_step_in_execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>aborts_if is_multi_step_proposal &amp;&amp; simple_map::spec_contains_key(metadata, is_multi_step_in_execution_key);<br/>let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let post post_metadata &#61; table::spec_get(post_voting_forum.proposals, proposal_id).metadata;<br/>ensures post_voting_forum.next_proposal_id &#61;&#61; voting_forum.next_proposal_id &#43; 1;<br/>ensures table::spec_contains(post_voting_forum.proposals, proposal_id);<br/>ensures if (is_multi_step_proposal) &#123;<br/>    simple_map::spec_get(post_metadata, is_multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(false)<br/>&#125; else &#123;<br/>    !simple_map::spec_contains_key(post_metadata, is_multi_step_in_execution_key)<br/>&#125;;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code>public fun vote&lt;ProposalType: store&gt;(_proof: &amp;ProposalType, voting_forum_address: address, proposal_id: u64, num_votes: u64, should_pass: bool)<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);<br/>aborts_if is_voting_period_over(proposal);<br/>aborts_if proposal.is_resolved;<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>let execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>aborts_if simple_map::spec_contains_key(proposal.metadata, execution_key) &amp;&amp;<br/>          simple_map::spec_get(proposal.metadata, execution_key) !&#61; std::bcs::serialize(false);<br/>aborts_if if (should_pass) &#123; proposal.yes_votes &#43; num_votes &gt; MAX_U128 &#125; else &#123; proposal.no_votes &#43; num_votes &gt; MAX_U128 &#125;;<br/>aborts_if !std::string::spec_internal_check_utf8(RESOLVABLE_TIME_METADATA_KEY);<br/>let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);<br/>ensures if (should_pass) &#123;<br/>    post_proposal.yes_votes &#61;&#61; proposal.yes_votes &#43; num_votes<br/>&#125; else &#123;<br/>    post_proposal.no_votes &#61;&#61; proposal.no_votes &#43; num_votes<br/>&#125;;<br/>let timestamp_secs_bytes &#61; std::bcs::serialize(timestamp::spec_now_seconds());<br/>let key &#61; std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY);<br/>ensures simple_map::spec_get(post_proposal.metadata, key) &#61;&#61; timestamp_secs_bytes;<br/></code></pre>



<a id="@Specification_1_is_proposal_resolvable"></a>

### Function `is_proposal_resolvable`


<pre><code>fun is_proposal_resolvable&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64)<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>include IsProposalResolvableAbortsIf&lt;ProposalType&gt;;<br/></code></pre>




<a id="0x1_voting_IsProposalResolvableAbortsIf"></a>


<pre><code>schema IsProposalResolvableAbortsIf&lt;ProposalType&gt; &#123;<br/>voting_forum_address: address;<br/>proposal_id: u64;<br/>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>let voting_closed &#61; spec_is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>aborts_if voting_closed &amp;&amp; (proposal.yes_votes &lt;&#61; proposal.no_votes &#124;&#124; proposal.yes_votes &#43; proposal.no_votes &lt; proposal.min_vote_threshold);<br/>aborts_if !voting_closed;<br/>aborts_if proposal.is_resolved;<br/>aborts_if !std::string::spec_internal_check_utf8(RESOLVABLE_TIME_METADATA_KEY);<br/>aborts_if !simple_map::spec_contains_key(proposal.metadata, std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY));<br/>aborts_if !from_bcs::deserializable&lt;u64&gt;(simple_map::spec_get(proposal.metadata, std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY)));<br/>aborts_if timestamp::spec_now_seconds() &lt;&#61; from_bcs::deserialize&lt;u64&gt;(simple_map::spec_get(proposal.metadata, std::string::spec_utf8(RESOLVABLE_TIME_METADATA_KEY)));<br/>aborts_if transaction_context::spec_get_script_hash() !&#61; proposal.execution_hash;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_resolve"></a>

### Function `resolve`


<pre><code>public fun resolve&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): ProposalType<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>include IsProposalResolvableAbortsIf&lt;ProposalType&gt;;<br/>aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>let multi_step_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>let has_multi_step_key &#61; simple_map::spec_contains_key(proposal.metadata, multi_step_key);<br/>aborts_if has_multi_step_key &amp;&amp; !from_bcs::deserializable&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));<br/>aborts_if has_multi_step_key &amp;&amp; from_bcs::deserialize&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));<br/>let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
ensures post_proposal.is_resolved &#61;&#61; true;<br/>ensures post_proposal.resolution_time_secs &#61;&#61; timestamp::spec_now_seconds();<br/>aborts_if option::spec_is_none(proposal.execution_content);<br/>ensures result &#61;&#61; option::spec_borrow(proposal.execution_content);<br/>ensures option::spec_is_none(post_proposal.execution_content);<br/></code></pre>



<a id="@Specification_1_resolve_proposal_v2"></a>

### Function `resolve_proposal_v2`


<pre><code>public fun resolve_proposal_v2&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, next_execution_hash: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 300;<br/>requires chain_status::is_operating();<br/>include IsProposalResolvableAbortsIf&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>let post post_voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let post post_proposal &#61; table::spec_get(post_voting_forum.proposals, proposal_id);<br/>let multi_step_in_execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>ensures (simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key) &amp;&amp; len(next_execution_hash) !&#61; 0) &#61;&#61;&gt;<br/>    simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(true);<br/>ensures (simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key) &amp;&amp;<br/>    (len(next_execution_hash) &#61;&#61; 0 &amp;&amp; !is_multi_step)) &#61;&#61;&gt;<br/>    simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(true);<br/>let multi_step_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_KEY);<br/>aborts_if simple_map::spec_contains_key(proposal.metadata, multi_step_key) &amp;&amp;<br/>    !from_bcs::deserializable&lt;bool&gt;(simple_map::spec_get(proposal.metadata, multi_step_key));<br/>let is_multi_step &#61; simple_map::spec_contains_key(proposal.metadata, multi_step_key) &amp;&amp;<br/>    from_bcs::deserialize(simple_map::spec_get(proposal.metadata, multi_step_key));<br/>aborts_if !is_multi_step &amp;&amp; len(next_execution_hash) !&#61; 0;<br/>aborts_if len(next_execution_hash) &#61;&#61; 0 &amp;&amp; !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>aborts_if len(next_execution_hash) &#61;&#61; 0 &amp;&amp; is_multi_step &amp;&amp; !simple_map::spec_contains_key(proposal.metadata, multi_step_in_execution_key);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
ensures len(next_execution_hash) &#61;&#61; 0 &#61;&#61;&gt; post_proposal.resolution_time_secs &#61;&#61; timestamp::spec_now_seconds();<br/>ensures len(next_execution_hash) &#61;&#61; 0 &#61;&#61;&gt; post_proposal.is_resolved &#61;&#61; true;<br/>ensures (len(next_execution_hash) &#61;&#61; 0 &amp;&amp; is_multi_step) &#61;&#61;&gt; simple_map::spec_get(post_proposal.metadata, multi_step_in_execution_key) &#61;&#61; std::bcs::serialize(false);<br/>ensures len(next_execution_hash) !&#61; 0 &#61;&#61;&gt; post_proposal.execution_hash &#61;&#61; next_execution_hash;<br/></code></pre>



<a id="@Specification_1_next_proposal_id"></a>

### Function `next_proposal_id`


<pre><code>&#35;[view]<br/>public fun next_proposal_id&lt;ProposalType: store&gt;(voting_forum_address: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>ensures result &#61;&#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address).next_proposal_id;<br/></code></pre>



<a id="@Specification_1_get_proposer"></a>

### Function `get_proposer`


<pre><code>&#35;[view]<br/>public fun get_proposer&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): address<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.proposer;<br/></code></pre>



<a id="@Specification_1_is_voting_closed"></a>

### Function `is_voting_closed`


<pre><code>&#35;[view]<br/>public fun is_voting_closed&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>ensures result &#61;&#61; spec_is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/></code></pre>




<a id="0x1_voting_spec_is_voting_closed"></a>


<pre><code>fun spec_is_voting_closed&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool &#123;<br/>   let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>   let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>   spec_can_be_resolved_early&lt;ProposalType&gt;(proposal) &#124;&#124; is_voting_period_over(proposal)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_can_be_resolved_early"></a>

### Function `can_be_resolved_early`


<pre><code>public fun can_be_resolved_early&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>ensures result &#61;&#61; spec_can_be_resolved_early&lt;ProposalType&gt;(proposal);<br/></code></pre>




<a id="0x1_voting_spec_can_be_resolved_early"></a>


<pre><code>fun spec_can_be_resolved_early&lt;ProposalType: store&gt;(proposal: Proposal&lt;ProposalType&gt;): bool &#123;<br/>   if (option::spec_is_some(proposal.early_resolution_vote_threshold)) &#123;<br/>       let early_resolution_threshold &#61; option::spec_borrow(proposal.early_resolution_vote_threshold);<br/>       if (proposal.yes_votes &gt;&#61; early_resolution_threshold &#124;&#124; proposal.no_votes &gt;&#61; early_resolution_threshold) &#123;<br/>           true<br/>       &#125; else&#123;<br/>           false<br/>       &#125;<br/>   &#125; else &#123;<br/>       false<br/>   &#125;<br/>&#125;<br/></code></pre>




<a id="0x1_voting_spec_get_proposal_state"></a>


<pre><code>fun spec_get_proposal_state&lt;ProposalType&gt;(<br/>   voting_forum_address: address,<br/>   proposal_id: u64,<br/>   voting_forum: VotingForum&lt;ProposalType&gt;<br/>): u64 &#123;<br/>   let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>   let voting_closed &#61; spec_is_voting_closed&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/>   let proposal_vote_cond &#61; (proposal.yes_votes &gt; proposal.no_votes &amp;&amp; proposal.yes_votes &#43; proposal.no_votes &gt;&#61; proposal.min_vote_threshold);<br/>   if (voting_closed &amp;&amp; proposal_vote_cond) &#123;<br/>       PROPOSAL_STATE_SUCCEEDED<br/>   &#125; else if (voting_closed &amp;&amp; !proposal_vote_cond) &#123;<br/>       PROPOSAL_STATE_FAILED<br/>   &#125; else &#123;<br/>       PROPOSAL_STATE_PENDING<br/>   &#125;<br/>&#125;<br/></code></pre>




<a id="0x1_voting_spec_get_proposal_expiration_secs"></a>


<pre><code>fun spec_get_proposal_expiration_secs&lt;ProposalType: store&gt;(<br/>   voting_forum_address: address,<br/>   proposal_id: u64,<br/>): u64 &#123;<br/>   let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>   let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>   proposal.expiration_secs<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_proposal_metadata"></a>

### Function `get_proposal_metadata`


<pre><code>&#35;[view]<br/>public fun get_proposal_metadata&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.metadata;<br/></code></pre>



<a id="@Specification_1_get_proposal_metadata_value"></a>

### Function `get_proposal_metadata_value`


<pre><code>&#35;[view]<br/>public fun get_proposal_metadata_value&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64, metadata_key: string::String): vector&lt;u8&gt;<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>aborts_if !simple_map::spec_contains_key(proposal.metadata, metadata_key);<br/>ensures result &#61;&#61; simple_map::spec_get(proposal.metadata, metadata_key);<br/></code></pre>



<a id="@Specification_1_get_proposal_state"></a>

### Function `get_proposal_state`


<pre><code>&#35;[view]<br/>public fun get_proposal_state&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>




<pre><code>pragma addition_overflow_unchecked;<br/>requires chain_status::is_operating();<br/>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>ensures result &#61;&#61; spec_get_proposal_state(voting_forum_address, proposal_id, voting_forum);<br/></code></pre>



<a id="@Specification_1_get_proposal_creation_secs"></a>

### Function `get_proposal_creation_secs`


<pre><code>&#35;[view]<br/>public fun get_proposal_creation_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.creation_time_secs;<br/></code></pre>



<a id="@Specification_1_get_proposal_expiration_secs"></a>

### Function `get_proposal_expiration_secs`


<pre><code>&#35;[view]<br/>public fun get_proposal_expiration_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>ensures result &#61;&#61; spec_get_proposal_expiration_secs&lt;ProposalType&gt;(voting_forum_address, proposal_id);<br/></code></pre>



<a id="@Specification_1_get_execution_hash"></a>

### Function `get_execution_hash`


<pre><code>&#35;[view]<br/>public fun get_execution_hash&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): vector&lt;u8&gt;<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.execution_hash;<br/></code></pre>



<a id="@Specification_1_get_min_vote_threshold"></a>

### Function `get_min_vote_threshold`


<pre><code>&#35;[view]<br/>public fun get_min_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u128<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.min_vote_threshold;<br/></code></pre>



<a id="@Specification_1_get_early_resolution_vote_threshold"></a>

### Function `get_early_resolution_vote_threshold`


<pre><code>&#35;[view]<br/>public fun get_early_resolution_vote_threshold&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): option::Option&lt;u128&gt;<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.early_resolution_vote_threshold;<br/></code></pre>



<a id="@Specification_1_get_votes"></a>

### Function `get_votes`


<pre><code>&#35;[view]<br/>public fun get_votes&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): (u128, u128)<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result_1 &#61;&#61; proposal.yes_votes;<br/>ensures result_2 &#61;&#61; proposal.no_votes;<br/></code></pre>



<a id="@Specification_1_is_resolved"></a>

### Function `is_resolved`


<pre><code>&#35;[view]<br/>public fun is_resolved&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.is_resolved;<br/></code></pre>




<a id="0x1_voting_AbortsIfNotContainProposalID"></a>


<pre><code>schema AbortsIfNotContainProposalID&lt;ProposalType&gt; &#123;<br/>proposal_id: u64;<br/>voting_forum_address: address;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>aborts_if !table::spec_contains(voting_forum.proposals, proposal_id);<br/>aborts_if !exists&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_resolution_time_secs"></a>

### Function `get_resolution_time_secs`


<pre><code>&#35;[view]<br/>public fun get_resolution_time_secs&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): u64<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals, proposal_id);<br/>ensures result &#61;&#61; proposal.resolution_time_secs;<br/></code></pre>



<a id="@Specification_1_is_multi_step_proposal_in_execution"></a>

### Function `is_multi_step_proposal_in_execution`


<pre><code>&#35;[view]<br/>public fun is_multi_step_proposal_in_execution&lt;ProposalType: store&gt;(voting_forum_address: address, proposal_id: u64): bool<br/></code></pre>




<pre><code>include AbortsIfNotContainProposalID&lt;ProposalType&gt;;<br/>let voting_forum &#61; global&lt;VotingForum&lt;ProposalType&gt;&gt;(voting_forum_address);<br/>let proposal &#61; table::spec_get(voting_forum.proposals,proposal_id);<br/>aborts_if !std::string::spec_internal_check_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>let execution_key &#61; std::string::spec_utf8(IS_MULTI_STEP_PROPOSAL_IN_EXECUTION_KEY);<br/>aborts_if !simple_map::spec_contains_key(proposal.metadata,execution_key);<br/>let is_multi_step_in_execution_key &#61; simple_map::spec_get(proposal.metadata,execution_key);<br/>aborts_if !from_bcs::deserializable&lt;bool&gt;(is_multi_step_in_execution_key);<br/>ensures result &#61;&#61; from_bcs::deserialize&lt;bool&gt;(is_multi_step_in_execution_key);<br/></code></pre>



<a id="@Specification_1_is_voting_period_over"></a>

### Function `is_voting_period_over`


<pre><code>fun is_voting_period_over&lt;ProposalType: store&gt;(proposal: &amp;voting::Proposal&lt;ProposalType&gt;): bool<br/></code></pre>




<pre><code>requires chain_status::is_operating();<br/>aborts_if false;<br/>ensures result &#61;&#61; (timestamp::spec_now_seconds() &gt; proposal.expiration_secs);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
