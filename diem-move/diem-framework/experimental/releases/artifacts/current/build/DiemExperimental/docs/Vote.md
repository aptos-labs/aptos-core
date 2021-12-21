
<a name="0x1_Vote"></a>

# Module `0x1::Vote`

The Vote module is used to allow voting on proposals on the chain.
It is typically not going to be used directly, but is intended to be
used as a library for modules which want to perform operations which
require m-of-n approvals from accounts on chain.
A typical workflow would look like the following
* Module M creates a ballot with a given <code>Proposal</code> and an approval policy using <code>create_ballot</code>
* It receives the BallotID corresponding to the ballot
* It submits votes using the <code>vote</code> function from the voters
* If a vote causes a ballot to be approved, <code>vote</code> returns <code><b>true</b></code> and Module M can proceed with the operation requested by the <code>Proposal</code>


-  [Struct `BallotID`](#0x1_Vote_BallotID)
-  [Struct `WeightedVoter`](#0x1_Vote_WeightedVoter)
-  [Struct `Ballot`](#0x1_Vote_Ballot)
-  [Resource `Ballots`](#0x1_Vote_Ballots)
-  [Resource `BallotCounter`](#0x1_Vote_BallotCounter)
-  [Struct `CreateBallotEvent`](#0x1_Vote_CreateBallotEvent)
-  [Struct `RemoveBallotEvent`](#0x1_Vote_RemoveBallotEvent)
-  [Struct `VotedEvent`](#0x1_Vote_VotedEvent)
-  [Struct `BallotApprovedEvent`](#0x1_Vote_BallotApprovedEvent)
-  [Constants](#@Constants_0)
-  [Function `new_ballot_id`](#0x1_Vote_new_ballot_id)
-  [Function `new_weighted_voter`](#0x1_Vote_new_weighted_voter)
-  [Function `create_ballot`](#0x1_Vote_create_ballot)
-  [Function `check_voter_present`](#0x1_Vote_check_voter_present)
-  [Function `vote`](#0x1_Vote_vote)
-  [Function `gc_ballots`](#0x1_Vote_gc_ballots)
-  [Function `gc_test_helper`](#0x1_Vote_gc_test_helper)
-  [Function `gc_internal`](#0x1_Vote_gc_internal)
-  [Function `remove_ballot_internal`](#0x1_Vote_remove_ballot_internal)
-  [Function `remove_ballot`](#0x1_Vote_remove_ballot)
-  [Function `incr_counter`](#0x1_Vote_incr_counter)
-  [Module Specification](#@Module_Specification_1)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/BCS.md#0x1_BCS">0x1::BCS</a>;
<b>use</b> <a href="DiemTimestamp.md#0x1_DiemTimestamp">0x1::DiemTimestamp</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event">0x1::Event</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_Vote_BallotID"></a>

## Struct `BallotID`

An unique identifier for a ballot. A counter is stored
under each proposers address which is incremented
every time a new ballot is created. The proposers
address is also part of the ballot id.


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_BallotID">BallotID</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>counter: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_WeightedVoter"></a>

## Struct `WeightedVoter`

WeightedVoter represents a voter with a weight
The voter is represented by the bcs serialization of address


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_WeightedVoter">WeightedVoter</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>weight: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>voter: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_Ballot"></a>

## Struct `Ballot`

Ballot is a struct which contains a Proposal on which
votes are gathered. A ballot is started by a proposer
and it carries the proposal, the approval policy,
expiration timestamp


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal: drop, store&gt; <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposal: Proposal</code>
</dt>
<dd>
 Details for the proposal being voted on.
</dd>
<dt>
<code>proposal_type: vector&lt;u8&gt;</code>
</dt>
<dd>
 A human readable type for the proposal - ex: "create_validator_owner",
 "freeze_account", "create_vasp", etc.
 This lives outside the <code>proposal: Proposal</code> to make it easy for
 indexers to index the ballots which have been proposed and
 categorize them into "types"
</dd>
<dt>
<code>num_votes_required: u64</code>
</dt>
<dd>
 The num_votes_required for this proposal to be approved
</dd>
<dt>
<code>allowed_voters: vector&lt;<a href="Vote.md#0x1_Vote_WeightedVoter">Vote::WeightedVoter</a>&gt;</code>
</dt>
<dd>
 A vector of addresses which are allowed to vote on this ballot.
</dd>
<dt>
<code>votes_received: vector&lt;<a href="Vote.md#0x1_Vote_WeightedVoter">Vote::WeightedVoter</a>&gt;</code>
</dt>
<dd>
 Votes received so far
</dd>
<dt>
<code>total_weighted_votes_received: u64</code>
</dt>
<dd>
 Total number of weighted votes received
</dd>
<dt>
<code>ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_timestamp_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_Ballots"></a>

## Resource `Ballots`

Ballots stores a list of ballots under a proposers address.
It is type parametrized by the Proposal. Some of these may
be expired and can be cleaned up using the <code>gc_ballots</code> function
It also contains the handles for all the events associated
with the ballots created by a proposer


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal: drop, store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ballots: vector&lt;<a href="Vote.md#0x1_Vote_Ballot">Vote::Ballot</a>&lt;Proposal&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_ballot_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="Vote.md#0x1_Vote_CreateBallotEvent">Vote::CreateBallotEvent</a>&lt;Proposal&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>remove_ballot_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="Vote.md#0x1_Vote_RemoveBallotEvent">Vote::RemoveBallotEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>voted_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="Vote.md#0x1_Vote_VotedEvent">Vote::VotedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ballot_approved_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_EventHandle">Event::EventHandle</a>&lt;<a href="Vote.md#0x1_Vote_BallotApprovedEvent">Vote::BallotApprovedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<details>
<summary>Specification</summary>

All <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code>s of <code><a href="Vote.md#0x1_Vote_Ballot">Ballot</a></code>s in a <code><a href="Vote.md#0x1_Vote_Ballots">Ballots</a>.ballots</code> vector are unique.


<pre><code><b>invariant</b> <a href="Vote.md#0x1_Vote_unique_ballots">unique_ballots</a>(ballots);
</code></pre>


Asserts that ballot ID is not in ballots vector.  Used in loop invariant
and post-condition of remove_ballot_internal


<a name="0x1_Vote_ballot_id_does_not_exist"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_ballot_id_does_not_exist">ballot_id_does_not_exist</a>&lt;Proposal&gt;(ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>, ballots: vector&lt;<a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal&gt;&gt;, i: u64): bool {
   <b>forall</b> j in 0..i: ballots[j].ballot_id != ballot_id
}
</code></pre>



</details>

<a name="0x1_Vote_BallotCounter"></a>

## Resource `BallotCounter`

A counter which is stored under the proposers address and gets incremented
everytime they create a new ballot. This is used for creating unique
global identifiers for Ballots


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>counter: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_CreateBallotEvent"></a>

## Struct `CreateBallotEvent`

CreateBallotEvent is emitted when a ballot is
created by a proposer


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_CreateBallotEvent">CreateBallotEvent</a>&lt;Proposal: drop, store&gt; <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>ballot: <a href="Vote.md#0x1_Vote_Ballot">Vote::Ballot</a>&lt;Proposal&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_RemoveBallotEvent"></a>

## Struct `RemoveBallotEvent`

RemoveBallotEvent is emitted when a ballot has
been removed. This can either happen because:
* ballot was approved
* ballot was manually removed by the proposer
* ballot was expired and garbage collected


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_VotedEvent"></a>

## Struct `VotedEvent`

VotedEvent is emitted when a valid vote has
been accepted by the ballot


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_VotedEvent">VotedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a></code>
</dt>
<dd>

</dd>
<dt>
<code>voter: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>vote_weight: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_BallotApprovedEvent"></a>

## Struct `BallotApprovedEvent`

BallotApprovedEvent is emitted when a ballot has
been approved by the voters


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_BallotApprovedEvent">BallotApprovedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_Vote_EALREADY_VOTED"></a>

Voter has already voted in this ballot


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_EALREADY_VOTED">EALREADY_VOTED</a>: u64 = 8;
</code></pre>



<a name="0x1_Vote_EBALLOT_EXPIRED"></a>

current timestamp > ballot expiration time


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_EBALLOT_EXPIRED">EBALLOT_EXPIRED</a>: u64 = 7;
</code></pre>



<a name="0x1_Vote_EBALLOT_NOT_FOUND"></a>

Ballot with the provided id was not found


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_EBALLOT_NOT_FOUND">EBALLOT_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a name="0x1_Vote_EBALLOT_PROPOSAL_MISMATCH"></a>

Proposal details in the vote do not match the proposal details
in the ballot


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_EBALLOT_PROPOSAL_MISMATCH">EBALLOT_PROPOSAL_MISMATCH</a>: u64 = 4;
</code></pre>



<a name="0x1_Vote_EINVALID_NUM_VOTES"></a>

Num_votes must be greater than 0, so that election is not won when started


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_EINVALID_NUM_VOTES">EINVALID_NUM_VOTES</a>: u64 = 9;
</code></pre>



<a name="0x1_Vote_EINVALID_TIMESTAMP"></a>

The provided timestamp(s) were invalid


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>: u64 = 1;
</code></pre>



<a name="0x1_Vote_EINVALID_VOTER"></a>

Voter not allowed to vote in the ballot


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_EINVALID_VOTER">EINVALID_VOTER</a>: u64 = 5;
</code></pre>



<a name="0x1_Vote_ETOO_MANY_BALLOTS"></a>

The address already contains has the maximum of ballots allowed
MAX_BALLOTS_PER_PROPOSAL_TYPE_PER_ADDRESS


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_ETOO_MANY_BALLOTS">ETOO_MANY_BALLOTS</a>: u64 = 2;
</code></pre>



<a name="0x1_Vote_MAX_BALLOTS_PER_PROPOSAL_TYPE_PER_ADDRESS"></a>

The maximum number of ballots allowed per proposal type
per address.


<pre><code><b>const</b> <a href="Vote.md#0x1_Vote_MAX_BALLOTS_PER_PROPOSAL_TYPE_PER_ADDRESS">MAX_BALLOTS_PER_PROPOSAL_TYPE_PER_ADDRESS</a>: u64 = 256;
</code></pre>



<a name="0x1_Vote_new_ballot_id"></a>

## Function `new_ballot_id`

A constructor for BallotID


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_new_ballot_id">new_ballot_id</a>(counter: u64, proposer: <b>address</b>): <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_new_ballot_id">new_ballot_id</a>(
    counter: u64,
    proposer: <b>address</b>,
): <a href="Vote.md#0x1_Vote_BallotID">BallotID</a> {
    <a href="Vote.md#0x1_Vote_BallotID">BallotID</a> {
        counter,
        proposer,
    }
}
</code></pre>



</details>

<a name="0x1_Vote_new_weighted_voter"></a>

## Function `new_weighted_voter`

A constructor for WeightedVoter


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_new_weighted_voter">new_weighted_voter</a>(weight: u64, voter: vector&lt;u8&gt;): <a href="Vote.md#0x1_Vote_WeightedVoter">Vote::WeightedVoter</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_new_weighted_voter">new_weighted_voter</a>(
    weight: u64,
    voter: vector&lt;u8&gt;,
): <a href="Vote.md#0x1_Vote_WeightedVoter">WeightedVoter</a> {
    <a href="Vote.md#0x1_Vote_WeightedVoter">WeightedVoter</a> {
        weight,
        voter,
    }
}
</code></pre>



</details>

<a name="0x1_Vote_create_ballot"></a>

## Function `create_ballot`

Create a ballot under the signer's address and return the <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code>


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_create_ballot">create_ballot</a>&lt;Proposal: <b>copy</b>, drop, store&gt;(ballot_account: &signer, proposal: Proposal, proposal_type: vector&lt;u8&gt;, num_votes_required: u64, allowed_voters: vector&lt;<a href="Vote.md#0x1_Vote_WeightedVoter">Vote::WeightedVoter</a>&gt;, expiration_timestamp_secs: u64): <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_create_ballot">create_ballot</a>&lt;Proposal: store + <b>copy</b> + drop&gt;(
    ballot_account: &signer,
    proposal: Proposal,
    proposal_type: vector&lt;u8&gt;,
    num_votes_required: u64,
    allowed_voters: vector&lt;<a href="Vote.md#0x1_Vote_WeightedVoter">WeightedVoter</a>&gt;,
    expiration_timestamp_secs: u64
): <a href="Vote.md#0x1_Vote_BallotID">BallotID</a> <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>, <a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a> {
    <b>let</b> ballot_address = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(ballot_account);

    <b>assert</b>!(<a href="DiemTimestamp.md#0x1_DiemTimestamp_now_seconds">DiemTimestamp::now_seconds</a>() &lt; expiration_timestamp_secs, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Vote.md#0x1_Vote_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>));
    <b>assert</b>!(num_votes_required &gt; 0, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Vote.md#0x1_Vote_EINVALID_NUM_VOTES">EINVALID_NUM_VOTES</a>));

    <b>if</b> (!<b>exists</b>&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(ballot_address)) {
        <b>move_to</b>(ballot_account, <a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a> {
            counter: 0,
        });
    };
    <b>if</b> (!<b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address)) {
        <b>move_to</b>(ballot_account, <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt; {
            ballots: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>(),
            create_ballot_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_CreateBallotEvent">CreateBallotEvent</a>&lt;Proposal&gt;&gt;(ballot_account),
            remove_ballot_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a>&gt;(ballot_account),
            voted_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_VotedEvent">VotedEvent</a>&gt;(ballot_account),
            ballot_approved_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_BallotApprovedEvent">BallotApprovedEvent</a>&gt;(ballot_account),
        });
    };

    <b>let</b> ballot_data = <b>borrow_global_mut</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address);

    // Remove any expired ballots
    <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal&gt;(ballot_data);
    <b>let</b> ballots = &<b>mut</b> ballot_data.ballots;

    <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(ballots) &lt; <a href="Vote.md#0x1_Vote_MAX_BALLOTS_PER_PROPOSAL_TYPE_PER_ADDRESS">MAX_BALLOTS_PER_PROPOSAL_TYPE_PER_ADDRESS</a>, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_limit_exceeded">Errors::limit_exceeded</a>(<a href="Vote.md#0x1_Vote_ETOO_MANY_BALLOTS">ETOO_MANY_BALLOTS</a>));
    <b>let</b> ballot_id = <a href="Vote.md#0x1_Vote_new_ballot_id">new_ballot_id</a>(<a href="Vote.md#0x1_Vote_incr_counter">incr_counter</a>(ballot_account), ballot_address);
    <b>let</b> ballot = <a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal&gt; {
        proposal,
        proposal_type,
        num_votes_required,
        allowed_voters,
        votes_received: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>(),
        total_weighted_votes_received: 0,
        ballot_id: *&ballot_id,
        expiration_timestamp_secs,
    };
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_push_back">Vector::push_back</a>(ballots, *&ballot);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>&lt;<a href="Vote.md#0x1_Vote_CreateBallotEvent">CreateBallotEvent</a>&lt;Proposal&gt;&gt;(
        &<b>mut</b> ballot_data.create_ballot_handle,
        <a href="Vote.md#0x1_Vote_CreateBallotEvent">CreateBallotEvent</a> {
            ballot_id: *&ballot_id,
            ballot,
        },
    );
    ballot_id
}
</code></pre>



</details>

<details>
<summary>Specification</summary>


create_ballot sets up a <code><a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;</code> resource at the <code>ballot_account</code>
address if one does not already exist.


<pre><code><b>ensures</b> <b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(ballot_account));
</code></pre>


returns a new active <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code>.


<pre><code><b>ensures</b> <a href="Vote.md#0x1_Vote_is_active">is_active</a>&lt;Proposal&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(ballot_account), result);
</code></pre>


Returns "true" iff there are no ballots in v at indices less than i whose
expiration time is less than or equal to the current time.


<a name="0x1_Vote_no_expired_ballots"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_no_expired_ballots">no_expired_ballots</a>&lt;Proposal&gt;(ballots: vector&lt;<a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal&gt;&gt;, now_seconds: u64, i: u64): bool {
   <b>forall</b> j in 0..i: ballots[j].expiration_timestamp_secs &gt;= now_seconds
}
</code></pre>




<a name="0x1_Vote_extract_ballot_ids"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_extract_ballot_ids">extract_ballot_ids</a>&lt;Proposal&gt;(v: vector&lt;<a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal&gt;&gt;): vector&lt;<a href="Vote.md#0x1_Vote_BallotID">BallotID</a>&gt; {
   <b>choose</b> result: vector&lt;<a href="Vote.md#0x1_Vote_BallotID">BallotID</a>&gt; <b>where</b> len(result) == len(v)
   && (<b>forall</b> i in 0..len(v): result[i] == v[i].ballot_id)
}
</code></pre>


Common post-conditions for <code>gc_internal</code> and <code>gc_ballots</code> (which just calls <code>gc_internal</code>)


<a name="0x1_Vote_GcEnsures"></a>


<pre><code><b>schema</b> <a href="Vote.md#0x1_Vote_GcEnsures">GcEnsures</a>&lt;Proposal&gt; {
    ballot_data: <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;;
    <b>let</b> pre_ballots = ballot_data.ballots;
    <b>let</b> <b>post</b> post_ballots = ballot_data.ballots;
}
</code></pre>


Ballots afterwards is a subset of ballots before.


<pre><code><b>schema</b> <a href="Vote.md#0x1_Vote_GcEnsures">GcEnsures</a>&lt;Proposal&gt; {
    <b>ensures</b> <a href="Vote.md#0x1_Vote_vector_subset">vector_subset</a>(post_ballots, pre_ballots);
}
</code></pre>


All expired ballots are removed


<pre><code><b>schema</b> <a href="Vote.md#0x1_Vote_GcEnsures">GcEnsures</a>&lt;Proposal&gt; {
    <b>ensures</b> <a href="Vote.md#0x1_Vote_no_expired_ballots">no_expired_ballots</a>&lt;Proposal&gt;(post_ballots, <a href="DiemTimestamp.md#0x1_DiemTimestamp_spec_now_seconds">DiemTimestamp::spec_now_seconds</a>(), len(post_ballots));
}
</code></pre>



</details>

<a name="0x1_Vote_check_voter_present"></a>

## Function `check_voter_present`



<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_check_voter_present">check_voter_present</a>(weighted_voters: &vector&lt;<a href="Vote.md#0x1_Vote_WeightedVoter">Vote::WeightedVoter</a>&gt;, voter: &vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_check_voter_present">check_voter_present</a>(
    weighted_voters: &vector&lt;<a href="Vote.md#0x1_Vote_WeightedVoter">WeightedVoter</a>&gt;,
    voter: &vector&lt;u8&gt;,
): bool {
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(weighted_voters);
    <b>while</b> (i &lt; len) {
        <b>if</b> (&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(weighted_voters, i).voter == voter) <b>return</b> <b>true</b>;
        i = i + 1;
    };
    <b>false</b>
}
</code></pre>



</details>

<a name="0x1_Vote_vote"></a>

## Function `vote`

Submit a vote from the <code>voter_account</code> to <code>ballot_id</code>
This also contains the <code>proposal_type</code> and <code>proposal</code> so that
the voter signs over these when sending their vote.
If this vote causes the ballot to be approved, then the ballot
is removed from the proposers address.
This returns a bool indicating whether this vote moved the ballot
to an approved status. true represents that the ballot got approved
after this vote and false represents that the ballot has not been
approved after this vote


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_vote">vote</a>&lt;Proposal: drop, store&gt;(voter_account: &signer, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>, proposal_type: vector&lt;u8&gt;, proposal: Proposal): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_vote">vote</a>&lt;Proposal: store + drop&gt;(
    voter_account: &signer,
    ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>,
    proposal_type: vector&lt;u8&gt;,
    proposal: Proposal,
): bool <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a> {
    <b>let</b> ballot_data = <b>borrow_global_mut</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_id.proposer);

    // Remove any expired ballots
    <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal&gt;(ballot_data);

    <b>let</b> ballots = &<b>mut</b> ballot_data.ballots;
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(ballots);
    <b>while</b> (i &lt; len) {
        <b>if</b> (&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(ballots, i).ballot_id == &ballot_id) <b>break</b>;
        i = i + 1;
    };
    <b>assert</b>!(i &lt; len, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_state">Errors::invalid_state</a>(<a href="Vote.md#0x1_Vote_EBALLOT_NOT_FOUND">EBALLOT_NOT_FOUND</a>));
    <b>let</b> ballot_index = i;
    <b>let</b> ballot = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow_mut">Vector::borrow_mut</a>(ballots, ballot_index);

    <b>assert</b>!(&ballot.proposal == &proposal, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Vote.md#0x1_Vote_EBALLOT_PROPOSAL_MISMATCH">EBALLOT_PROPOSAL_MISMATCH</a>));
    <b>assert</b>!(&ballot.proposal_type == &proposal_type, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Vote.md#0x1_Vote_EBALLOT_PROPOSAL_MISMATCH">EBALLOT_PROPOSAL_MISMATCH</a>));

    <b>let</b> voter_address = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(voter_account);
    <b>let</b> voter_address_bcs = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/BCS.md#0x1_BCS_to_bytes">BCS::to_bytes</a>(&voter_address);
    <b>let</b> allowed_voters = &ballot.allowed_voters;

    <b>assert</b>!(<a href="Vote.md#0x1_Vote_check_voter_present">check_voter_present</a>(allowed_voters, &voter_address_bcs), <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_state">Errors::invalid_state</a>(<a href="Vote.md#0x1_Vote_EINVALID_VOTER">EINVALID_VOTER</a>));
    <b>assert</b>!(<a href="DiemTimestamp.md#0x1_DiemTimestamp_now_seconds">DiemTimestamp::now_seconds</a>() &lt;= ballot.expiration_timestamp_secs, <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_state">Errors::invalid_state</a>(<a href="Vote.md#0x1_Vote_EBALLOT_EXPIRED">EBALLOT_EXPIRED</a>));

    <b>assert</b>!(!<a href="Vote.md#0x1_Vote_check_voter_present">check_voter_present</a>(&ballot.votes_received, &voter_address_bcs), <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_state">Errors::invalid_state</a>(<a href="Vote.md#0x1_Vote_EALREADY_VOTED">EALREADY_VOTED</a>));

    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(allowed_voters);
    <b>while</b> (i &lt; len) {
        <b>let</b> weighted_voter = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(allowed_voters, i);
        <b>if</b> (&weighted_voter.voter == &voter_address_bcs) {
            <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_push_back">Vector::push_back</a>(&<b>mut</b> ballot.votes_received, *weighted_voter);
            ballot.total_weighted_votes_received = ballot.total_weighted_votes_received + weighted_voter.weight;
            <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>&lt;<a href="Vote.md#0x1_Vote_VotedEvent">VotedEvent</a>&gt;(
                &<b>mut</b> ballot_data.voted_handle,
                <a href="Vote.md#0x1_Vote_VotedEvent">VotedEvent</a> {
                    ballot_id: *&ballot_id,
                    voter: voter_address,
                    vote_weight: weighted_voter.weight,
                },
            );
            <b>break</b>
        };
        i = i + 1;
    };
    <b>let</b> ballot_approved = ballot.total_weighted_votes_received &gt;= ballot.num_votes_required;
    // If the ballot gets approved, remove the ballot immediately
    <b>if</b> (ballot_approved) {
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_swap_remove">Vector::swap_remove</a>(ballots, ballot_index);
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>&lt;<a href="Vote.md#0x1_Vote_BallotApprovedEvent">BallotApprovedEvent</a>&gt;(
            &<b>mut</b> ballot_data.ballot_approved_handle,
            <a href="Vote.md#0x1_Vote_BallotApprovedEvent">BallotApprovedEvent</a> {
                ballot_id,
            },
        );
    };
    ballot_approved
}
</code></pre>



</details>

<a name="0x1_Vote_gc_ballots"></a>

## Function `gc_ballots`

gc_ballots deletes all the expired ballots of the type <code>Proposal</code>
under the provided address <code>addr</code>. The signer can be anybody
and does not need to have the same address as <code>addr</code>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_ballots">gc_ballots</a>&lt;Proposal: drop, store&gt;(_signer: signer, addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_ballots">gc_ballots</a>&lt;Proposal: store + drop&gt;(
    _signer: signer,
    addr: <b>address</b>,
) <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a> {
    <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal&gt;(<b>borrow_global_mut</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(addr));
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>include</b> <a href="Vote.md#0x1_Vote_GcEnsures">GcEnsures</a>&lt;Proposal&gt;{ballot_data: <b>global</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(addr)};
</code></pre>




<a name="0x1_Vote_ballot_ids_have_correct_ballot_address"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_ballot_ids_have_correct_ballot_address">ballot_ids_have_correct_ballot_address</a>&lt;Proposal&gt;(proposer_address: <b>address</b>): bool {
  <b>let</b> ballots = <a href="Vote.md#0x1_Vote_get_ballots">get_ballots</a>&lt;Proposal&gt;(proposer_address);
  <b>forall</b> i in 0..len(ballots): ballots[i].ballot_id.proposer == proposer_address
}
</code></pre>


Every ballot for Proposal at proposer_address has a ballot counter field that is less
than the current value of the BallotCounter.counter published at proposer_address.
This property is necessary to show that the ballot IDs are not repeated in the
Ballots.ballots vector


<a name="0x1_Vote_existing_ballots_have_small_counters"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_existing_ballots_have_small_counters">existing_ballots_have_small_counters</a>&lt;Proposal&gt;(proposer_address: <b>address</b>): bool {
   // Just <b>return</b> <b>true</b> <b>if</b> there is no <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt; published at proposer_address
   // get_ballots may be undefined here, but we only <b>use</b> it when we know the <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>
   // is published (in the next property.
   <b>let</b> ballots = <a href="Vote.md#0x1_Vote_get_ballots">get_ballots</a>&lt;Proposal&gt;(proposer_address);
   <b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(proposer_address)
   ==&gt; (<b>forall</b> i in 0..len(ballots):
           ballots[i].ballot_id.counter &lt; <b>global</b>&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(proposer_address).counter)
}
</code></pre>


Every ballot in Ballots<Proposal>.ballots is active or expired.
I.e., none have sum >= required.
TODO: This should be part of is_active/expired, and should follow from an invariant
that every BallotID is in one of the legal states.


<a name="0x1_Vote_no_winning_ballots_in_vector"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_no_winning_ballots_in_vector">no_winning_ballots_in_vector</a>&lt;Proposal&gt;(proposer_address: <b>address</b>): bool {
   <b>let</b> ballots = <a href="Vote.md#0x1_Vote_get_ballots">get_ballots</a>&lt;Proposal&gt;(proposer_address);
   <b>forall</b> i in 0..len(ballots):
       ballots[i].total_weighted_votes_received &lt; ballots[i].num_votes_required
}
</code></pre>



ballots in vector all have the proposer address in their ballot IDs.


<pre><code><b>invariant</b>&lt;Proposal&gt; [suspendable] <b>forall</b> proposer_address: <b>address</b>:
    <a href="Vote.md#0x1_Vote_ballot_ids_have_correct_ballot_address">ballot_ids_have_correct_ballot_address</a>&lt;Proposal&gt;(proposer_address);
<b>invariant</b>&lt;Proposal&gt;
    (<b>forall</b> addr: <b>address</b>: <a href="Vote.md#0x1_Vote_existing_ballots_have_small_counters">existing_ballots_have_small_counters</a>&lt;Proposal&gt;(addr))
    && (<b>forall</b> ballot_addr: <b>address</b>: <a href="Vote.md#0x1_Vote_ballot_counter_initialized_first">ballot_counter_initialized_first</a>&lt;Proposal&gt;(ballot_addr));
</code></pre>


Every ballot in the vector has total_weighted_votes_received < num_votes_required
So the ballot will eventually be removed either by accumulating enough votes or by expiring
and being garbage-collected


<pre><code><b>invariant</b>&lt;Proposal&gt; <b>forall</b> addr: <b>address</b>: <a href="Vote.md#0x1_Vote_no_winning_ballots_in_vector">no_winning_ballots_in_vector</a>&lt;Proposal&gt;(addr);
</code></pre>


There are no duplicate Ballot IDs in the Ballots<Proposer>.ballots vector


<a name="0x1_Vote_unique_ballots"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_unique_ballots">unique_ballots</a>&lt;Proposal&gt;(ballots: vector&lt;<a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal&gt;&gt;): bool {
   <b>forall</b> i in 0..len(ballots), j in 0..len(ballots):
       ballots[i].ballot_id == ballots[j].ballot_id ==&gt; i == j
}
</code></pre>



</details>

<a name="0x1_Vote_gc_test_helper"></a>

## Function `gc_test_helper`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_test_helper">gc_test_helper</a>&lt;Proposal: drop, store&gt;(addr: <b>address</b>): vector&lt;<a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_test_helper">gc_test_helper</a>&lt;Proposal: store + drop&gt;(
    addr: <b>address</b>,
): vector&lt;<a href="Vote.md#0x1_Vote_BallotID">BallotID</a>&gt;  <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a> {
    <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal&gt;(<b>borrow_global_mut</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(addr))
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a name="0x1_Vote_vector_subset"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_vector_subset">vector_subset</a>&lt;Elt&gt;(v1: vector&lt;Elt&gt;, v2: vector&lt;Elt&gt;): bool {
   <b>forall</b> e in v1: <b>exists</b> i in 0..len(v2): v2[i] == e
}
</code></pre>



</details>

<a name="0x1_Vote_gc_internal"></a>

## Function `gc_internal`



<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal: drop, store&gt;(ballot_data: &<b>mut</b> <a href="Vote.md#0x1_Vote_Ballots">Vote::Ballots</a>&lt;Proposal&gt;): vector&lt;<a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal: store + drop&gt;(
    ballot_data: &<b>mut</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;,
): vector&lt;<a href="Vote.md#0x1_Vote_BallotID">BallotID</a>&gt; {
    <b>let</b> ballots = &<b>mut</b> ballot_data.ballots;
    <b>let</b> remove_handle = &<b>mut</b> ballot_data.remove_ballot_handle;
    <b>let</b> i = 0;
    <b>let</b> removed_ballots = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>();
    <b>while</b> ({
        <b>spec</b> {
            <b>invariant</b> <a href="Vote.md#0x1_Vote_no_expired_ballots">no_expired_ballots</a>(ballots, <a href="DiemTimestamp.md#0x1_DiemTimestamp_spec_now_seconds">DiemTimestamp::spec_now_seconds</a>(), i);
            <b>invariant</b> <a href="Vote.md#0x1_Vote_vector_subset">vector_subset</a>(ballots, <b>old</b>(ballot_data).ballots);
            <b>invariant</b> i &lt;= len(ballots);
            <b>invariant</b> 0 &lt;= i;
        };
        i &lt; <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(ballots)
    }) {
        <b>let</b> ballot = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(ballots, i);
        <b>if</b> (ballot.expiration_timestamp_secs &lt; <a href="DiemTimestamp.md#0x1_DiemTimestamp_now_seconds">DiemTimestamp::now_seconds</a>()) {
            <b>let</b> ballot_id = *(&ballot.ballot_id);
            <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_swap_remove">Vector::swap_remove</a>(ballots, i);
            <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_push_back">Vector::push_back</a>(&<b>mut</b> removed_ballots, *&ballot_id);
            <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>&lt;<a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a>&gt;(
                remove_handle,
                <a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a> {
                    ballot_id
                },
            );
        } <b>else</b> {
            i = i + 1;
        };
    };
    removed_ballots
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque;
<b>include</b> <a href="Vote.md#0x1_Vote_GcEnsures">GcEnsures</a>&lt;Proposal&gt;;
</code></pre>



</details>

<a name="0x1_Vote_remove_ballot_internal"></a>

## Function `remove_ballot_internal`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_remove_ballot_internal">remove_ballot_internal</a>&lt;Proposal: drop, store&gt;(account: signer, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_remove_ballot_internal">remove_ballot_internal</a>&lt;Proposal: store + drop&gt;(
    account: signer,
    ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>,
) <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a> {
    <b>let</b> addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(&account);
    <b>let</b> ballot_data = <b>borrow_global_mut</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(addr);
    <b>let</b> ballots = &<b>mut</b> ballot_data.ballots;
    <b>let</b> remove_handle = &<b>mut</b> ballot_data.remove_ballot_handle;
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(ballots);
    <b>while</b> ({
        <b>spec</b> { <b>invariant</b> <a href="Vote.md#0x1_Vote_ballot_id_does_not_exist">ballot_id_does_not_exist</a>&lt;Proposal&gt;(ballot_id, ballots, i); };
        i &lt; len
    }) {
        <b>if</b> (&<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(ballots, i).ballot_id == &ballot_id) {
            <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_swap_remove">Vector::swap_remove</a>(ballots, i);
            <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_emit_event">Event::emit_event</a>&lt;<a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a>&gt;(
                remove_handle,
                <a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a> {
                    ballot_id
                },
            );
            <b>return</b> ()
        };
        i = i + 1;
    };
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>let</b> <b>post</b> ballots = <a href="Vote.md#0x1_Vote_get_ballots">get_ballots</a>&lt;Proposal&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account));
<b>ensures</b>
    <a href="Vote.md#0x1_Vote_ballot_id_does_not_exist">ballot_id_does_not_exist</a>&lt;Proposal&gt;(ballot_id, ballots, len(ballots));
</code></pre>



</details>

<a name="0x1_Vote_remove_ballot"></a>

## Function `remove_ballot`

remove_ballot removes the ballot with the provided
<code>ballot_id</code> from the address of the provided <code>account</code>
If a ballot with <code>ballot_id</code> is not found, then it
does nothing


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_remove_ballot">remove_ballot</a>&lt;Proposal: drop, store&gt;(account: signer, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_remove_ballot">remove_ballot</a>&lt;Proposal: store + drop&gt;(
    account: signer,
    ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>,
) <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a> {
    <a href="Vote.md#0x1_Vote_remove_ballot_internal">remove_ballot_internal</a>&lt;Proposal&gt;(account, ballot_id)
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>let</b> <b>post</b> ballots = <a href="Vote.md#0x1_Vote_get_ballots">get_ballots</a>&lt;Proposal&gt;(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account));
<b>ensures</b>
    <a href="Vote.md#0x1_Vote_ballot_id_does_not_exist">ballot_id_does_not_exist</a>&lt;Proposal&gt;(ballot_id, ballots, len(ballots));
</code></pre>



</details>

<a name="0x1_Vote_incr_counter"></a>

## Function `incr_counter`

incr_counter increments the counter stored under the signer's
account


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_incr_counter">incr_counter</a>(account: &signer): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_incr_counter">incr_counter</a>(account: &signer): u64 <b>acquires</b> <a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a> {
    <b>let</b> addr = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(account);
    <b>let</b> counter = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(addr).counter;
    <b>let</b> count = *counter;
    *counter = *counter + 1;
    count
}
</code></pre>



</details>

<a name="@Module_Specification_1"></a>

## Module Specification

****************************************************************
Specs
****************************************************************
I (DD) was experimenting with some new ideas about top-down specification.
This is a partial specification, but it does have some interesting properties
and is good for testing the Prover.
A "Ballot" keeps track of an election for a "Proposal" type at a particular address.
To conduct an election at a particular address, there must be a BallotCounter
published at that address.  This keeps track of a counter that is used to generate
unique BallotIDs.

Once the BallotCounter is published, it remains published forever


<pre><code><b>invariant</b> <b>update</b> <b>forall</b> ballot_addr: <b>address</b> <b>where</b> <b>old</b>(<b>exists</b>&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(ballot_addr)):
    <b>exists</b>&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(ballot_addr);
</code></pre>


Once a proposal is initialized, it stays initialized forever.


<pre><code><b>invariant</b>&lt;Proposal&gt; <b>update</b> <b>forall</b> ballot_addr: <b>address</b>
    <b>where</b> <b>old</b>(<b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_addr)):
        <b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_addr);
</code></pre>


Predicate to test if a <code><a href="Vote.md#0x1_Vote_Ballots">Ballots</a></code> resource for <code>Proposal</code> is published at <code>ballot_addr</code>,
there is a <code><a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a></code> published at <code>ballot_addr</code>.


<a name="0x1_Vote_ballot_counter_initialized_first"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_ballot_counter_initialized_first">ballot_counter_initialized_first</a>&lt;Proposal&gt;(ballot_addr: <b>address</b>): bool {
   <b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_addr) ==&gt; <b>exists</b>&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(ballot_addr)
}
</code></pre>



Get the ballots vector from published Ballots<Proposal>
CAUTION: Returns an arbitrary value if no Ballots<Proposal> is publised at ballot_address.


<a name="0x1_Vote_get_ballots"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_get_ballots">get_ballots</a>&lt;Proposal&gt;(ballot_address: <b>address</b>): vector&lt;<a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal&gt;&gt; {
  <b>global</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address).ballots
}
</code></pre>


Get the ballot matching ballot_id out of the ballots vector, if it is there.
CAUTION: Returns a arbitrary value if it's not there.


<a name="0x1_Vote_get_ballot"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_get_ballot">get_ballot</a>&lt;Proposal&gt;(ballot_address: <b>address</b>, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>): <a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal&gt; {
    <b>let</b> ballots = <b>global</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address).ballots;
    <a href="Vote.md#0x1_Vote_get_ballots">get_ballots</a>&lt;Proposal&gt;(ballot_address)[<b>choose</b> <b>min</b> i in 0..len(ballots) <b>where</b> ballots[i].ballot_id == ballot_id]
}
</code></pre>


Tests whether ballot_id is represented in the ballots vector. Returns false if there is no
ballots vector.


<a name="0x1_Vote_ballot_exists"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_ballot_exists">ballot_exists</a>&lt;Proposal&gt;(ballot_address: <b>address</b>, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>): bool {
   <b>if</b> (<b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address)) {
       <b>let</b> ballots = <b>global</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address).ballots;
       <b>exists</b> i in 0..len(ballots): ballots[i].ballot_id == ballot_id
   }
   <b>else</b>
       <b>false</b>
}
</code></pre>


Assuming ballot exists, check if it's expired. Returns an arbitrary result if the
ballot does not exist.
NOTE: Maybe this should be "<=" not "<"


<a name="0x1_Vote_is_expired_if_exists"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_is_expired_if_exists">is_expired_if_exists</a>&lt;Proposal&gt;(ballot_address: <b>address</b>, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>): bool {
   <a href="Vote.md#0x1_Vote_get_ballot">get_ballot</a>&lt;Proposal&gt;(ballot_address, ballot_id).expiration_timestamp_secs
       &lt;= <a href="DiemTimestamp.md#0x1_DiemTimestamp_spec_now_seconds">DiemTimestamp::spec_now_seconds</a>()
}
</code></pre>


