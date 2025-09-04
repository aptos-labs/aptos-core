/// This module defines a struct storing the metadata of the block and new block events.
module velor_framework::block {
    use std::error;
    use std::vector;
    use std::option;
    use velor_std::table_with_length::{Self, TableWithLength};
    use std::option::Option;
    use velor_framework::randomness;

    use velor_framework::account;
    use velor_framework::event::{Self, EventHandle};
    use velor_framework::reconfiguration;
    use velor_framework::reconfiguration_with_dkg;
    use velor_framework::stake;
    use velor_framework::state_storage;
    use velor_framework::system_addresses;
    use velor_framework::timestamp;

    friend velor_framework::genesis;

    const MAX_U64: u64 = 18446744073709551615;

    /// Should be in-sync with BlockResource rust struct in new_block.rs
    struct BlockResource has key {
        /// Height of the current block
        height: u64,
        /// Time period between epochs.
        epoch_interval: u64,
        /// Handle where events with the time of new blocks are emitted
        new_block_events: EventHandle<NewBlockEvent>,
        update_epoch_interval_events: EventHandle<UpdateEpochIntervalEvent>,
    }

    /// Store new block events as a move resource, internally using a circular buffer.
    struct CommitHistory has key {
        max_capacity: u32,
        next_idx: u32,
        table: TableWithLength<u32, NewBlockEvent>,
    }

    /// Should be in-sync with NewBlockEvent rust struct in new_block.rs
    struct NewBlockEvent has copy, drop, store {
        hash: address,
        epoch: u64,
        round: u64,
        height: u64,
        previous_block_votes_bitvec: vector<u8>,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        /// On-chain time during the block at the given height
        time_microseconds: u64,
    }

    /// Event emitted when a proposal is created.
    struct UpdateEpochIntervalEvent has drop, store {
        old_epoch_interval: u64,
        new_epoch_interval: u64,
    }

    #[event]
    /// Should be in-sync with NewBlockEvent rust struct in new_block.rs
    struct NewBlock has drop, store {
        hash: address,
        epoch: u64,
        round: u64,
        height: u64,
        previous_block_votes_bitvec: vector<u8>,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        /// On-chain time during the block at the given height
        time_microseconds: u64,
    }

    #[event]
    /// Event emitted when a proposal is created.
    struct UpdateEpochInterval has drop, store {
        old_epoch_interval: u64,
        new_epoch_interval: u64,
    }

    /// The number of new block events does not equal the current block height.
    const ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT: u64 = 1;
    /// An invalid proposer was provided. Expected the proposer to be the VM or an active validator.
    const EINVALID_PROPOSER: u64 = 2;
    /// Epoch interval cannot be 0.
    const EZERO_EPOCH_INTERVAL: u64 = 3;
    /// The maximum capacity of the commit history cannot be 0.
    const EZERO_MAX_CAPACITY: u64 = 3;

    /// This can only be called during Genesis.
    public(friend) fun initialize(velor_framework: &signer, epoch_interval_microsecs: u64) {
        system_addresses::assert_velor_framework(velor_framework);
        assert!(epoch_interval_microsecs > 0, error::invalid_argument(EZERO_EPOCH_INTERVAL));

        move_to<CommitHistory>(velor_framework, CommitHistory {
            max_capacity: 2000,
            next_idx: 0,
            table: table_with_length::new(),
        });

        move_to<BlockResource>(
            velor_framework,
            BlockResource {
                height: 0,
                epoch_interval: epoch_interval_microsecs,
                new_block_events: account::new_event_handle<NewBlockEvent>(velor_framework),
                update_epoch_interval_events: account::new_event_handle<UpdateEpochIntervalEvent>(velor_framework),
            }
        );
    }

    /// Initialize the commit history resource if it's not in genesis.
    public fun initialize_commit_history(fx: &signer, max_capacity: u32) {
        assert!(max_capacity > 0, error::invalid_argument(EZERO_MAX_CAPACITY));
        move_to<CommitHistory>(fx, CommitHistory {
            max_capacity,
            next_idx: 0,
            table: table_with_length::new(),
        });
    }

