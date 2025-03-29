spec aptos_framework::block {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During the module's initialization, it guarantees that the BlockResource resource moves under the
    /// Aptos framework account with initial values.
    /// Criticality: High
    /// Implementation: The initialize function is responsible for setting up the initial state of the module, ensuring
    /// that the following conditions are met (1) the BlockResource resource is created, indicating its existence within
    /// the module's context, and moved under the Aptos framework account, (2) the block height is set to zero during
    /// initialization, and (3) the epoch interval is greater than zero.
    /// Enforcement: Formally Verified via [high-level-req-1](Initialize).
    ///
    /// No.: 2
    /// Requirement: Only the Aptos framework address may execute the following functionalities: (1) initialize
    /// BlockResource, and (2) update the epoch interval.
    /// Criticality: Critical
    /// Implementation: The initialize and  update_epoch_interval_microsecs functions ensure that only aptos_framework
    /// can call them.
    /// Enforcement: Formally Verified via [high-level-req-2.1](Initialize) and [high-level-req-2.2](update_epoch_interval_microsecs).
    ///
    /// No.: 3
    /// Requirement: When updating the epoch interval, its value must be greater than zero and BlockResource must exist.
    /// Criticality: High
    /// Implementation: The update_epoch_interval_microsecs function asserts that new_epoch_interval is greater than
    /// zero and updates BlockResource's state.
    /// Enforcement: Formally verified via [high-level-req-3.1](UpdateEpochIntervalMicrosecs) and [high-level-req-3.2](epoch_interval).
    ///
    /// No.: 4
    /// Requirement: Only a valid proposer or the virtual machine is authorized to produce blocks.
    /// Criticality: Critical
    /// Implementation: During the execution of the block_prologue function, the validity of the proposer address is
    /// verified when setting the metadata for the current block.
    /// Enforcement: Formally Verified via [high-level-req-4](block_prologue).
    ///
    /// No.: 5
    /// Requirement: While emitting a new block event, the number of them is equal to the current block height.
    /// Criticality: Medium
    /// Implementation: The emit_new_block_event function asserts that the number of new block events equals the current
    /// block height.
    /// Enforcement: Formally Verified via [high-level-req-5](emit_new_block_event).
    /// </high-level-req>
    ///
    spec module {
        use aptos_framework::chain_status;
        pragma verify = false;
        // After genesis, `BlockResource` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<BlockResource>(@aptos_framework);
        // After genesis, `CommitHistory` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<CommitHistory>(@aptos_framework);
    }

    spec BlockResource {
        /// [high-level-req-3.2]
        invariant epoch_interval > 0;
    }

    spec CommitHistory {
        invariant max_capacity > 0;
    }

    spec block_prologue_common {
        pragma verify_duration_estimate = 1000; // TODO: set because of timeout (property proved)
        include BlockRequirement;
        aborts_if false;
    }

    spec block_prologue {

        pragma verify_duration_estimate = 1000; // TODO: set because of timeout (property proved)
        requires timestamp >= reconfiguration::last_reconfiguration_time();
        include BlockRequirement;
        aborts_if false;
    }

    spec block_prologue_ext {
        pragma verify_duration_estimate = 1000; // TODO: set because of timeout (property proved)
        requires timestamp >= reconfiguration::last_reconfiguration_time();
        include BlockRequirement;
        include stake::ResourceRequirement;
        include stake::GetReconfigStartTimeRequirement;
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
        /// [high-level-req-5]
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
        use std::signer;
        /// [high-level-req-1]
        include Initialize;
        include NewEventHandle;

        let addr = signer::address_of(aptos_framework);
        let account = global<account::Account>(addr);
        aborts_if account.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
    }

    spec schema BlockRequirement {
        use aptos_framework::chain_status;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::staking_config;

        vm: signer;
        hash: address;
        epoch: u64;
        round: u64;
        proposer: address;
        failed_proposer_indices: vector<u64>;
        previous_block_votes_bitvec: vector<u8>;
        timestamp: u64;

        requires chain_status::is_operating();
        requires system_addresses::is_vm(vm);
        /// [high-level-req-4]
        requires proposer == @vm_reserved || stake::spec_is_current_epoch_validator(proposer);
        requires (proposer == @vm_reserved) ==> (timestamp::spec_now_microseconds() == timestamp);
        requires (proposer != @vm_reserved) ==> (timestamp::spec_now_microseconds() < timestamp);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        include staking_config::StakingRewardsConfigRequirement;
    }

    spec schema Initialize {
        use std::signer;
        aptos_framework: signer;
        epoch_interval_microsecs: u64;

        let addr = signer::address_of(aptos_framework);
        /// [high-level-req-2.1]
        aborts_if addr != @aptos_framework;
        aborts_if epoch_interval_microsecs == 0;
        aborts_if exists<BlockResource>(addr);
        aborts_if exists<CommitHistory>(addr);
        ensures exists<BlockResource>(addr);
        ensures exists<CommitHistory>(addr);
        ensures global<BlockResource>(addr).height == 0;
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
        /// [high-level-req-3.1]
        include UpdateEpochIntervalMicrosecs;
    }

    spec schema UpdateEpochIntervalMicrosecs {
        use std::signer;
        aptos_framework: signer;
        new_epoch_interval: u64;

        let addr = signer::address_of(aptos_framework);

        /// [high-level-req-2.2]
        aborts_if addr != @aptos_framework;
        aborts_if new_epoch_interval == 0;
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
        use aptos_framework::chain_status;
        requires chain_status::is_operating();
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
