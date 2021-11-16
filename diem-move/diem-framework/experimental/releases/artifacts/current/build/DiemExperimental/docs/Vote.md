
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


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_BallotID">BallotID</a> has <b>copy</b>, drop, store
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
<code>proposer: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_Vote_WeightedVoter"></a>

## Struct `WeightedVoter`

WeightedVoter represents a voter with a weight
The voter is represented by the bcs serialization of address


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_WeightedVoter">WeightedVoter</a> has <b>copy</b>, drop, store
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


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_Ballot">Ballot</a>&lt;Proposal: drop, store&gt; has <b>copy</b>, drop, store
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


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal: drop, store&gt; has key
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

<a name="0x1_Vote_BallotCounter"></a>

## Resource `BallotCounter`

A counter which is stored under the proposers address and gets incremented
everytime they create a new ballot. This is used for creating unique
global identifiers for Ballots


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a> has key
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


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_CreateBallotEvent">CreateBallotEvent</a>&lt;Proposal: drop, store&gt; has drop, store
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


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a> has drop, store
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


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_VotedEvent">VotedEvent</a> has drop, store
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
<code>voter: address</code>
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


<pre><code><b>struct</b> <a href="Vote.md#0x1_Vote_BallotApprovedEvent">BallotApprovedEvent</a> has drop, store
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


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_new_ballot_id">new_ballot_id</a>(counter: u64, proposer: address): <a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Vote.md#0x1_Vote_new_ballot_id">new_ballot_id</a>(
    counter: u64,
    proposer: address,
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

    <b>if</b> (!<b>exists</b>&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(ballot_address)) {
        move_to(ballot_account, <a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a> {
            counter: 0,
        });
    };
    <b>if</b> (!<b>exists</b>&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address)) {
        move_to(ballot_account, <a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt; {
            ballots: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>(),
            create_ballot_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_CreateBallotEvent">CreateBallotEvent</a>&lt;Proposal&gt;&gt;(ballot_account),
            remove_ballot_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_RemoveBallotEvent">RemoveBallotEvent</a>&gt;(ballot_account),
            voted_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_VotedEvent">VotedEvent</a>&gt;(ballot_account),
            ballot_approved_handle: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;<a href="Vote.md#0x1_Vote_BallotApprovedEvent">BallotApprovedEvent</a>&gt;(ballot_account),
        });
    };

    <b>let</b> ballot_data = borrow_global_mut&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_address);

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
    <b>let</b> ballot_data = borrow_global_mut&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(ballot_id.proposer);

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


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_ballots">gc_ballots</a>&lt;Proposal: drop, store&gt;(_signer: signer, addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_ballots">gc_ballots</a>&lt;Proposal: store + drop&gt;(
    _signer: signer,
    addr: address,
) <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a> {
    <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal&gt;(borrow_global_mut&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(addr));
}
</code></pre>



</details>

<a name="0x1_Vote_gc_test_helper"></a>

## Function `gc_test_helper`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_test_helper">gc_test_helper</a>&lt;Proposal: drop, store&gt;(addr: address): vector&lt;<a href="Vote.md#0x1_Vote_BallotID">Vote::BallotID</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Vote.md#0x1_Vote_gc_test_helper">gc_test_helper</a>&lt;Proposal: store + drop&gt;(
    addr: address,
): vector&lt;<a href="Vote.md#0x1_Vote_BallotID">BallotID</a>&gt;  <b>acquires</b> <a href="Vote.md#0x1_Vote_Ballots">Ballots</a> {
    <a href="Vote.md#0x1_Vote_gc_internal">gc_internal</a>&lt;Proposal&gt;(borrow_global_mut&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(addr))
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
    <b>while</b> (i &lt; <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(ballots)) {
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
    <b>let</b> ballot_data = borrow_global_mut&lt;<a href="Vote.md#0x1_Vote_Ballots">Ballots</a>&lt;Proposal&gt;&gt;(addr);
    <b>let</b> ballots = &<b>mut</b> ballot_data.ballots;
    <b>let</b> remove_handle = &<b>mut</b> ballot_data.remove_ballot_handle;
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(ballots);
    <b>while</b> (i &lt; len) {
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
    <b>let</b> counter = &<b>mut</b> borrow_global_mut&lt;<a href="Vote.md#0x1_Vote_BallotCounter">BallotCounter</a>&gt;(addr).counter;
    <b>let</b> count = *counter;
    *counter = *counter + 1;
    count
}
</code></pre>



</details>