    /// Update the epoch interval.
    /// Can only be called as part of the Velor governance proposal process established by the VelorGovernance module.
    public fun update_epoch_interval_microsecs(
        velor_framework: &signer,
        new_epoch_interval: u64,
    ) acquires BlockResource {
        system_addresses::assert_velor_framework(velor_framework);
        assert!(new_epoch_interval > 0, error::invalid_argument(EZERO_EPOCH_INTERVAL));

        let block_resource = borrow_global_mut<BlockResource>(@velor_framework);
        let old_epoch_interval = block_resource.epoch_interval;
        block_resource.epoch_interval = new_epoch_interval;

        if (std::features::module_event_migration_enabled()) {
            event::emit(
                UpdateEpochInterval { old_epoch_interval, new_epoch_interval },
            );
        } else {
            event::emit_event<UpdateEpochIntervalEvent>(
                &mut block_resource.update_epoch_interval_events,
                UpdateEpochIntervalEvent { old_epoch_interval, new_epoch_interval },
            );
        };
    }

    #[view]
    /// Return epoch interval in seconds.
    public fun get_epoch_interval_secs(): u64 acquires BlockResource {
        borrow_global<BlockResource>(@velor_framework).epoch_interval / 1000000
    }


    fun block_prologue_common(
        vm: &signer,
        hash: address,
        epoch: u64,
        round: u64,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        previous_block_votes_bitvec: vector<u8>,
        timestamp: u64
    ): u64 acquires BlockResource, CommitHistory {
        // Operational constraint: can only be invoked by the VM.
        system_addresses::assert_vm(vm);

        // Blocks can only be produced by a valid proposer or by the VM itself for Nil blocks (no user txs).
        assert!(
            proposer == @vm_reserved || stake::is_current_epoch_validator(proposer),
            error::permission_denied(EINVALID_PROPOSER),
        );

        let proposer_index = option::none();
        if (proposer != @vm_reserved) {
            proposer_index = option::some(stake::get_validator_index(proposer));
        };

        let block_metadata_ref = borrow_global_mut<BlockResource>(@velor_framework);
        block_metadata_ref.height = event::counter(&block_metadata_ref.new_block_events);

        let new_block_event = NewBlockEvent {
            hash,
            epoch,
            round,
            height: block_metadata_ref.height,
            previous_block_votes_bitvec,
            proposer,
            failed_proposer_indices,
            time_microseconds: timestamp,
        };
        emit_new_block_event(vm, &mut block_metadata_ref.new_block_events, new_block_event);

        // Performance scores have to be updated before the epoch transition as the transaction that triggers the
        // transition is the last block in the previous epoch.
        stake::update_performance_statistics(proposer_index, failed_proposer_indices);
        state_storage::on_new_block(reconfiguration::current_epoch());

        block_metadata_ref.epoch_interval
    }

    /// Set the metadata for the current block.
    /// The runtime always runs this before executing the transactions in a block.
    fun block_prologue(
        vm: signer,
        hash: address,
        epoch: u64,
        round: u64,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        previous_block_votes_bitvec: vector<u8>,
        timestamp: u64
    ) acquires BlockResource, CommitHistory {
        let epoch_interval = block_prologue_common(&vm, hash, epoch, round, proposer, failed_proposer_indices, previous_block_votes_bitvec, timestamp);
        randomness::on_new_block(&vm, epoch, round, option::none());
        if (timestamp - reconfiguration::last_reconfiguration_time() >= epoch_interval) {
            reconfiguration::reconfigure();
        };
    }

    /// `block_prologue()` but trigger reconfiguration with DKG after epoch timed out.
    fun block_prologue_ext(
        vm: signer,
        hash: address,
        epoch: u64,
        round: u64,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        previous_block_votes_bitvec: vector<u8>,
        timestamp: u64,
        randomness_seed: Option<vector<u8>>,
    ) acquires BlockResource, CommitHistory {
        let epoch_interval = block_prologue_common(
            &vm,
            hash,
            epoch,
            round,
            proposer,
            failed_proposer_indices,
            previous_block_votes_bitvec,
            timestamp
        );
        randomness::on_new_block(&vm, epoch, round, randomness_seed);

        if (timestamp - reconfiguration::last_reconfiguration_time() >= epoch_interval) {
            reconfiguration_with_dkg::try_start();
        };
    }

    fun block_epilogue(
        vm: &signer,
        fee_distribution_validator_indices: vector<u64>,
        fee_amounts_octa: vector<u64>,
    ) {
        stake::record_fee(vm, fee_distribution_validator_indices, fee_amounts_octa);
    }

