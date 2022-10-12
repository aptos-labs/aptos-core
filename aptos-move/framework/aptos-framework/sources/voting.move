/**
 * This is the general Voting module that can be used as part of a DAO Governance. Voting is designed to be used by
 * standalone governance modules, who has full control over the voting flow and is responsible for voting power
 * calculation and including proper capabilities when creating the proposal so resolution can go through.
 * On-chain governance of the Aptos network also uses Voting.
 *
 * The voting flow:
 * 1. The Voting module can be deployed at a known address (e.g. 0x1 for Aptos on-chain governance)
 * 2. The governance module, e.g. AptosGovernance, can be deployed later and define a GovernanceProposal resource type
 * that can also contain other information such as Capability resource for authorization.
 * 3. The governance module's owner can then register the ProposalType with Voting. This also hosts the proposal list
 * (forum) on the calling account.
 * 4. A proposer, through the governance module, can call Voting::create_proposal to create a proposal. create_proposal
 * cannot be called directly not through the governance module. A script hash of the resolution script that can later
 * be called to execute the proposal is required.
 * 5. A voter, through the governance module, can call Voting::vote on a proposal. vote requires passing a &ProposalType
 * and thus only the governance module that registers ProposalType can call vote.
 * 6. Once the proposal's expiration time has passed and more than the defined threshold has voted yes on the proposal,
 * anyone can call resolve which returns the content of the proposal (of type ProposalType) that can be used to execute.
 * 7. Only the resolution script with the same script hash specified in the proposal can call Voting::resolve as part of
 * the resolution process.
 */
module aptos_framework::voting {
    use std::bcs::to_bytes;

    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{String, utf8};
    use std::vector;

    use aptos_std::from_bcs::to_u64;
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info::{Self, TypeInfo};

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_framework::transaction_context;

    /// Current script's execution hash does not match the specified proposal's
    const EPROPOSAL_EXECUTION_HASH_NOT_MATCHING: u64 = 1;
    /// Proposal cannot be resolved. Either voting duration has not passed, not enough votes, or fewer yes than no votes
    const EPROPOSAL_CANNOT_BE_RESOLVED: u64 = 2;
    /// Proposal cannot be resolved more than once
    const EPROPOSAL_ALREADY_RESOLVED: u64 = 3;
    /// Proposal cannot contain an empty execution script hash
    const EPROPOSAL_EMPTY_EXECUTION_HASH: u64 = 4;
    /// Proposal's voting period has already ended.
    const EPROPOSAL_VOTING_ALREADY_ENDED: u64 = 5;
    /// Voting forum has already been registered.
    const EVOTING_FORUM_ALREADY_REGISTERED: u64 = 6;
    /// Minimum vote threshold cannot be higher than early resolution threshold.
    const EINVALID_MIN_VOTE_THRESHOLD: u64 = 7;
    /// Resolution of a proposal cannot happen atomically in the same transaction as the last vote.
    const ERESOLUTION_CANNOT_BE_ATOMIC: u64 = 8;

    /// ProposalStateEnum representing proposal state.
    const PROPOSAL_STATE_PENDING: u64 = 0;
    const PROPOSAL_STATE_SUCCEEDED: u64 = 1;
    /// Proposal has failed because either the min vote threshold is not met or majority voted no.
    const PROPOSAL_STATE_FAILED: u64 = 3;

    /// Key used to track the resolvable time in the proposal's metadata.
    const RESOLVABLE_TIME_METADATA_KEY: vector<u8> = b"RESOLVABLE_TIME_METADATA_KEY";

    /// Extra metadata (e.g. description, code url) can be part of the ProposalType struct.
    struct Proposal<ProposalType: store> has store {
        /// Required. The address of the proposer.
        proposer: address,

        /// Required. Should contain enough information to execute later, for example the required capability.
        /// This is stored as an option so we can return it to governance when the proposal is resolved.
        execution_content: Option<ProposalType>,

        /// Optional. Extra metadata about the proposal and can be empty.
        /// Value is serialized value of an attribute.
        metadata: SimpleMap<String, vector<u8>>,

        /// Timestamp when the proposal was created.
        creation_time_secs: u64,

        /// Required. The hash for the execution script module. Only the same exact script module can resolve this
        /// proposal.
        execution_hash: vector<u8>,

