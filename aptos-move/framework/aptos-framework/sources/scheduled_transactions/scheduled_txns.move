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

    const U64_MAX: u64 = 18446744073709551615;

    /// Conversion factor between our time granularity (100ms) and microseconds
    const MICRO_CONVERSION_FACTOR: u64 = 100000;

    /// Conversion factor between our time granularity (100ms) and milliseconds
    const MILLI_CONVERSION_FACTOR: u64 = 100;

    /// If we cannot schedule in 100 * time granularity (10s, i.e 100 blocks), we will abort the txn
    const EXPIRY_DELTA_DEFAULT: u64 = 100;

    /// The maximum number of scheduled transactions that can be run in a block
    const GET_READY_TRANSACTIONS_LIMIT: u64 = 1000;

    /// The maximum number of transactions that can be cancelled in a block during shutdown
    const SHUTDOWN_CANCEL_LIMIT: u64 = GET_READY_TRANSACTIONS_LIMIT * 2;

    /// SHA3-256 produces 32 bytes
    const TXN_ID_SIZE: u16 = 32;

    /// The average size of a scheduled transaction to provide an estimate of leaf nodes of BigOrderedMap
    const AVG_SCHED_TXN_SIZE: u16 = 1024;

    // todo: get rid of this, store it externally if the size > 10 KB
    /// Max size of a scheduled transaction
    const MAX_SCHED_TXN_SIZE: u64 = 10 * 1024;

    /// ScheduledTransaction with scheduled_time, gas params, and function
    struct ScheduledTransaction has copy, drop, store {
        /// 32 bytes
        sender_addr: address,
        /// UTC timestamp in milliseconds
        scheduled_time_ms: u64,
        /// Maximum gas to spend for this transaction
        max_gas_amount: u64,
        /// Charged @ lesser of {max_gas_unit_price, max_gas_unit_price other than this in the block executed}
        max_gas_unit_price: u64,
        /// Option to pass a signer to the function
        pass_signer: bool,
        /// Variables are captured in the closure; optionally a signer is passed; no return
        f: |Option<signer>| has copy + store + drop,
    }

    /// We pass around only needed info
    struct ScheduledTransactionInfoWithKey has drop {
        sender_addr: address,
        max_gas_amount: u64,
        max_gas_unit_price: u64,
        /// To be determined during execution
        gas_unit_price_charged: u64,
        key: ScheduleMapKey
    }

    /// First sorted in ascending order of time, then on gas priority, and finally on txn_id
    /// gas_priority = U64_MAX - gas_unit_price; we want higher gas_unit_price to come before lower gas_unit_price
    /// The goal is to have fixed (less variable) size 'key', 'val' entries in BigOrderedMap, hence we use txn_id
    /// as a key. That is we have "{time, gas_priority, txn_id} -> ScheduledTxn" instead of
    /// "{time, gas_priority} --> List<(txn_id, ScheduledTxn)>".
    /// Note: ScheduledTxn is still variable size though due to its closure.
    struct ScheduleMapKey has copy, drop, store {
        /// UTC timestamp in the granularity of 100ms
        time: u64,
        gas_priority: u64,
        /// SHA3-256
        txn_id: vector<u8>
    }

    struct ScheduleQueue has key {
        /// key_size = 48 bytes; value_size = key_size + AVG_SCHED_TXN_SIZE
        schedule_map: BigOrderedMap<ScheduleMapKey, ScheduledTransaction>,
    }

    /// BigOrderedMap has MAX_NODE_BYTES = 409600 (400KB), MAX_DEGREE = 4096, DEFAULT_TARGET_NODE_SIZE = 4096;
    const BIG_ORDRD_MAP_TGT_ND_SZ: u16 = 4096;
    const SCHEDULE_MAP_KEY_SIZE: u16 = TXN_ID_SIZE + 8 + 8; // 32 + 8 + 8 = 48 bytes

    /// Signer for the store for gas fee deposits
    struct AuxiliaryData has key {
        // todo: check if this is secure
        gas_fee_deposit_store_signer_cap: account::SignerCapability,
        stop_scheduling: bool,
        /// If we cannot schedule in expiry_delta * time granularity(100ms), we will abort the txn
        expiry_delta: u64,
    }

    /// We want reduce the contention while scheduled txns are being executed
    // todo: check if 32 is a good number
    const TO_REMOVE_PARALLELISM: u64 = 1024;
    struct ToRemoveTbl has key {
        remove_tbl: Table<u16, vector<ScheduleMapKey>>
    }

    enum CancelledTxnCode has drop, store {
        /// Scheduling service is stopped
        Shutdown,
        /// Transaction was expired
        Expired,
    }

    #[event]
    struct TransactionExpiredEvent has drop, store {
        key: ScheduleMapKey,
        sender_addr: address,
        cancelled_txn_code: CancelledTxnCode,
    }

    #[event]
    struct ShutdownEvent has drop, store {
        complete: bool,
    }

    // temporary non persistent struct
    struct KeyAndTxnInfo has drop {
        key: ScheduleMapKey,
        account_addr: address,
        deposit_amt: u64,
    }

    /// Can be called only by the framework
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);

        // Create owner account for handling deposits
        let owner_addr = @0xb; // Replace with your desired address
        let (owner_signer, owner_cap) =
            account::create_framework_reserved_account(owner_addr);

        // Initialize fungible store for the owner
        let metadata = ensure_paired_metadata<AptosCoin>();
        let deposit_store = primary_fungible_store::ensure_primary_store_exists(
            signer::address_of(&owner_signer), metadata
        );
        upgrade_store_to_concurrent(&owner_signer, deposit_store);

        // Store the capability
        move_to(framework, AuxiliaryData { gas_fee_deposit_store_signer_cap: owner_cap, stop_scheduling: false, expiry_delta: EXPIRY_DELTA_DEFAULT });

        // Initialize queue
        let queue = ScheduleQueue {
            schedule_map: big_ordered_map::new_with_config(
                BIG_ORDRD_MAP_TGT_ND_SZ / SCHEDULE_MAP_KEY_SIZE,
                (BIG_ORDRD_MAP_TGT_ND_SZ / (SCHEDULE_MAP_KEY_SIZE + AVG_SCHED_TXN_SIZE)),
                true
            ),
        };
        move_to(framework, queue);

        // Parallelizable data structure used to track executed txn_ids.
        move_to(
            framework,
            ToRemoveTbl {
                remove_tbl: table::new<u16, vector<ScheduleMapKey>>()
            }
        );
    }

    /// Stop, remove and refund all scheduled txns; can be called only by the framework
    public fun shutdown(
        framework: &signer
    ) acquires ScheduleQueue, ToRemoveTbl, AuxiliaryData {
        system_addresses::assert_aptos_framework(framework);

        // set stop_scheduling flag
        let aux_data = borrow_global_mut<AuxiliaryData>(signer::address_of(framework));
        aux_data.stop_scheduling = true;

        let txns_to_cancel = vector::empty<KeyAndTxnInfo>();
        // Make a list of txns to cancel with their keys and signers
        {
            let queue = borrow_global<ScheduleQueue>(signer::address_of(framework));

            // Iterate through schedule_map to get all transactions
            let iter = queue.schedule_map.new_begin_iter();
            let cancel_count = 0;
            while ((!iter.iter_is_end(&queue.schedule_map)) && (cancel_count < SHUTDOWN_CANCEL_LIMIT)) {
                let key = iter.iter_borrow_key();
                let txn = iter.iter_borrow(&queue.schedule_map);
                let deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;
                txns_to_cancel.push_back(KeyAndTxnInfo {
                    key: *key,
                    account_addr: txn.sender_addr,
                    deposit_amt
                });
                cancel_count = cancel_count + 1;
                iter = iter.iter_next(&queue.schedule_map);
            };
        };

        // Cancel transactions
        while (!txns_to_cancel.is_empty()) {
            let KeyAndTxnInfo { key, account_addr, deposit_amt } = txns_to_cancel.pop_back();
            cancel_internal(account_addr, key, deposit_amt);
            event::emit(TransactionExpiredEvent {
                key,
                sender_addr: account_addr,
                cancelled_txn_code: CancelledTxnCode::Shutdown
            });
        };

        // Remove and destroy schedule_map if empty
        let queue = borrow_global<ScheduleQueue>(signer::address_of(framework));
        if (queue.schedule_map.is_empty()) {
            event::emit(ShutdownEvent { complete: true });
        };

        // Clean up ToRemoveTbl
        let ToRemoveTbl { remove_tbl } = borrow_global_mut<ToRemoveTbl>(signer::address_of(framework));
        let i = 0;
        while (i < TO_REMOVE_PARALLELISM) {
            if (remove_tbl.contains((i as u16))) {
                remove_tbl.remove((i as u16));
            };
            i = i + 1;
        };
    }

    /// todo: Do we need a function to pause/unpause without issuing refund of deposit ???

    /// Change the expiry delta for scheduled transactions; can be called only by the framework
    public fun set_expiry_delta(
        framework: &signer,
        new_expiry_delta: u64
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
        max_gas_unit_price: u64,
        pass_signer: bool,
        f: |Option<signer>| has copy + store + drop,
    ): ScheduledTransaction {
        ScheduledTransaction {
            sender_addr,
            scheduled_time_ms,
            max_gas_amount,
            max_gas_unit_price,
            pass_signer,
            f,
        }
    }

    /// Insert a scheduled transaction into the queue. ScheduleMapKey is returned to user, which can be used to cancel the txn.
    public fun insert(
        sender: &signer,
        txn: ScheduledTransaction
    ): ScheduleMapKey acquires ScheduleQueue, AuxiliaryData {
        // If scheduling is shutdown, we cannot schedule any more transactions
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        assert!(!aux_data.stop_scheduling, error::unavailable(EUNAVAILABLE));

        // we expect the sender to be a permissioned signer
        assert!(
            signer::address_of(sender) == txn.sender_addr,
            error::permission_denied(EINVALID_SIGNER)
        );

        // Only schedule txns in the future
        let txn_time = txn.scheduled_time_ms / MILLI_CONVERSION_FACTOR; // Round down to the nearest 100ms
        let block_time = timestamp::now_microseconds() / MICRO_CONVERSION_FACTOR;
        assert!(txn_time > block_time, error::invalid_argument(EINVALID_TIME));

        assert!(
            txn.max_gas_unit_price >= 100,
            error::invalid_argument(ELOW_GAS_UNIT_PRICE)
        );

        assert!(
            bcs::serialized_size(&txn) < MAX_SCHED_TXN_SIZE,
            error::invalid_argument(ETXN_TOO_LARGE)
        );

        // Generate unique transaction ID
        let txn_id = sha3_256(bcs::to_bytes(&txn));

        // Insert the transaction into the schedule_map
        // Create schedule map key
        let key = ScheduleMapKey {
            time: txn_time,
            gas_priority: U64_MAX - txn.max_gas_unit_price,
            txn_id
        };

        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);
        queue.schedule_map.add(key, txn);

        // Collect deposit
        // Get owner signer from capability
        let gas_deposit_store_cap =
            borrow_global<AuxiliaryData>(@aptos_framework);
        let gas_deposit_store_signer =
            account::create_signer_with_capability(&gas_deposit_store_cap.gas_fee_deposit_store_signer_cap);
        let gas_deposit_store_addr = signer::address_of(&gas_deposit_store_signer);

        coin::transfer<AptosCoin>(
            sender,
            gas_deposit_store_addr,
            txn.max_gas_amount * txn.max_gas_unit_price
        );

        key
    }

    /// Cancel a scheduled transaction, must be called by the signer who originally scheduled the transaction.
    public fun cancel(
        sender: &signer,
        key: ScheduleMapKey
    ) acquires ScheduleQueue, AuxiliaryData {
        // If scheduling is shutdown, we cannot schedule any more transactions
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        assert!(!aux_data.stop_scheduling, error::unavailable(EUNAVAILABLE));

        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        if (!queue.schedule_map.contains(&key)) {
            return
        };

        let txn = *queue.schedule_map.borrow(&key);
        let deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;

        // verify sender
        assert!(
            signer::address_of(sender) == txn.sender_addr,
            error::permission_denied(EINVALID_SIGNER)
        );
        cancel_internal(signer::address_of(sender), key, deposit_amt);
    }

    /// Internal cancel function that takes an address instead of signer. No signer verification, assumes key is present
    /// in the schedule_map.
    fun cancel_internal(
        account_addr: address,
        key: ScheduleMapKey,
        deposit_amt: u64,
    ) acquires ScheduleQueue, AuxiliaryData {
        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);

        // Remove the transaction from schedule_map
        queue.schedule_map.remove(&key);

        // Refund the deposit
        // Get owner signer from capability
        let gas_deposit_store_cap =
            borrow_global<AuxiliaryData>(@aptos_framework);
        let gas_deposit_store_signer =
            account::create_signer_with_capability(&gas_deposit_store_cap.gas_fee_deposit_store_signer_cap);

        // Refund deposit from owner's store to sender
        coin::transfer<AptosCoin>(
            &gas_deposit_store_signer,
            account_addr,
            deposit_amt
        );
    }

    /// Gets txns due to be run; also expire txns that could not be run for a while (mostly due to low gas priority)
    fun get_ready_transactions(
        timestamp_ms: u64
    ): vector<ScheduledTransactionInfoWithKey> acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        remove_txns();
        // If scheduling is shutdown, we cannot schedule any more transactions
        let aux_data = borrow_global<AuxiliaryData>(@aptos_framework);
        if (aux_data.stop_scheduling) {
            return vector::empty<ScheduledTransactionInfoWithKey>();
        };

        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        let block_time = timestamp_ms / MILLI_CONVERSION_FACTOR;
        let scheduled_txns = vector::empty<ScheduledTransactionInfoWithKey>();
        let count = 0;
        let txns_to_expire = vector::empty<KeyAndTxnInfo>();

        let iter = queue.schedule_map.new_begin_iter();
        while (!iter.iter_is_end(&queue.schedule_map) && count < GET_READY_TRANSACTIONS_LIMIT) {
            let key = iter.iter_borrow_key();
            if (key.time > block_time) {
                break;
            };
            let txn = *iter.iter_borrow(&queue.schedule_map);
            let scheduled_txn_info_with_key = ScheduledTransactionInfoWithKey {
                sender_addr: txn.sender_addr,
                max_gas_amount: txn.max_gas_amount,
                max_gas_unit_price: txn.max_gas_unit_price,
                gas_unit_price_charged: txn.max_gas_unit_price,
                key: *key,
            };

            if ((block_time - key.time) > aux_data.expiry_delta) {
                let deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;
                txns_to_expire.push_back(KeyAndTxnInfo {
                    key: *key,
                    account_addr: txn.sender_addr,
                    deposit_amt
                });
            } else {
                scheduled_txns.push_back(scheduled_txn_info_with_key);
            };
            // we do not want an unbounded size of ready or expirable txns; hence we increment either way
            count = count + 1;
            iter = iter.iter_next(&queue.schedule_map);
        };

        // Cancel expired transactions
        while (!txns_to_expire.is_empty()) {
            let KeyAndTxnInfo { key, account_addr, deposit_amt } = txns_to_expire.pop_back();
            cancel_internal(
                account_addr,
                key,
                deposit_amt
            );
            event::emit(TransactionExpiredEvent {
                key,
                sender_addr: account_addr,
                cancelled_txn_code: CancelledTxnCode::Expired
            });
        };
        scheduled_txns
    }

    /// Increment after every scheduled transaction is run
    /// IMP: Make sure this does not affect parallel execution of txns
    public(friend) fun finish_execution(key: ScheduleMapKey) acquires ToRemoveTbl {
        // Get first 8 bytes of the hash as u64 and then mod
        let hash_bytes = key.txn_id;
        assert!(hash_bytes.length() == 32, hash_bytes.length()); // SHA3-256 produces 32 bytes

        // Take last 8 bytes and convert to u64
        let hash_last_8_bytes = vector::empty();
        let idx = hash_bytes.length() - 8;
        while (idx < hash_bytes.length()) {
            hash_last_8_bytes.push_back(hash_bytes[idx]);
            idx = idx + 1;
        };
        let value = from_bcs::to_u64(hash_last_8_bytes);

        // Calculate table index using hash
        let tbl_idx = ((value % TO_REMOVE_PARALLELISM) as u16);
        let to_remove = borrow_global_mut<ToRemoveTbl>(@aptos_framework);

        if (!to_remove.remove_tbl.contains(tbl_idx)) {
            let keys = vector::empty<ScheduleMapKey>();
            keys.push_back(key);
            to_remove.remove_tbl.add(tbl_idx, keys);
        } else {
            let keys = to_remove.remove_tbl.borrow_mut(tbl_idx);
            keys.push_back(key);
        };
    }

    /// Remove the txns that are run
    public(friend) fun remove_txns() acquires ToRemoveTbl, ScheduleQueue {
        let to_remove = borrow_global_mut<ToRemoveTbl>(@aptos_framework);
        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);
        let tbl_idx: u16 = 0;

        let remove_count = 0;
        while ((tbl_idx as u64) < TO_REMOVE_PARALLELISM) {
            if (to_remove.remove_tbl.contains(tbl_idx)) {
                let keys = to_remove.remove_tbl.borrow_mut(tbl_idx);

                while (!keys.is_empty()) {
                    let key = keys.pop_back();
                    if (queue.schedule_map.contains(&key)) {
                        // Remove transaction from schedule_map
                        remove_count = remove_count + 1;
                        queue.schedule_map.remove(&key);
                    };
                };
            };
            tbl_idx = tbl_idx + 1;
        };
    }

    /// Called by the executor when the scheduled transaction is run
    fun execute_user_function_wrapper(
        signer: signer,
        txn_key: ScheduleMapKey,
    ) acquires ScheduleQueue {
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        assert!(queue.schedule_map.contains(&txn_key), 0);

        let txn = *queue.schedule_map.borrow(&txn_key);
        let pass_signer = txn.pass_signer;
        let f = txn.f;
        if (pass_signer) {
            f(some(signer));
        } else {
            f(std::option::none());
        };
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
        let owner_signer = account::create_signer_with_capability(&owner_cap.gas_fee_deposit_store_signer_cap);
        owner_signer
    }

    #[test_only]
    public fun get_func_from_txn_key(
        key: ScheduleMapKey
    ): |Option<signer>| has copy acquires ScheduleQueue {
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        assert!(queue.schedule_map.contains(&key), 0);
        let txn = *queue.schedule_map.borrow(&key);
        txn.f
    }

    #[test_only]
    public fun shutdown_test(
        fx: &signer
    ) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        shutdown(fx);
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
    fun test_basic(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
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
            user_addr,
            schedule_time1,
            100,
            200,
            false,
            foo
        ); // time: 1s, gas: 20
        let txn2 = new_scheduled_transaction(
            user_addr,
            schedule_time1,
            100,
            300,
            false,
            foo
        ); // time: 1s, gas: 30
        let txn3 = new_scheduled_transaction(
            user_addr,
            schedule_time1,
            100,
            100,
            false,
            foo
        ); // time: 1s, gas: 10

        // Create transactions with same scheduled_time and gas price
        let txn4 = new_scheduled_transaction(
            user_addr,
            schedule_time2,
            1000,
            200,
            false,
            foo
        ); // time: 2s, gas: 20
        let txn5 = new_scheduled_transaction(
            user_addr,
            schedule_time2,
            100,
            200,
            false,
            foo
        );
        let txn6 = new_scheduled_transaction(
            user_addr,
            schedule_time2,
            200,
            200,
            false,
            foo
        ); // time: 2s, gas: 20

        let txn7 = new_scheduled_transaction(
            user_addr,
            schedule_time3,
            100,
            200,
            false,
            foo
        ); // time: 2s, gas: 20
        let txn8 = new_scheduled_transaction(
            user_addr,
            schedule_time3,
            200,
            200,
            false,
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
        finish_execution(txn1_key);
        finish_execution(txn2_key);
        finish_execution(txn3_key);
        remove_txns(); // Should remove first 3 txns
        assert!(get_num_txns() == 5, get_num_txns());

        // Test get_ready_transactions at t > schedule_time2 (should return next 3 txns)
        let ready_txns = get_ready_transactions(schedule_time2 + 1000);
        assert!(ready_txns.length() == 3, ready_txns.length());

        // Execute and remove 2 transactions
        finish_execution(txn4_key);
        finish_execution(txn6_key);
        remove_txns(); // Should remove 2 txns
        assert!(get_num_txns() == 3, get_num_txns());

        let ready_txns = get_ready_transactions(schedule_time2 + 1000);
        assert!(ready_txns.length() == 1, ready_txns.length());

        // Execute and remove txns 7 and 8; lets expire txn 5
        finish_execution(txn7_key);
        finish_execution(txn8_key);

        remove_txns();
        assert!(get_num_txns() == 1, get_num_txns());

        // try expiring a txn by getting it late
        let expired_time =
            schedule_time2 + EXPIRY_DELTA_DEFAULT * MILLI_CONVERSION_FACTOR
                + 1000;
        assert!(
            get_ready_transactions(expired_time).length() == 0,
            get_ready_transactions(expired_time).length()
        );
        assert!(get_num_txns() == 0, get_num_txns());

        shutdown(fx);
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 65538)]
    // error::invalid_argument(EINVALID_TIME)
    fun test_insert_past_time(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, AuxiliaryData {
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
    #[expected_failure(abort_code = 851971)] // error::unavailable(EUNAVAILABLE)
    fun test_insert_when_stopped(
        fx: &signer,
        user: signer
    ) acquires ScheduleQueue, AuxiliaryData, ToRemoveTbl {
        let user_addr = signer::address_of(&user);
        let curr_mock_time_micro_s = 1000000;

        // Setup test environment
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Stop scheduling
        shutdown(fx);

        // Try to insert a transaction after shutdown
        let future_time = curr_mock_time_micro_s / 1000 + 1000;
        let state = State { count: 8 };
        let foo = |s: Option<signer>| step(state, s);
        let txn = new_scheduled_transaction(
            user_addr,
            future_time,
            100,
            200,
            false,
            foo
        );

        // This should fail with EUNAVAILABLE
        insert(&user, txn);
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_exceeding_limits(
        fx: &signer,
        user: signer
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
            let txn = new_scheduled_transaction(
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
        shutdown(fx);
        let post_shutdown_count = get_num_txns();
        let cancelled_count = pre_shutdown_count - post_shutdown_count;
        assert!(cancelled_count == SHUTDOWN_CANCEL_LIMIT, cancelled_count);

        // Verify that next call to shutdown complete without error
        shutdown(fx);
    }
}
