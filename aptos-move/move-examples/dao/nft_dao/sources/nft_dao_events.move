module dao_platform::nft_dao_events {
    use aptos_framework::event;
    use aptos_token::property_map::PropertyMap;
    use std::string::String;

    friend dao_platform::nft_dao;

    #[event]
    struct CreateDAOEvent has drop, store {
        dao_address: address,
        dao_name: String,
        dao_resolve_threshold: u64,
        voting_duration: u64,
        min_proposal_weight: u64,
        governance_token_creator: address,
        governance_token_collection: String,
        admin: address,
    }

    #[event]
    struct CreateProposalEvent has drop, store {
        dao_address: address,
        proposer: address,
        proposal_id: u64,
        name: String,
        description: String,
        function_names: vector<String>,
        prosposal_arguments: vector<PropertyMap>,
        start_time_sec: u64,
        token_names: vector<String>,
        property_versions: vector<u64>,
    }

    #[event]
    struct VoteEvent has drop, store {
        dao_address: address,
        voter: address,
        proposal_id: u64,
        vote: bool,
        token_names: vector<String>,
        property_versions: vector<u64>,
    }

    #[event]
    struct ResolveEvent has drop, store {
        dao_address: address,
        proposal_id: u64,
        result: u8,
    }

    #[event]
    struct AdminResolveEvent has drop, store {
        dao_address: address,
        proposal_id: u64,
        admin: address,
        reason: String,
    }

    #[event]
    struct AdminVetoEvent has drop, store {
        dao_address: address,
        proposal_id: u64,
        admin: address,
        reason: String,
    }

    #[event]
    struct AdminOfferEvent has drop, store {
        dao_address: address,
        new_admin: address,
        admin: address,
    }

    #[event]
    struct AdminClaimEvent has drop, store {
        dao_address: address,
        new_admin: address,
        admin: address,
    }

    #[event]
    struct AdminOfferCancelEvent has drop, store {
        dao_address: address,
        admin: address,
    }

    #[event]
    struct DAONameChangeEvent has drop, store {
        dao_address: address,
        old_name: String,
        new_name: String,
    }

    #[event]
    struct DAOThresholdChangeEvent has drop, store {
        dao_address: address,
        old_threshold: u64,
        new_threshold: u64,
    }

    #[event]
    struct DAOVoteDurationChangeEvent has drop, store {
        dao_address: address,
        old_duration: u64,
        new_duration: u64,
    }

    #[event]
    struct DAOReqiredVotingPowerChangeEvent has drop, store {
        dao_address: address,
        old_power: u64,
        new_power: u64,
    }

    public(friend) fun emit_create_dao_event(
        dao_address: address,
        dao_name: String,
        dao_resolve_threshold: u64,
        voting_duration: u64,
        min_proposal_weight: u64,
        governance_token_creator: address,
        governance_token_collection: String,
        admin: address,
    ) {
        event::emit(CreateDAOEvent {
            dao_address,
            dao_name,
            dao_resolve_threshold,
            voting_duration,
            min_proposal_weight,
            governance_token_creator,
            governance_token_collection,
            admin,
        });
    }

    public(friend) fun emit_create_proposal_event(
        proposer: address,
        dao_address: address,
        proposal_id: u64,
        name: String,
        description: String,
        function_names: vector<String>,
        prosposal_arguments: vector<PropertyMap>,
        start_time_sec: u64,
        token_names: vector<String>,
        property_versions: vector<u64>,
    ) {
        event::emit(CreateProposalEvent {
            dao_address,
            proposer,
            proposal_id,
            name,
            description,
            function_names,
            prosposal_arguments,
            start_time_sec,
            token_names,
            property_versions,
        });
    }

    public(friend) fun emit_voting_event(
        voter: address,
        dao_address: address,
        proposal_id: u64,
        vote: bool,
        token_names: vector<String>,
        property_versions: vector<u64>,
    ) {
        event::emit(VoteEvent {
            dao_address,
            voter,
            proposal_id,
            vote,
            token_names,
            property_versions,
        });
    }

    public(friend) fun emit_resolve_event(proposal_id: u64, dao_address: address, result: u8) {
        event::emit(ResolveEvent {
            dao_address,
            proposal_id,
            result,
        });
    }

    public(friend) fun emit_admin_offer_event(admin: address, new_admin: address, dao_address: address) {
        event::emit(AdminOfferEvent {
            dao_address,
            new_admin,
            admin,
        });
    }

    public(friend) fun emit_admin_claim_event(admin: address, new_admin: address, dao_address: address) {
        event::emit(AdminClaimEvent {
            dao_address,
            new_admin,
            admin,
        });
    }

    public(friend) fun emit_admin_offer_cancel_event(admin: address, dao_address: address) {
        event::emit(AdminOfferCancelEvent {
            dao_address,
            admin,
        });
    }

    public(friend) fun emit_change_name_event(old_name: String, new_name: String, dao_address: address) {
        event::emit(DAONameChangeEvent {
            dao_address,
            old_name,
            new_name,
        });
    }

    public(friend) fun emit_change_threshold_event(old_threshold: u64, new_threshold: u64, dao_address: address) {
        event::emit(DAOThresholdChangeEvent {
            dao_address,
            old_threshold,
            new_threshold,
        });
    }

    public(friend) fun emit_change_duration_event(old_duration: u64, new_duration: u64, dao_address: address) {
        event::emit(DAOVoteDurationChangeEvent {
            dao_address,
            old_duration,
            new_duration,
        });
    }

    public(friend) fun emit_change_voting_power_event(old_power: u64, new_power: u64, dao_address: address) {
        event::emit(DAOReqiredVotingPowerChangeEvent {
            dao_address,
            old_power,
            new_power,
        });
    }

    public(friend) fun emit_admin_veto_event(proposal_id: u64, admin: address, dao_address: address, reason: String) {
        event::emit(AdminVetoEvent {
            dao_address,
            proposal_id,
            admin,
            reason,
        });
    }

    public(friend) fun emit_admin_resolve_event(proposal_id: u64, admin: address, dao_address: address, reason: String) {
        event::emit(AdminResolveEvent {
            dao_address,
            proposal_id,
            admin,
            reason,
        });
    }
}
