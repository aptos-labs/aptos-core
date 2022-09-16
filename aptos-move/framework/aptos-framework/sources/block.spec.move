spec aptos_framework::block {
    spec module {
        use aptos_std::chain_status;
        // After genesis, `BlockResource` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<BlockResource>(@aptos_framework);
    }

    spec block_prologue {
        use aptos_framework::chain_status;
        requires chain_status::is_operating();
        requires system_addresses::is_vm(vm);
        requires proposer == @vm_reserved || stake::spec_is_current_epoch_validator(proposer);
        requires timestamp >= reconfiguration::last_reconfiguration_time();
        requires (proposer == @vm_reserved) ==> (timestamp::spec_now_microseconds() == timestamp);
        requires (proposer != @vm_reserved) ==> (timestamp::spec_now_microseconds() < timestamp);

        aborts_if false;
    }

    spec emit_genesis_block_event {
        use aptos_framework::chain_status;

        requires chain_status::is_operating();
        requires system_addresses::is_vm(vm);
        requires event::counter(global<BlockResource>(@aptos_framework).new_block_events) == 0;
        requires (timestamp::spec_now_microseconds() == 0);

        aborts_if false;
    }

    spec emit_new_block_event {
        use aptos_framework::chain_status;
        let proposer = new_block_event.proposer;
        let timestamp = new_block_event.time_microseconds;

        requires chain_status::is_operating();
        requires system_addresses::is_vm(vm);
        requires (proposer == @vm_reserved) ==> (timestamp::spec_now_microseconds() == timestamp);
        requires (proposer != @vm_reserved) ==> (timestamp::spec_now_microseconds() < timestamp);
        requires event::counter(event_handle) == new_block_event.height;

        aborts_if false;
    }
}
