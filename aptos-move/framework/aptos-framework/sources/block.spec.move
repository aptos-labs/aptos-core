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

    /// The caller is aptos_framework.
    /// The new_epoch_interval must be greater than 0.
    /// The BlockResource is not under the caller before initializing.
    /// The Account is not under the caller until the BlockResource is created for the caller.
    /// Make sure The BlockResource under the caller existed after initializing.
    /// The number of new events created does not exceed MAX_U64.
    spec initialize(aptos_framework: &signer, epoch_interval_microsecs: u64) {
        include Initialize;
        include NewEventHandle;
    }

    spec schema Initialize {
        use std::signer;
        aptos_framework: signer;
        epoch_interval_microsecs: u64;

        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if epoch_interval_microsecs <= 0;
        aborts_if exists<BlockResource>(addr);
        ensures exists<BlockResource>(addr);
    }

    spec schema NewEventHandle {
        use std::signer;
        aptos_framework: signer;

        let addr = signer::address_of(aptos_framework);
        let account = global<account::Account>(addr);
        aborts_if !exists<account::Account>(addr);
        aborts_if account.guid_creation_num + 2 > MAX_U64;
    }

    /// The caller is @aptos_framework.
    /// The new_epoch_interval must be greater than 0.
    /// The BlockResource existed under the @aptos_framework.
    spec update_epoch_interval_microsecs(
        aptos_framework: &signer,
        new_epoch_interval: u64,
    ) {
        include UpdateEpochIntervalMicrosecs;
    }

    spec schema UpdateEpochIntervalMicrosecs {
        use std::signer;
        aptos_framework: signer;
        new_epoch_interval: u64;

        let addr = signer::address_of(aptos_framework);

        aborts_if addr != @aptos_framework;
        aborts_if new_epoch_interval <= 0;
        aborts_if !exists<BlockResource>(addr);
        let post block_resource = global<BlockResource>(addr);
        ensures block_resource.epoch_interval == new_epoch_interval;
    }

    spec get_epoch_interval_secs(): u64 {
        aborts_if !exists<BlockResource>(@aptos_framework);
    }

    spec get_current_block_height(): u64 {
        aborts_if !exists<BlockResource>(@aptos_framework);
    }

    /// The caller is @vm_reserved.
    /// The BlockResource existed under the @aptos_framework.
    /// The Configuration existed under the @aptos_framework.
    /// The CurrentTimeMicroseconds existed under the @aptos_framework.
    spec emit_writeset_block_event(vm_signer: &signer, fake_block_hash: address) {
        include EmitWritesetBlockEvent;
    }

    spec schema EmitWritesetBlockEvent {
        use std::signer;
        vm_signer: signer;

        let addr = signer::address_of(vm_signer);
        aborts_if addr != @vm_reserved;
        aborts_if !exists<BlockResource>(@aptos_framework);
        aborts_if !exists<reconfiguration::Configuration>(@aptos_framework);
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }
}
