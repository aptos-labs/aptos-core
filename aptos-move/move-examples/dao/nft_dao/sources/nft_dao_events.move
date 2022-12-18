module dao_platform::nft_dao_events {
    use aptos_framework::event::EventHandle;
    use std::string::String;
    use aptos_token::property_map::PropertyMap;
    use std::option::Option;
    use aptos_std::any::Any;
    use std::signer;
    use aptos_framework::account;
    use std::option;
    use aptos_framework::event;

    struct CreateDAOEvent has drop, store {
        dao_name: String,
        dao_resolve_threshold: u64,
        voting_duration: u64,
        min_proposal_weight: u64,
        governance_token_creator: address,
        governance_token_collection: String,
        dao_address: address,
    }

    struct AddMemberTokenEvent has drop, store {
        token_names: vector<String>,
        property_versions: vector<u64>,
    }

    struct CreateProposalEvent has drop, store {
        name: String,
        description: String,
        function_name: String,
        function_args: PropertyMap,
        start_time_sec: u64,
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

    struct DAOEventStoreV1 has key {
        create_dao_events: EventHandle<CreateDAOEvent>,
        add_member_token_events: EventHandle<AddMemberTokenEvent>,
        create_proposal_events: EventHandle<CreateProposalEvent>,
        vote_events: EventHandle<VoteEvent>,
        resolve_events: EventHandle<ResolveEvent>,
        extension: Option<Any>,
    }

    fun initialize_dao_event_store(acct: &signer) {
        if (!exists<DAOEventStoreV1>(signer::address_of(acct))) {
            move_to(acct, DAOEventStoreV1 {
                create_dao_events: account::new_event_handle<CreateDAOEvent>(acct),
                add_member_token_events: account::new_event_handle<AddMemberTokenEvent>(acct),
                create_proposal_events: account::new_event_handle<CreateProposalEvent>(acct),
                vote_events: account::new_event_handle<VoteEvent>(acct),
                resolve_events: account::new_event_handle<ResolveEvent>(acct),
                extension: option::none<Any>(),
            });
        };
    }

    public fun emit_create_dao_event(
        dao: &signer,
        dao_name: String,
        dao_resolve_threshold: u64,
        voting_duration: u64,
        min_proposal_weight: u64,
        governance_token_creator: address,
        governance_token_collection: String,
        dao_address: address,
    ) acquires DAOEventStoreV1 {
        let event = CreateDAOEvent {
            dao_name,
            dao_resolve_threshold,
            voting_duration,
            min_proposal_weight,
            governance_token_creator,
            governance_token_collection,
            dao_address,
        };
        initialize_dao_event_store(dao);
        let dao_event_store = borrow_global_mut<DAOEventStoreV1>(signer::address_of(dao));
        event::emit_event<CreateDAOEvent>(
            &mut dao_event_store.create_dao_events,
            event,
        );
    }
}
