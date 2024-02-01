module 0x1::reconfiguration {
    struct Configuration has key {
        epoch: u64,
        last_reconfiguration_time: u64,
        events: 0x1::event::EventHandle<NewEpochEvent>,
    }
    
    struct DisableReconfiguration has key {
        dummy_field: bool,
    }
    
    struct NewEpochEvent has drop, store {
        epoch: u64,
    }
    
    public fun current_epoch() : u64 acquires Configuration {
        borrow_global<Configuration>(@0x1).epoch
    }
    
    fun disable_reconfiguration(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(reconfiguration_enabled(), 0x1::error::invalid_state(1));
        let v0 = DisableReconfiguration{dummy_field: false};
        move_to<DisableReconfiguration>(arg0, v0);
    }
    
    fun emit_genesis_reconfiguration_event() acquires Configuration {
        let v0 = borrow_global_mut<Configuration>(@0x1);
        assert!(v0.epoch == 0 && v0.last_reconfiguration_time == 0, 0x1::error::invalid_state(1));
        v0.epoch = 1;
        let v1 = NewEpochEvent{epoch: v0.epoch};
        0x1::event::emit_event<NewEpochEvent>(&mut v0.events, v1);
    }
    
    fun enable_reconfiguration(arg0: &signer) acquires DisableReconfiguration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(!reconfiguration_enabled(), 0x1::error::invalid_state(1));
        let DisableReconfiguration {  } = move_from<DisableReconfiguration>(0x1::signer::address_of(arg0));
    }
    
    public(friend) fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = 0x1::account::get_guid_next_creation_num(0x1::signer::address_of(arg0)) == 2;
        assert!(v0, 0x1::error::invalid_state(5));
        let v1 = 0x1::account::new_event_handle<NewEpochEvent>(arg0);
        let v2 = Configuration{
            epoch                     : 0, 
            last_reconfiguration_time : 0, 
            events                    : v1,
        };
        move_to<Configuration>(arg0, v2);
    }
    
    public fun last_reconfiguration_time() : u64 acquires Configuration {
        borrow_global<Configuration>(@0x1).last_reconfiguration_time
    }
    
    fun reconfiguration_enabled() : bool {
        !exists<DisableReconfiguration>(@0x1)
    }
    
    public(friend) fun reconfigure() acquires Configuration {
        if (0x1::chain_status::is_genesis() || 0x1::timestamp::now_microseconds() == 0 || !reconfiguration_enabled()) {
            return
        };
        let v0 = borrow_global_mut<Configuration>(@0x1);
        let v1 = 0x1::timestamp::now_microseconds();
        if (v1 == v0.last_reconfiguration_time) {
            return
        };
        if (0x1::features::collect_and_distribute_gas_fees()) {
            0x1::transaction_fee::process_collected_fees();
        };
        0x1::stake::on_new_epoch();
        0x1::storage_gas::on_reconfig();
        assert!(v1 > v0.last_reconfiguration_time, 0x1::error::invalid_state(4));
        v0.last_reconfiguration_time = v1;
        v0.epoch = v0.epoch + 1;
        let v2 = NewEpochEvent{epoch: v0.epoch};
        0x1::event::emit_event<NewEpochEvent>(&mut v0.events, v2);
    }
    
    // decompiled from Move bytecode v6
}
