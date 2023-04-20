module dao_platform::nft_dao_events {
    use aptos_framework::account;
    use aptos_framework::event::EventHandle;
    use aptos_framework::event;
    use aptos_std::any::Any;
    use aptos_token::property_map::PropertyMap;
    use std::option::Option;
    use std::option;
    use std::signer;
    use std::string::String;
    friend dao_platform::nft_dao;

    struct CreateDAOEvent has drop, store {
        dao_name: String,
        dao_resolve_threshold: u64,
        voting_duration: u64,
        min_proposal_weight: u64,
        governance_token_creator: address,
        governance_token_collection: String,
        admin: address,
    }

    struct CreateProposalEvent has drop, store {
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

    struct VoteEvent has drop, store {
        voter: address,
        proposal_id: u64,
        vote: bool,
        token_names: vector<String>,
        property_versions: vector<u64>,
    }

    struct ResolveEvent has drop, store {
        proposal_id: u64,
        result: u8,
    }

    struct AdminResolveEvent has drop, store {
        proposal_id: u64,
        admin: address,
        reason: String,
    }

    struct AdminVetoEvent has drop, store {
        proposal_id: u64,
        admin: address,
        reason: String,
    }

    struct AdminOfferEvent has drop, store {
        new_admin: address,
        admin: address,
    }

    struct AdminClaimEvent has drop, store {
        new_admin: address,
        admin: address,
    }

    struct AdminOfferCancelEvent has drop, store {
        admin: address,
    }

    struct DAONameChangeEvent has drop, store {
        old_name: String,
        new_name: String,
    }

    struct DAOThresholdChangeEvent has drop, store {
        old_threshold: u64,
        new_threshold: u64,
    }

    struct DAOVoteDurationChangeEvent has drop, store {
        old_duration: u64,
        new_duration: u64,
    }

    struct DAOReqiredVotingPowerChangeEvent has drop, store {
        old_power: u64,
        new_power: u64,
    }

    struct DAOEventStoreV1 has key {
        create_dao_events: EventHandle<CreateDAOEvent>,
        create_proposal_events: EventHandle<CreateProposalEvent>,
        vote_events: EventHandle<VoteEvent>,
        resolve_events: EventHandle<ResolveEvent>,
        admin_offer_events: EventHandle<AdminOfferEvent>,
        admin_claim_events: EventHandle<AdminClaimEvent>,
        admin_offer_cancel_events: EventHandle<AdminOfferCancelEvent>,
        change_name_events: EventHandle<DAONameChangeEvent>,
        change_threshold_events: EventHandle<DAOThresholdChangeEvent>,
        change_duration_events: EventHandle<DAOVoteDurationChangeEvent>,
        change_voting_power_events: EventHandle<DAOReqiredVotingPowerChangeEvent>,
        admin_resolve_events: EventHandle<AdminResolveEvent>,
        admin_veto_events: EventHandle<AdminVetoEvent>,
        extension: Option<Any>,
    }

    fun initialize_dao_event_store(acct: &signer) {
        if (!exists<DAOEventStoreV1>(signer::address_of(acct))) {
            move_to(acct, DAOEventStoreV1 {
                create_dao_events: account::new_event_handle<CreateDAOEvent>(acct),
                create_proposal_events: account::new_event_handle<CreateProposalEvent>(acct),
                vote_events: account::new_event_handle<VoteEvent>(acct),
                resolve_events: account::new_event_handle<ResolveEvent>(acct),
                admin_offer_events: account::new_event_handle<AdminOfferEvent>(acct),
                admin_claim_events: account::new_event_handle<AdminClaimEvent>(acct),
                admin_offer_cancel_events: account::new_event_handle<AdminOfferCancelEvent>(acct),
                change_name_events: account::new_event_handle<DAONameChangeEvent>(acct),
                change_threshold_events: account::new_event_handle<DAOThresholdChangeEvent>(acct),
                change_duration_events: account::new_event_handle<DAOVoteDurationChangeEvent>(acct),
                change_voting_power_events: account::new_event_handle<DAOReqiredVotingPowerChangeEvent>(acct),
                admin_resolve_events: account::new_event_handle<AdminResolveEvent>(acct),
                admin_veto_events: account::new_event_handle<AdminVetoEvent>(acct),
                extension: option::none<Any>(),
            });
        };
    }

    public(friend) fun emit_create_dao_event(
        dao: &signer,
        dao_name: String,
        dao_resolve_threshold: u64,
        voting_duration: u64,
        min_proposal_weight: u64,
        governance_token_creator: address,
        governance_token_collection: String,
        admin: address,
    ) acquires DAOEventStoreV1 {
        let event = CreateDAOEvent {
            dao_name,
            dao_resolve_threshold,
            voting_duration,
            min_proposal_weight,
            governance_token_creator,
            governance_token_collection,
            admin,
        };
        initialize_dao_event_store(dao);
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(signer::address_of(dao));
        event::emit_event<CreateDAOEvent>(
            &mut dao_event_store.create_dao_events,
            event,
        );
    }

    public(friend) fun emit_create_proposal_event(
        proposer: address,
        nft_dao: address,
        proposal_id: u64,
        name: String,
        description: String,
        function_names: vector<String>,
        prosposal_arguments: vector<PropertyMap>,
        start_time_sec: u64,
        token_names: vector<String>,
        property_versions: vector<u64>,
    ) acquires DAOEventStoreV1 {
        let event = CreateProposalEvent {
            proposer,
            proposal_id,
            name,
            description,
            function_names,
            prosposal_arguments,
            start_time_sec,
            token_names,
            property_versions,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);
        event::emit_event<CreateProposalEvent>(
            &mut dao_event_store.create_proposal_events,
            event,
        );
    }

    public(friend) fun emit_voting_event(
        voter: address,
        nft_dao_address: address,
        proposal_id: u64,
        vote: bool,
        token_names: vector<String>,
        property_versions: vector<u64>,
    ) acquires DAOEventStoreV1 {
        let event = VoteEvent {
            voter,
            proposal_id,
            vote,
            token_names,
            property_versions,
        };

        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao_address);
        event::emit_event<VoteEvent>(
            &mut dao_event_store.vote_events,
            event,
        );
    }

    public(friend) fun emit_resolve_event(proposal_id: u64, nft_dao: address, result: u8) acquires DAOEventStoreV1 {
        let event = ResolveEvent {
            proposal_id,
            result,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<ResolveEvent>(
            &mut dao_event_store.resolve_events,
            event,
        );
    }

    public(friend) fun emit_admin_offer_event(admin: address, new_admin: address, nft_dao: address) acquires DAOEventStoreV1 {
        let event = AdminOfferEvent {
            new_admin,
            admin,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<AdminOfferEvent>(
            &mut dao_event_store.admin_offer_events,
            event,
        );
    }

    public(friend) fun emit_admin_claim_event(admin: address, new_admin: address, nft_dao: address) acquires DAOEventStoreV1 {
        let event = AdminClaimEvent {
            new_admin,
            admin,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<AdminClaimEvent>(
            &mut dao_event_store.admin_claim_events,
            event,
        );
    }

    public(friend) fun emit_admin_offer_cancel_event(admin: address, nft_dao: address) acquires DAOEventStoreV1 {
        let event = AdminOfferCancelEvent {
            admin,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<AdminOfferCancelEvent>(
            &mut dao_event_store.admin_offer_cancel_events,
            event,
        );
    }

    public(friend) fun emit_change_name_event(old_name: String, new_name: String, nft_dao: address) acquires DAOEventStoreV1 {
        let event = DAONameChangeEvent {
            old_name,
            new_name,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<DAONameChangeEvent>(
            &mut dao_event_store.change_name_events,
            event,
        );
    }

    public(friend) fun emit_change_threshold_event(old_threshold: u64, new_threshold: u64, nft_dao: address) acquires DAOEventStoreV1 {
        let event = DAOThresholdChangeEvent {
            old_threshold,
            new_threshold,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<DAOThresholdChangeEvent>(
            &mut dao_event_store.change_threshold_events,
            event,
        );
    }

    public(friend) fun emit_change_duration_event(old_duration: u64, new_duration: u64, nft_dao: address) acquires DAOEventStoreV1 {
        let event = DAOVoteDurationChangeEvent {
            old_duration,
            new_duration,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<DAOVoteDurationChangeEvent>(
            &mut dao_event_store.change_duration_events,
            event,
        );
    }

    public(friend) fun emit_change_voting_power_event(old_power: u64, new_power: u64, nft_dao: address) acquires DAOEventStoreV1 {
        let event = DAOReqiredVotingPowerChangeEvent {
            old_power,
            new_power,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<DAOReqiredVotingPowerChangeEvent>(
            &mut dao_event_store.change_voting_power_events,
            event,
        );
    }

    public(friend) fun emit_admin_veto_event(proposal_id: u64, admin: address, nft_dao: address, reason: String) acquires DAOEventStoreV1 {
        let event = AdminVetoEvent {
            proposal_id,
            admin,
            reason,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<AdminVetoEvent>(
            &mut dao_event_store.admin_veto_events,
            event,
        );
    }

    public(friend) fun emit_admin_resolve_event(proposal_id: u64, admin: address, nft_dao: address, reason: String) acquires DAOEventStoreV1 {
        let event = AdminResolveEvent {
            proposal_id,
            admin,
            reason,
        };
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(nft_dao);

        event::emit_event<AdminResolveEvent>(
            &mut dao_event_store.admin_resolve_events,
            event,
        );
    }
}
