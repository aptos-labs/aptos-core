module 0x1::voting {
    struct CreateProposalEvent has drop, store {
        proposal_id: u64,
        early_resolution_vote_threshold: 0x1::option::Option<u128>,
        execution_hash: vector<u8>,
        expiration_secs: u64,
        metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
        min_vote_threshold: u128,
    }
    
    struct Proposal<T0: store> has store {
        proposer: address,
        execution_content: 0x1::option::Option<T0>,
        metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
        creation_time_secs: u64,
        execution_hash: vector<u8>,
        min_vote_threshold: u128,
        expiration_secs: u64,
        early_resolution_vote_threshold: 0x1::option::Option<u128>,
        yes_votes: u128,
        no_votes: u128,
        is_resolved: bool,
        resolution_time_secs: u64,
    }
    
    struct RegisterForumEvent has drop, store {
        hosting_account: address,
        proposal_type_info: 0x1::type_info::TypeInfo,
    }
    
    struct ResolveProposal has drop, store {
        proposal_id: u64,
        yes_votes: u128,
        no_votes: u128,
        resolved_early: bool,
    }
    
    struct VoteEvent has drop, store {
        proposal_id: u64,
        num_votes: u64,
    }
    
    struct VotingEvents has store {
        create_proposal_events: 0x1::event::EventHandle<CreateProposalEvent>,
        register_forum_events: 0x1::event::EventHandle<RegisterForumEvent>,
        resolve_proposal_events: 0x1::event::EventHandle<ResolveProposal>,
        vote_events: 0x1::event::EventHandle<VoteEvent>,
    }
    
    struct VotingForum<T0: store> has key {
        proposals: 0x1::table::Table<u64, Proposal<T0>>,
        events: VotingEvents,
        next_proposal_id: u64,
    }
    
    public fun can_be_resolved_early<T0: store>(arg0: &Proposal<T0>) : bool {
        if (0x1::option::is_some<u128>(&arg0.early_resolution_vote_threshold)) {
            let v0 = *0x1::option::borrow<u128>(&arg0.early_resolution_vote_threshold);
            if (arg0.yes_votes >= v0 || arg0.no_votes >= v0) {
                return true
            };
        };
        false
    }
    
    public fun create_proposal<T0: store>(arg0: address, arg1: address, arg2: T0, arg3: vector<u8>, arg4: u128, arg5: u64, arg6: 0x1::option::Option<u128>, arg7: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>) : u64 acquires VotingForum {
        create_proposal_v2<T0>(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, false)
    }
    
    public fun create_proposal_v2<T0: store>(arg0: address, arg1: address, arg2: T0, arg3: vector<u8>, arg4: u128, arg5: u64, arg6: 0x1::option::Option<u128>, arg7: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>, arg8: bool) : u64 acquires VotingForum {
        if (0x1::option::is_some<u128>(&arg6)) {
            assert!(arg4 <= *0x1::option::borrow<u128>(&arg6), 0x1::error::invalid_argument(7));
        };
        assert!(0x1::vector::length<u8>(&arg3) > 0, 0x1::error::invalid_argument(4));
        let v0 = borrow_global_mut<VotingForum<T0>>(arg1);
        let v1 = v0.next_proposal_id;
        v0.next_proposal_id = v0.next_proposal_id + 1;
        0x1::simple_map::add<0x1::string::String, vector<u8>>(&mut arg7, 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_KEY"), 0x1::bcs::to_bytes<bool>(&arg8));
        let v2 = 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_IN_EXECUTION");
        if (arg8) {
            let v3 = false;
            0x1::simple_map::add<0x1::string::String, vector<u8>>(&mut arg7, v2, 0x1::bcs::to_bytes<bool>(&v3));
        } else {
            if (0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(&mut arg7, &v2)) {
                let (_, _) = 0x1::simple_map::remove<0x1::string::String, vector<u8>>(&mut arg7, &v2);
            };
        };
        let v6 = 0x1::timestamp::now_seconds();
        let v7 = 0x1::option::some<T0>(arg2);
        let v8 = arg3;
        let v9 = arg7;
        let v10 = arg6;
        let v11 = Proposal<T0>{
            proposer                        : arg0, 
            execution_content               : v7, 
            metadata                        : v9, 
            creation_time_secs              : v6, 
            execution_hash                  : v8, 
            min_vote_threshold              : arg4, 
            expiration_secs                 : arg5, 
            early_resolution_vote_threshold : v10, 
            yes_votes                       : 0, 
            no_votes                        : 0, 
            is_resolved                     : false, 
            resolution_time_secs            : 0,
        };
        0x1::table::add<u64, Proposal<T0>>(&mut v0.proposals, v1, v11);
        let v12 = CreateProposalEvent{
            proposal_id                     : v1, 
            early_resolution_vote_threshold : arg6, 
            execution_hash                  : arg3, 
            expiration_secs                 : arg5, 
            metadata                        : arg7, 
            min_vote_threshold              : arg4,
        };
        0x1::event::emit_event<CreateProposalEvent>(&mut v0.events.create_proposal_events, v12);
        v1
    }
    
    public fun get_early_resolution_vote_threshold<T0: store>(arg0: address, arg1: u64) : 0x1::option::Option<u128> acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.early_resolution_vote_threshold
    }
    
    public fun get_execution_hash<T0: store>(arg0: address, arg1: u64) : vector<u8> acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.execution_hash
    }
    
    public fun get_min_vote_threshold<T0: store>(arg0: address, arg1: u64) : u128 acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.min_vote_threshold
    }
    
    public fun get_proposal_creation_secs<T0: store>(arg0: address, arg1: u64) : u64 acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.creation_time_secs
    }
    
    public fun get_proposal_expiration_secs<T0: store>(arg0: address, arg1: u64) : u64 acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.expiration_secs
    }
    
    public fun get_proposal_metadata<T0: store>(arg0: address, arg1: u64) : 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>> acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.metadata
    }
    
    public fun get_proposal_metadata_value<T0: store>(arg0: address, arg1: u64, arg2: 0x1::string::String) : vector<u8> acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        *0x1::simple_map::borrow<0x1::string::String, vector<u8>>(&v0.metadata, &arg2)
    }
    
    public fun get_proposal_state<T0: store>(arg0: address, arg1: u64) : u64 acquires VotingForum {
        if (is_voting_closed<T0>(arg0, arg1)) {
            let v1 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
            let v2 = v1.yes_votes;
            let v3 = v1.no_votes;
            let v4 = if (v2 > v3 && v2 + v3 >= v1.min_vote_threshold) {
                1
            } else {
                3
            };
            v4
        } else {
            0
        }
    }
    
    public fun get_proposer<T0: store>(arg0: address, arg1: u64) : address acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.proposer
    }
    
    public fun get_resolution_time_secs<T0: store>(arg0: address, arg1: u64) : u64 acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.resolution_time_secs
    }
    
    public fun get_votes<T0: store>(arg0: address, arg1: u64) : (u128, u128) acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        (v0.yes_votes, v0.no_votes)
    }
    
    public fun is_multi_step_proposal_in_execution<T0: store>(arg0: address, arg1: u64) : bool acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        let v1 = 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_IN_EXECUTION");
        let v2 = 0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(&v0.metadata, &v1);
        assert!(v2, 0x1::error::invalid_argument(12));
        0x1::from_bcs::to_bool(*0x1::simple_map::borrow<0x1::string::String, vector<u8>>(&v0.metadata, &v1))
    }
    
    fun is_proposal_resolvable<T0: store>(arg0: address, arg1: u64) acquires VotingForum {
        let v0 = get_proposal_state<T0>(arg0, arg1);
        assert!(v0 == 1, 0x1::error::invalid_state(2));
        let v1 = &mut borrow_global_mut<VotingForum<T0>>(arg0).proposals;
        let v2 = 0x1::table::borrow_mut<u64, Proposal<T0>>(v1, arg1);
        assert!(!v2.is_resolved, 0x1::error::invalid_state(3));
        let v3 = 0x1::string::utf8(b"RESOLVABLE_TIME_METADATA_KEY");
        let v4 = 0x1::from_bcs::to_u64(*0x1::simple_map::borrow<0x1::string::String, vector<u8>>(&v2.metadata, &v3));
        assert!(0x1::timestamp::now_seconds() > v4, 0x1::error::invalid_state(8));
        let v5 = 0x1::transaction_context::get_script_hash() == v2.execution_hash;
        assert!(v5, 0x1::error::invalid_argument(1));
    }
    
    public fun is_resolved<T0: store>(arg0: address, arg1: u64) : bool acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        v0.is_resolved
    }
    
    public fun is_voting_closed<T0: store>(arg0: address, arg1: u64) : bool acquires VotingForum {
        let v0 = 0x1::table::borrow<u64, Proposal<T0>>(&borrow_global<VotingForum<T0>>(arg0).proposals, arg1);
        can_be_resolved_early<T0>(v0) || is_voting_period_over<T0>(v0)
    }
    
    fun is_voting_period_over<T0: store>(arg0: &Proposal<T0>) : bool {
        0x1::timestamp::now_seconds() > arg0.expiration_secs
    }
    
    public fun next_proposal_id<T0: store>(arg0: address) : u64 acquires VotingForum {
        borrow_global<VotingForum<T0>>(arg0).next_proposal_id
    }
    
    public fun register<T0: store>(arg0: &signer) {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(!exists<VotingForum<T0>>(v0), 0x1::error::already_exists(6));
        let v1 = 0x1::account::new_event_handle<CreateProposalEvent>(arg0);
        let v2 = 0x1::account::new_event_handle<RegisterForumEvent>(arg0);
        let v3 = 0x1::account::new_event_handle<ResolveProposal>(arg0);
        let v4 = 0x1::account::new_event_handle<VoteEvent>(arg0);
        let v5 = VotingEvents{
            create_proposal_events  : v1, 
            register_forum_events   : v2, 
            resolve_proposal_events : v3, 
            vote_events             : v4,
        };
        let v6 = VotingForum<T0>{
            proposals        : 0x1::table::new<u64, Proposal<T0>>(), 
            events           : v5, 
            next_proposal_id : 0,
        };
        let v7 = RegisterForumEvent{
            hosting_account    : v0, 
            proposal_type_info : 0x1::type_info::type_of<T0>(),
        };
        0x1::event::emit_event<RegisterForumEvent>(&mut v6.events.register_forum_events, v7);
        move_to<VotingForum<T0>>(arg0, v6);
    }
    
    public fun resolve<T0: store>(arg0: address, arg1: u64) : T0 acquires VotingForum {
        is_proposal_resolvable<T0>(arg0, arg1);
        let v0 = borrow_global_mut<VotingForum<T0>>(arg0);
        let v1 = 0x1::table::borrow_mut<u64, Proposal<T0>>(&mut v0.proposals, arg1);
        let v2 = 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_KEY");
        if (0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(&v1.metadata, &v2)) {
            let v3 = &v2;
            let v4 = !0x1::from_bcs::to_bool(*0x1::simple_map::borrow<0x1::string::String, vector<u8>>(&v1.metadata, v3));
            assert!(v4, 0x1::error::permission_denied(10));
        };
        let v5 = can_be_resolved_early<T0>(v1);
        v1.is_resolved = true;
        v1.resolution_time_secs = 0x1::timestamp::now_seconds();
        let v6 = v1.yes_votes;
        let v7 = ResolveProposal{
            proposal_id    : arg1, 
            yes_votes      : v6, 
            no_votes       : v1.no_votes, 
            resolved_early : v5,
        };
        0x1::event::emit_event<ResolveProposal>(&mut v0.events.resolve_proposal_events, v7);
        0x1::option::extract<T0>(&mut v1.execution_content)
    }
    
    public fun resolve_proposal_v2<T0: store>(arg0: address, arg1: u64, arg2: vector<u8>) acquires VotingForum {
        is_proposal_resolvable<T0>(arg0, arg1);
        let v0 = borrow_global_mut<VotingForum<T0>>(arg0);
        let v1 = 0x1::table::borrow_mut<u64, Proposal<T0>>(&mut v0.proposals, arg1);
        let v2 = 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_IN_EXECUTION");
        if (0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(&v1.metadata, &v2)) {
            let v3 = true;
            *0x1::simple_map::borrow_mut<0x1::string::String, vector<u8>>(&mut v1.metadata, &v2) = 0x1::bcs::to_bytes<bool>(&v3);
        };
        let v4 = 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_KEY");
        let v5 = 0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(&v1.metadata, &v4);
        let v6 = v5 && 0x1::from_bcs::to_bool(*0x1::simple_map::borrow<0x1::string::String, vector<u8>>(&v1.metadata, &v4));
        let v7 = 0x1::vector::length<u8>(&arg2) == 0;
        assert!(v6 || v7, 0x1::error::invalid_argument(11));
        if (v7) {
            v1.is_resolved = true;
            v1.resolution_time_secs = 0x1::timestamp::now_seconds();
            if (v6) {
                let v8 = false;
                *0x1::simple_map::borrow_mut<0x1::string::String, vector<u8>>(&mut v1.metadata, &v2) = 0x1::bcs::to_bytes<bool>(&v8);
            };
        } else {
            v1.execution_hash = arg2;
        };
        let v9 = can_be_resolved_early<T0>(v1);
        let v10 = ResolveProposal{
            proposal_id    : arg1, 
            yes_votes      : v1.yes_votes, 
            no_votes       : v1.no_votes, 
            resolved_early : v9,
        };
        0x1::event::emit_event<ResolveProposal>(&mut v0.events.resolve_proposal_events, v10);
    }
    
    public fun vote<T0: store>(arg0: &T0, arg1: address, arg2: u64, arg3: u64, arg4: bool) acquires VotingForum {
        let v0 = borrow_global_mut<VotingForum<T0>>(arg1);
        let v1 = 0x1::table::borrow_mut<u64, Proposal<T0>>(&mut v0.proposals, arg2);
        assert!(!is_voting_period_over<T0>(v1), 0x1::error::invalid_state(5));
        assert!(!v1.is_resolved, 0x1::error::invalid_state(3));
        let v2 = 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_IN_EXECUTION");
        let v3 = if (!0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(&v1.metadata, &v2)) {
            true
        } else {
            let v4 = 0x1::string::utf8(b"IS_MULTI_STEP_PROPOSAL_IN_EXECUTION");
            let v5 = false;
            *0x1::simple_map::borrow<0x1::string::String, vector<u8>>(&v1.metadata, &v4) == 0x1::bcs::to_bytes<bool>(&v5)
        };
        assert!(v3, 0x1::error::invalid_state(9));
        if (arg4) {
            v1.yes_votes = v1.yes_votes + (arg3 as u128);
        } else {
            v1.no_votes = v1.no_votes + (arg3 as u128);
        };
        let v6 = 0x1::timestamp::now_seconds();
        let v7 = 0x1::bcs::to_bytes<u64>(&v6);
        let v8 = 0x1::string::utf8(b"RESOLVABLE_TIME_METADATA_KEY");
        if (0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(&v1.metadata, &v8)) {
            *0x1::simple_map::borrow_mut<0x1::string::String, vector<u8>>(&mut v1.metadata, &v8) = v7;
        } else {
            0x1::simple_map::add<0x1::string::String, vector<u8>>(&mut v1.metadata, v8, v7);
        };
        let v9 = VoteEvent{
            proposal_id : arg2, 
            num_votes   : arg3,
        };
        0x1::event::emit_event<VoteEvent>(&mut v0.events.vote_events, v9);
    }
    
    // decompiled from Move bytecode v6
}
