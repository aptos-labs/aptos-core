module voting::governance {
    use aptos_framework::code::PackageRegistry;
    use aptos_framework::event;
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::object_code_deployment;
    use aptos_framework::timestamp;
    use aptos_framework::voting;
    use aptos_std::simple_map;
    use aptos_std::table::{Self, Table};
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use voting::ve_token;

    const VOTING_DURATION: u64 = 604800; // 7 days
    const PROPOSAL_MINIMUM_VOTING_POWER: u128 = 100000000000000; // 1M tokens with 8 decimals
    const MIN_VOTING_THRESHOLD: u128 = 10000000000000000; // 100M tokens with 8 decimals
    const PROPOSAL_DESCRIPTION_KEY: vector<u8> = b"proposal_description";

    // We don't use native enums since enums cannot be created or read outside of the module that defines them.
    const REQUEST_TYPE_GLOBAL_LOAN_BOOK_FEES: u64 = 1;
    const REQUEST_TYPE_GLOBAL_FACILITIES_FEES: u64 = 2;
    const REQUEST_TYPE_TOKEN_LOCKER_PARAMS: u64 = 3;
    const REQUEST_TYPE_GOVERNANCE_PARAMS: u64 = 4;
    const REQUEST_TYPE_TOKEN_TREASURY_DISTRIBUTION: u64 = 5;
    const REQUEST_TYPE_STAKING_REWARD_PARAMS: u64 = 6;
    const REQUEST_TYPE_BUY_BACK_PARAMS: u64 = 7;
    const MODULE_UPGRADE: u64 = 8;

    /// Proposer's voting power is not enough to create a proposal.
    const EINSUFFICIENT_PROPOSER_VOTING_POWER: u64 = 1;
    /// User has already voted on this proposal.
    const EALREADY_VOTED: u64 = 2;
    /// The resource request type is not valid for updating governance params.
    const EINVALID_RESOURCE_REQUEST_FOR_GOV_PARAMS: u64 = 3;
    /// The resource request type is not valid for upgrading modules.
    const EINVALID_RESOURCE_REQUEST_FOR_UPGRADE: u64 = 4;

    struct GovernanceParameters has key {
        /// Duration in seconds for voting on proposal.
        voting_duration: u64,
        /// Minimum voting power required to create a proposal.
        proposal_minimum_voting_power: u128,
        /// Minimum total voting power required for a proposal to be considered passing.
        min_voting_threshold: u128,
        /// Track whether a user has voted on a proposal.
        voting_records: Table<RecordKey, bool>,
        /// Object owned by the governance that's used to control upgrades.
        /// Any code module that needs to be upgraded needs to first have their ownership transferred to the upgrader.
        upgrader: ExtendRef,
    }

    struct RecordKey has copy, drop, store {
        voter: address,
        proposal_id: u64,
    }

    struct GovernanceProposal has drop, store {}

    struct ResourceRequest has drop {
        type: u64
    }

    #[event]
    struct CreateProposal has drop, store {
        proposer: address,
        proposer_voting_power: u128,
        proposal_id: u64,
        proposal_description: String,
        proposal_hash: vector<u8>,
        is_multi_step_proposal: bool,
    }

    #[event]
    struct Vote has drop, store {
        voter: address,
        voting_power: u64,
        proposal_id: u64,
        should_pass: bool,
    }

    #[event]
    struct UpdateGovernanceParameters has drop, store {
        old_voting_duration: u64,
        new_voting_duration: u64,
        old_proposal_minimum_voting_power: u128,
        new_proposal_minimum_voting_power: u128,
        old_min_voting_threshold: u128,
        new_min_voting_threshold: u128,
    }

    fun init_module(voting_signer: &signer) {
        voting::register<GovernanceProposal>(voting_signer);
        let upgrader = &object::create_object(@voting);
        move_to(voting_signer, GovernanceParameters {
            voting_duration: VOTING_DURATION,
            proposal_minimum_voting_power: PROPOSAL_MINIMUM_VOTING_POWER,
            min_voting_threshold: MIN_VOTING_THRESHOLD,
            voting_records: table::new(),
            upgrader: object::generate_extend_ref(upgrader),
        });
    }

    #[view]
    public fun governance_parameters(): (u64, u128, u128) acquires GovernanceParameters {
        let governance_params = &GovernanceParameters[@voting];
        (
            governance_params.voting_duration,
            governance_params.proposal_minimum_voting_power,
            governance_params.min_voting_threshold
        )
    }

    #[view]
    public fun has_voted(voter: address, proposal_id: u64): bool acquires GovernanceParameters {
        let governance_params = &GovernanceParameters[@voting];
        governance_params.voting_records.contains(RecordKey {
            voter,
            proposal_id
        })
    }

    #[view]
    public fun upgrader_address(): address acquires GovernanceParameters {
        signer::address_of(upgrader_signer())
    }

    #[view]
    public fun proposal_data(proposal_id: u64): (u64, vector<u8>, u128, Option<u128>, u128, u128) {
        let expiration_secs = voting::get_proposal_expiration_secs<GovernanceProposal>(@voting, proposal_id);
        let execution_hash = voting::get_execution_hash<GovernanceProposal>(@voting, proposal_id);
        let min_vote_threshold = voting::get_min_vote_threshold<GovernanceProposal>(@voting, proposal_id);
        let early_resolution_vote_threshold = voting::get_early_resolution_vote_threshold<GovernanceProposal>(@voting, proposal_id);
        let (yes_votes, no_votes) = voting::get_votes<GovernanceProposal>(@voting, proposal_id);
        (expiration_secs, execution_hash, min_vote_threshold, early_resolution_vote_threshold, yes_votes, no_votes)
    }

    public fun type(self: &ResourceRequest): u64 {
        self.type
    }

    /// Create a single-step or multi-step proposal.
    ///
    /// @param proposer Required. The proposer account
    /// @param proposal_description Required. The description of the proposal.
    /// @param execution_hash Required. This is the hash of the resolution script. When the proposal is resolved,
    ///     only the exact script with matching hash can be successfully executed.
    /// @param is_multi_step_proposal Required. If true, the proposal is a multi-step proposal with multiple scripts
    ///     chained together.
    public entry fun create_proposal(
        proposer: &signer,
        proposal_description: String,
        execution_hash: vector<u8>,
        is_multi_step_proposal: bool,
    ) acquires GovernanceParameters {
        let proposer_address = signer::address_of(proposer);
        let proposer_voting_power = ve_token::voting_power_at(proposer_address, ve_token::current_epoch());
        let governance_params = &GovernanceParameters[@voting];
        assert!(
            proposer_voting_power >= governance_params.proposal_minimum_voting_power,
            EINSUFFICIENT_PROPOSER_VOTING_POWER
        );

        let current_time = timestamp::now_seconds();
        let proposal_expiration = current_time + governance_params.voting_duration;
        let proposal_metadata = simple_map::new_from(
            vector[string::utf8(PROPOSAL_DESCRIPTION_KEY)],
            vector[*proposal_description.bytes()],
        );
        let proposal_id = voting::create_proposal_v2(
            proposer_address,
            @voting,
            GovernanceProposal {},
            execution_hash,
            governance_params.min_voting_threshold,
            proposal_expiration,
            option::none(), // No early voting resolution
            proposal_metadata,
            is_multi_step_proposal,
        );

        event::emit(CreateProposal {
            proposer: proposer_address,
            proposer_voting_power,
            proposal_id,
            proposal_description,
            proposal_hash: execution_hash,
            is_multi_step_proposal,
        });
    }

    /// Vote on a proposal with all of the voter's voting power.
    ///
    /// @param voter Required. The voter's account.
    /// @param proposal_id Required. The proposal ID to vote on.
    /// @param should_pass Required. If true, the vote is a yes vote. Otherwise, it's a no vote.
    public entry fun vote(
        voter: &signer,
        proposal_id: u64,
        should_pass: bool,
    ) acquires GovernanceParameters {
        let voter_address = signer::address_of(voter);
        let voting_record_key = RecordKey {
            voter: voter_address,
            proposal_id,
        };
        let governance_params = &mut GovernanceParameters[@voting];
        assert!(
            !governance_params.voting_records.contains(voting_record_key),
            EALREADY_VOTED,
        );
        governance_params.voting_records.add(voting_record_key, true);

        let voting_power = ve_token::voting_power_at(voter_address, ve_token::current_epoch());
        voting::vote<GovernanceProposal>(
            &GovernanceProposal {},
            @voting,
            proposal_id,
            voting_power as u64,
            should_pass,
        );

        event::emit(Vote {
            voter: voter_address,
            voting_power: voting_power as u64,
            proposal_id,
            should_pass,
        });
    }

    /// Resolve a step from a successful multi-step proposal and return the resource type authorized to make the
    /// proposed changes.
    ///
    /// @param proposal_id Required. The proposal ID to resolve.
    /// @param resource_type Required. The resource type to be created or modified.
    /// @param next_execution_hash Required. The hash of the next execution script to be executed. If there's no next
    ///     step, pass an empty vector for `next_execution_hash`.
    ///
    /// This can be only be called from the proposal execution script as the script hash will be checked against the
    /// approved proposal's stored script hash.
    public fun resolve_proposal(
        proposal_id: u64,
        resource_type: u64,
        next_execution_hash: vector<u8>
    ): ResourceRequest {
        // Voting module automatically handles checking that the proposal has succeeded.
        // It'd abort if the voting period is not over, proposal has failed (no votes > yes votes), or the minimum
        // threshold wasn't met.
        voting::resolve_proposal_v2<GovernanceProposal>(@voting, proposal_id, next_execution_hash);
        ResourceRequest { type: resource_type }
    }

    /// Update the governance parameters.
    ///
    /// @param resource_request Required. The resource request object that contains the type of request.
    /// @param voting_duration Required. The new voting duration in seconds.
    /// @param proposal_minimum_voting_power Required. The new minimum voting power required to create a proposal.
    /// @param min_voting_threshold Required. The new minimum total voting power required for a proposal to be
    public fun update_governance_parameters(
        resource_request: ResourceRequest,
        voting_duration: u64,
        proposal_minimum_voting_power: u128,
        min_voting_threshold: u128
    ) acquires GovernanceParameters {
        assert!(
            resource_request.type() == REQUEST_TYPE_GOVERNANCE_PARAMS,
            EINVALID_RESOURCE_REQUEST_FOR_GOV_PARAMS
        );

        let governance_params = &mut GovernanceParameters[@voting];
        event::emit(UpdateGovernanceParameters {
            old_voting_duration: governance_params.voting_duration,
            new_voting_duration: voting_duration,
            old_proposal_minimum_voting_power: governance_params.proposal_minimum_voting_power,
            new_proposal_minimum_voting_power: proposal_minimum_voting_power,
            old_min_voting_threshold: governance_params.min_voting_threshold,
            new_min_voting_threshold: min_voting_threshold,
        });
        governance_params.voting_duration = voting_duration;
        governance_params.proposal_minimum_voting_power = proposal_minimum_voting_power;
        governance_params.min_voting_threshold = min_voting_threshold;
    }

    /// Upgrade a smart contract owned by the governance upgrader object.
    /// Note this only works for smart contract deployed via code objects.
    ///
    /// @param resource_request Required. The resource request object that contains the type of request.
    /// @param metadata_serialized Required. The serialized metadata of the smart contract package.
    /// @param code Required. The new code to be deployed.
    /// @param code_object Required. The object that contains the code to be upgraded.
    public fun upgrade_contract(
        resource_request: ResourceRequest,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
        code_object: Object<PackageRegistry>,
    ) acquires GovernanceParameters {
        assert!(resource_request.type() == MODULE_UPGRADE, EINVALID_RESOURCE_REQUEST_FOR_GOV_PARAMS);
        object_code_deployment::upgrade(
            upgrader_signer(),
            metadata_serialized,
            code,
            code_object,
        );
    }

    inline fun upgrader_signer(): &signer {
        &object::generate_signer_for_extending(&GovernanceParameters[@voting].upgrader)
    }

    #[test_only]
    public fun init_for_test(voting_signer: &signer) {
        init_module(voting_signer);
    }

    #[test_only]
    public fun create_resource_request(resource_type: u64,): ResourceRequest {
        ResourceRequest { type: resource_type }
    }
}