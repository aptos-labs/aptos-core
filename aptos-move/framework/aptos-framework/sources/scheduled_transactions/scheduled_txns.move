module aptos_framework::scheduled_txns {
    use std::bcs;
    use std::error;
    use std::hash::sha3_256;
    use std::option::{Option, some, none};
    use std::signer;
    use std::vector;
    use aptos_std::from_bcs;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_framework::account;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};

    use aptos_framework::event;
    use aptos_framework::fungible_asset::{upgrade_store_to_concurrent, Metadata};
    use aptos_framework::object::address_to_object;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::transaction_context::{
        payload_config,
        payload_scheduled_txn_config_auth_expiration,
        payload_scheduled_txn_config_allow_resched,
        payload_scheduled_txn_config_auth_num
    };
    use aptos_framework::sched_txns_auth_num;

    friend aptos_framework::block;
    friend aptos_framework::transaction_validation;
    friend aptos_framework::user_func_wrapper;

    #[test_only]
    use aptos_framework::transaction_fee;
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
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

    /// Gas amout too low, not enough to cover fixed costs while running the scheduled transaction
    const ETOO_LOW_GAS_AMOUNT: u64 = 5;

    /// Txn size is too large; beyond 10KB
    const ETXN_TOO_LARGE: u64 = 6;

    /// Indicates error in SHA3-256 generation
    const EINVALID_HASH_SIZE: u64 = 7;

    /// Trying to start shutdown when module is not in Active state
    const EINVALID_SHUTDOWN_START: u64 = 8;

    /// Shutdown attempted without starting it
    const EINVALID_SHUTDOWN_ATTEMPT: u64 = 9;

    /// Shutdown is already in progress
    const ESHUTDOWN_IN_PROGRESS: u64 = 10;

    /// Can be paused only when module is in Active state
    const EINVALID_PAUSE_ATTEMPT: u64 = 11;

    /// Can be paused only when module is in Paused state
    const EINVALID_UNPAUSE_ATTEMPT: u64 = 12;

    /// Cannot cancel a transaction that is about to be run or has already been run
    const ECANCEL_TOO_LATE: u64 = 13;

    /// Authorization token not found in payload config
    const EAUTH_TOKEN_NOT_FOUND: u64 = 14;

    /// Current time is after expiration time
    const EAUTH_TOKEN_EXPIRED: u64 = 15;

    /// Schedule time is after expiration time
    const EAUTH_TOKEN_INSUFFICIENT_DURATION: u64 = 15;

    /// Authorization number mismatch
    const EAUTH_NUM_MISMATCH: u64 = 16;

    /// Authorization number not found - must be initialized first via get_or_init_auth_num
    const EAUTH_NUM_NOT_FOUND: u64 = 17;

    const U64_MAX: u64 = 18446744073709551615;

    /// Can't cancel a transaction that is going to be run in next 10 seconds
    const CANCEL_DELTA_DEFAULT: u64 = 10 * 1000;

    /// If we cannot schedule in 10s, we will abort the txn
    const EXPIRY_DELTA_DEFAULT: u64 = 10 * 1000;

    /// Maximum number of scheduled transactions that can be run in a block
    const GET_READY_TRANSACTIONS_LIMIT: u64 = 100;

    /// Maximum number of transactions that can be cancelled in a block during shutdown
    const SHUTDOWN_CANCEL_BATCH_SIZE_DEFAULT: u64 = GET_READY_TRANSACTIONS_LIMIT * 2;

    /// SHA3-256 produces 32 bytes
    const TXN_ID_SIZE: u16 = 32;

    /// The average size of a scheduled transaction to provide an estimate of leaf nodes of BigOrderedMap
    const AVG_SCHED_TXN_SIZE: u16 = 1024;

    /// Max size of a scheduled transaction; 1MB for now as we are bounded by the slot size
    const MAX_SCHED_TXN_SIZE: u64 = 1024 * 1024;

    /// Framework owned address that stores the deposits for all scheduled txns
    const DEPOSIT_STORE_OWNER_ADDR: address = @0xb;

    /// Min gas unit price
    const MIN_GAS_UNIT_PRICE: u64 = 100;

    /// Min gas amount
    const MIN_GAS_AMOUNT: u64 = 100;

    enum ScheduledFunction has copy, store, drop {
        V1(|| has copy + store + drop),
        V1WithAuthToken(|&signer, ScheduledTxnAuthToken| has copy + store + drop, ScheduledTxnAuthToken),
    }

    struct ScheduledTxnAuthToken has copy, drop, store {
        allow_rescheduling: bool,
        expiration_time: u64,
        authorization_num: u64
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
        /// Expiry delta used to determine when this scheduled transaction becomes invalid (and subsequently aborted)
        expiry_delta: u64,
        /// Variables are captured in the closure; optionally a signer is passed; no return
        f: ScheduledFunction
    }

    /// We pass around only needed info
    struct ScheduledTransactionInfoWithKey has drop {
        sender_addr: address,
        max_gas_amount: u64,
        gas_unit_price: u64,
        block_timestamp_ms: u64,
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
        /// key: txn_id; value: ScheduledTransaction (metadata, function and capture)
        txn_table: Table<u256, ScheduledTransaction>
    }

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

    /// Stores module level auxiliary data
    struct AuxiliaryData has key {
        // todo: check if this is secure
        /// Capability for managing the gas fee deposit store
        gas_fee_deposit_store_signer_cap: account::SignerCapability,
        // Current status of the scheduled transactions module
        module_status: ScheduledTxnsModuleStatus
    }

    const TO_REMOVE_PARALLELISM: u64 = GET_READY_TRANSACTIONS_LIMIT;
    struct ToRemoveTbl has key {
        /// After a transaction is executed, it is marked for removal from the ScheduleQueue using this table.
        /// Direct removal from the ScheduleQueue is avoided to prevent serialization on access of ScheduleQueue.
        /// The remove table has as many slots as the number of transactions run in a block, minimizing the chances of
        /// serialization
        remove_tbl: Table<u16, vector<ScheduleMapKey>>
    }

    enum CancelledTxnCode has drop, store {
        /// Scheduling service is stopped
        Shutdown,
        /// Transaction was expired
        Expired,
        /// Auth token expired
        AuthExpired
    }

    #[event]
    struct TransactionScheduledEvent has drop, store {
        block_time_ms: u64,
        scheduled_txn_hash: u256,
        sender_addr: address,
        scheduled_time_ms: u64,
        max_gas_amount: u64,
        gas_unit_price: u64,
        auth_required: bool
    }

    #[event]
    struct TransactionCancelledEvent has drop, store {
        scheduled_txn_time: u64,
        scheduled_txn_hash: u256,
        sender_addr: address
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
    public entry fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);

        // Create owner account for handling deposits
        let owner_addr = DEPOSIT_STORE_OWNER_ADDR;
        let (owner_signer, owner_cap) =
            account::create_framework_reserved_account(owner_addr);

        // Initialize fungible store for the owner
        let metadata = address_to_object<Metadata>(@aptos_fungible_asset);
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
                module_status: ScheduledTxnsModuleStatus::Active
            }
        );

        // Initialize authorization number map
        sched_txns_auth_num::initialize(framework);

        // Initialize queue
        let queue = ScheduleQueue {
            schedule_map: big_ordered_map::new_with_reusable(),
            txn_table: table::new<u256, ScheduledTransaction>()
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
    public entry fun start_shutdown(framework: &signer) acquires AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);

        let aux_data = borrow_global_mut<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
            error::invalid_state(EINVALID_SHUTDOWN_START)
        );
        aux_data.module_status = ScheduledTxnsModuleStatus::ShutdownInProgress;

        // we don't process_shutdown_batch() immediately here to avoid race conditions with the scheduled transactions
        // that are being run in the same block
    }

    /// Continues shutdown process. Can only be called when module status is ShutdownInProgress.
    entry fun continue_shutdown(
        cancel_batch_size: u64
    ) acquires ScheduleQueue, ToRemoveTbl, AuxiliaryData {
        process_shutdown_batch(cancel_batch_size);
    }

    /// Re-initialize the module after the shutdown is complete
    /// We need a governance proposal to re-initialize the module.
    public entry fun re_initialize(framework: &signer) acquires AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);
        let aux_data = borrow_global_mut<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::ShutdownComplete),
            error::invalid_state(ESHUTDOWN_IN_PROGRESS)
        );
        aux_data.module_status = ScheduledTxnsModuleStatus::Active;
    }

    /// Stop, remove and refund all scheduled txns
    fun process_shutdown_batch(
        cancel_batch_size: u64
    ) acquires ScheduleQueue, ToRemoveTbl, AuxiliaryData {
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
                && (cancel_count < cancel_batch_size)) {
                let key = iter.iter_borrow_key();
                if (!queue.txn_table.contains(key.txn_id)) {
                    // the scheduled txn is run in the same block, but before this 'shutdown txn'
                    continue;
                };
                let txn = queue.txn_table.borrow(key.txn_id);
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
    fun pause_scheduled_txns(vm: &signer) acquires AuxiliaryData {
        system_addresses::assert_vm(vm);
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

    /// Returns the current authorization number for an address
    /// Lazy initialization: starts from 1 and stores in map upon first use
    public fun get_or_init_auth_num(addr: address): u64 {
        sched_txns_auth_num::get_or_init_auth_num(addr)
    }

    fun get_auth_num(addr: address): u64 {
        sched_txns_auth_num::get_auth_num(addr)
    }

    /// Increments the authorization number for an address
    /// Requires that the address already exists in auth_num_map (initialized via get_or_init_auth_num)
    fun increment_auth_num(addr: address) {
        sched_txns_auth_num::increment_auth_num(addr)
    }

    /// Read functions for ScheduleMapKey individual parameters
    public fun schedule_map_key_time(key: &ScheduleMapKey): u64 {
        key.time
    }

    public fun schedule_map_key_gas_priority(key: &ScheduleMapKey): u64 {
        key.gas_priority
    }

    public fun schedule_map_key_txn_id(key: &ScheduleMapKey): u256 {
        key.txn_id
    }

    /// Create ScheduleMapKey from individual parameters
    fun create_schedule_map_key(
        time: u64, gas_priority: u64, txn_id: u256
    ): ScheduleMapKey {
        ScheduleMapKey { time, gas_priority, txn_id }
    }

    /// Common validation helper for auth token based scheduled transactions
    fun validate_auth_token(
        sender_addr: address, scheduled_time_ms: u64, auth_token: &ScheduledTxnAuthToken
    ) {
        // Get current time and validate expiration
        let current_time_ms = timestamp::now_microseconds() / 1000;
        assert!(
            current_time_ms <= auth_token.expiration_time,
            error::invalid_argument(EAUTH_TOKEN_EXPIRED)
        );
        assert!(
            scheduled_time_ms <= auth_token.expiration_time,
            error::invalid_argument(EAUTH_TOKEN_INSUFFICIENT_DURATION)
        );

        // Validate authorization number
        let sender_auth_num = get_auth_num(sender_addr);
        assert!(
            auth_token.authorization_num == sender_auth_num,
            error::invalid_argument(EAUTH_NUM_MISMATCH)
        );
    }

    /// Constructor
    public fun new_scheduled_transaction_no_signer(
        sender_addr: address,
        scheduled_time_ms: u64,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiry_delta: u64,
        f: || has copy + store + drop
    ): ScheduledTransaction {
        ScheduledTransaction {
            sender_addr,
            scheduled_time_ms,
            max_gas_amount,
            gas_unit_price,
            expiry_delta,
            f: ScheduledFunction::V1(f)
        }
    }

    public fun new_scheduled_transaction_gen_auth_token(
        sender: &signer,
        scheduled_time_ms: u64,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiry_delta: u64,
        f: |&signer, ScheduledTxnAuthToken| has copy + store + drop
    ): ScheduledTransaction {
        let sender_addr = signer::address_of(sender);
        // Extract the payload config from the current transaction context
        let payload_config_opt = payload_config();
        assert!(
            payload_config_opt.is_some(),
            error::invalid_argument(EAUTH_TOKEN_NOT_FOUND)
        );

        let payload_config = payload_config_opt.extract();
        let scheduled_config_opt =
            aptos_framework::transaction_context::payload_config_scheduled_txn_auth_token(
                &payload_config
            );
        assert!(
            scheduled_config_opt.is_some(),
            error::invalid_argument(EAUTH_TOKEN_NOT_FOUND)
        );

        let scheduled_config = scheduled_config_opt.extract();

        // Create the required auth token
        let auth_token = ScheduledTxnAuthToken {
            allow_rescheduling: payload_scheduled_txn_config_allow_resched(
                &scheduled_config
            ),
            expiration_time: payload_scheduled_txn_config_auth_expiration(
                &scheduled_config
            ),
            authorization_num: payload_scheduled_txn_config_auth_num(
                &scheduled_config
            )
        };

        // Validate the auth token
        validate_auth_token(sender_addr, scheduled_time_ms, &auth_token);

        ScheduledTransaction {
            sender_addr,
            scheduled_time_ms,
            max_gas_amount,
            gas_unit_price,
            expiry_delta,
            f: ScheduledFunction::V1WithAuthToken(f, auth_token)
        }
    }

    public fun new_scheduled_transaction_reuse_auth_token(
        sender: &signer,
        auth_token: ScheduledTxnAuthToken,
        scheduled_time_ms: u64,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiry_delta: u64,
        f: |&signer, ScheduledTxnAuthToken| has copy + store + drop,
    ): ScheduledTransaction {
        let sender_addr = signer::address_of(sender);

        // Validate the auth token (same checks as new_auth_token)
        validate_auth_token(sender_addr, scheduled_time_ms, &auth_token);

        ScheduledTransaction {
            sender_addr,
            scheduled_time_ms,
            max_gas_amount,
            gas_unit_price,
            expiry_delta,
            f: ScheduledFunction::V1WithAuthToken(f, auth_token)
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

        assert!(
            signer::address_of(sender) == txn.sender_addr,
            error::permission_denied(EINVALID_SIGNER)
        );

        // Only schedule txns in the future
        let txn_time = txn.scheduled_time_ms;
        let block_time_ms = timestamp::now_microseconds() / 1000;
        assert!(txn_time > block_time_ms, error::invalid_argument(EINVALID_TIME));

        assert!(
            txn.gas_unit_price >= MIN_GAS_UNIT_PRICE,
            error::invalid_argument(ELOW_GAS_UNIT_PRICE)
        );

        assert!(
            txn.max_gas_amount >= MIN_GAS_AMOUNT,
            error::invalid_argument(ETOO_LOW_GAS_AMOUNT)
        );

        match (txn.f) {
            ScheduledFunction::V1(_f) => { },
            ScheduledFunction::V1WithAuthToken(_f, auth_token) => {
                validate_auth_token(txn.sender_addr, txn_time, &auth_token);
            }
        };

        let txn_bytes = bcs::to_bytes(&txn);
        assert!(
            txn_bytes.length() < MAX_SCHED_TXN_SIZE,
            error::invalid_argument(ETXN_TOO_LARGE)
        );

        // Generate unique transaction ID
        let hash = sha3_256(txn_bytes);
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
        queue.txn_table.add(key.txn_id, txn);

        // Collect deposit
        // Get owner signer from capability
        let gas_deposit_store_cap = borrow_global<AuxiliaryData>(@aptos_framework);
        let gas_deposit_store_addr =
            account::get_signer_capability_address(
                &gas_deposit_store_cap.gas_fee_deposit_store_signer_cap
            );

        primary_fungible_store::transfer(
            sender,
            address_to_object<Metadata>(@aptos_fungible_asset),
            gas_deposit_store_addr,
            txn.max_gas_amount * txn.gas_unit_price,
        );

        // Emit event that txn has been scheduled; for now indexer wants to consume this
        let auth_required = match (txn.f) {
            ScheduledFunction::V1(_f) => false,
            ScheduledFunction::V1WithAuthToken(_f, _auth_token) => true,
        };

        event::emit(
            TransactionScheduledEvent {
                block_time_ms,
                scheduled_txn_hash: txn_id,
                sender_addr: txn.sender_addr,
                scheduled_time_ms: txn.scheduled_time_ms,
                max_gas_amount: txn.max_gas_amount,
                gas_unit_price: txn.gas_unit_price,
                auth_required: auth_required
            }
        );

        key
    }

    /// Cancel a scheduled transaction, must be called by the signer who originally scheduled the transaction.
    public fun cancel_with_key(
        sender: &signer, key: ScheduleMapKey
    ) acquires ScheduleQueue, AuxiliaryData {
        // If scheduling is shutdown, we cannot schedule any more transactions
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
            error::unavailable(EUNAVAILABLE)
        );

        let curr_time_ms = timestamp::now_microseconds() / 1000;
        assert!(
            (curr_time_ms < key.time) && ((key.time - curr_time_ms)
                > CANCEL_DELTA_DEFAULT),
            error::invalid_argument(ECANCEL_TOO_LATE)
        );

        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        if (!queue.schedule_map.contains(&key)
            || !queue.txn_table.contains(key.txn_id)) {
            // this is more of a paranoid check, we should never get here, rather throw ECANCEL_TOO_LATE error
            // Second check if for the case: the scheduled txn is run in the same block, but before this 'cancel txn'
            return
        };

        let txn = queue.txn_table.borrow(key.txn_id);
        let deposit_amt = txn.max_gas_amount * txn.gas_unit_price;

        // verify sender
        let sender_addr = signer::address_of(sender);
        assert!(
            sender_addr == txn.sender_addr,
            error::permission_denied(EINVALID_SIGNER)
        );
        cancel_internal(sender_addr, key, deposit_amt);

        // emit cancel event
        event::emit(
            TransactionCancelledEvent {
                scheduled_txn_time: key.time,
                scheduled_txn_hash: key.txn_id,
                sender_addr
            }
        );
    }

    /// Entry function for cancel that takes individual ScheduleMapKey parameters
    public entry fun cancel(
        sender: &signer,
        time: u64,
        gas_priority: u64,
        txn_id: u256
    ) acquires ScheduleQueue, AuxiliaryData {
        let key = create_schedule_map_key(time, gas_priority, txn_id);
        cancel_with_key(sender, key);
    }

    /// Cancel all scheduled transactions for a sender using lazy cancel approach.
    /// This increments the sender's authorization number, which will cause
    /// all existing scheduled transactions with auth tokens to fail validation when executed.
    public entry fun cancel_all(sender: &signer) acquires AuxiliaryData {
        // Check module is active
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        assert!(
            (aux_data.module_status == ScheduledTxnsModuleStatus::Active),
            error::unavailable(EUNAVAILABLE)
        );

        let sender_addr = signer::address_of(sender);

        // Increment the sender's auth number to invalidate all existing auth tokens
        // This will fail if sender is not initialized (which is the desired behavior)
        increment_auth_num(sender_addr);
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
        account_addr: address, key: ScheduleMapKey, deposit_amt: u64
    ) acquires ScheduleQueue, AuxiliaryData {
        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);

        // Remove the transaction from schedule_map and txn_table
        queue.schedule_map.remove(&key);
        queue.txn_table.remove(key.txn_id);

        // Refund the deposit
        // Get owner signer from capability
        let gas_deposit_store_cap = borrow_global<AuxiliaryData>(@aptos_framework);
        let gas_deposit_store_signer =
            account::create_signer_with_capability(
                &gas_deposit_store_cap.gas_fee_deposit_store_signer_cap
            );

        // Refund deposit from owner's store to sender
        primary_fungible_store::transfer(
            &gas_deposit_store_signer,
            address_to_object<Metadata>(@aptos_fungible_asset),
            account_addr,
            deposit_amt,
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
        remove_txns();
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
            let txn = queue.txn_table.borrow(key.txn_id);

            // Include transaction without checking expiry - expiry will be checked during execution
            let scheduled_txn_info_with_key =
                ScheduledTransactionInfoWithKey {
                    sender_addr: txn.sender_addr,
                    max_gas_amount: txn.max_gas_amount,
                    gas_unit_price: txn.gas_unit_price,
                    block_timestamp_ms,
                    key: *key
                };

            scheduled_txns.push_back(scheduled_txn_info_with_key);
            count = count + 1;
            iter = iter.iter_next(&queue.schedule_map);
        };

        scheduled_txns
    }

    /// Increment after every scheduled transaction is run
    /// IMP: Make sure this does not affect parallel execution of txns
    fun mark_txn_to_remove(key: ScheduleMapKey) acquires ToRemoveTbl {
        // Calculate table index using hash
        let tbl_idx = ((truncate_to_u64(key.txn_id) % TO_REMOVE_PARALLELISM) as u16);
        let to_remove = borrow_global_mut<ToRemoveTbl>(@aptos_framework);
        let keys = to_remove.remove_tbl.borrow_mut(tbl_idx);
        keys.push_back(key);
    }

    /// Remove the txns that are run
    public(friend) fun remove_txns() acquires ToRemoveTbl, ScheduleQueue {
        let to_remove = borrow_global_mut<ToRemoveTbl>(@aptos_framework);
        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);
        let tbl_idx: u16 = 0;

        while ((tbl_idx as u64) < TO_REMOVE_PARALLELISM) {
            if (to_remove.remove_tbl.contains(tbl_idx)) {
                let keys = to_remove.remove_tbl.borrow_mut(tbl_idx);

                while (!keys.is_empty()) {
                    let key = keys.pop_back();
                    if (queue.schedule_map.contains(&key)) {
                        // Remove transaction from schedule_map and txn_table
                        if (queue.txn_table.contains(key.txn_id)) {
                            queue.txn_table.remove(key.txn_id);
                        };
                        queue.schedule_map.remove(&key);
                    };
                };
            };
            tbl_idx = tbl_idx + 1;
        };
    }

    /// Helper to check if scheduled function is V1 (no auth token)
    public(friend) fun is_scheduled_function_v1(
        txn: &ScheduledTransaction
    ): bool {
        match(txn.f) {
            ScheduledFunction::V1(_f) => true,
            ScheduledFunction::V1WithAuthToken(_f, _auth_token) => false
        }
    }

    /// Helper to get V1 function (no auth token)
    public(friend) fun get_scheduled_function_v1(
        txn: &ScheduledTransaction
    ): || {
        match(txn.f) {
            ScheduledFunction::V1(f) => f,
            ScheduledFunction::V1WithAuthToken(_f, _auth_token) => {
                abort(error::invalid_state(EAUTH_TOKEN_NOT_FOUND))
            }
        }
    }

    /// Helper to get V1WithAuthToken function
    public(friend) fun get_scheduled_function_v1_with_auth_token(
        txn: &ScheduledTransaction
    ): |&signer, ScheduledTxnAuthToken| {
        match(txn.f) {
            ScheduledFunction::V1(_f) => {
                abort(error::invalid_state(EAUTH_TOKEN_NOT_FOUND))
            },
            ScheduledFunction::V1WithAuthToken(f, _auth_token) => f
        }
    }

    /// Helper to get transaction by key for friends
    public(friend) fun get_txn_by_key(
        key: ScheduleMapKey
    ): Option<ScheduledTransaction> acquires ScheduleQueue {
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        if (queue.txn_table.contains(key.txn_id)) {
            some(*queue.txn_table.borrow(key.txn_id))
        } else { none() }
    }

    /// Helper to get auth token from transaction
    public(friend) fun get_auth_token_from_txn(
        txn: &ScheduledTransaction
    ): ScheduledTxnAuthToken {
        match (txn.f) {
            ScheduledFunction::V1(_f) => {
                abort(error::internal(EAUTH_TOKEN_NOT_FOUND))
            },
            ScheduledFunction::V1WithAuthToken(_f, auth_token) => auth_token
        }
    }

    /// Validate auth token and cancel transaction if invalid
    /// Returns true if token was invalid and transaction was cancelled, false if token is valid
    public(friend) fun fail_txn_on_invalid_auth_token(
        txn: &ScheduledTransaction, txn_key: ScheduleMapKey, block_timestamp_ms: u64
    ): bool {
        let auth_token = get_auth_token_from_txn(txn);
        let sender_addr = txn.sender_addr;

        // Check if token is expired or auth number mismatched
        if (auth_token.expiration_time <= block_timestamp_ms
            || auth_token.authorization_num != get_auth_num(sender_addr)) {
            event::emit(
                TransactionFailedEvent {
                    scheduled_txn_time: txn_key.time,
                    scheduled_txn_hash: txn_key.txn_id,
                    sender_addr,
                    cancelled_txn_code: CancelledTxnCode::AuthExpired
                }
            );
            return true
        };
        false
    }

    /// Check if transaction has expired and emit failure event if so
    /// Returns true if transaction was expired and should be cancelled, false if not expired
    public(friend) fun fail_txn_on_expired(
        txn: &ScheduledTransaction, txn_key: ScheduleMapKey, block_timestamp_ms: u64
    ): bool {
        let sender_addr = txn.sender_addr;

        // Check if this transaction has expired based on its individual expiry_delta
        let transaction_expiry_time =
            if (txn.expiry_delta > 0) {
                txn_key.time + txn.expiry_delta
            } else {
                U64_MAX // If expiry_delta is 0, never expire
            };

        if (block_timestamp_ms > transaction_expiry_time) {
            // Transaction is expired
            event::emit(
                TransactionFailedEvent {
                    scheduled_txn_time: txn_key.time,
                    scheduled_txn_hash: txn_key.txn_id,
                    sender_addr,
                    cancelled_txn_code: CancelledTxnCode::Expired
                }
            );
            return true
        };
        false
    }

    /// Create updated auth token for execution (invalidates auth num if rescheduling not allowed)
    public(friend) fun create_updated_auth_token_for_execution(
        txn: &ScheduledTransaction
    ): ScheduledTxnAuthToken {
        let auth_token = get_auth_token_from_txn(txn);

        if (!auth_token.allow_rescheduling) {
            // Set authorization_num = 0 to invalidate the token for any use in the user function
            ScheduledTxnAuthToken {
                allow_rescheduling: auth_token.allow_rescheduling,
                expiration_time: auth_token.expiration_time,
                authorization_num: 0
            }
        } else {
            auth_token
        }
    }

    /// Helper function to remove scheduled transaction from txn_table after execution
    /// The scheduled transaction is removed from two data structures at different times:
    /// 1. From schedule_map (BigOrderedMap): Removed in next block's prologue to allow parallel execution
    ///    of all scheduled transactions in the current block
    /// 2. From txn_table: Removed immediately after transaction execution to enable
    ///    proper refunding of storage gas fees to the user
    public(friend) fun remove_txn_from_table(txn_id: u256) acquires ScheduleQueue {
        let queue_mut = borrow_global_mut<ScheduleQueue>(@aptos_framework);
        queue_mut.txn_table.remove(txn_id);
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
    public fun shutdown_test(fx: &signer) acquires AuxiliaryData {
        start_shutdown(fx);
    }

    #[test_only]
    public fun continue_shutdown_test(
        batch_size: u64
    ) acquires AuxiliaryData, ScheduleQueue, ToRemoveTbl {
        continue_shutdown(batch_size);
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
    public fun mark_txn_to_remove_test(key: ScheduleMapKey) acquires ToRemoveTbl {
        mark_txn_to_remove(key);
    }

    #[test_only]
    public fun cancel_test(
        sender: &signer,
        time: u64,
        gas_priority: u64,
        txn_id: u256
    ) acquires ScheduleQueue, AuxiliaryData {
        cancel(sender, time, gas_priority, txn_id);
    }

    #[test_only]
    public fun cancel_all_test(sender: &signer) acquires AuxiliaryData {
        cancel_all(sender);
    }

    #[test_only]
    fun get_module_status(): ScheduledTxnsModuleStatus acquires AuxiliaryData {
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        aux_data.module_status
    }

    #[test_only]
    public fun create_mock_auth_token(
        allow_rescheduling: bool, expiration_time: u64, authorization_num: u64
    ): ScheduledTxnAuthToken {
        ScheduledTxnAuthToken { allow_rescheduling, expiration_time, authorization_num }
    }

    struct State has copy, drop, store {
        count: u64
    }

    #[persistent]
    fun step(state: State) {
        if (state.count < 10) {
            state.count = state.count + 1;
        }
    }

    #[persistent]
    fun step_with_auth_token(
        state: State, _signer: &signer, _auth_token: ScheduledTxnAuthToken
    ) {
        if (state.count < 10) {
            state.count = state.count + 1;
        }
    }

    #[persistent]
    fun rescheduling_test_func(
        sender: &signer, auth_token: ScheduledTxnAuthToken
    ) {
        let current_time = timestamp::now_microseconds() / 1000;
        let next_schedule_time = current_time + 1000; // Would schedule 1 second later

        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| rescheduling_test_func(
                signer, auth_token
            );

        let txn =
            new_scheduled_transaction_reuse_auth_token(
                sender,
                auth_token,
                next_schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        insert(sender, txn);
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_basic(fx: &signer, user: signer) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        // Setup test environment
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Create transactions with same scheduled_time but different gas prices
        let state = State { count: 8 };
        let foo = || step(state);
        let schedule_time1 = curr_mock_time_micro_s / 1000 + 1000;
        let schedule_time2 = schedule_time1 * 2;
        let schedule_time3 = schedule_time1 * 4;
        let txn1 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time1,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            ); // time: 1s, gas: 20
        let txn2 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time1,
                100,
                300,
                EXPIRY_DELTA_DEFAULT,
                foo
            ); // time: 1s, gas: 30
        let txn3 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time1,
                100,
                100,
                EXPIRY_DELTA_DEFAULT,
                foo
            ); // time: 1s, gas: 10

        // Create transactions with same scheduled_time and gas price
        let txn4 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time2,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            ); // time: 2s, gas: 20
        let txn5 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time2,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        let txn6 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time2,
                200,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            ); // time: 2s, gas: 20

        let txn7 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time3,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            ); // time: 2s, gas: 20
        let txn8 =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time3,
                200,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
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
        remove_txns(); // Should remove first 3 txns
        assert!(get_num_txns() == 5, get_num_txns());

        // Test get_ready_transactions at t > schedule_time2 (should return next 3 txns)
        let ready_txns = get_ready_transactions(schedule_time2 + 1000);
        assert!(ready_txns.length() == 3, ready_txns.length());

        // Execute and remove 2 transactions
        mark_txn_to_remove(txn4_key);
        mark_txn_to_remove(txn6_key);
        remove_txns(); // Should remove 2 txns
        assert!(get_num_txns() == 3, get_num_txns());

        let ready_txns = get_ready_transactions(schedule_time2 + 1000);
        assert!(ready_txns.length() == 1, ready_txns.length());

        // Execute and remove txns 7 and 8; lets expire txn 5
        mark_txn_to_remove(txn7_key);
        mark_txn_to_remove(txn8_key);

        remove_txns();
        assert!(get_num_txns() == 1, get_num_txns());

        // try expiring a txn by getting it late; we should get the txn even if it is expired as expiration happens
        // in user_func_wrapper::execute_user_function()
        let expired_time = schedule_time2 + EXPIRY_DELTA_DEFAULT + 1000;
        assert!(
            get_ready_transactions(expired_time).length() == 1,
            get_ready_transactions(expired_time).length()
        );
        assert!(get_num_txns() == 1, get_num_txns());

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
        let foo = || step(state);
        let txn =
            new_scheduled_transaction_no_signer(
                user_addr,
                past_time,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

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
        let foo = || step(state);
        let txn =
            new_scheduled_transaction_no_signer(
                user_addr,
                future_time,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
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
        let foo = || step(state);
        let txn =
            new_scheduled_transaction_no_signer(
                user_addr,
                future_time,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        let txn_id = insert(&user, txn);

        // Try to cancel the transaction with wrong user
        cancel_with_key(&other_user, txn_id); // Should fail with EINVALID_SIGNER
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 851971)]
    // error::unavailable(EUNAVAILABLE)
    fun test_insert_when_stopped(fx: &signer, user: signer) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;

        // Setup test environment
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Stop scheduling
        start_shutdown(fx);

        // Try to insert a transaction after shutdown
        let future_time = curr_mock_time_micro_s / 1000 + 1000;
        let state = State { count: 8 };
        let foo = || step(state);
        let txn =
            new_scheduled_transaction_no_signer(
                user_addr,
                future_time,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

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
            let foo = || step(state);
            let txn =
                new_scheduled_transaction_no_signer(
                    user_addr,
                    curr_mock_time_micro_s / 1000 + 1000 + i, // Different times to ensure unique ordering
                    100,
                    200,
                    EXPIRY_DELTA_DEFAULT,
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

        // Verify start_shutdown behavior
        let txns_count_pre_batch = get_num_txns();
        start_shutdown(fx);

        // Verify that next call to shutdown complete without error
        while (txns_count_pre_batch > 0) {
            continue_shutdown(SHUTDOWN_CANCEL_BATCH_SIZE_DEFAULT);
            let txns_count_post_batch = get_num_txns();
            assert!(
                txns_count_pre_batch
                    <= txns_count_post_batch + SHUTDOWN_CANCEL_BATCH_SIZE_DEFAULT,
                txns_count_post_batch
            );
            txns_count_pre_batch = txns_count_post_batch;
        };

        // Check that shutdown is complete and also capability to re-initialize
        re_initialize(fx);

        let state = State { count: 8 };
        let foo = || step(state);
        let txn1 =
            new_scheduled_transaction_no_signer(
                user_addr,
                curr_mock_time_micro_s / 1000 + 1000,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        insert(&user, txn1);
    }

    #[test(fx = @0x1, user = @0x123, vm = @0x0)]
    fun test_pause_unpause_flow(fx: &signer, user: signer, vm: &signer) acquires AuxiliaryData, ScheduleQueue {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Initially module should be active
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Active, 0);

        // Insert a transaction to verify state
        let user_addr = signer::address_of(&user);
        let schedule_time = curr_mock_time_micro_s / 1000 + 1000;
        let state = State { count: 8 };
        let foo = || step(state);
        let txn =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        let _txn_key = insert(&user, txn);
        assert!(get_num_txns() == 1, 1);

        // Pause the module
        pause_scheduled_txns(vm);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Paused, 2);

        // Unpause the module
        unpause_scheduled_txns(fx);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Active, 3);

        // Verify transactions are preserved
        assert!(get_num_txns() == 1, 6);
    }

    #[test(fx = @0x1, user = @0x123, vm = @0x0)]
    #[expected_failure(abort_code = 196619)]
    fun test_cannot_pause_from_paused_state(fx: &signer, user: signer, vm: &signer) acquires AuxiliaryData {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // First pause should succeed
        pause_scheduled_txns(vm);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Paused, 0);

        // Second pause should fail
        pause_scheduled_txns(vm);
    }

    #[test(fx = @0x1, user = @0x123)]
    #[expected_failure(abort_code = 196620)]
    fun test_cannot_unpause_from_active_state(fx: &signer, user: signer) acquires AuxiliaryData {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Module starts in Active state
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Active, 0);

        // Attempting to unpause while active should fail
        unpause_scheduled_txns(fx);
    }

    #[test(fx = @0x1, user = @0x123, vm = @0x0)]
    #[expected_failure(abort_code = 851971)]
    fun test_cannot_insert_when_paused(
        fx: &signer, user: signer, vm: &signer
    ) acquires AuxiliaryData, ScheduleQueue {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Pause the module
        pause_scheduled_txns(vm);
        assert!(get_module_status() == ScheduledTxnsModuleStatus::Paused, 0);

        // Try to insert a transaction while paused
        let user_addr = signer::address_of(&user);
        let schedule_time = curr_mock_time_micro_s / 1000 + 1000;
        let state = State { count: 8 };
        let foo = || step(state);
        let txn =
            new_scheduled_transaction_no_signer(
                user_addr,
                schedule_time,
                100,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );
        insert(&user, txn); // Should fail with EUNAVAILABLE
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_insert_with_reused_auth_token(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Initialize sender auth num to 1
        let sender_auth_num = get_or_init_auth_num(user_addr);
        assert!(sender_auth_num == 1, sender_auth_num);

        // Create transaction with auth token using test helper
        let schedule_time = curr_mock_time_micro_s / 1000 + 1000;
        let expiration_time = schedule_time + 10000; // 10 seconds after scheduled time

        let state = State { count: 8 };
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| step_with_auth_token(
                state, signer, auth_token
            );

        let auth_token = create_mock_auth_token(true, expiration_time, sender_auth_num);
        let txn =
            new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        // Insert transaction
        let txn_key = insert(&user, txn);
        assert!(get_num_txns() == 1, get_num_txns());

        // Verify the transaction was scheduled correctly
        assert!(txn_key.time == schedule_time, 0);
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_cancel_with_reused_auth_token(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 20000; // 20 seconds in future (more than CANCEL_DELTA_DEFAULT)
        let expiration_time = schedule_time + 10000;

        let state = State { count: 8 };
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| step_with_auth_token(
                state, signer, auth_token
            );

        let auth_token = create_mock_auth_token(true, expiration_time, sender_auth_num);
        let txn =
            new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        let txn_key = insert(&user, txn);
        assert!(get_num_txns() == 1, get_num_txns());

        // Cancel the transaction
        cancel_with_key(&user, txn_key);
        assert!(get_num_txns() == 0, get_num_txns());
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_cancel_entry_function(fx: &signer, user: signer) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 20000; // 20 seconds in future (more than CANCEL_DELTA_DEFAULT)
        let expiration_time = schedule_time + 10000;

        let state = State { count: 8 };
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| step_with_auth_token(
                state, signer, auth_token
            );

        // Create and insert a transaction with auth token
        let auth_token = create_mock_auth_token(true, expiration_time, sender_auth_num);
        let txn =
            new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        let txn_key = insert(&user, txn);
        assert!(get_num_txns() == 1, get_num_txns());

        // Test the cancel entry function using individual parameters
        cancel_test(
            &user,
            txn_key.time,
            txn_key.gas_priority,
            txn_key.txn_id
        );
        assert!(get_num_txns() == 0, get_num_txns());
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 65551)]
    // EAUTH_TOKEN_EXPIRED
    fun test_auth_token_expired_validation(fx: &signer, user: signer) {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 2000;
        let expiration_time = curr_mock_time_micro_s / 1000 - 1000; // Already expired!

        let state = State { count: 8 };
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| step_with_auth_token(
                state, signer, auth_token
            );

        let auth_token = create_mock_auth_token(true, expiration_time, sender_auth_num);
        new_scheduled_transaction_reuse_auth_token(
            &user,
            auth_token,
            schedule_time,
            1000,
            200,
            EXPIRY_DELTA_DEFAULT,
            foo
        );
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 65551)]
    // EAUTH_TOKEN_INSUFFICIENT_DURATION
    fun test_auth_token_insufficient_duration(fx: &signer, user: signer) {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 10000; // 10 seconds in future
        let expiration_time = curr_mock_time_micro_s / 1000 + 5000; // Expires before schedule time

        let state = State { count: 8 };
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| step_with_auth_token(
                state, signer, auth_token
            );

        let auth_token = create_mock_auth_token(true, expiration_time,sender_auth_num);
        new_scheduled_transaction_reuse_auth_token(
            &user,
            auth_token,
            schedule_time,
            1000,
            200,
            EXPIRY_DELTA_DEFAULT,
            foo
        );
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 65552)]
    // EAUTH_NUM_MISMATCH
    fun test_auth_num_mismatch_validation(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, AuxiliaryData {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        let sender_auth_num = get_or_init_auth_num(user_addr);
        let schedule_time = curr_mock_time_micro_s / 1000 + 2000;
        let expiration_time = schedule_time + 10000;

        let state = State { count: 8 };
        let foo =
            |signer: &signer, auth_token: ScheduledTxnAuthToken| step_with_auth_token(
                state, signer, auth_token
            );

        // This should fail due to auth num mismatch
        let auth_token = create_mock_auth_token(true, expiration_time,sender_auth_num + 5);
        let txn =
            new_scheduled_transaction_reuse_auth_token(
                &user,
                auth_token,
                schedule_time,
                1000,
                200,
                EXPIRY_DELTA_DEFAULT,
                foo
            );

        insert(&user, txn); // Should fail
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 196609)]
    // EAUTH_NUM_NOT_FOUND
    fun test_cancel_all_uninitialized_sender(fx: &signer, user: signer) acquires AuxiliaryData {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // This should fail because increment_auth_num requires sender to exist
        cancel_all(&user);
    }
}