        /// A proposal is only resolved if expiration has passed and the number of votes is above threshold.
        min_vote_threshold: u128,
        expiration_secs: u64,

        /// Optional. Early resolution threshold. If specified, the proposal can be resolved early if the total
        /// number of yes or no votes passes this threshold.
        /// For example, this can be set to 50% of the total supply of the voting token, so if > 50% vote yes or no,
        /// the proposal can be resolved before expiration.
        early_resolution_vote_threshold: Option<u128>,

        /// Number of votes for each outcome.
        /// u128 since the voting power is already u64 and can add up to more than u64 can hold.
        yes_votes: u128,
        no_votes: u128,

        /// Whether the proposal has been resolved.
        is_resolved: bool,
        /// Resolution timestamp if the proposal has been resolved. 0 otherwise.
        resolution_time_secs: u64,
    }

    struct VotingForum<ProposalType: store> has key {
        /// Use Table for execution optimization instead of Vector for gas cost since Vector is read entirely into memory
        /// during execution while only relevant Table entries are.
        proposals: Table<u64, Proposal<ProposalType>>,
        events: VotingEvents,
        /// Unique identifier for a proposal. This allows for 2 * 10**19 proposals.
        next_proposal_id: u64,
    }

    struct VotingEvents has store {
        create_proposal_events: EventHandle<CreateProposalEvent>,
        register_forum_events: EventHandle<RegisterForumEvent>,
        resolve_proposal_events: EventHandle<ResolveProposal>,
        vote_events: EventHandle<VoteEvent>,
    }

    struct CreateProposalEvent has drop, store {
        proposal_id: u64,
        early_resolution_vote_threshold: Option<u128>,
        execution_hash: vector<u8>,
        expiration_secs: u64,
        metadata: SimpleMap<String, vector<u8>>,
        min_vote_threshold: u128,
    }

    struct RegisterForumEvent has drop, store {
        hosting_account: address,
        proposal_type_info: TypeInfo,
    }

    struct VoteEvent has drop, store {
        proposal_id: u64,
        num_votes: u64,
    }

    struct ResolveProposal has drop, store {
        proposal_id: u64,
        yes_votes: u128,
        no_votes: u128,
        resolved_early: bool
    }

    public fun register<ProposalType: store>(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<VotingForum<ProposalType>>(addr), error::already_exists(EVOTING_FORUM_ALREADY_REGISTERED));

        let voting_forum = VotingForum<ProposalType> {
            next_proposal_id: 0,
            proposals: table::new<u64, Proposal<ProposalType>>(),
            events: VotingEvents {
                create_proposal_events: account::new_event_handle<CreateProposalEvent>(account),
                register_forum_events: account::new_event_handle<RegisterForumEvent>(account),
                resolve_proposal_events: account::new_event_handle<ResolveProposal>(account),
                vote_events: account::new_event_handle<VoteEvent>(account),
            }
        };

        event::emit_event<RegisterForumEvent>(
            &mut voting_forum.events.register_forum_events,
            RegisterForumEvent {
                hosting_account: addr,
                proposal_type_info: type_info::type_of<ProposalType>(),
            },
        );