There is a state machine for every <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a>&lt;Proposal&gt;</code>.  Two of the states don't
need to appear in formal specifications, but they are here for completeness.
State: unborn -- The <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code> has a count that is greater than the count in BallotCounter.
So the <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code> has not yet been generated, and may be generated in the future.
It is not in use, so we won't see values in this state.
A <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code> in the unborn state may transition to the active state if <code>create_ballot</code>
generates that <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code>.
State: dead -- The <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code> was generated and then was either and accepted (when
a call to <code>vote</code> causes the vote total to exceed the threshold),
or expired and garbage collected.  So, it is no longer in use and we won't see these
values.  Note that garbage collection occurs in several functions.
State active -- The <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code> is in a <code><a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;.ballots</code> vector for some Proposal and
address.  It is not expired and may eventually be accepted.
Active BallotIDs are created and returned by <code>create_ballot</code>.
An active <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code> may transition to the expired state if it is not accepted and the current
time exceeds its expiration time, or it may transition to the dead state if it expires and is
garbage-collected in the same transaction or if it is accepted.
State expired -- The <code><a href="Vote.md#0x1_Vote_BallotID">BallotID</a></code> is in a <code><a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;.ballots</code> vector for some Proposal and
address but is expired.  It will be removed from the ballots vector and change to the dead state
if and when it is garbage-collected.
A BallotID is in the expired state if it is in the ballots vector and the
current time is >= the expiration time.


<a name="0x1_Vote_is_expired"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_is_expired">is_expired</a>&lt;Proposal&gt;(ballot_address: <b>address</b>, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>): bool {
   <a href="Vote.md#0x1_Vote_ballot_exists">ballot_exists</a>&lt;Proposal&gt;(ballot_address, ballot_id)
   && <a href="Vote.md#0x1_Vote_is_expired_if_exists">is_expired_if_exists</a>&lt;Proposal&gt;(ballot_address, ballot_id)
}
</code></pre>


A BallotID is active state if it is in the ballots vector and not expired.


<a name="0x1_Vote_is_active"></a>


<pre><code><b>fun</b> <a href="Vote.md#0x1_Vote_is_active">is_active</a>&lt;Proposal&gt;(ballot_address: <b>address</b>, ballot_id: <a href="Vote.md#0x1_Vote_BallotID">BallotID</a>): bool {
  <a href="Vote.md#0x1_Vote_ballot_exists">ballot_exists</a>&lt;Proposal&gt;(ballot_address, ballot_id)
  && !<a href="Vote.md#0x1_Vote_is_expired_if_exists">is_expired_if_exists</a>&lt;Proposal&gt;(ballot_address, ballot_id)
}
</code></pre>