    #[view]
    /// Get the current block height
    public fun get_current_block_height(): u64 acquires BlockResource {
        borrow_global<BlockResource>(@velor_framework).height
    }

    /// Emit the event and update height and global timestamp
    fun emit_new_block_event(
        vm: &signer,
        event_handle: &mut EventHandle<NewBlockEvent>,
        new_block_event: NewBlockEvent,
    ) acquires CommitHistory {
        if (exists<CommitHistory>(@velor_framework)) {
            let commit_history_ref = borrow_global_mut<CommitHistory>(@velor_framework);
            let idx = commit_history_ref.next_idx;
            if (table_with_length::contains(&commit_history_ref.table, idx)) {
                table_with_length::remove(&mut commit_history_ref.table, idx);
            };
            table_with_length::add(&mut commit_history_ref.table, idx, copy new_block_event);
            spec {
                assume idx + 1 <= MAX_U32;
            };
            commit_history_ref.next_idx = (idx + 1) % commit_history_ref.max_capacity;
        };
        timestamp::update_global_time(vm, new_block_event.proposer, new_block_event.time_microseconds);
        assert!(
            event::counter(event_handle) == new_block_event.height,
            error::invalid_argument(ENUM_NEW_BLOCK_EVENTS_DOES_NOT_MATCH_BLOCK_HEIGHT),
        );
        event::emit_event<NewBlockEvent>(event_handle, new_block_event);
    }

    /// Emit a `NewBlockEvent` event. This function will be invoked by genesis directly to generate the very first
    /// reconfiguration event.
    fun emit_genesis_block_event(vm: signer) acquires BlockResource, CommitHistory {
        let block_metadata_ref = borrow_global_mut<BlockResource>(@velor_framework);
        let genesis_id = @0x0;
        emit_new_block_event(
            &vm,
            &mut block_metadata_ref.new_block_events,
            NewBlockEvent {
                hash: genesis_id,
                epoch: 0,
                round: 0,
                height: 0,
                previous_block_votes_bitvec: vector::empty(),
                proposer: @vm_reserved,
                failed_proposer_indices: vector::empty(),
                time_microseconds: 0,
            },
        );
    }

    ///  Emit a `NewBlockEvent` event. This function will be invoked by write set script directly to generate the
    ///  new block event for WriteSetPayload.
    public fun emit_writeset_block_event(vm_signer: &signer, fake_block_hash: address) acquires BlockResource, CommitHistory {
        system_addresses::assert_vm(vm_signer);
        let block_metadata_ref = borrow_global_mut<BlockResource>(@velor_framework);
        block_metadata_ref.height = event::counter(&block_metadata_ref.new_block_events);

        emit_new_block_event(
            vm_signer,
            &mut block_metadata_ref.new_block_events,
            NewBlockEvent {
                hash: fake_block_hash,
                epoch: reconfiguration::current_epoch(),
                round: MAX_U64,
                height: block_metadata_ref.height,
                previous_block_votes_bitvec: vector::empty(),
                proposer: @vm_reserved,
                failed_proposer_indices: vector::empty(),
                time_microseconds: timestamp::now_microseconds(),
            },
        );
    }

    #[test_only]
    public fun initialize_for_test(account: &signer, epoch_interval_microsecs: u64) {
        initialize(account, epoch_interval_microsecs);
    }

    #[test(velor_framework = @velor_framework)]
    public entry fun test_update_epoch_interval(velor_framework: signer) acquires BlockResource {
        account::create_account_for_test(@velor_framework);
        initialize(&velor_framework, 1);
        assert!(borrow_global<BlockResource>(@velor_framework).epoch_interval == 1, 0);
        update_epoch_interval_microsecs(&velor_framework, 2);
        assert!(borrow_global<BlockResource>(@velor_framework).epoch_interval == 2, 1);
    }

    #[test(velor_framework = @velor_framework, account = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = velor_framework::system_addresses)]
    public entry fun test_update_epoch_interval_unauthorized_should_fail(
        velor_framework: signer,
        account: signer,
    ) acquires BlockResource {
        account::create_account_for_test(@velor_framework);
        initialize(&velor_framework, 1);
        update_epoch_interval_microsecs(&account, 2);
    }
}
