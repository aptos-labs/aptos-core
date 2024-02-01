module 0x1::block {
    struct BlockResource has key {
        height: u64,
        epoch_interval: u64,
        new_block_events: 0x1::event::EventHandle<NewBlockEvent>,
        update_epoch_interval_events: 0x1::event::EventHandle<UpdateEpochIntervalEvent>,
    }
    
    struct NewBlockEvent has drop, store {
        hash: address,
        epoch: u64,
        round: u64,
        height: u64,
        previous_block_votes_bitvec: vector<u8>,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        time_microseconds: u64,
    }
    
    struct UpdateEpochIntervalEvent has drop, store {
        old_epoch_interval: u64,
        new_epoch_interval: u64,
    }
    
    fun block_prologue(arg0: signer, arg1: address, arg2: u64, arg3: u64, arg4: address, arg5: vector<u64>, arg6: vector<u8>, arg7: u64) acquires BlockResource {
        0x1::system_addresses::assert_vm(&arg0);
        assert!(arg4 == @0x3001 || 0x1::stake::is_current_epoch_validator(arg4), 0x1::error::permission_denied(2));
        let v0 = 0x1::option::none<u64>();
        if (arg4 != @0x3001) {
            v0 = 0x1::option::some<u64>(0x1::stake::get_validator_index(arg4));
        };
        let v1 = borrow_global_mut<BlockResource>(@0x1);
        v1.height = 0x1::event::counter<NewBlockEvent>(&v1.new_block_events);
        let v2 = v1.height;
        let v3 = NewBlockEvent{
            hash                        : arg1, 
            epoch                       : arg2, 
            round                       : arg3, 
            height                      : v2, 
            previous_block_votes_bitvec : arg6, 
            proposer                    : arg4, 
            failed_proposer_indices     : arg5, 
            time_microseconds           : arg7,
        };
        emit_new_block_event(&arg0, &mut v1.new_block_events, v3);
        if (0x1::features::collect_and_distribute_gas_fees()) {
            0x1::transaction_fee::process_collected_fees();
            0x1::transaction_fee::register_proposer_for_fee_collection(arg4);
        };
        0x1::stake::update_performance_statistics(v0, arg5);
        0x1::state_storage::on_new_block(0x1::reconfiguration::current_epoch());
        if (arg7 - 0x1::reconfiguration::last_reconfiguration_time() >= v1.epoch_interval) {
            0x1::reconfiguration::reconfigure();
        };
    }
    
    fun emit_genesis_block_event(arg0: signer) acquires BlockResource {
        let v0 = 0x1::vector::empty<u8>();
        let v1 = 0x1::vector::empty<u64>();
        let v2 = NewBlockEvent{
            hash                        : @0x0, 
            epoch                       : 0, 
            round                       : 0, 
            height                      : 0, 
            previous_block_votes_bitvec : v0, 
            proposer                    : @0x3001, 
            failed_proposer_indices     : v1, 
            time_microseconds           : 0,
        };
        emit_new_block_event(&arg0, &mut borrow_global_mut<BlockResource>(@0x1).new_block_events, v2);
    }
    
    fun emit_new_block_event(arg0: &signer, arg1: &mut 0x1::event::EventHandle<NewBlockEvent>, arg2: NewBlockEvent) {
        0x1::timestamp::update_global_time(arg0, arg2.proposer, arg2.time_microseconds);
        assert!(0x1::event::counter<NewBlockEvent>(arg1) == arg2.height, 0x1::error::invalid_argument(1));
        0x1::event::emit_event<NewBlockEvent>(arg1, arg2);
    }
    
    public fun emit_writeset_block_event(arg0: &signer, arg1: address) acquires BlockResource {
        0x1::system_addresses::assert_vm(arg0);
        let v0 = borrow_global_mut<BlockResource>(@0x1);
        v0.height = 0x1::event::counter<NewBlockEvent>(&v0.new_block_events);
        let v1 = 0x1::reconfiguration::current_epoch();
        let v2 = v0.height;
        let v3 = 0x1::vector::empty<u8>();
        let v4 = 0x1::vector::empty<u64>();
        let v5 = 0x1::timestamp::now_microseconds();
        let v6 = NewBlockEvent{
            hash                        : arg1, 
            epoch                       : v1, 
            round                       : 18446744073709551615, 
            height                      : v2, 
            previous_block_votes_bitvec : v3, 
            proposer                    : @0x3001, 
            failed_proposer_indices     : v4, 
            time_microseconds           : v5,
        };
        0x1::event::emit_event<NewBlockEvent>(&mut v0.new_block_events, v6);
    }
    
    public fun get_current_block_height() : u64 acquires BlockResource {
        borrow_global<BlockResource>(@0x1).height
    }
    
    public fun get_epoch_interval_secs() : u64 acquires BlockResource {
        borrow_global<BlockResource>(@0x1).epoch_interval / 1000000
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: u64) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(arg1 > 0, 0x1::error::invalid_argument(3));
        let v0 = 0x1::account::new_event_handle<NewBlockEvent>(arg0);
        let v1 = 0x1::account::new_event_handle<UpdateEpochIntervalEvent>(arg0);
        let v2 = BlockResource{
            height                       : 0, 
            epoch_interval               : arg1, 
            new_block_events             : v0, 
            update_epoch_interval_events : v1,
        };
        move_to<BlockResource>(arg0, v2);
    }
    
    public fun update_epoch_interval_microsecs(arg0: &signer, arg1: u64) acquires BlockResource {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(arg1 > 0, 0x1::error::invalid_argument(3));
        let v0 = borrow_global_mut<BlockResource>(@0x1);
        v0.epoch_interval = arg1;
        let v1 = UpdateEpochIntervalEvent{
            old_epoch_interval : v0.epoch_interval, 
            new_epoch_interval : arg1,
        };
        0x1::event::emit_event<UpdateEpochIntervalEvent>(&mut v0.update_epoch_interval_events, v1);
    }
    
    // decompiled from Move bytecode v6
}
