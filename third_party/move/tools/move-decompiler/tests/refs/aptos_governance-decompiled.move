module 0x1::aptos_governance {
    struct ApprovedExecutionHashes has key {
        hashes: 0x1::simple_map::SimpleMap<u64, vector<u8>>,
    }
    
    struct CreateProposalEvent has drop, store {
        proposer: address,
        stake_pool: address,
        proposal_id: u64,
        execution_hash: vector<u8>,
        proposal_metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
    }
    
    struct GovernanceConfig has key {
        min_voting_threshold: u128,
        required_proposer_stake: u64,
        voting_duration_secs: u64,
    }
    
    struct GovernanceEvents has key {
        create_proposal_events: 0x1::event::EventHandle<CreateProposalEvent>,
        update_config_events: 0x1::event::EventHandle<UpdateConfigEvent>,
        vote_events: 0x1::event::EventHandle<VoteEvent>,
    }
    
    struct GovernanceResponsbility has key {
        signer_caps: 0x1::simple_map::SimpleMap<address, 0x1::account::SignerCapability>,
    }
    
    struct RecordKey has copy, drop, store {
        stake_pool: address,
        proposal_id: u64,
    }
    
    struct UpdateConfigEvent has drop, store {
        min_voting_threshold: u128,
        required_proposer_stake: u64,
        voting_duration_secs: u64,
    }
    
    struct VoteEvent has drop, store {
        proposal_id: u64,
        voter: address,
        stake_pool: address,
        num_votes: u64,
        should_pass: bool,
    }
    
    struct VotingRecords has key {
        votes: 0x1::table::Table<RecordKey, bool>,
    }
    
    struct VotingRecordsV2 has key {
        votes: 0x1::smart_table::SmartTable<RecordKey, u64>,
    }
    
    public entry fun create_proposal(arg0: &signer, arg1: address, arg2: vector<u8>, arg3: vector<u8>, arg4: vector<u8>) acquires GovernanceConfig, GovernanceEvents {
        create_proposal_v2(arg0, arg1, arg2, arg3, arg4, false);
    }
    
    public fun reconfigure(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::reconfiguration::reconfigure();
    }
    
    public entry fun create_proposal_v2(arg0: &signer, arg1: address, arg2: vector<u8>, arg3: vector<u8>, arg4: vector<u8>, arg5: bool) acquires GovernanceConfig, GovernanceEvents {
        create_proposal_v2_impl(arg0, arg1, arg2, arg3, arg4, arg5);
    }
    
    public fun resolve(arg0: u64, arg1: address) : signer acquires ApprovedExecutionHashes, GovernanceResponsbility {
        0x1::voting::resolve<0x1::governance_proposal::GovernanceProposal>(@0x1, arg0);
        remove_approved_hash(arg0);
        get_signer(arg1)
    }
    
    public entry fun vote(arg0: &signer, arg1: address, arg2: u64, arg3: bool) acquires ApprovedExecutionHashes, GovernanceEvents, VotingRecords, VotingRecordsV2 {
        vote_internal(arg0, arg1, arg2, 18446744073709551615, arg3);
    }
    
    public fun add_approved_script_hash(arg0: u64) acquires ApprovedExecutionHashes {
        let v0 = borrow_global_mut<ApprovedExecutionHashes>(@0x1);
        let v1 = 0x1::voting::get_proposal_state<0x1::governance_proposal::GovernanceProposal>(@0x1, arg0) == 1;
        assert!(v1, 0x1::error::invalid_argument(6));
        let v2 = 0x1::voting::get_execution_hash<0x1::governance_proposal::GovernanceProposal>(@0x1, arg0);
        if (0x1::simple_map::contains_key<u64, vector<u8>>(&v0.hashes, &arg0)) {
            *0x1::simple_map::borrow_mut<u64, vector<u8>>(&mut v0.hashes, &arg0) = v2;
        } else {
            0x1::simple_map::add<u64, vector<u8>>(&mut v0.hashes, arg0, v2);
        };
    }
    
    public entry fun add_approved_script_hash_script(arg0: u64) acquires ApprovedExecutionHashes {
        add_approved_script_hash(arg0);
    }
    
    fun assert_voting_initialization() {
        if (0x1::features::partial_governance_voting_enabled()) {
            assert!(exists<VotingRecordsV2>(@0x1), 0x1::error::invalid_state(13));
        };
    }
    
    fun create_proposal_metadata(arg0: vector<u8>, arg1: vector<u8>) : 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>> {
        let v0 = 0x1::string::utf8(arg0);
        assert!(0x1::string::length(&v0) <= 256, 0x1::error::invalid_argument(9));
        let v1 = 0x1::string::utf8(arg1);
        assert!(0x1::string::length(&v1) <= 256, 0x1::error::invalid_argument(10));
        let v2 = 0x1::simple_map::create<0x1::string::String, vector<u8>>();
        let v3 = 0x1::string::utf8(b"metadata_location");
        0x1::simple_map::add<0x1::string::String, vector<u8>>(&mut v2, v3, arg0);
        let v4 = 0x1::string::utf8(b"metadata_hash");
        0x1::simple_map::add<0x1::string::String, vector<u8>>(&mut v2, v4, arg1);
        v2
    }
    
    public fun create_proposal_v2_impl(arg0: &signer, arg1: address, arg2: vector<u8>, arg3: vector<u8>, arg4: vector<u8>, arg5: bool) : u64 acquires GovernanceConfig, GovernanceEvents {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(0x1::stake::get_delegated_voter(arg1) == v0, 0x1::error::invalid_argument(2));
        let v1 = borrow_global<GovernanceConfig>(@0x1);
        assert!(get_voting_power(arg1) >= v1.required_proposer_stake, 0x1::error::invalid_argument(1));
        let v2 = 0x1::timestamp::now_seconds() + v1.voting_duration_secs;
        assert!(0x1::stake::get_lockup_secs(arg1) >= v2, 0x1::error::invalid_argument(3));
        let v3 = create_proposal_metadata(arg3, arg4);
        let v4 = 0x1::coin::supply<0x1::aptos_coin::AptosCoin>();
        let v5 = 0x1::option::none<u128>();
        if (0x1::option::is_some<u128>(&v4)) {
            v5 = 0x1::option::some<u128>(*0x1::option::borrow<u128>(&v4) / 2 + 1);
        };
        let v6 = 0x1::governance_proposal::create_proposal();
        let v7 = v1.min_voting_threshold;
        let v8 = 0x1::voting::create_proposal_v2<0x1::governance_proposal::GovernanceProposal>(v0, @0x1, v6, arg2, v7, v2, v5, v3, arg5);
        let v9 = CreateProposalEvent{
            proposer          : v0, 
            stake_pool        : arg1, 
            proposal_id       : v8, 
            execution_hash    : arg2, 
            proposal_metadata : v3,
        };
        0x1::event::emit_event<CreateProposalEvent>(&mut borrow_global_mut<GovernanceEvents>(@0x1).create_proposal_events, v9);
        v8
    }
    
    public fun get_min_voting_threshold() : u128 acquires GovernanceConfig {
        borrow_global<GovernanceConfig>(@0x1).min_voting_threshold
    }
    
    public fun get_remaining_voting_power(arg0: address, arg1: u64) : u64 acquires VotingRecords, VotingRecordsV2 {
        assert_voting_initialization();
        let v0 = 0x1::voting::get_proposal_expiration_secs<0x1::governance_proposal::GovernanceProposal>(@0x1, arg1);
        if (v0 > 0x1::stake::get_lockup_secs(arg0) || 0x1::timestamp::now_seconds() > v0) {
            return 0
        };
        if (has_entirely_voted(arg0, arg1)) {
            return 0
        };
        let v1 = RecordKey{
            stake_pool  : arg0, 
            proposal_id : arg1,
        };
        let v2 = 0;
        if (0x1::features::partial_governance_voting_enabled()) {
            let v3 = &borrow_global<VotingRecordsV2>(@0x1).votes;
            let v4 = 0;
            v2 = *0x1::smart_table::borrow_with_default<RecordKey, u64>(v3, v1, &v4);
        };
        get_voting_power(arg0) - v2
    }
    
    public fun get_required_proposer_stake() : u64 acquires GovernanceConfig {
        borrow_global<GovernanceConfig>(@0x1).required_proposer_stake
    }
    
    fun get_signer(arg0: address) : signer acquires GovernanceResponsbility {
        let v0 = borrow_global<GovernanceResponsbility>(@0x1);
        let v1 = 0x1::simple_map::borrow<address, 0x1::account::SignerCapability>(&v0.signer_caps, &arg0);
        0x1::account::create_signer_with_capability(v1)
    }
    
    public fun get_signer_testnet_only(arg0: &signer, arg1: address) : signer acquires GovernanceResponsbility {
        0x1::system_addresses::assert_core_resource(arg0);
        assert!(0x1::aptos_coin::has_mint_capability(arg0), 0x1::error::unauthenticated(11));
        get_signer(arg1)
    }
    
    public fun get_voting_duration_secs() : u64 acquires GovernanceConfig {
        borrow_global<GovernanceConfig>(@0x1).voting_duration_secs
    }
    
    public fun get_voting_power(arg0: address) : u64 {
        let v0 = 0x1::staking_config::get();
        if (0x1::staking_config::get_allow_validator_set_change(&v0)) {
            let (v2, _, v4, v5) = 0x1::stake::get_stake(arg0);
            v2 + v4 + v5
        } else {
            0x1::stake::get_current_epoch_voting_power(arg0)
        }
    }
    
    public fun has_entirely_voted(arg0: address, arg1: u64) : bool acquires VotingRecords {
        let v0 = RecordKey{
            stake_pool  : arg0, 
            proposal_id : arg1,
        };
        0x1::table::contains<RecordKey, bool>(&borrow_global<VotingRecords>(@0x1).votes, v0)
    }
    
    fun initialize(arg0: &signer, arg1: u128, arg2: u64, arg3: u64) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::voting::register<0x1::governance_proposal::GovernanceProposal>(arg0);
        let v0 = GovernanceConfig{
            min_voting_threshold    : arg1, 
            required_proposer_stake : arg2, 
            voting_duration_secs    : arg3,
        };
        move_to<GovernanceConfig>(arg0, v0);
        let v1 = 0x1::account::new_event_handle<CreateProposalEvent>(arg0);
        let v2 = 0x1::account::new_event_handle<UpdateConfigEvent>(arg0);
        let v3 = 0x1::account::new_event_handle<VoteEvent>(arg0);
        let v4 = GovernanceEvents{
            create_proposal_events : v1, 
            update_config_events   : v2, 
            vote_events            : v3,
        };
        move_to<GovernanceEvents>(arg0, v4);
        let v5 = VotingRecords{votes: 0x1::table::new<RecordKey, bool>()};
        move_to<VotingRecords>(arg0, v5);
        let v6 = ApprovedExecutionHashes{hashes: 0x1::simple_map::create<u64, vector<u8>>()};
        move_to<ApprovedExecutionHashes>(arg0, v6);
    }
    
    public fun initialize_partial_voting(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = VotingRecordsV2{votes: 0x1::smart_table::new<RecordKey, u64>()};
        move_to<VotingRecordsV2>(arg0, v0);
    }
    
    public entry fun partial_vote(arg0: &signer, arg1: address, arg2: u64, arg3: u64, arg4: bool) acquires ApprovedExecutionHashes, GovernanceEvents, VotingRecords, VotingRecordsV2 {
        vote_internal(arg0, arg1, arg2, arg3, arg4);
    }
    
    public fun remove_approved_hash(arg0: u64) acquires ApprovedExecutionHashes {
        let v0 = 0x1::voting::is_resolved<0x1::governance_proposal::GovernanceProposal>(@0x1, arg0);
        assert!(v0, 0x1::error::invalid_argument(8));
        let v1 = &mut borrow_global_mut<ApprovedExecutionHashes>(@0x1).hashes;
        if (0x1::simple_map::contains_key<u64, vector<u8>>(v1, &arg0)) {
            let (_, _) = 0x1::simple_map::remove<u64, vector<u8>>(v1, &arg0);
        };
    }
    
    public fun resolve_multi_step_proposal(arg0: u64, arg1: address, arg2: vector<u8>) : signer acquires ApprovedExecutionHashes, GovernanceResponsbility {
        0x1::voting::resolve_proposal_v2<0x1::governance_proposal::GovernanceProposal>(@0x1, arg0, arg2);
        if (0x1::vector::length<u8>(&arg2) == 0) {
            remove_approved_hash(arg0);
        } else {
            add_approved_script_hash(arg0);
        };
        get_signer(arg1)
    }
    
    public fun store_signer_cap(arg0: &signer, arg1: address, arg2: 0x1::account::SignerCapability) acquires GovernanceResponsbility {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::system_addresses::assert_framework_reserved(arg1);
        if (!exists<GovernanceResponsbility>(@0x1)) {
            let v0 = 0x1::simple_map::create<address, 0x1::account::SignerCapability>();
            let v1 = GovernanceResponsbility{signer_caps: v0};
            move_to<GovernanceResponsbility>(arg0, v1);
        };
        let v2 = &mut borrow_global_mut<GovernanceResponsbility>(@0x1).signer_caps;
        0x1::simple_map::add<address, 0x1::account::SignerCapability>(v2, arg1, arg2);
    }
    
    public fun toggle_features(arg0: &signer, arg1: vector<u64>, arg2: vector<u64>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::features::change_feature_flags(arg0, arg1, arg2);
        0x1::reconfiguration::reconfigure();
    }
    
    public fun update_governance_config(arg0: &signer, arg1: u128, arg2: u64, arg3: u64) acquires GovernanceConfig, GovernanceEvents {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = borrow_global_mut<GovernanceConfig>(@0x1);
        v0.voting_duration_secs = arg3;
        v0.min_voting_threshold = arg1;
        v0.required_proposer_stake = arg2;
        let v1 = &mut borrow_global_mut<GovernanceEvents>(@0x1).update_config_events;
        let v2 = UpdateConfigEvent{
            min_voting_threshold    : arg1, 
            required_proposer_stake : arg2, 
            voting_duration_secs    : arg3,
        };
        0x1::event::emit_event<UpdateConfigEvent>(v1, v2);
    }
    
    fun vote_internal(arg0: &signer, arg1: address, arg2: u64, arg3: u64, arg4: bool) acquires ApprovedExecutionHashes, GovernanceEvents, VotingRecords, VotingRecordsV2 {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(0x1::stake::get_delegated_voter(arg1) == v0, 0x1::error::invalid_argument(2));
        let v1 = 0x1::voting::get_proposal_expiration_secs<0x1::governance_proposal::GovernanceProposal>(@0x1, arg2);
        assert!(0x1::stake::get_lockup_secs(arg1) >= v1, 0x1::error::invalid_argument(3));
        let v2 = get_remaining_voting_power(arg1, arg2);
        let v3 = 0x1::math64::min(arg3, v2);
        assert!(v3 > 0, 0x1::error::invalid_argument(5));
        let v4 = 0x1::governance_proposal::create_empty_proposal();
        0x1::voting::vote<0x1::governance_proposal::GovernanceProposal>(&v4, @0x1, arg2, v3, arg4);
        let v5 = RecordKey{
            stake_pool  : arg1, 
            proposal_id : arg2,
        };
        if (0x1::features::partial_governance_voting_enabled()) {
            let v6 = &mut borrow_global_mut<VotingRecordsV2>(@0x1).votes;
            let v7 = 0x1::smart_table::borrow_mut_with_default<RecordKey, u64>(v6, v5, 0);
            *v7 = *v7 + v3;
        } else {
            let v8 = borrow_global_mut<VotingRecords>(@0x1);
            assert!(!0x1::table::contains<RecordKey, bool>(&v8.votes, v5), 0x1::error::invalid_argument(4));
            0x1::table::add<RecordKey, bool>(&mut v8.votes, v5, true);
        };
        let v9 = VoteEvent{
            proposal_id : arg2, 
            voter       : v0, 
            stake_pool  : arg1, 
            num_votes   : v3, 
            should_pass : arg4,
        };
        0x1::event::emit_event<VoteEvent>(&mut borrow_global_mut<GovernanceEvents>(@0x1).vote_events, v9);
        if (0x1::voting::get_proposal_state<0x1::governance_proposal::GovernanceProposal>(@0x1, arg2) == 1) {
            add_approved_script_hash(arg2);
        };
    }
    
    // decompiled from Move bytecode v6
}
