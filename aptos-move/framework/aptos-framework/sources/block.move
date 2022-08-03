/// This module defines a struct storing the metadata of the block and new block events.
module aptos_framework::block {
    use std::error;
    use std::vector;
    use aptos_std::event;
    use aptos_std::event::EventHandle;

    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use aptos_framework::reconfiguration;
    use aptos_framework::stake;

    struct BlockMetadata has key {
        /// Height of the current block
        height: u64,
        /// Time period between epochs.
        epoch_interval: u64,
        /// Handle where events with the time of new blocks are emitted
        new_block_events: event::EventHandle<Self::NewBlockEvent>,
    }

    struct NewBlockEvent has drop, store {
        epoch: u64,
        round: u64,
        height: u64,
        previous_block_votes: vector<bool>,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        /// On-chain time during the block at the given height
        time_microseconds: u64,
    }

    /// The `BlockMetadata` resource is in an invalid state
    const EBLOCK_METADATA: u64 = 0;
    /// An invalid signer was provided. Expected the signer to be the VM or a Validator.
    const EVM_OR_VALIDATOR: u64 = 1;

    /// This can only be called during Genesis.
    public fun initialize_block_metadata(account: &signer, epoch_interval: u64) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(!is_initialized(), error::already_exists(EBLOCK_METADATA));
        move_to<BlockMetadata>(
            account,
            BlockMetadata {
                height: 0,
                epoch_interval,
                new_block_events: event::new_event_handle<Self::NewBlockEvent>(account),
            }
        );
    }

    /// Update the epoch interval.
    /// Can only be called as part of the Aptos governance proposal process established by the AptosGovernance module.
    public fun update_epoch_interval(
        aptos_framework: &signer,
        new_epoch_interval: u64,
    ) acquires BlockMetadata {
        system_addresses::assert_aptos_framework(aptos_framework);
        let block_metadata = borrow_global_mut<BlockMetadata>(@aptos_framework);
        block_metadata.epoch_interval = new_epoch_interval;
    }

    /// Helper function to determine whether this module has been initialized.
    fun is_initialized(): bool {
        exists<BlockMetadata>(@aptos_framework)
    }

    /// Set the metadata for the current block.
    /// The runtime always runs this before executing the transactions in a block.
    fun block_prologue(
        vm: signer,
        epoch: u64,
        round: u64,
        previous_block_votes: vector<bool>,
        missed_votes: vector<u64>,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        timestamp: u64
    ) acquires BlockMetadata {
        timestamp::assert_operating();
        // Operational constraint: can only be invoked by the VM.
        system_addresses::assert_vm(&vm);

        // Authorization
        assert!(
            proposer == @vm_reserved || stake::is_current_epoch_validator(proposer),
        error::permission_denied(EVM_OR_VALIDATOR)
        );

        let block_metadata_ref = borrow_global_mut<BlockMetadata>(@aptos_framework);
        block_metadata_ref.height = event::counter(&block_metadata_ref.new_block_events);

        let new_block_event = NewBlockEvent {
            epoch,
            round,
            height: block_metadata_ref.height,
            previous_block_votes,
            proposer,
            failed_proposer_indices,
            time_microseconds: timestamp,
        };
        emit_new_block_event(&vm, &mut block_metadata_ref.new_block_events, new_block_event);

        // Performance scores have to be updated before the epoch transition as the transaction that triggers the
        // transition is the last block in the previous epoch.
        stake::update_performance_statistics(missed_votes);

        if (timestamp - reconfiguration::last_reconfiguration_time() > block_metadata_ref.epoch_interval) {
            reconfiguration::reconfigure();
        };
    }

    /// Get the current block height
    public fun get_current_block_height(): u64 acquires BlockMetadata {
        assert!(is_initialized(), error::not_found(EBLOCK_METADATA));
        borrow_global<BlockMetadata>(@aptos_framework).height
    }

    /// Emit the event and update height and global timestamp
    fun emit_new_block_event(vm: &signer, event_handle: &mut EventHandle<NewBlockEvent>, new_block_event: NewBlockEvent) {
        timestamp::update_global_time(vm, new_block_event.proposer, new_block_event.time_microseconds);
        assert!(event::counter(event_handle) == new_block_event.height, error::invalid_argument(EBLOCK_METADATA));
        event::emit_event<NewBlockEvent>(event_handle, new_block_event);
    }

    /// Emit a `NewEpochEvent` event. This function will be invoked by genesis directly to generate the very first
    /// reconfiguration event.
    fun emit_genesis_block_event(vm: signer) acquires BlockMetadata {
        let block_metadata_ref = borrow_global_mut<BlockMetadata>(@aptos_framework);
        emit_new_block_event(
            &vm,
            &mut block_metadata_ref.new_block_events,
            NewBlockEvent {
                epoch: 0,
                round: 0,
                height: 0,
                previous_block_votes: vector::empty(),
                proposer: @vm_reserved,
                failed_proposer_indices: vector::empty(),
                time_microseconds: 0,
            }
        );
    }


    #[test(aptos_framework = @aptos_framework)]
    public entry fun test_update_epoch_interval(aptos_framework: signer) acquires BlockMetadata {
        initialize_block_metadata(&aptos_framework, 1);
        assert!(borrow_global<BlockMetadata>(@aptos_framework).epoch_interval == 1, 0);
        update_epoch_interval(&aptos_framework, 2);
        assert!(borrow_global<BlockMetadata>(@aptos_framework).epoch_interval == 2, 1);
    }

    #[test(aptos_framework = @aptos_framework, account = @0x123)]
    #[expected_failure(abort_code = 0x50002)]
    public entry fun test_update_epoch_interval_unauthorized_should_fail(
        aptos_framework: signer,
        account: signer,
    ) acquires BlockMetadata {
        initialize_block_metadata(&aptos_framework, 1);
        update_epoch_interval(&account, 2);
    }
}
