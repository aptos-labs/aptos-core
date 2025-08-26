module aptos_framework::scheduled_txns {
    use std::bcs;
    use std::error;
    use std::hash::sha3_256;
    use std::option::{Option, some};
    use std::signer;
    use std::vector;
    use aptos_std::from_bcs;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::coin;
    use aptos_framework::coin::ensure_paired_metadata;
    use aptos_framework::event;
    use aptos_framework::fungible_asset::upgrade_store_to_concurrent;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;

    #[test_only]
    use aptos_framework::transaction_fee;

    friend aptos_framework::block;
    friend aptos_framework::transaction_validation;
    #[test_only]
    friend aptos_framework::test_scheduled_txns;

    /// Map key already exists
    const EINVALID_SIGNER: u64 = 1;

    /// Scheduled time is in the past
    const EINVALID_TIME: u64 = 2;

    /// Scheduling is stopped
    const EUNAVAILABLE: u64 = 3;

    /// Gas unit price is too low
    const ELOW_GAS_UNIT_PRICE: u64 = 4;

    // todo: should we also specify a minimum 'max_gas_amount' ?

    /// Txn size is too large; beyond 10KB
    const ETXN_TOO_LARGE: u64 = 5;

    /// Indicates error in SHA3-256 generation
    const EINVALID_HASH_SIZE: u64 = 6;

    /// Trying to start shutdown when module is not in Active state
    const EINVALID_SHUTDOWN_START: u64 = 7;

    /// Shutdown attempted without starting it
    const EINVALID_SHUTDOWN_ATTEMPT: u64 = 8;

    /// Shutdown is already in progress
    const ESHUTDOWN_IN_PROGRESS: u64 = 9;

    /// Can be paused only when module is in Active state
    const EINVALID_PAUSE_ATTEMPT: u64 = 10;

    /// Can be paused only when module is in Paused state
    const EINVALID_UNPAUSE_ATTEMPT: u64 = 11;

    const U64_MAX: u64 = 18446744073709551615;

    /// If we cannot schedule in 10s, we will abort the txn
    const EXPIRY_DELTA_DEFAULT: u64 = 10 * 1000;

    /// Maximum number of scheduled transactions that can be run in a block
    const GET_READY_TRANSACTIONS_LIMIT: u64 = 100;

    /// Maximum number of transactions that can be expired during block prologue
    const EXPIRE_TRANSACTIONS_LIMIT: u64 = GET_READY_TRANSACTIONS_LIMIT * 2;

    /// Maximum number of transactions that can be cancelled in a block during shutdown
    const SHUTDOWN_CANCEL_LIMIT: u64 = GET_READY_TRANSACTIONS_LIMIT * 2;

    /// SHA3-256 produces 32 bytes
    const TXN_ID_SIZE: u16 = 32;

    /// The average size of a scheduled transaction to provide an estimate of leaf nodes of BigOrderedMap
    const AVG_SCHED_TXN_SIZE: u16 = 1024;

    /// Max size of a scheduled transaction; 1MB for now as we are bounded by the slot size
    const MAX_SCHED_TXN_SIZE: u64 = 1024 * 1024;

    /// Framework owned address that stores the deposits for all scheduled txns
    const DEPOSIT_STORE_OWNER_ADDR: address = @0xb;

    enum ScheduledFunction has copy, store, drop {
         V1(|Option<signer>| has copy + store + drop),
    }

    /// ScheduledTransaction with scheduled_time, gas params, and function
    struct ScheduledTransaction has copy, drop, store {
        /// 32 bytes
        sender_addr: address,
        /// UTC timestamp in milliseconds
        scheduled_time_ms: u64,
        /// Maximum gas to spend for this transaction
        max_gas_amount: u64,
        /// Gas unit price that the user is willing to pay for this txn when it is scheduled
        gas_unit_price: u64,
        /// Option to pass a signer to the function
        pass_signer: bool,
        /// Variables are captured in the closure; optionally a signer is passed; no return
        f: ScheduledFunction
    }

    /// We pass around only needed info
    struct ScheduledTransactionInfoWithKey has drop {
        sender_addr: address,
        max_gas_amount: u64,
        gas_unit_price: u64,
        key: ScheduleMapKey
    }

    /// First sorted in ascending order of time, then on gas priority, and finally on txn_id
    /// The goal is to have fixed (less variable) size 'key', 'val' entries in BigOrderedMap, hence we use txn_id
    /// as a key. That is we have "{time, gas_priority, txn_id} -> ScheduledTxn" instead of
    /// "{time, gas_priority} --> List<(txn_id, ScheduledTxn)>".
    /// Note: ScheduledTxn is still variable size though due to its closure.
    struct ScheduleMapKey has copy, drop, store {
        /// UTC timestamp ms
        time: u64,
        /// gas_priority = U64_MAX - gas_unit_price; we want higher gas_unit_price to come before lower gas_unit_price
        gas_priority: u64,
        /// SHA3-256
        txn_id: u256
    }

    /// Dummy struct to use as a value type in BigOrderedMap
    struct Empty has copy, drop, store {}

    struct ScheduleQueue has key {
        /// key_size = 48 bytes
        schedule_map: BigOrderedMap<ScheduleMapKey, Empty>,
        ///
        txn_tbl: Table<u256, ScheduledTransaction>
    }

    /// BigOrderedMap has MAX_NODE_BYTES = 409600 (400KB), MAX_DEGREE = 4096, DEFAULT_TARGET_NODE_SIZE = 4096;
    const BIG_ORDRD_MAP_TGT_ND_SZ: u16 = 4096;
    const SCHEDULE_MAP_KEY_SIZE: u16 = TXN_ID_SIZE + 8 + 8; // 32 + 8 + 8 = 48 bytes

    enum ScheduledTxnsModuleStatus has copy, store, drop {
        /// Module is initialized and ready to use
        Active,
        /// Module is paused, no new transactions can be scheduled and existing transactions will not be executed
        Paused,
        /// Shutdown is in progress
        ShutdownInProgress,
        /// Shutdown is complete
        ShutdownComplete
    }

    /// Signer for the store for gas fee deposits
    struct AuxiliaryData has key {
        // todo: check if this is secure
        gas_fee_deposit_store_signer_cap: account::SignerCapability,
        module_status: ScheduledTxnsModuleStatus,
        /// If run outside the expiry_delta (from the time txn is expected to run), we will abort the txn
        expiry_delta: u64
    }

    /// We want reduce the contention while scheduled txns are being executed
    // todo: check if 32 is a good number
    const TO_REMOVE_PARALLELISM: u64 = 32;
    struct ToRemoveTbl has key {
        remove_tbl: Table<u16, vector<ScheduleMapKey>>
    }

    enum CancelledTxnCode has drop, store {
        /// Scheduling service is stopped
        Shutdown,
        /// Transaction was expired
        Expired,
        /// Transcation failed to execute
        Failed
    }

    #[event]
    struct TransactionScheduledEvent has drop, store {
        block_time_ms: u64,
        scheduled_txn_hash: u256,
        sender_addr: address,
        scheduled_time_ms: u64,
        max_gas_amount: u64,
        gas_unit_price: u64
    }

    #[event]
    struct TransactionFailedEvent has drop, store {
        scheduled_txn_time: u64,
        scheduled_txn_hash: u256,
        sender_addr: address,
        cancelled_txn_code: CancelledTxnCode
    }

    #[event]
    struct ShutdownEvent has drop, store {
        complete: bool
    }

    // temporary non persistent struct
    struct KeyAndTxnInfo has drop {
        key: ScheduleMapKey,
        account_addr: address,
        deposit_amt: u64
    }

    /// Can be called only by the framework
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);

        // Create owner account for handling deposits
        let owner_addr = DEPOSIT_STORE_OWNER_ADDR;
        let (owner_signer, owner_cap) =
            account::create_framework_reserved_account(owner_addr);

        // Initialize fungible store for the owner
        let metadata = ensure_paired_metadata<AptosCoin>();
        let deposit_store =
            primary_fungible_store::ensure_primary_store_exists(
                signer::address_of(&owner_signer), metadata
            );
        upgrade_store_to_concurrent(&owner_signer, deposit_store);

        // Store the capability
        move_to(
            framework,
            AuxiliaryData {
                gas_fee_deposit_store_signer_cap: owner_cap,
                module_status: ScheduledTxnsModuleStatus::Active,
                expiry_delta: EXPIRY_DELTA_DEFAULT
            }
        );

        // Initialize queue
        let queue = ScheduleQueue {
            schedule_map: big_ordered_map::new_with_reusable(),
            txn_tbl: table::new<u256, ScheduledTransaction>()
        };
        move_to(framework, queue);

        // Initialize remove_tbl with empty vectors for all slots
        let remove_tbl = table::new<u16, vector<ScheduleMapKey>>();
        let i: u16 = 0;
        while ((i as u64) < TO_REMOVE_PARALLELISM) {
            remove_tbl.add(i, vector::empty<ScheduleMapKey>());
            i = i + 1;
        };

        // Parallelizable data structure used to track executed txn_ids.
        move_to(framework, ToRemoveTbl { remove_tbl });
    }

    /// Starts the shutdown process. Can only be called when module status is Active.
    /// We need a governance proposal to shutdown the module. Possible reasons to shutdown are:
    ///      (a) the stakeholders decide the feature is no longer needed
    ///      (b) there is an invariant violation detected, and the only way out is to shutdown and cancel all txns
    public fun start_shutdown(framework: &signer) acquires ScheduleQueue, ToRemoveTbl, AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);

        let aux_data = borrow_global_mut<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
            error::invalid_state(EINVALID_SHUTDOWN_START)
        );
        aux_data.module_status = ScheduledTxnsModuleStatus::ShutdownInProgress;

        // remove txns that have already been run
        remove_txns(timestamp::now_microseconds() / 1000);

        process_shutdown_batch();
    }

    /// Continues shutdown process. Can only be called when module status is ShutdownInProgress.
    /// todo: should continue_shutdown() be called automatically by the system ???
    public fun continue_shutdown() acquires ScheduleQueue, ToRemoveTbl, AuxiliaryData {
        process_shutdown_batch();
    }

    /// Re-initialize the module after the shutdown is complete
    /// We need a governance proposal to re-initialize the module.
    public fun re_initialize(framework: &signer) acquires AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);
        let aux_data = borrow_global_mut<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::ShutdownComplete),
            error::invalid_state(ESHUTDOWN_IN_PROGRESS)
        );
        aux_data.module_status = ScheduledTxnsModuleStatus::Active;
    }

    /// Stop, remove and refund all scheduled txns
    fun process_shutdown_batch() acquires ScheduleQueue, ToRemoveTbl, AuxiliaryData {
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::ShutdownInProgress),
            error::invalid_state(EINVALID_SHUTDOWN_ATTEMPT)
        );

        let txns_to_cancel = vector::empty<KeyAndTxnInfo>();
        // Make a list of txns to cancel with their keys and signers
        {
            let queue = borrow_global<ScheduleQueue>(@aptos_framework);

            // Iterate through schedule_map to get all transactions
            let iter = queue.schedule_map.new_begin_iter();
            let cancel_count = 0;
            while ((!iter.iter_is_end(&queue.schedule_map))
                && (cancel_count < SHUTDOWN_CANCEL_LIMIT)) {
                let key = iter.iter_borrow_key();
                if (!queue.txn_tbl.contains(key.txn_id)) {
                    // the scheduled txn is run in the same block, but before this 'shutdown txn'
                    continue;
                };
                let txn = queue.txn_tbl.borrow(key.txn_id);
                let deposit_amt = txn.max_gas_amount * txn.gas_unit_price;
                txns_to_cancel.push_back(
                    KeyAndTxnInfo { key: *key, account_addr: txn.sender_addr, deposit_amt }
                );
                cancel_count = cancel_count + 1;
                iter = iter.iter_next(&queue.schedule_map);
            };
        };

        // Cancel transactions
        while (!txns_to_cancel.is_empty()) {
            let KeyAndTxnInfo { key, account_addr, deposit_amt } =
                txns_to_cancel.pop_back();
            cancel_internal(account_addr, key, deposit_amt);
            event::emit(
                TransactionFailedEvent {
                    scheduled_txn_time: key.time,
                    scheduled_txn_hash: key.txn_id,
                    sender_addr: account_addr,
                    cancelled_txn_code: CancelledTxnCode::Shutdown
                }
            );
        };

        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        if (queue.schedule_map.is_empty()) {
            complete_shutdown();
        };
    }

    fun complete_shutdown() acquires AuxiliaryData, ToRemoveTbl {
        let aux_data = borrow_global_mut<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::ShutdownInProgress),
            error::invalid_state(EINVALID_SHUTDOWN_ATTEMPT)
        );
        aux_data.module_status = ScheduledTxnsModuleStatus::ShutdownComplete;

        // Clean up ToRemoveTbl
        let ToRemoveTbl { remove_tbl } = borrow_global_mut<ToRemoveTbl>(@aptos_framework);
        let i = 0;
        while (i < TO_REMOVE_PARALLELISM) {
            if (remove_tbl.contains((i as u16))) {
                remove_tbl.remove((i as u16));
            };
            i = i + 1;
        };

        event::emit(ShutdownEvent { complete: true });
    }

    /// Pause the scheduled transactions module
    /// Internally called by the system if any system level invariant of scheduled txns is violated.
    /// Next steps is to have a governance proposal to:
    ///     (a) unpause the module or
    ///     (b) start the shutdown process
    public fun pause_scheduled_txns(framework: &signer) acquires AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);
        let aux_data = borrow_global_mut<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
            error::invalid_state(EINVALID_PAUSE_ATTEMPT)
        );
        aux_data.module_status = ScheduledTxnsModuleStatus::Paused;
    }

    /// Unpause the scheduled transactions module.
    /// This can be called by a governace proposal. It is advised that this be called only after ensuring that the
    /// system invariants won't be violated again.
    public fun unpause_scheduled_txns(framework: &signer) acquires AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);
        let aux_data = borrow_global_mut<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Paused),
            error::invalid_state(EINVALID_UNPAUSE_ATTEMPT)
        );
        aux_data.module_status = ScheduledTxnsModuleStatus::Active;
    }

    /// Change the expiry delta for scheduled transactions; can be called only by the framework
    public fun set_expiry_delta(
        framework: &signer, new_expiry_delta: u64
    ) acquires AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);
        let aux_data = borrow_global_mut<AuxiliaryData>(signer::address_of(framework));
        aux_data.expiry_delta = new_expiry_delta;
    }

    /// Constructor
    public fun new_scheduled_transaction(
        sender_addr: address,
        scheduled_time_ms: u64,
        max_gas_amount: u64,
        gas_unit_price: u64,
        pass_signer: bool,
        f: |Option<signer>| has copy + store + drop
    ): ScheduledTransaction {
        ScheduledTransaction {
            sender_addr,
            scheduled_time_ms,
            max_gas_amount,
            gas_unit_price,
            pass_signer,
            f: ScheduledFunction::V1(f)
        }
    }

    /// Insert a scheduled transaction into the queue. ScheduleMapKey is returned to user, which can be used to cancel the txn.
    public fun insert(
        sender: &signer, txn: ScheduledTransaction
    ): ScheduleMapKey acquires ScheduleQueue, AuxiliaryData {
        // If scheduling is shutdown, we cannot schedule any more transactions
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
            error::unavailable(EUNAVAILABLE)
        );

        // we expect the sender to be a permissioned signer
        assert!(
            signer::address_of(sender) == txn.sender_addr,
            error::permission_denied(EINVALID_SIGNER)
        );

        // Only schedule txns in the future
        let txn_time = txn.scheduled_time_ms;
        let block_time_ms = timestamp::now_microseconds() / 1000;
        assert!(txn_time > block_time_ms, error::invalid_argument(EINVALID_TIME));

        assert!(
            txn.gas_unit_price >= 100,
            error::invalid_argument(ELOW_GAS_UNIT_PRICE)
        );

        assert!(
            bcs::serialized_size(&txn) < MAX_SCHED_TXN_SIZE,
            error::invalid_argument(ETXN_TOO_LARGE)
        );

        // Generate unique transaction ID
        let hash = sha3_256(bcs::to_bytes(&txn));
        let txn_id = hash_to_u256(hash);

        // Insert the transaction into the schedule_map
        // Create schedule map key
        let key = ScheduleMapKey {
            time: txn_time,
            gas_priority: U64_MAX - txn.gas_unit_price,
            txn_id
        };

        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);
        queue.schedule_map.add(key, Empty {});
        queue.txn_tbl.add(key.txn_id, txn);

        // Collect deposit
        // Get owner signer from capability
        let gas_deposit_store_cap = borrow_global<AuxiliaryData>(@aptos_framework);
        let gas_deposit_store_addr =
            account::get_signer_capability_address(
                &gas_deposit_store_cap.gas_fee_deposit_store_signer_cap
            );

        coin::transfer<AptosCoin>(
            sender,
            gas_deposit_store_addr,
            txn.max_gas_amount * txn.gas_unit_price
        );

        // Emit event that txn has been scheduled; for now indexer wants to consume this
        event::emit(
            TransactionScheduledEvent {
                block_time_ms,
                scheduled_txn_hash: txn_id,
                sender_addr: txn.sender_addr,
                scheduled_time_ms: txn.scheduled_time_ms,
                max_gas_amount: txn.max_gas_amount,
                gas_unit_price: txn.gas_unit_price
            }
        );

        key
    }

    /// Cancel a scheduled transaction, must be called by the signer who originally scheduled the transaction.
    public fun cancel(sender: &signer, key: ScheduleMapKey) acquires ScheduleQueue, AuxiliaryData {
        // If scheduling is shutdown, we cannot schedule any more transactions
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
            error::unavailable(EUNAVAILABLE)
        );

        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        if (!queue.schedule_map.contains(&key) || !queue.txn_tbl.contains(key.txn_id)) {
            // Second check if for the case: the scheduled txn is run in the same block, but before this 'cancel txn'
            return
        };

        let txn = queue.txn_tbl.borrow(key.txn_id);
        let deposit_amt = txn.max_gas_amount * txn.gas_unit_price;

        // verify sender
        assert!(
            signer::address_of(sender) == txn.sender_addr,
            error::permission_denied(EINVALID_SIGNER)
        );
        cancel_internal(signer::address_of(sender), key, deposit_amt);
    }

    const MASK_64: u256 = 0xffffffffffffffff; // 2^64 - 1

    fun truncate_to_u64(val: u256): u64 {
        let masked = val & MASK_64; // Truncate high bits
        (masked as u64) // Now safe: always <= u64::MAX
    }

    // Converting 32-byte hash to u256
    fun hash_to_u256(hash: vector<u8>): u256 {
        assert!(hash.length() == 32, error::internal(EINVALID_HASH_SIZE));
        from_bcs::to_u256(hash)
    }

    /// Internal cancel function that takes an address instead of signer. No signer verification, assumes key is present
    /// in the schedule_map.
    fun cancel_internal(
        account_addr: address,
        key: ScheduleMapKey,
        deposit_amt: u64
    ) acquires ScheduleQueue, AuxiliaryData {
        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);

        // Remove the transaction from schedule_map and txn_tbl
        queue.schedule_map.remove(&key);
        queue.txn_tbl.remove(key.txn_id);

        // Refund the deposit
        // Get owner signer from capability
        let gas_deposit_store_cap = borrow_global<AuxiliaryData>(@aptos_framework);
        let gas_deposit_store_signer =
            account::create_signer_with_capability(
                &gas_deposit_store_cap.gas_fee_deposit_store_signer_cap
            );

        // Refund deposit from owner's store to sender
        coin::transfer<AptosCoin>(
            &gas_deposit_store_signer,
            account_addr,
            deposit_amt
        );
    }

    /// Gets txns due to be run; also expire txns that could not be run for a while (mostly due to low gas priority)
    fun get_ready_transactions(
        block_timestamp_ms: u64
    ): vector<ScheduledTransactionInfoWithKey> acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        get_ready_transactions_with_limit(
            block_timestamp_ms, GET_READY_TRANSACTIONS_LIMIT
        )
    }

    fun get_ready_transactions_with_limit(
        block_timestamp_ms: u64, limit: u64
    ): vector<ScheduledTransactionInfoWithKey> acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        remove_txns(block_timestamp_ms);
        // If scheduling is shutdown, we cannot schedule any more transactions
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        if (aux_data.module_status != ScheduledTxnsModuleStatus::Active) {
            return vector::empty<ScheduledTransactionInfoWithKey>();
        };

        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        let scheduled_txns = vector::empty<ScheduledTransactionInfoWithKey>();
        let count = 0;

        let iter = queue.schedule_map.new_begin_iter();
        while ((count < limit) && !iter.iter_is_end(&queue.schedule_map)) {
            let key = iter.iter_borrow_key();
            if (key.time > block_timestamp_ms) {
                break;
            };
            let txn = queue.txn_tbl.borrow(key.txn_id);

            let scheduled_txn_info_with_key =
                ScheduledTransactionInfoWithKey {
                    sender_addr: txn.sender_addr,
                    max_gas_amount: txn.max_gas_amount,
                    gas_unit_price: txn.gas_unit_price,
                    key: *key
                };

            if ((block_timestamp_ms > key.time)
                && ((block_timestamp_ms - key.time) > aux_data.expiry_delta)) {
                continue;
            } else {
                scheduled_txns.push_back(scheduled_txn_info_with_key);
            };
            // we do not want an unbounded size of ready or expirable txns; hence we increment either way
            count = count + 1;
            iter = iter.iter_next(&queue.schedule_map);
        };

        scheduled_txns
    }

    /// Increment after every scheduled transaction is run
    /// IMP: Make sure this does not affect parallel execution of txns
    public(friend) fun mark_txn_to_remove(key: ScheduleMapKey) acquires ToRemoveTbl {
        // Calculate table index using hash
        let tbl_idx = ((truncate_to_u64(key.txn_id) % TO_REMOVE_PARALLELISM) as u16);
        let to_remove = borrow_global_mut<ToRemoveTbl>(@aptos_framework);
        let keys = to_remove.remove_tbl.borrow_mut(tbl_idx);
        keys.push_back(key);
    }

    fun cancel_and_remove_expired_txns(
        block_timestamp_ms: u64
    ) acquires ScheduleQueue, AuxiliaryData {
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        let txns_to_expire = vector::empty<KeyAndTxnInfo>();

        // collect expired transactions
        let iter = queue.schedule_map.new_begin_iter();
        let expire_count = 0;
        while (!iter.iter_is_end(&queue.schedule_map)
            && expire_count < EXPIRE_TRANSACTIONS_LIMIT) {
            let key = iter.iter_borrow_key();
            if ((block_timestamp_ms < key.time)
                || ((block_timestamp_ms - key.time) <= aux_data.expiry_delta)) {
                break;
            };

            // Get transaction info before cancelling
            let txn = queue.txn_tbl.borrow(key.txn_id);
            let deposit_amt = txn.max_gas_amount * txn.gas_unit_price;

            txns_to_expire.push_back(
                KeyAndTxnInfo { key: *key, account_addr: txn.sender_addr, deposit_amt }
            );
            expire_count = expire_count + 1;
            iter = iter.iter_next(&queue.schedule_map);
        };

        // cancel expired transactions
        while (!txns_to_expire.is_empty()) {
            let KeyAndTxnInfo { key, account_addr, deposit_amt } =
                txns_to_expire.pop_back();
            cancel_internal(account_addr, key, deposit_amt);
            event::emit(
                TransactionFailedEvent {
                    scheduled_txn_time: key.time,
                    scheduled_txn_hash: key.txn_id,
                    sender_addr: account_addr,
                    cancelled_txn_code: CancelledTxnCode::Expired
                }
            );
        };
    }

    /// Remove the txns that are run
    public(friend) fun remove_txns(
        block_timestamp_ms: u64
    ) acquires ToRemoveTbl, ScheduleQueue, AuxiliaryData {
        {
            let to_remove = borrow_global_mut<ToRemoveTbl>(@aptos_framework);
            let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);
            let tbl_idx: u16 = 0;

            while ((tbl_idx as u64) < TO_REMOVE_PARALLELISM) {
                if (to_remove.remove_tbl.contains(tbl_idx)) {
                    let keys = to_remove.remove_tbl.borrow_mut(tbl_idx);

                    while (!keys.is_empty()) {
                        let key = keys.pop_back();
                        if (queue.schedule_map.contains(&key)) {
                            // Remove transaction from schedule_map and txn_tbl
                            if (queue.txn_tbl.contains(key.txn_id)) {
                                queue.txn_tbl.remove(key.txn_id);
                            };
                            queue.schedule_map.remove(&key);
                        };
                    };
                };
                tbl_idx = tbl_idx + 1;
            };
        };
        cancel_and_remove_expired_txns(block_timestamp_ms);
    }

    /// Called by the executor when the scheduled transaction is run
    fun execute_user_function_wrapper(
        signer: signer, txn_key: ScheduleMapKey
    ): bool acquires ScheduleQueue {
        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);

        if (!queue.schedule_map.contains(&txn_key)) {
            // It is possible that the scheduled transaction was cancelled before in the same block
            return false;
        };
        let txn = queue.txn_tbl.borrow(txn_key.txn_id);
        let pass_signer = txn.pass_signer;

        match(txn.f) {
            ScheduledFunction::V1(f) => {
                if (pass_signer) {
                    f(some(signer));
                } else {
                    f(std::option::none());
                };
            }
        };

        // The scheduled transaction is removed from two data structures at different times:
        // 1. From schedule_map (BigOrderedMap): Removed in next block's prologue to allow parallel execution
        //    of all scheduled transactions in the current block
        // 2. From txn_tbl: Removed immediately after transaction execution in this function to enable
        //    proper refunding of storage gas fees to the user
        queue.txn_tbl.remove(txn_key.txn_id);
        true
    }

    public(friend) fun emit_transaction_failed_event(
        key: ScheduleMapKey, sender_addr: address
    ) {
        event::emit(
            TransactionFailedEvent {
                scheduled_txn_time: key.time,
                scheduled_txn_hash: key.txn_id,
                sender_addr,
                cancelled_txn_code: CancelledTxnCode::Failed
            }
        );
    }

    ////////////////////////// TESTS //////////////////////////
    #[test_only]
    public fun get_num_txns(): u64 acquires ScheduleQueue {
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        let num_txns = queue.schedule_map.compute_length();
        num_txns
    }

    #[test_only]
    public fun get_ready_transactions_test(
        timestamp: u64
    ): vector<ScheduledTransactionInfoWithKey> acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        get_ready_transactions(timestamp)
    }

    #[test_only]
    public fun get_deposit_owner_signer(): signer acquires AuxiliaryData {
        let owner_cap = borrow_global<AuxiliaryData>(@aptos_framework);
        let owner_signer =
            account::create_signer_with_capability(
                &owner_cap.gas_fee_deposit_store_signer_cap
            );
        owner_signer
    }

    #[test_only]
    public fun get_func_from_txn_key(
        key: ScheduleMapKey
    ): |Option<signer>| has copy acquires ScheduleQueue {
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        assert!(queue.schedule_map.contains(&key), 0);
        let txn = queue.txn_tbl.borrow(key.txn_id);
        match(txn.f) {
            ScheduledFunction::V1(f) => { f }
        }
    }

    #[test_only]
    public fun shutdown_test(fx: &signer) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        start_shutdown(fx);
    }

    #[test_only]
    public fun setup_test_env(
        fx: &signer, user: &signer, curr_mock_time_ms: u64
    ) {
        let (burn, mint) = aptos_framework::aptos_coin::initialize_for_test(fx);
        transaction_fee::store_aptos_coin_burn_cap_for_test(fx, burn);
        transaction_fee::store_aptos_coin_mint_cap_for_test(fx, mint);
        let user_addr = signer::address_of(user);
        aptos_framework::aptos_account::create_account(user_addr);
        initialize(fx);
        timestamp::set_time_has_started_for_testing(fx);
        timestamp::update_global_time_for_test(curr_mock_time_ms);

        // Fund user account
        let coin = coin::mint<AptosCoin>(100000000, &mint);
        coin::deposit(user_addr, coin);

        coin::destroy_burn_cap(burn);
        coin::destroy_mint_cap(mint);
    }

    #[test_only]
    fun get_module_status(): ScheduledTxnsModuleStatus acquires AuxiliaryData {
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        aux_data.module_status
    }

    struct State has copy, drop, store {
        count: u64
    }

    #[persistent]
    fun step(state: State, _s: Option<signer>) {
        if (state.count < 10) {
            state.count = state.count + 1;
        }
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_basic(fx: &signer, user: signer) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        // Setup test environment
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Create transactions with same scheduled_time but different gas prices
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let schedule_time1 = curr_mock_time_micro_s / 1000 + 1000;
        let schedule_time2 = schedule_time1 * 2;
        let schedule_time3 = schedule_time1 * 4;
        let txn1 = new_scheduled_transaction(
            user_addr, schedule_time1, 100, 200, false, foo
        ); // time: 1s, gas: 20
        let txn2 = new_scheduled_transaction(
            user_addr, schedule_time1, 100, 300, false, foo
        ); // time: 1s, gas: 30
        let txn3 = new_scheduled_transaction(
            user_addr, schedule_time1, 100, 100, false, foo
        ); // time: 1s, gas: 10

        // Create transactions with same scheduled_time and gas price
        let txn4 = new_scheduled_transaction(
            user_addr, schedule_time2, 1000, 200, false, foo
        ); // time: 2s, gas: 20
        let txn5 = new_scheduled_transaction(
            user_addr, schedule_time2, 100, 200, false, foo
        );
        let txn6 = new_scheduled_transaction(
            user_addr, schedule_time2, 200, 200, false, foo
        ); // time: 2s, gas: 20

        let txn7 = new_scheduled_transaction(
            user_addr, schedule_time3, 100, 200, false, foo
        ); // time: 2s, gas: 20
        let txn8 = new_scheduled_transaction(
            user_addr, schedule_time3, 200, 200, false, foo
        ); // time: 2s, gas: 20

        // Insert all transactions
        let txn1_key = insert(&user, txn1);
        let txn2_key = insert(&user, txn2);
        let txn3_key = insert(&user, txn3);
        let txn4_key = insert(&user, txn4);
        let _txn5_key = insert(&user, txn5);
        let txn6_key = insert(&user, txn6);
        let txn7_key = insert(&user, txn7);
        let txn8_key = insert(&user, txn8);

        assert!(get_num_txns() == 8, get_num_txns());

        // Test get_ready_transactions at t < schedule_time1 (should return empty)
        let ready_txns = get_ready_transactions(schedule_time1 - 1000);
        assert!(ready_txns.length() == 0, ready_txns.length());

        // Test get_ready_transactions at t > schedule_time1 (should return first 3 txns)
        let ready_txns = get_ready_transactions(schedule_time1 + 1000);
        assert!(ready_txns.length() == 3, ready_txns.length());

        // Check the order of transactions
        assert!(ready_txns[0].key.txn_id == txn2_key.txn_id, 1);
        assert!(ready_txns[1].key.txn_id == txn1_key.txn_id, 2);

        // Remove transactions
        mark_txn_to_remove(txn1_key);
        mark_txn_to_remove(txn2_key);
        mark_txn_to_remove(txn3_key);
        remove_txns(timestamp::now_microseconds() / 1000); // Should remove first 3 txns
        assert!(get_num_txns() == 5, get_num_txns());

        // Test get_ready_transactions at t > schedule_time2 (should return next 3 txns)
        let ready_txns = get_ready_transactions(schedule_time2 + 1000);
        assert!(ready_txns.length() == 3, ready_txns.length());

        // Execute and remove 2 transactions
        mark_txn_to_remove(txn4_key);
        mark_txn_to_remove(txn6_key);
        remove_txns(timestamp::now_microseconds() / 1000); // Should remove 2 txns
        assert!(get_num_txns() == 3, get_num_txns());

        let ready_txns = get_ready_transactions(schedule_time2 + 1000);
        assert!(ready_txns.length() == 1, ready_txns.length());

        // Execute and remove txns 7 and 8; lets expire txn 5
        mark_txn_to_remove(txn7_key);
        mark_txn_to_remove(txn8_key);

        remove_txns(timestamp::now_microseconds() / 1000);
        assert!(get_num_txns() == 1, get_num_txns());

        // try expiring a txn by getting it late
        let expired_time = schedule_time2 + EXPIRY_DELTA_DEFAULT + 1000;
        assert!(
            get_ready_transactions(expired_time).length() == 0,
            get_ready_transactions(expired_time).length()
        );
        assert!(get_num_txns() == 0, get_num_txns());

        start_shutdown(fx);
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 65538)]
    // error::invalid_argument(EINVALID_TIME)
    fun test_insert_past_time(fx: &signer, user: signer) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Try to schedule transaction in the past
        let past_time = curr_mock_time_micro_s / 1000 - 100;
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn = new_scheduled_transaction(user_addr, past_time, 100, 200, false, foo);

        // Create a signer from the handle to use for insert
        insert(&user, txn); // Should fail with EINVALID_TIME since time is in the past
    }

    #[test(fx = @0x1, user = @0x1234, other_user = @0x5678)]
    #[expected_failure(abort_code = 327681)]
    // error::permission_denied(EINVALID_SIGNER)
    fun test_insert_wrong_user(
        fx: &signer, user: signer, other_user: signer
    ) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time = 1000000;
        setup_test_env(fx, &user, curr_mock_time);

        // Try to schedule transaction with wrong user
        let future_time = curr_mock_time + 1000;
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn = new_scheduled_transaction(user_addr, future_time, 100, 200, false, foo);
        insert(&other_user, txn); // Should fail with EINVALID_SIGNER
    }

    #[test(fx = @0x1, user = @0x1234, other_user = @0x5678)]
    #[expected_failure(abort_code = 327681)]
    // error::permission_denied(EINVALID_SIGNER)
    fun test_cancel_wrong_user(
        fx: &signer, user: signer, other_user: signer
    ) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time = 1000000;
        setup_test_env(fx, &user, curr_mock_time);

        // Schedule a valid transaction
        let future_time = curr_mock_time + 1000;
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn = new_scheduled_transaction(user_addr, future_time, 100, 200, false, foo);
        let txn_id = insert(&user, txn);

        // Try to cancel the transaction with wrong user
        cancel(&other_user, txn_id); // Should fail with EINVALID_SIGNER
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 851971)]
    // error::unavailable(EUNAVAILABLE)
    fun test_insert_when_stopped(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;

        // Setup test environment
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Stop scheduling
        start_shutdown(fx);

        // Try to insert a transaction after shutdown
        let future_time = curr_mock_time_micro_s / 1000 + 1000;
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn = new_scheduled_transaction(user_addr, future_time, 100, 200, false, foo);

        // This should fail with EUNAVAILABLE
        insert(&user, txn);
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_exceeding_limits(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        let total_txns = GET_READY_TRANSACTIONS_LIMIT * 5; // Exceeds both limits

        // Insert more transactions than both limits
        let i = 0;
        while (i < total_txns) {
            let state = State { count: 8 };
            let foo = |s: Option<signer>| step(state, s);
            let txn =
                new_scheduled_transaction(
                    user_addr,
                    curr_mock_time_micro_s / 1000 + 1000 + i, // Different times to ensure unique ordering
                    100,
                    200,
                    false,
                    foo
                );
            insert(&user, txn);
            i = i + 1;
        };

        assert!(get_num_txns() == total_txns, get_num_txns());

        // Test GET_READY_TRANSACTIONS_LIMIT
        let time_for_all = curr_mock_time_micro_s / 1000 + total_txns + 1000;
        let ready_txns = get_ready_transactions(time_for_all);
        assert!(ready_txns.length() == GET_READY_TRANSACTIONS_LIMIT, ready_txns.length());

        // Verify transactions are ordered by time
        let prev_time = 0;
        let i = 0;
        while (i < ready_txns.length()) {
            let txn = ready_txns.borrow(i);
            assert!(txn.key.time >= prev_time, 0);
            prev_time = txn.key.time;
            i = i + 1;
        };

        // Verify SHUTDOWN_CANCEL_LIMIT behavior
        let pre_shutdown_count = get_num_txns();
        start_shutdown(fx);
        let post_shutdown_count = get_num_txns();
        let cancelled_count = pre_shutdown_count - post_shutdown_count;
        assert!(cancelled_count == SHUTDOWN_CANCEL_LIMIT, cancelled_count);

        // Verify that next call to shutdown complete without error
        while (get_num_txns() > 0) {
            continue_shutdown();
        };

        // Check that shutdown is complete and also capability to re-initialize
        re_initialize(fx);

        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn1 =
            new_scheduled_transaction(
                user_addr,
                curr_mock_time_micro_s / 1000 + 1000,
                100,
                200,
                false,
                foo
            );
        insert(&user, txn1);
    }

    #[test(fx = @0x1, user = @0x123)]
    fun test_pause_unpause_flow(fx: &signer, user: signer) acquires AuxiliaryData, ScheduleQueue {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Initially module should be active
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Active, 0);

        // Insert a transaction to verify state
        let user_addr = signer::address_of(&user);
        let schedule_time = curr_mock_time_micro_s / 1000 + 1000;
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn = new_scheduled_transaction(
            user_addr, schedule_time, 100, 200, false, foo
        );
        let _txn_key = insert(&user, txn);
        assert!(get_num_txns() == 1, 1);

        // Pause the module
        pause_scheduled_txns(fx);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Paused, 2);

        // Unpause the module
        unpause_scheduled_txns(fx);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Active, 3);

        // Verify transactions are preserved
        assert!(get_num_txns() == 1, 6);
    }

    #[test(fx = @0x1, user = @0x123)]
    #[expected_failure(abort_code = 196618)]
    fun test_cannot_pause_from_paused_state(fx: &signer, user: signer) acquires AuxiliaryData {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // First pause should succeed
        pause_scheduled_txns(fx);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Paused, 0);

        // Second pause should fail
        pause_scheduled_txns(fx);
    }

    #[test(fx = @0x1, user = @0x123)]
    #[expected_failure(abort_code = 196619)]
    fun test_cannot_unpause_from_active_state(fx: &signer, user: signer) acquires AuxiliaryData {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Module starts in Active state
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Active, 0);

        // Attempting to unpause while active should fail
        unpause_scheduled_txns(fx);
    }

    #[test(fx = @0x1, user = @0x123)]
    #[expected_failure(abort_code = 851971)]
    fun test_cannot_insert_when_paused(
        fx: &signer, user: signer
    ) acquires AuxiliaryData, ScheduleQueue {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Pause the module
        pause_scheduled_txns(fx);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Paused, 0);

        // Try to insert a transaction while paused
        let user_addr = signer::address_of(&user);
        let schedule_time = curr_mock_time_micro_s / 1000 + 1000;
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn = new_scheduled_transaction(
            user_addr, schedule_time, 100, 200, false, foo
        );
        insert(&user, txn); // Should fail with EUNAVAILABLE
    }
}
