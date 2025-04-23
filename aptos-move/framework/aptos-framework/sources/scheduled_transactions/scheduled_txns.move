module aptos_framework::scheduled_txns {
    use std::bcs;
    use std::error;
    use std::hash::sha3_256;
    use std::signer;
    use std::vector;
    use aptos_std::debug;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::coin;
    use aptos_framework::coin::ensure_paired_metadata;
    use aptos_framework::permissioned_signer;
    use aptos_framework::permissioned_signer::StorablePermissionedHandle;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    #[test_only]
    use aptos_framework::fungible_asset::Metadata;
    #[test_only]
    use aptos_framework::object::Object;
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

    const U64_MAX: u64 = 18446744073709551615;

    /// Conversion factor between our time granularity (100ms) and microseconds
    const MICRO_CONVERSION_FACTOR: u64 = 100000;

    /// Conversion factor between our time granularity (100ms) and milliseconds
    const MILLI_CONVERSION_FACTOR: u64 = 100;

    /// If we cannot schedule in 100 * time granularity (10s, i.e 100 blocks), we will abort the txn
    const EXPIRY_DELTA: u64 = 100;

    /// SHA3-256 produces 32 bytes
    const TXN_ID_SIZE: u16 = 32;

    // Todo: Confirm this.
    /// The maximum size of a function in bytes
    const MAX_FUNC_SIZE: u16 = 1024;

    // Todo: Confirm this is a reasonable estimate
    /// The maximum size of a function in bytes
    const AVG_FUNC_SIZE: u16 = 1000;
    const AVG_SCHED_TXN_SIZE: u16 = 128 + AVG_FUNC_SIZE; // strictly it is 112 + AVG_FUNC_SIZE

    /// ScheduledTransaction with permission signer handle, scheduled_time, gas params, and function
    struct ScheduledTransaction has copy, drop, store {
        /// 72 bytes (32 + 32 + 8)
        sender_handle: StorablePermissionedHandle,
        /// 100ms granularity
        scheduled_time: u64,
        /// Maximum gas to spend for this transaction
        max_gas_amount: u64,
        /// Charged @ lesser of {max_gas_unit_price, max_gas_unit_price other than this in the block executed}
        max_gas_unit_price: u64,
        /// txn to be rescheduled at scheduled_time + next_schedule_delta_time.
        /// Note: (1) Once set, the txn will be rescheduled at the same delta interval next time, and so on.
        ///       (2) Can be cancelled, with the same id returned in insert(), to stop the perpetual rescheduling.
        ///       (3) If one rescheduled fails or is expired, the perpetual rescheduling chain will be broken.
        ///       (4) If scheduled_time + next_schedule_delta_time < current_time, the txn reschedule will fail.
        next_schedule_delta_time: u64,
        /// Variables are captured in the closure; no arguments passed; no return
        f: ||
    }

    /// We pass the id around instead re-computing it
    struct ScheduledTransactionWithKey has copy, drop, store {
        txn: ScheduledTransaction,
        key: ScheduleMapKey
    }

    /// SHA3-256
    struct TransactionId has copy, drop, store {
        hash: vector<u8>
    }

    /// First sorted in ascending order of time, then on gas priority, and finally on txn_id
    /// gas_priority = U64_MAX - gas_unit_price; we want higher gas_unit_price to come before lower gas_unit_price
    /// The goal is to have fixed size key, val entries in BigOrderedMap, hence we use txn_id as a key instead of
    /// having {time, gas_priority} --> List<txn_id>
    struct ScheduleMapKey has copy, drop, store {
        time: u64,
        gas_priority: u64,
        txn_id: TransactionId
    }

    /// Dummy struct to use as a value type in BigOrderedMap
    struct Empty has copy, drop, store {}

    struct ScheduleQueue has key {
        /// key_size = 48 bytes; value_size = key_size + AVG_SCHED_TXN_SIZE
        schedule_map: BigOrderedMap<ScheduleMapKey, ScheduledTransaction>,
    }

    /// BigOrderedMap has MAX_NODE_BYTES = 409600 (400KB), MAX_DEGREE = 4096, DEFAULT_TARGET_NODE_SIZE = 4096;
    const BIG_ORDRD_MAP_TGT_ND_SZ: u16 = 4096;
    const SCHEDULE_MAP_KEY_SIZE: u16 = 48;
    // Leaf node size = 48 * 80 (leaf_degree) = 3840 bytes (<= DEFAULT_TARGET_NODE_SIZE)
    // todo: check if it can be DEFAULT_TARGET_NODE_SIZE/SCHEDULE_MAP_KEY_SIZE; check if value size is indeed 0
    const SCHEDULE_MAP_LEAF_DEGREE: u16 = 80;

    /// Signer for the store for gas fee deposits
    // todo: check if this is secure
    struct GasFeeDepositStoreSignerCap has key {
        cap: account::SignerCapability
    }

    /// We want reduce the contention while scheduled txns are being executed
    // todo: check if 32 is a good number
    const TO_REMOVE_PARALLELISM: u64 = 32;
    struct ToRemoveTbl has key {
        remove_tbl: Table<u16, vector<ScheduleMapKey>>
    }

    enum Result<T> has copy, drop {
        Ok(T),
        Err(u64)
    }

    struct KeySignerPair has drop {
        key: ScheduleMapKey,
        signer: signer,
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
        primary_fungible_store::ensure_primary_store_exists(
            signer::address_of(&owner_signer), metadata
        );

        // Store the capability
        move_to(framework, GasFeeDepositStoreSignerCap { cap: owner_cap });

        // Initialize queue
        let queue = ScheduleQueue {
            schedule_map: big_ordered_map::new_with_config(
                BIG_ORDRD_MAP_TGT_ND_SZ / SCHEDULE_MAP_KEY_SIZE,
                (BIG_ORDRD_MAP_TGT_ND_SZ / (TXN_ID_SIZE + AVG_SCHED_TXN_SIZE)),
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
    ) acquires ScheduleQueue, ToRemoveTbl, GasFeeDepositStoreSignerCap {
        system_addresses::assert_aptos_framework(framework);

        // Make a list of txns to cancel with their keys and signers
        let txns_to_cancel = vector::empty<KeySignerPair>();
        let queue = borrow_global<ScheduleQueue>(signer::address_of(framework));

        // Iterate through schedule_map to get all transactions
        let iter = queue.schedule_map.new_begin_iter();
        while (!iter.iter_is_end(&queue.schedule_map)) {
            let key = iter.iter_borrow_key();
            let txn = iter.iter_borrow(&queue.schedule_map);
            let schedule_txn_signer = permissioned_signer::signer_from_storable_permissioned_handle(
                &txn.sender_handle
            );
            txns_to_cancel.push_back(KeySignerPair {
                key: *key,
                signer: schedule_txn_signer
            });
            iter = iter.iter_next(&queue.schedule_map);
        };

        // Cancel all transactions
        while (!txns_to_cancel.is_empty()) {
            let KeySignerPair { key, signer } = txns_to_cancel.pop_back();
            cancel(&signer, key);
        };

        // Remove and destroy resource
        let ScheduleQueue { schedule_map } = move_from<ScheduleQueue>(signer::address_of(framework));
        schedule_map.destroy(|_| {});

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

    /// todo: Do we need a function to pause ???
    ///
    /*public fun create_scheduled_txn(
        s: &signer,
        f: ||,
    ): ScheduledTransaction {

    }*/

    /// Insert a scheduled transaction into the queue. Txn_id is returned to user, which can be used to cancel the txn.
    public fun insert(
        sender: &signer,
        txn: ScheduledTransaction
    ): ScheduleMapKey acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        // Generate unique transaction ID
        let txn_id = TransactionId {
            hash: sha3_256(bcs::to_bytes(&txn))
        };

        // we expect the sender to be a permissioned signer
        let schedule_txn_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(
                &txn.sender_handle
            );
        assert!(
            signer::address_of(sender) == signer::address_of(&schedule_txn_signer),
            error::permission_denied(EINVALID_SIGNER)
        );

        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);

        // Only schedule txns in the future
        let txn_time = txn.scheduled_time / MILLI_CONVERSION_FACTOR; // Round down to the nearest 100ms
        let block_time = timestamp::now_microseconds() / MICRO_CONVERSION_FACTOR;
        assert!(txn_time > block_time, error::invalid_argument(EINVALID_TIME));

        // Insert the transaction into the schedule_map
        // Create schedule map key
        let key = ScheduleMapKey {
            time: txn_time,
            gas_priority: U64_MAX - txn.max_gas_unit_price,
            txn_id
        };
        queue.schedule_map.add(key, txn);

        // Collect deposit
        // Get owner signer from capability
        let gas_deposit_store_cap =
            borrow_global<GasFeeDepositStoreSignerCap>(@aptos_framework);
        let gas_deposit_store_signer =
            account::create_signer_with_capability(&gas_deposit_store_cap.cap);
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
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        let queue = borrow_global_mut<ScheduleQueue>(@aptos_framework);
        if (!queue.schedule_map.contains(&key)) {
            return
        };

        let txn = *queue.schedule_map.borrow(&key);
        let deposit_amt = txn.max_gas_amount * txn.max_gas_unit_price;

        // we expect the sender to be a permissioned signer
        let schedule_txn_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(
                &txn.sender_handle
            );
        assert!(
            signer::address_of(sender) == signer::address_of(&schedule_txn_signer),
            error::permission_denied(EINVALID_SIGNER)
        );

        // Remove the transaction from schedule_map
        queue.schedule_map.remove(&key);

        // Refund the deposit
        // Get owner signer from capability
        let gas_deposit_store_cap =
            borrow_global<GasFeeDepositStoreSignerCap>(@aptos_framework);
        let gas_deposit_store_signer =
            account::create_signer_with_capability(&gas_deposit_store_cap.cap);

        // Refund deposit from owner's store to sender
        coin::transfer<AptosCoin>(
            &gas_deposit_store_signer,
            signer::address_of(sender),
            deposit_amt
        );
    }

    /// Gets txns due to be run; also expire txns that could not be run for a while (mostly due to low gas priority)
    fun get_ready_transactions(
        timestamp: u64, limit: u64
    ): vector<ScheduledTransactionWithKey> acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        let block_time = timestamp / MILLI_CONVERSION_FACTOR;
        let scheduled_txns = vector::empty<ScheduledTransactionWithKey>();
        let count = 0;
        let txns_to_expire = vector::empty<KeySignerPair>();

        let iter = queue.schedule_map.new_begin_iter();
        while (!iter.iter_is_end(&queue.schedule_map) && count < limit) {
            let key = iter.iter_borrow_key();
            if (key.time > block_time) {
                return scheduled_txns;
            };
            let txn = *iter.iter_borrow(&queue.schedule_map);
            let scheduled_txn_with_id = ScheduledTransactionWithKey {
                txn,
                key: *key,
            };

            if (key.time + EXPIRY_DELTA < block_time) {
                // Transaction has expired
                let schedule_txn_signer = permissioned_signer::signer_from_storable_permissioned_handle(
                    &txn.sender_handle
                );
                txns_to_expire.push_back(KeySignerPair {
                    key: *key,
                    signer: schedule_txn_signer
                });
            } else {
                scheduled_txns.push_back(scheduled_txn_with_id);
                count = count + 1;
            };
            iter = iter.iter_next(&queue.schedule_map);
        };

        // Cancel expired transactions
        while (!txns_to_expire.is_empty()) {
            let KeySignerPair { key, signer } = txns_to_expire.pop_back();
            cancel(&signer, key);
        };

        scheduled_txns
    }

    /// Increment after every scheduled transaction is run
    /// IMP: Make sure this does not affect parallel execution of txns
    public(friend) fun finish_execution(key: ScheduleMapKey) acquires ToRemoveTbl {
        // Get first 8 bytes of the hash as u64 and then mod
        let hash_bytes = key.txn_id.hash;
        assert!(hash_bytes.length() == 32, hash_bytes.length()); // SHA3-256 produces 32 bytes

        // Take first 8 bytes and convert to u64
        let value =
            ((hash_bytes[0] as u64) << 56) | ((hash_bytes[1] as u64) << 48)
                | ((hash_bytes[2] as u64) << 40) | ((hash_bytes[3] as u64) << 32)
                | ((hash_bytes[4] as u64) << 24) | ((hash_bytes[5] as u64) << 16)
                | ((hash_bytes[6] as u64) << 8) | (hash_bytes[7] as u64);

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
        let idx: u16 = 0;

        while ((idx as u64) < TO_REMOVE_PARALLELISM) {
            if (to_remove.remove_tbl.contains(idx)) {
                let keys = to_remove.remove_tbl.remove(idx);
                let keys_len = keys.length();
                let key_idx = 0;

                while (key_idx < keys_len) {
                    let key = keys[key_idx];
                    if (queue.schedule_map.contains(&key)) {
                        // Remove transaction from schedule_map
                        queue.schedule_map.remove(&key);
                    };
                    key_idx = key_idx + 1;
                };
            };
            idx = idx + 1;
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
        timestamp: u64, limit: u64
    ): vector<ScheduledTransactionWithKey> acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        get_ready_transactions(timestamp, limit)
    }

    #[test_only]
    public fun create_transaction_id(hash: vector<u8>): TransactionId {
        let txn_id = TransactionId { hash };
        txn_id
    }

    #[test_only]
    public fun get_deposit_owner_signer(): signer acquires GasFeeDepositStoreSignerCap {
        let owner_cap = borrow_global<GasFeeDepositStoreSignerCap>(@aptos_framework);
        let owner_signer = account::create_signer_with_capability(&owner_cap.cap);
        owner_signer
    }

    #[test_only]
    public fun get_metadata_for_AptosCoin(): Object<Metadata> {
        coin::ensure_paired_metadata<AptosCoin>()
    }

    struct State has copy, drop, store {
        count: u64
    }

    #[persistent]
    fun step(state: State, _val: u64) {
        if (state.count < 10) {
            state.count = state.count + 1;
        }
    }

    #[persistent]
    fun self_rescheduling_func(
        storable_perm_handle: StorablePermissionedHandle,
        scheduled_time: u64,
        max_gas_unit_price: u64,
        max_gas_amount: u64,
        delta_time: u64,
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        // do work

        // reschedule
        //let new_scheduled_time = scheduled_time + delta_time;
        let storable_perm_handle_copy = storable_perm_handle;
        let next_txn = {
            let foo = || self_rescheduling_func(
                storable_perm_handle_copy,
                scheduled_time,  // Update scheduled time here
                max_gas_unit_price,
                max_gas_amount,
                delta_time
            );
            ScheduledTransaction {
                sender_handle: storable_perm_handle,
                scheduled_time: scheduled_time + delta_time,
                max_gas_amount,
                max_gas_unit_price,
                next_schedule_delta_time: 0,
                f: foo
            }
        };
        // Get signer from handle
        let signer = permissioned_signer::signer_from_storable_permissioned_handle(&storable_perm_handle_copy);

        // Schedule next execution
        insert(&signer, next_txn);
    }

    /*#[persistent]
    fun self_rescheduling_func(
        storable_perm_handle: StorablePermissionedHandle,
        scheduled_time: u64,
        max_gas_unit_price: u64,
        max_gas_amount: u64,
        delta_time: u64,
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        // Perform the work
        //do_work();

        // Prepare the next transaction
        let next_scheduled_time = scheduled_time + delta_time;
        let next_txn = ScheduledTransaction {
            sender_handle: storable_perm_handle,
            scheduled_time: next_scheduled_time,
            max_gas_amount,
            max_gas_unit_price,
            next_schedule_delta_time: delta_time,
            f: || defer_reschedule(
                storable_perm_handle,
                next_scheduled_time,
                max_gas_unit_price,
                max_gas_amount,
                delta_time
            )
        };

        // Get the signer from the handle
        let signer = permissioned_signer::signer_from_storable_permissioned_handle(&storable_perm_handle);

        // Insert the next transaction into the queue
        insert(&signer, next_txn);
    }

    fun defer_reschedule(
        storable_perm_handle: StorablePermissionedHandle,
        scheduled_time: u64,
        max_gas_unit_price: u64,
        max_gas_amount: u64,
        delta_time: u64,
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap, store {
        // Reschedule the transaction
        self_rescheduling_func(
            storable_perm_handle,
            scheduled_time,
            max_gas_unit_price,
            max_gas_amount,
            delta_time
        );
    }*/

    #[test_only]
    public fun get_txn_by_key(
        key: ScheduleMapKey
    ): ScheduledTransaction acquires ScheduleQueue {
        let queue = borrow_global<ScheduleQueue>(@aptos_framework);
        assert!(queue.schedule_map.contains(&key), 0);
        let txn = *queue.schedule_map.borrow(&key);
        txn
    }

    #[test_only]
    public fun execute_f(txn: &ScheduledTransaction) {
        (txn.f)();
    }

    #[test_only]
    public fun create_scheduled_txn_reschedule(
        storable_perm_handle: StorablePermissionedHandle,
        scheduled_time: u64,
        max_gas_unit_price: u64,
        max_gas_amount: u64,
        next_schedule_delta_time: u64,
    ): ScheduledTransaction acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        let state = State { count: 8 };
        let foo = || self_rescheduling_func(
            storable_perm_handle,
            scheduled_time,
            max_gas_unit_price,
            max_gas_amount,
            next_schedule_delta_time
        );

        ScheduledTransaction {
            sender_handle: storable_perm_handle,
            scheduled_time,
            max_gas_amount,
            max_gas_unit_price,
            next_schedule_delta_time,
            f: foo
        }
    }

    #[test_only]
    public fun create_scheduled_txn(
        storable_perm_handle: StorablePermissionedHandle,
        scheduled_time: u64,
        max_gas_unit_price: u64,
        max_gas_amount: u64,
        next_schedule_delta_time: u64,
    ): ScheduledTransaction {
        let state = State { count: 8 };
        let foo = || step(state, 5);

        ScheduledTransaction {
            sender_handle: storable_perm_handle,
            scheduled_time,
            max_gas_amount,
            max_gas_unit_price,
            next_schedule_delta_time,
            f: foo
        }
    }

    #[test_only]
    public fun shutdown_test(
        fx: &signer
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap, ToRemoveTbl {
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
        let coin = coin::mint<AptosCoin>(1000000, &mint);
        coin::deposit(user_addr, coin);

        coin::destroy_burn_cap(burn);
        coin::destroy_mint_cap(mint);
    }

    #[test_only]
    public fun setup_permissions(user: &signer): StorablePermissionedHandle {
        let storable_perm_handle =
            permissioned_signer::create_storable_permissioned_handle(user, 60);
        let perm_signer =
            permissioned_signer::signer_from_storable_permissioned_handle(
                &storable_perm_handle
            );
        let metadata = coin::ensure_paired_metadata<AptosCoin>();
        primary_fungible_store::grant_permission(user, &perm_signer, metadata, 1000000);
        storable_perm_handle
    }

    #[test_only]
    public fun mock_execute(key: ScheduleMapKey) acquires ScheduleQueue, ToRemoveTbl {
        let txn = {
            let queue = borrow_global<ScheduleQueue>(@aptos_framework);
            assert!(queue.schedule_map.contains(&key), 0);
            *queue.schedule_map.borrow(&key)
        };
        // Execute the transaction
        (txn.f)();
        // Finish execution
        finish_execution(key);
    }

    #[test(fx = @0x1, user = @0x1234)]
    fun test_basic(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap, ToRemoveTbl {
        let curr_mock_time_micro_s = 1000000;
        // Setup test environment
        setup_test_env(fx, &user, curr_mock_time_micro_s);

        // Create permissioned handle and set permissions
        let storable_perm_handle = setup_permissions(&user);

        // Create transactions with same scheduled_time but different gas prices
        let schedule_time1 = curr_mock_time_micro_s / 1000 + 1000;
        let schedule_time2 = schedule_time1 * 2;
        let schedule_time3 = schedule_time1 * 4;
        let reschedule_delta = schedule_time3 + 10000;
        let txn1 = create_scheduled_txn(
            storable_perm_handle,
            schedule_time1,
            20,
            100,
            0
        ); // time: 1s, gas: 20
        let txn2 = create_scheduled_txn(
            storable_perm_handle,
            schedule_time1,
            30,
            100,
            0
        ); // time: 1s, gas: 30
        let txn3 = create_scheduled_txn(
            storable_perm_handle,
            schedule_time1,
            10,
            100,
            0
        ); // time: 1s, gas: 10

        // Create transactions with same scheduled_time and gas price
        let txn4 = create_scheduled_txn(
            storable_perm_handle,
            schedule_time2,
            20,
            1000,
            0
        ); // time: 2s, gas: 20
        /*let txn5 =
            create_scheduled_txn(
                storable_perm_handle,
                schedule_time2,
                20,
                100,
                reschedule_delta
            );*/ // time: 2s, gas: 20
        let txn5 = create_scheduled_txn_reschedule(
            storable_perm_handle,
            schedule_time2,
            20,
            100,
            reschedule_delta
        );
        let txn6 = create_scheduled_txn(
            storable_perm_handle,
            schedule_time2,
            20,
            200,
            0
        ); // time: 2s, gas: 20

        let txn7 = create_scheduled_txn(
            storable_perm_handle,
            schedule_time3,
            20,
            100,
            0
        ); // time: 2s, gas: 20
        let txn8 = create_scheduled_txn(
            storable_perm_handle,
            schedule_time3,
            20,
            200,
            0
        ); // time: 2s, gas: 20

        // Insert all transactions
        let txn1_key = insert(&user, txn1);
        let txn2_key = insert(&user, txn2);
        let txn3_key = insert(&user, txn3);
        let txn4_key = insert(&user, txn4);
        let txn5_key = insert(&user, txn5);
        let txn6_key = insert(&user, txn6);
        let txn7_key = insert(&user, txn7);
        let txn8_key = insert(&user, txn8);

        assert!(get_num_txns() == 8, get_num_txns());

        // Test get_ready_transactions at t < schedule_time1 (should return empty)
        let ready_txns = get_ready_transactions(schedule_time1 - 1000, 10);
        assert!(ready_txns.length() == 0, ready_txns.length());

        // Test get_ready_transactions at t > schedule_time1 (should return first 3 txns)
        let ready_txns = get_ready_transactions(schedule_time1 + 1000, 10);
        assert!(ready_txns.length() == 3, ready_txns.length());

        // Test the limit param of get_ready_transactions
        ready_txns = get_ready_transactions(schedule_time1 + 1000, 2);
        assert!(ready_txns.length() == 2, ready_txns.length());
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
        let ready_txns = get_ready_transactions(schedule_time2 + 1000, 10);
        assert!(ready_txns.length() == 3, ready_txns.length());

        // Execute and remove 2 transactions
        finish_execution(txn4_key);
        finish_execution(txn6_key);
        remove_txns(); // Should remove 2 txns
        assert!(get_num_txns() == 3, get_num_txns());

        let ready_txns = get_ready_transactions(schedule_time2 + 1000, 10);
        assert!(ready_txns.length() == 1, ready_txns.length());

        // Execute and remove last 3 transactions
        mock_execute(txn5_key); // we expect txn5 to reschedule itself
        finish_execution(txn7_key);
        finish_execution(txn8_key);

        remove_txns(); // Should remove all but txn which has to be rescheduled
        assert!(get_num_txns() == 1, get_num_txns());
        assert!(
            get_ready_transactions(schedule_time2, 10).length() == 0,
            get_ready_transactions(schedule_time2, 10).length()
        );
        assert!(
            get_ready_transactions(schedule_time2 + reschedule_delta, 10).length() == 1,
            get_ready_transactions(schedule_time2 + reschedule_delta, 10).length()
        );

        // try expiring a txn by getting it late
        let expired_time =
            schedule_time2 + reschedule_delta + EXPIRY_DELTA * MILLI_CONVERSION_FACTOR
                + 1000;
        assert!(
            get_ready_transactions(expired_time, 10).length() == 0,
            get_ready_transactions(expired_time, 10).length()
        );
        assert!(get_num_txns() == 0, get_num_txns());
    }

    #[test(fx = @0x1, user = @0x1234)]
    #[expected_failure(abort_code = 65538)]
    // error::invalid_argument(EINVALID_TIME)
    fun test_insert_past_time(
        fx: &signer, user: signer
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        let curr_mock_time_micro_s = 1000000;
        setup_test_env(fx, &user, curr_mock_time_micro_s);
        let storable_perm_handle = setup_permissions(&user);

        // Try to schedule transaction in the past
        let past_time = curr_mock_time_micro_s / 1000 - 100;
        let txn = create_scheduled_txn(storable_perm_handle, past_time, 20, 100, 0);

        // Create a signer from the handle to use for insert
        insert(&user, txn); // Should fail with EINVALID_TIME since time is in the past
    }

    #[test(fx = @0x1, user = @0x1234, other_user = @0x5678)]
    #[expected_failure(abort_code = 327681)]
    // error::permission_denied(EINVALID_SIGNER)
    fun test_insert_wrong_user(
        fx: &signer, user: signer, other_user: signer
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        let curr_mock_time = 1000000;
        setup_test_env(fx, &user, curr_mock_time);
        let storable_perm_handle = setup_permissions(&user);

        // Try to schedule transaction with wrong user
        let future_time = curr_mock_time + 1000;
        let txn = create_scheduled_txn(storable_perm_handle, future_time, 20, 100, 0);
        insert(&other_user, txn); // Should fail with EINVALID_SIGNER
    }

    #[test(fx = @0x1, user = @0x1234, other_user = @0x5678)]
    #[expected_failure(abort_code = 327681)]
    // error::permission_denied(EINVALID_SIGNER)
    fun test_cancel_wrong_user(
        fx: &signer, user: signer, other_user: signer
    ) acquires ScheduleQueue, GasFeeDepositStoreSignerCap {
        let curr_mock_time = 1000000;
        setup_test_env(fx, &user, curr_mock_time);
        let storable_perm_handle = setup_permissions(&user);

        // Schedule a valid transaction
        let future_time = curr_mock_time + 1000;
        let txn = create_scheduled_txn(storable_perm_handle, future_time, 20, 100, 0);
        let txn_id = insert(&user, txn);

        // Try to cancel the transaction with wrong user
        cancel(&other_user, txn_id); // Should fail with EINVALID_SIGNER
    }
}