        move_to(account, voting_forum);
    }

    /// Create a proposal with the given parameters
    ///
    /// @param voting_forum_address The forum's address where the proposal will be stored.
    /// @param execution_content The execution content that will be given back at resolution time. This can contain
    /// data such as a capability resource used to scope the execution.
    /// @param execution_hash The hash for the execution script module. Only the same exact script module can resolve
    /// this proposal.
    /// @param min_vote_threshold The minimum number of votes needed to consider this proposal successful.
    /// @param expiration_secs The time in seconds at which the proposal expires and can potentially be resolved.
    /// @return The proposal id.
    public fun create_proposal<ProposalType: store>(
        proposer: address,
        voting_forum_address: address,
        execution_content: ProposalType,
        execution_hash: vector<u8>,
        min_vote_threshold: u128,
        expiration_secs: u64,
        early_resolution_vote_threshold: Option<u128>,
        metadata: SimpleMap<String, vector<u8>>,
    ): u64 acquires VotingForum {
        if (option::is_some(&early_resolution_vote_threshold)) {
            assert!(
                min_vote_threshold <= *option::borrow(&early_resolution_vote_threshold),
                error::invalid_argument(EINVALID_MIN_VOTE_THRESHOLD),
            );
        };
        // Make sure the execution script's hash is not empty.
        assert!(vector::length(&execution_hash) > 0, error::invalid_argument(EPROPOSAL_EMPTY_EXECUTION_HASH));

        let voting_forum = borrow_global_mut<VotingForum<ProposalType>>(voting_forum_address);
        let proposal_id = voting_forum.next_proposal_id;
        voting_forum.next_proposal_id = voting_forum.next_proposal_id + 1;

        table::add(&mut voting_forum.proposals, proposal_id, Proposal {
            proposer,
            creation_time_secs: timestamp::now_seconds(),
            execution_content: option::some<ProposalType>(execution_content),
            execution_hash,
            metadata,
            min_vote_threshold,
            expiration_secs,
            early_resolution_vote_threshold,
            yes_votes: 0,
            no_votes: 0,
            is_resolved: false,
            resolution_time_secs: 0,
        });

        event::emit_event<CreateProposalEvent>(
            &mut voting_forum.events.create_proposal_events,
            CreateProposalEvent {
                proposal_id,
                early_resolution_vote_threshold,
                execution_hash,
                expiration_secs,
                metadata,
                min_vote_threshold,
            },
        );

        proposal_id
    }

    /// Vote on the given proposal.
    ///
    /// @param _proof Required so only the governance module that defines ProposalType can initiate voting.
    ///               This guarantees that voting eligibility and voting power are controlled by the right governance.
    /// @param voting_forum_address The address of the forum where the proposals are stored.
    /// @param proposal_id The proposal id.
    /// @param num_votes Number of votes. Voting power should be calculated by governance.
    /// @param should_pass Whether the votes are for yes or no.
    public fun vote<ProposalType: store>(
        _proof: &ProposalType,
        voting_forum_address: address,
        proposal_id: u64,
        num_votes: u64,
        should_pass: bool,
    ) acquires VotingForum {
        let voting_forum = borrow_global_mut<VotingForum<ProposalType>>(voting_forum_address);
        let proposal = table::borrow_mut(&mut voting_forum.proposals, proposal_id);
        // Voting might still be possible after the proposal has enough yes votes to be resolved early. This would only
        // lead to possible proposal resolution failure if the resolve early threshold is not definitive (e.g. < 50% + 1
        // of the total voting token's supply). In this case, more voting might actually still be desirable.
        // Governance mechanisms built on this voting module can apply additional rules on when voting is closed as
        // appropriate.
        assert!(!is_voting_period_over(proposal), error::invalid_state(EPROPOSAL_VOTING_ALREADY_ENDED));
        assert!(!proposal.is_resolved, error::invalid_state(EPROPOSAL_ALREADY_RESOLVED));

        if (should_pass) {
            proposal.yes_votes = proposal.yes_votes + (num_votes as u128);
        } else {
            proposal.no_votes = proposal.no_votes + (num_votes as u128);
        };

        // Record the resolvable time to ensure that resolution has to be done non-atomically.
        let timestamp_secs_bytes = to_bytes(&timestamp::now_seconds());
        let key = utf8(RESOLVABLE_TIME_METADATA_KEY);
        if (simple_map::contains_key(&proposal.metadata, &key)) {
            *simple_map::borrow_mut(&mut proposal.metadata, &key) = timestamp_secs_bytes;
        } else {
            simple_map::add(&mut proposal.metadata, key, timestamp_secs_bytes);
        };

        event::emit_event<VoteEvent>(
            &mut voting_forum.events.vote_events,
            VoteEvent { proposal_id, num_votes },
        );
    }

    /// Resolve the proposal with given id. Can only be done if there are at least as many votes as min required and
    /// there are more yes votes than no. If either of these conditions is not met, this will revert.
    ///
    /// @param voting_forum_address The address of the forum where the proposals are stored.
    /// @param proposal_id The proposal id.
    public fun resolve<ProposalType: store>(
        voting_forum_address: address,
        proposal_id: u64,
    ): ProposalType acquires VotingForum {
        let proposal_state = get_proposal_state<ProposalType>(voting_forum_address, proposal_id);
        assert!(proposal_state == PROPOSAL_STATE_SUCCEEDED, error::invalid_state(EPROPOSAL_CANNOT_BE_RESOLVED));

        let voting_forum = borrow_global_mut<VotingForum<ProposalType>>(voting_forum_address);
        let proposal = table::borrow_mut(&mut voting_forum.proposals, proposal_id);
        assert!(!proposal.is_resolved, error::invalid_state(EPROPOSAL_ALREADY_RESOLVED));

        // We need to make sure that the resolution is happening in
        // a separate transaction from the last vote to guard against any potential flashloan attacks.
        let resolvable_time = to_u64(*simple_map::borrow(&proposal.metadata, &utf8(RESOLVABLE_TIME_METADATA_KEY)));
        assert!(timestamp::now_seconds() > resolvable_time, error::invalid_state(ERESOLUTION_CANNOT_BE_ATOMIC));

        let resolved_early = can_be_resolved_early(proposal);
        proposal.is_resolved = true;
        proposal.resolution_time_secs = timestamp::now_seconds();

        assert!(
            transaction_context::get_script_hash() == proposal.execution_hash,
            error::invalid_argument(EPROPOSAL_EXECUTION_HASH_NOT_MATCHING),
        );

        event::emit_event<ResolveProposal>(
            &mut voting_forum.events.resolve_proposal_events,
            ResolveProposal {
                proposal_id,
                yes_votes: proposal.yes_votes,
                no_votes: proposal.no_votes,
                resolved_early,
            },
        );

        option::extract(&mut proposal.execution_content)
    }

    public fun is_voting_closed<ProposalType: store>(voting_forum_address: address, proposal_id: u64): bool acquires VotingForum {
        let voting_forum = borrow_global_mut<VotingForum<ProposalType>>(voting_forum_address);
        let proposal = table::borrow_mut(&mut voting_forum.proposals, proposal_id);
        can_be_resolved_early(proposal) || is_voting_period_over(proposal)
    }

    /// Return true if the proposal has reached early resolution threshold (if specified).
    public fun can_be_resolved_early<ProposalType: store>(proposal: &Proposal<ProposalType>): bool {
        if (option::is_some(&proposal.early_resolution_vote_threshold)) {
            let early_resolution_threshold = *option::borrow(&proposal.early_resolution_vote_threshold);
            if (proposal.yes_votes >= early_resolution_threshold || proposal.no_votes >= early_resolution_threshold) {
                return true
            };
        };
        false
    }

    /// Return the state of the proposal with given id.
    ///
    /// @param voting_forum_address The address of the forum where the proposals are stored.
    /// @param proposal_id The proposal id.
    /// @return Proposal state as an enum value.
    public fun get_proposal_state<ProposalType: store>(
        voting_forum_address: address,
        proposal_id: u64,
    ): u64 acquires VotingForum {
        if (is_voting_closed<ProposalType>(voting_forum_address, proposal_id)) {
            let voting_forum = borrow_global<VotingForum<ProposalType>>(voting_forum_address);
            let proposal = table::borrow(&voting_forum.proposals, proposal_id);
            let yes_votes = proposal.yes_votes;
            let no_votes = proposal.no_votes;

            if (yes_votes > no_votes && yes_votes + no_votes >= proposal.min_vote_threshold) {
                PROPOSAL_STATE_SUCCEEDED
            } else {
                PROPOSAL_STATE_FAILED
            }
        } else {
            PROPOSAL_STATE_PENDING
        }
    }

    /// Return the proposal's expiration time.
    public fun get_proposal_expiration_secs<ProposalType: store>(
        voting_forum_address: address,
        proposal_id: u64,
    ): u64 acquires VotingForum {
        let voting_forum = borrow_global_mut<VotingForum<ProposalType>>(voting_forum_address);
        let proposal = table::borrow_mut(&mut voting_forum.proposals, proposal_id);
        proposal.expiration_secs
    }

    /// Return the proposal's execution hash.
    public fun get_execution_hash<ProposalType: store>(
        voting_forum_address: address,
        proposal_id: u64,
    ): vector<u8> acquires VotingForum {
        let voting_forum = borrow_global_mut<VotingForum<ProposalType>>(voting_forum_address);
        let proposal = table::borrow_mut(&mut voting_forum.proposals, proposal_id);
        proposal.execution_hash
    }

    /// Return true if the governance proposal has already been resolved.
    public fun is_resolved<ProposalType: store>(
        voting_forum_address: address,
        proposal_id: u64,
    ): bool acquires VotingForum {
        let voting_forum = borrow_global_mut<VotingForum<ProposalType>>(voting_forum_address);
        let proposal = table::borrow_mut(&mut voting_forum.proposals, proposal_id);
        proposal.is_resolved
    }

    /// Return true if the voting period of the given proposal has already ended.
    fun is_voting_period_over<ProposalType: store>(proposal: &Proposal<ProposalType>): bool {
        timestamp::now_seconds() > proposal.expiration_secs
    }

    #[test_only]
    struct TestProposal has store {}

    #[test_only]
    const VOTING_DURATION_SECS: u64 = 100000;

    #[test_only]
    public fun create_test_proposal(
        governance: &signer,
        early_resolution_threshold: Option<u128>,
    ): u64 acquires VotingForum {
        // Register voting forum and create a proposal.
        register<TestProposal>(governance);
        let governance_address = signer::address_of(governance);
        let proposal = TestProposal {};

        // This works because our Move unit test extensions mock out the execution hash to be [1].
        let execution_hash = vector::empty<u8>();
        vector::push_back(&mut execution_hash, 1);
        let proposal_id = create_proposal<TestProposal>(
            governance_address,
            governance_address,
            proposal,
            execution_hash,
            10,
            timestamp::now_seconds() + VOTING_DURATION_SECS,
            early_resolution_threshold,
            simple_map::create<String, vector<u8>>(),
        );

        proposal_id
    }

    #[test(governance = @0x123)]
    #[expected_failure(abort_code = 0x10004)]
    public fun create_proposal_with_empty_execution_hash_should_fail(governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        let governance_address = signer::address_of(governance);
        account::create_account_for_test(governance_address);
        register<TestProposal>(governance);
        let proposal = TestProposal {};

        // This should fail because execution hash is empty.
        create_proposal<TestProposal>(
            governance_address,
            governance_address,
            proposal,
            b"",
            10,
            100000,
            option::none<u128>(),
            simple_map::create<String, vector<u8>>(),
        );
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    public entry fun test_voting_passed(aptos_framework: &signer, governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Register voting forum and create a proposal.
        let governance_address = signer::address_of(governance);
        account::create_account_for_test(governance_address);
        let proposal_id = create_test_proposal(governance, option::none<u128>());
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_PENDING, 0);

        // Vote.
        let proof = TestProposal {};
        vote<TestProposal>(&proof, governance_address, proposal_id, 10, true);
        let TestProposal {} = proof;

        // Resolve.
        timestamp::fast_forward_seconds(VOTING_DURATION_SECS + 1);
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_SUCCEEDED, 1);
        let proposal = resolve<TestProposal>(governance_address, proposal_id);
        let voting_forum = borrow_global<VotingForum<TestProposal>>(governance_address);
        assert!(table::borrow(&voting_forum.proposals, proposal_id).is_resolved, 2);

        let TestProposal {} = proposal;
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    #[expected_failure(abort_code = 0x30003)]
    public entry fun test_cannot_resolve_twice(aptos_framework: &signer, governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Register voting forum and create a proposal.
        let governance_address = signer::address_of(governance);
        account::create_account_for_test(governance_address);
        let proposal_id = create_test_proposal(governance, option::none<u128>());
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_PENDING, 0);

        // Vote.
        let proof = TestProposal {};
        vote<TestProposal>(&proof, governance_address, proposal_id, 10, true);
        let TestProposal {} = proof;

        // Resolve.
        timestamp::fast_forward_seconds(VOTING_DURATION_SECS + 1);
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_SUCCEEDED, 1);
        let TestProposal {} = resolve<TestProposal>(governance_address, proposal_id);

        // Resolve a second time should fail.
        let TestProposal {} = resolve<TestProposal>(governance_address, proposal_id);
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    public entry fun test_voting_passed_early(aptos_framework: &signer, governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Register voting forum and create a proposal.
        let governance_address = signer::address_of(governance);
        account::create_account_for_test(governance_address);
        let proposal_id = create_test_proposal(governance, option::some(100));
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_PENDING, 0);

        // Vote.
        let proof = TestProposal {};
        vote<TestProposal>(&proof, governance_address, proposal_id, 100, true);
        vote<TestProposal>(&proof, governance_address, proposal_id, 10, false);
        let TestProposal {} = proof;

        // Resolve early. Need to increase timestamp as resolution cannot happen in the same tx.
        timestamp::fast_forward_seconds(1);
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_SUCCEEDED, 1);
        let proposal = resolve<TestProposal>(governance_address, proposal_id);
        let voting_forum = borrow_global<VotingForum<TestProposal>>(governance_address);
        assert!(table::borrow(&voting_forum.proposals, proposal_id).is_resolved, 2);

        let TestProposal {} = proposal;
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    #[expected_failure(abort_code = 0x30008)]
    public entry fun test_voting_passed_early_in_same_tx_should_fail(
        aptos_framework: &signer, governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let governance_address = signer::address_of(governance);
        account::create_account_for_test(governance_address);
        let proposal_id = create_test_proposal(governance, option::some(100));
        let proof = TestProposal {};
        vote<TestProposal>(&proof, governance_address, proposal_id, 40, true);
        vote<TestProposal>(&proof, governance_address, proposal_id, 60, true);
        let TestProposal {} = proof;

        // Resolving early should fail since timestamp hasn't changed since the last vote.
        let proposal = resolve<TestProposal>(governance_address, proposal_id);
        let TestProposal {} = proposal;
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    #[expected_failure(abort_code = 0x30002)]
    public entry fun test_voting_failed(aptos_framework: &signer, governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Register voting forum and create a proposal.
        let governance_address = signer::address_of(governance);
        account::create_account_for_test(governance_address);
        let proposal_id = create_test_proposal(governance, option::none<u128>());

        // Vote.
        let proof = TestProposal {};
        vote<TestProposal>(&proof, governance_address, proposal_id, 10, true);
        vote<TestProposal>(&proof, governance_address, proposal_id, 100, false);
        let TestProposal {} = proof;

        // Resolve.
        timestamp::fast_forward_seconds(VOTING_DURATION_SECS + 1);
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_FAILED, 1);
        let proposal = resolve<TestProposal>(governance_address, proposal_id);
        let TestProposal {} = proposal;
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    #[expected_failure(abort_code = 0x30005)]
    public entry fun test_cannot_vote_after_voting_period_is_over(
        aptos_framework: signer, governance: signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        let governance_address = signer::address_of(&governance);
        account::create_account_for_test(governance_address);
        let proposal_id = create_test_proposal(&governance, option::none<u128>());
        // Voting period is over. Voting should now fail.
        timestamp::fast_forward_seconds(VOTING_DURATION_SECS + 1);
        let proof = TestProposal {};
        vote<TestProposal>(&proof, governance_address, proposal_id, 10, true);
        let TestProposal {} = proof;
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    #[expected_failure(abort_code = 0x30002)]
    public entry fun test_voting_failed_early(aptos_framework: &signer, governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Register voting forum and create a proposal.
        let governance_address = signer::address_of(governance);
        account::create_account_for_test(governance_address);
        let proposal_id = create_test_proposal(governance, option::some(100));

        // Vote.
        let proof = TestProposal {};
        vote<TestProposal>(&proof, governance_address, proposal_id, 100, true);
        vote<TestProposal>(&proof, governance_address, proposal_id, 100, false);
        let TestProposal {} = proof;

        // Resolve.
        timestamp::fast_forward_seconds(VOTING_DURATION_SECS + 1);
        assert!(get_proposal_state<TestProposal>(governance_address, proposal_id) == PROPOSAL_STATE_FAILED, 1);
        let proposal = resolve<TestProposal>(governance_address, proposal_id);
        let TestProposal {} = proposal;
    }

    #[test(aptos_framework = @aptos_framework, governance = @0x123)]
    #[expected_failure(abort_code = 0x10007)]
    public entry fun test_cannot_set_min_threshold_higher_than_early_resolution(
        aptos_framework: &signer, governance: &signer) acquires VotingForum {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(signer::address_of(governance));
        // This should fail.
        create_test_proposal(governance, option::some(5));
    }
}
