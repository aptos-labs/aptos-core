module aptos_framework::lockstream {

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::math64;
    use aptos_std::big_vector::{Self, BigVector};
    use aptos_std::table::{Self, Table};
    use aptos_std::type_info::{Self, TypeInfo};
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;

    /// All times in UNIX seconds.
    struct LockstreamPool<
        phantom BaseType,
        phantom QuoteType,
    > has key {
        base_locked: Coin<BaseType>,
        quote_locked: Coin<QuoteType>,
        locker_addresses: BigVector<address>,
        lockers: Table<address, LockerInfo>,
        initial_base_locked: u64,
        initial_quote_locked: u64,
        premier_locker: address,
        premier_locker_initial_quote_locked: u64,
        creation_time: u64,
        stream_start_time: u64,
        stream_end_time: u64,
        claim_last_call_time: u64,
        premier_sweep_last_call_time: u64,
        creation_event_handle: EventHandle<LockstreamCreationEvent>,
        lock_event_handle: EventHandle<LockstreamLockEvent>,
        new_premier_locker_event_handle:
            EventHandle<LockstreamNewPremierLockerEvent>,
        claim_event_handle: EventHandle<LockstreamClaimEvent>,
        sweep_event_handle: EventHandle<LockstreamSweepEvent>,
    }

    struct LockerInfo has copy, drop, store {
        initial_quote_locked: u64,
        base_claimed: u64,
        quote_claimed: u64,
    }

    struct LockstreamPoolID has copy, drop, store {
        creator: address,
        base_type: TypeInfo,
        quote_type: TypeInfo,
    }

    struct LockstreamCreationEvent has copy, drop, store {
        pool_id: LockstreamPoolID,
        initial_base_locked: u64,
        creation_time: u64,
        stream_start_time: u64,
        stream_end_time: u64,
        claim_last_call_time: u64,
        premier_sweep_last_call_time: u64,
    }

    struct LockstreamLockEvent has copy, drop, store {
        pool_id: LockstreamPoolID,
        lock_time: u64,
        locker: address,
        quote_lock_amount: u64,
        total_quote_locked_for_locker: u64,
        total_quote_locked_for_pool: u64,
    }

    struct LockstreamNewPremierLockerEvent has copy, drop, store {
        pool_id: LockstreamPoolID,
        lock_time: u64,
        new_premier_locker: address,
        old_premier_locker: address,
        new_premier_locker_total_quote_locked: u64,
        old_premier_locker_total_quote_locked: u64,
        total_quote_locked_for_pool: u64,
    }

    struct LockstreamClaimEvent has copy, drop, store {
        pool_id: LockstreamPoolID,
        claim_time: u64,
        locker: address,
        base_claimed: u64,
        quote_claimed: u64,
        total_base_claimed_for_locker: u64,
        total_quote_claimed_for_locker: u64,
    }

    struct LockstreamSweepEvent has copy, drop, store {
        pool_id: LockstreamPoolID,
        sweep_time: u64,
        locker: address,
        base_sweep_amount: u64,
        quote_sweep_amount: u64,
    }

    struct LockstreamLockerEventHandles has key {
        lock_event_handle: EventHandle<LockstreamLockEvent>,
        new_premier_locker_event_handle:
            EventHandle<LockstreamNewPremierLockerEvent>,
        claim_event_handle: EventHandle<LockstreamClaimEvent>,
        sweep_event_handle: EventHandle<LockstreamSweepEvent>,
    }

    struct LockstreamPoolMetadataView has copy, drop, store {
        pool_id: LockstreamPoolID,
        base_locked: u64,
        quote_locked: u64,
        n_lockers: u64,
        initial_base_locked: u64,
        initial_quote_locked: u64,
        premier_locker: address,
        premier_locker_initial_quote_locked: u64,
        creation_time: u64,
        stream_start_time: u64,
        stream_end_time: u64,
        claim_last_call_time: u64,
        premier_sweep_last_call_time: u64,
        current_period: u8,
    }

    struct LockerInfoView has copy, drop, store {
        pool_id: LockstreamPoolID,
        locker: address,
        pro_rata_base_share: u64,
        initial_quote_locked: u64,
        base_claimed: u64,
        quote_claimed: u64,
        claimable_base: u64,
        claimable_quote: u64,
    }

    /// Minimum number of bytes required to encode the number of
    /// elements in a vector (for a vector with less than 128 elements).
    const MIN_BYTES_BCS_SEQUENCE_LENGTH: u64 = 1;
    /// Free number of bytes for a global storage write.
    const FREE_WRITE_BYTES_QUOTA: u64 = 1024;

    const PERIOD_LOCKING: u8 = 1;
    const PERIOD_STREAMING: u8 = 2;
    const PERIOD_CLAIMING_GRACE_PERIOD: u8 = 3;
    const PERIOD_PREMIER_SWEEP: u8 = 4;
    const PERIOD_MERCENARY_SWEEP: u8 = 5;

    /// Time window bounds provided by creator are invalid.
    const E_TIME_WINDOWS_INVALID: u64 = 0;
    /// Quote type provided by creator is not a coin type.
    const E_QUOTE_NOT_COIN: u64 = 1;
    /// No lockstream pool for base type, quote type, and creator.
    const E_NO_LOCKSTREAM_POOL: u64 = 2;
    /// Lockstream pool for base tye, quote type, and creator exists.
    const E_LOCKSTREAM_POOL_EXISTS: u64 = 3;
    /// Too late to lock more quote into lockstream pool.
    const E_TOO_LATE_TO_LOCK: u64 = 4;
    /// No quote lock amount specified.
    const E_NO_QUOTE_LOCK_AMOUNT: u64 = 5;
    /// Signer is not a locker in the lockstream.
    const E_NOT_A_LOCKER: u64 = 6;
    /// Too early to claim from lockstream.
    const E_TOO_EARLY_TO_CLAIM: u64 = 7;
    /// Too late to claim from lockstream.
    const E_TOO_LATE_TO_CLAIM: u64 = 8;
    /// Too early for premier locker to sweep lockstream pool.
    const E_TOO_EARLY_FOR_PREMIER_SWEEP: u64 = 9;
    /// Too late for premier locker to sweep lockstream pool.
    const E_TOO_LATE_FOR_PREMIER_SWEEP: u64 = 10;
    /// Too early for mercenary locker to sweep lockstream pool.
    const E_TOO_EARLY_FOR_MERCENARY_SWEEP: u64 = 11;
    /// No coins in lockstream pool left to sweep.
    const E_NOTHING_TO_SWEEP: u64 = 12;

    /// All times in UNIX seconds.
    public entry fun create<
        BaseType,
        QuoteType
    >(
        creator: &signer,
        initial_base_locked: u64,
        stream_start_time: u64,
        stream_end_time: u64,
        claim_last_call_time: u64,
        premier_sweep_last_call_time: u64,
    ) {
        let creator_addr = signer::address_of(creator);
        assert!(
            !exists<LockstreamPool<BaseType, QuoteType>>(creator_addr),
            E_LOCKSTREAM_POOL_EXISTS
        );
        let creation_time = timestamp::now_seconds();
        assert!(
            creation_time        < stream_start_time &&
            stream_start_time    < stream_end_time &&
            stream_end_time      < claim_last_call_time &&
            claim_last_call_time < premier_sweep_last_call_time,
            E_TIME_WINDOWS_INVALID
        );
        assert!(coin::is_coin_initialized<QuoteType>(), E_QUOTE_NOT_COIN);
        let creation_event_handle = account::new_event_handle(creator);
        event::emit_event(&mut creation_event_handle, LockstreamCreationEvent {
            pool_id: LockstreamPoolID {
                creator: creator_addr,
                base_type: type_info::type_of<BaseType>(),
                quote_type: type_info::type_of<QuoteType>(),
            },
            initial_base_locked,
            creation_time,
            stream_start_time,
            stream_end_time,
            claim_last_call_time,
            premier_sweep_last_call_time,
        });
        let big_vector_bucket_size =
            (FREE_WRITE_BYTES_QUOTA - MIN_BYTES_BCS_SEQUENCE_LENGTH) /
            type_info::size_of_val(&@0x0);
        move_to(creator, LockstreamPool<BaseType, QuoteType> {
            base_locked: coin::withdraw(creator, initial_base_locked),
            quote_locked: coin::zero(),
            locker_addresses: big_vector::empty(big_vector_bucket_size),
            lockers: table::new(),
            initial_base_locked,
            initial_quote_locked: 0,
            premier_locker: @0x0,
            premier_locker_initial_quote_locked: 0,
            creation_time,
            stream_start_time,
            stream_end_time,
            claim_last_call_time,
            premier_sweep_last_call_time,
            creation_event_handle,
            lock_event_handle: account::new_event_handle(creator),
            new_premier_locker_event_handle:
                account::new_event_handle(creator),
            claim_event_handle: account::new_event_handle(creator),
            sweep_event_handle: account::new_event_handle(creator),
        });
    }

    public entry fun lock<
        BaseType,
        QuoteType
    >(
        locker: &signer,
        creator: address,
        quote_lock_amount: u64,
    ) acquires
        LockstreamLockerEventHandles,
        LockstreamPool
    {
        let (pool_id, pool_ref_mut) =
            pool_id_and_mutable_reference<BaseType, QuoteType>(creator);
        assert!(quote_lock_amount > 0, E_NO_QUOTE_LOCK_AMOUNT);
        let lock_time = timestamp::now_seconds();
        let period = period(pool_ref_mut, lock_time);
        assert!(period == PERIOD_LOCKING, E_TOO_LATE_TO_LOCK);
        coin::merge(
            &mut pool_ref_mut.quote_locked,
            coin::withdraw(locker, quote_lock_amount)
        );
        let total_quote_locked_for_pool =
            coin::value(&pool_ref_mut.quote_locked);
        let lockers_ref_mut = &mut pool_ref_mut.lockers;
        let locker_addr = signer::address_of(locker);
        let locking_more = table::contains(lockers_ref_mut, locker_addr);
        let total_quote_locked_for_locker = if (locking_more) {
            let locker_info_ref_mut =
                table::borrow_mut(lockers_ref_mut, locker_addr);
            let already_locked = locker_info_ref_mut.initial_quote_locked;
            let total_locked = already_locked + quote_lock_amount;
            locker_info_ref_mut.initial_quote_locked = total_locked;
            total_locked
        } else {
            table::add(lockers_ref_mut, locker_addr, LockerInfo {
                initial_quote_locked: quote_lock_amount,
                base_claimed: 0,
                quote_claimed: 0,
            });
            big_vector::push_back(
                &mut pool_ref_mut.locker_addresses,
                locker_addr
            );
            quote_lock_amount
        };
        let lock_event = LockstreamLockEvent {
            pool_id,
            lock_time,
            locker: locker_addr,
            quote_lock_amount,
            total_quote_locked_for_locker,
            total_quote_locked_for_pool
        };
        event::emit_event(&mut pool_ref_mut.lock_event_handle, lock_event);
        if (!exists<LockstreamLockerEventHandles>(locker_addr))
            move_to(locker, LockstreamLockerEventHandles {
                lock_event_handle: account::new_event_handle(locker),
                new_premier_locker_event_handle:
                    account::new_event_handle(locker),
                claim_event_handle: account::new_event_handle(locker),
                sweep_event_handle: account::new_event_handle(locker),
            });
        let locker_handles_ref_mut =
            borrow_global_mut<LockstreamLockerEventHandles>(locker_addr);
        event::emit_event(
            &mut locker_handles_ref_mut.lock_event_handle,
            lock_event
        );
        let new_premier_locker =
            total_quote_locked_for_locker >
            pool_ref_mut.premier_locker_initial_quote_locked;
        if (new_premier_locker) {
            let premier_locker_event = LockstreamNewPremierLockerEvent {
                pool_id,
                lock_time,
                new_premier_locker: locker_addr,
                old_premier_locker: pool_ref_mut.premier_locker,
                new_premier_locker_total_quote_locked:
                    total_quote_locked_for_locker,
                old_premier_locker_total_quote_locked:
                    pool_ref_mut.premier_locker_initial_quote_locked,
                total_quote_locked_for_pool,
            };
            event::emit_event(
                &mut pool_ref_mut.new_premier_locker_event_handle,
                premier_locker_event
            );
            event::emit_event(
                &mut locker_handles_ref_mut.new_premier_locker_event_handle,
                premier_locker_event
            );
            pool_ref_mut.premier_locker = locker_addr;
            pool_ref_mut.premier_locker_initial_quote_locked =
                total_quote_locked_for_locker;
        }
    }

    public entry fun claim<
        BaseType,
        QuoteType
    >(
        locker: &signer,
        creator: address,
    ) acquires
        LockstreamLockerEventHandles,
        LockstreamPool
    {
        let (pool_id, pool_ref_mut) =
            pool_id_and_mutable_reference<BaseType, QuoteType>(creator);
        let claim_time = timestamp::now_seconds();
        let period = period(pool_ref_mut, claim_time);
        assert!(!(period < PERIOD_STREAMING), E_TOO_EARLY_TO_CLAIM);
        assert!(!(period > PERIOD_CLAIMING_GRACE_PERIOD), E_TOO_LATE_TO_CLAIM);
        let locker_addr = signer::address_of(locker);
        let (_, base_claimed, quote_claimed) = locker_amounts_derived(
            pool_ref_mut,
            locker_addr,
            claim_time
        );
        let locker_info_ref_mut =
            table::borrow_mut(&mut pool_ref_mut.lockers, locker_addr);
        if (base_claimed > 0) {
            coin::register<BaseType>(locker);
            coin::deposit(
                locker_addr,
                coin::extract(&mut pool_ref_mut.base_locked, base_claimed)
            );
        };
        if (quote_claimed > 0) {
            coin::deposit(
                locker_addr,
                coin::extract(&mut pool_ref_mut.quote_locked, quote_claimed)
            );
        };
        if (base_claimed > 0 || quote_claimed > 0) {
            let total_base_claimed =
                base_claimed + locker_info_ref_mut.base_claimed;
            let total_quote_claimed =
                quote_claimed + locker_info_ref_mut.quote_claimed;
            let claim_event = LockstreamClaimEvent {
                pool_id,
                claim_time,
                locker: locker_addr,
                base_claimed,
                quote_claimed,
                total_base_claimed_for_locker: total_base_claimed,
                total_quote_claimed_for_locker: total_quote_claimed,
            };
            locker_info_ref_mut.base_claimed = total_base_claimed;
            locker_info_ref_mut.quote_claimed = total_quote_claimed;
            event::emit_event(
                &mut pool_ref_mut.claim_event_handle,
                claim_event
            );
            let locker_handles_ref_mut =
                borrow_global_mut<LockstreamLockerEventHandles>(locker_addr);
            event::emit_event(
                &mut locker_handles_ref_mut.claim_event_handle,
                claim_event
            );
        }
    }

    public entry fun sweep<
        BaseType,
        QuoteType
    >(
        locker: &signer,
        creator: address,
    ) acquires
        LockstreamLockerEventHandles,
        LockstreamPool
    {
        let (pool_id, pool_ref_mut) =
            pool_id_and_mutable_reference<BaseType, QuoteType>(creator);
        let lockers_ref_mut = &mut pool_ref_mut.lockers;
        let locker_addr = signer::address_of(locker);
        assert!(
            table::contains(lockers_ref_mut, locker_addr),
            E_NOT_A_LOCKER
        );
        let sweep_time = timestamp::now_seconds();
        let period = period(pool_ref_mut, sweep_time);
        if (locker_addr == pool_ref_mut.premier_locker) {
            assert!(
                !(period < PERIOD_PREMIER_SWEEP),
                E_TOO_EARLY_FOR_PREMIER_SWEEP
            );
            assert!(
                !(period > PERIOD_MERCENARY_SWEEP),
                E_TOO_LATE_FOR_PREMIER_SWEEP
            );
        } else {
            assert!(
                period == PERIOD_MERCENARY_SWEEP,
                E_TOO_EARLY_FOR_MERCENARY_SWEEP
            );
        };
        let base_to_sweep = coin::value(&pool_ref_mut.base_locked);
        let quote_to_sweep = coin::value(&pool_ref_mut.quote_locked);
        assert!(
            base_to_sweep > 0 || quote_to_sweep > 0,
            E_NOTHING_TO_SWEEP
        );
        if (base_to_sweep > 0) {
            coin::register<BaseType>(locker);
            coin::deposit(
                locker_addr,
                coin::extract_all(&mut pool_ref_mut.base_locked)
            );
        };
        if (quote_to_sweep > 0) {
            coin::deposit(
                locker_addr,
                coin::extract_all(&mut pool_ref_mut.quote_locked)
            );
        };
        let sweep_event = LockstreamSweepEvent {
            pool_id,
            sweep_time,
            locker: locker_addr,
            base_sweep_amount: base_to_sweep,
            quote_sweep_amount: quote_to_sweep,
        };
        event::emit_event(&mut pool_ref_mut.sweep_event_handle, sweep_event);
        let locker_handles_ref_mut =
            borrow_global_mut<LockstreamLockerEventHandles>(locker_addr);
        event::emit_event(
            &mut locker_handles_ref_mut.sweep_event_handle,
            sweep_event
        );
    }

    #[view]
    public fun current_period<
        BaseType,
        QuoteType
    >(creator: address):
    Option<u8>
    acquires LockstreamPool {
        if (exists<LockstreamPool<BaseType, QuoteType>>(creator)) {
            let pool_ref = borrow_global<
                LockstreamPool<BaseType, QuoteType>>(creator);
            option::some(period(pool_ref, timestamp::now_seconds()))
        } else option::none()
    }

    #[view]
    public fun locker<
        BaseType,
        QuoteType
    >(
        creator: address,
        locker: address,
    ):
    Option<LockerInfoView>
    acquires LockstreamPool {
        if (exists<LockstreamPool<BaseType, QuoteType>>(creator)) {
            let (pool_id, pool_ref) =
                pool_id_and_immutable_reference<BaseType, QuoteType>(creator);
            if (!table::contains(&pool_ref.lockers, locker))
                return option::none();
            let time_seconds = timestamp::now_seconds();
            let (pro_rata_base_share, claimable_base, claimable_quote) =
                locker_amounts_derived(pool_ref, locker, time_seconds);
            let locker_info_ref = table::borrow(&pool_ref.lockers, locker);
            option::some(LockerInfoView {
                pool_id,
                locker,
                pro_rata_base_share,
                initial_quote_locked: locker_info_ref.initial_quote_locked,
                base_claimed: locker_info_ref.base_claimed,
                quote_claimed: locker_info_ref.quote_claimed,
                claimable_base,
                claimable_quote,
            })
        } else option::none()
    }

    #[view]
    public fun lockers<
        BaseType,
        QuoteType
    >(creator: address):
    Option<vector<LockerInfoView>>
    acquires LockstreamPool {
        if (exists<LockstreamPool<BaseType, QuoteType>>(creator)) {
            let pool_ref = borrow_global<
                LockstreamPool<BaseType, QuoteType>>(creator);
            let lockers = big_vector::to_vector(&pool_ref.locker_addresses);
            option::some(vector::map(lockers, |e| {
                option::destroy_some(locker<BaseType, QuoteType>(creator, e))
            }))
        } else option::none()
    }

    #[view]
    public fun lockers_paginated<
        BaseType,
        QuoteType
    >(
        creator: address,
        start_index: u64,
        end_index: u64,
    ): Option<vector<LockerInfoView>>
    acquires LockstreamPool {
        if (exists<LockstreamPool<BaseType, QuoteType>>(creator)) {
            let pool_ref = borrow_global<
                LockstreamPool<BaseType, QuoteType>>(creator);
            let n_lockers = big_vector::length(&pool_ref.locker_addresses);
            if ((end_index < start_index) ||
                (start_index >= n_lockers) ||
                (end_index >= n_lockers)) return option::none();
            let i = start_index;
            let lockers = vector[];
            while (i <= end_index) {
                let pool_ref = borrow_global<
                    LockstreamPool<BaseType, QuoteType>>(creator);
                let locker_address =
                    *big_vector::borrow(&pool_ref.locker_addresses, i);
                let locker = option::destroy_some(
                    locker<BaseType, QuoteType>(creator, locker_address)
                );
                vector::push_back(&mut lockers, locker);
                i = i + 1;
            };
            option::some(lockers)
        } else option::none()
    }

    #[view]
    public fun metadata<
        BaseType,
        QuoteType
    >(creator: address):
    Option<LockstreamPoolMetadataView>
    acquires LockstreamPool {
        if (exists<LockstreamPool<BaseType, QuoteType>>(creator)) {
            let (pool_id, pool_ref) =
                pool_id_and_immutable_reference<BaseType, QuoteType>(creator);
            option::some(LockstreamPoolMetadataView{
                pool_id,
                base_locked: coin::value(&pool_ref.base_locked),
                quote_locked: coin::value(&pool_ref.quote_locked),
                n_lockers: big_vector::length(&pool_ref.locker_addresses),
                initial_base_locked: pool_ref.initial_base_locked,
                initial_quote_locked: pool_ref.initial_quote_locked,
                premier_locker: pool_ref.premier_locker,
                premier_locker_initial_quote_locked:
                    pool_ref.premier_locker_initial_quote_locked,
                creation_time: pool_ref.creation_time,
                stream_start_time: pool_ref.stream_start_time,
                stream_end_time: pool_ref.stream_end_time,
                claim_last_call_time: pool_ref.stream_end_time,
                premier_sweep_last_call_time:
                    pool_ref.premier_sweep_last_call_time,
                current_period: period(pool_ref, timestamp::now_seconds()),
            })
        } else option::none()
    }

    #[view]
    public fun pool_id<
        BaseType,
        QuoteType
    >(creator: address):
    Option<LockstreamPoolID> {
        if (exists<LockstreamPool<BaseType, QuoteType>>(creator)) {
            option::some(LockstreamPoolID {
                creator,
                base_type: type_info::type_of<BaseType>(),
                quote_type: type_info::type_of<QuoteType>(),
            })
        } else option::none()
    }

    fun locker_amounts_derived<
        BaseType,
        QuoteType
    >(
        pool_ref: &LockstreamPool<BaseType, QuoteType>,
        locker: address,
        time_seconds: u64,
    ): (
       u64,
       u64,
       u64,
    ) {
        let period = period(pool_ref, time_seconds);
        let lockers_ref = &pool_ref.lockers;
        assert!(table::contains(lockers_ref, locker), E_NOT_A_LOCKER);
        let locker_info_ref = table::borrow(lockers_ref, locker);
        let initial_quote_locked = locker_info_ref.initial_quote_locked;
        let pro_rata_base_share = math64::mul_div(
            pool_ref.initial_base_locked,
            initial_quote_locked,
            pool_ref.initial_quote_locked,
        );
        let claimable_period =
            period == PERIOD_STREAMING ||
            period == PERIOD_CLAIMING_GRACE_PERIOD;
        let (claimable_base, claimable_quote) = if (claimable_period) {
            let (claimable_base_ceiling, claimable_quote_ceiling) =
                if (period == PERIOD_CLAIMING_GRACE_PERIOD)
                    (pro_rata_base_share, initial_quote_locked) else
            {
                let stream_start = pool_ref.stream_start_time;
                let elapsed = time_seconds - stream_start;
                let duration = pool_ref.stream_end_time - stream_start;
                (
                    math64::mul_div(pro_rata_base_share, elapsed, duration),
                    math64::mul_div(initial_quote_locked, elapsed, duration),
                )
            };
            (
                claimable_base_ceiling - locker_info_ref.base_claimed,
                claimable_quote_ceiling - locker_info_ref.quote_claimed,
            )
        } else {
            (0, 0)
        };
        (
            pro_rata_base_share,
            claimable_base,
            claimable_quote,
        )
    }

    inline fun period<
        BaseType,
        QuoteType
    >(
        pool_ref: &LockstreamPool<BaseType, QuoteType>,
        time_seconds: u64
    ): u8 {
        if (time_seconds < pool_ref.stream_start_time) PERIOD_LOCKING else
        if (time_seconds <= pool_ref.stream_end_time) PERIOD_STREAMING else
        if (time_seconds <= pool_ref.claim_last_call_time)
            PERIOD_CLAIMING_GRACE_PERIOD else
        if (time_seconds <= pool_ref.premier_sweep_last_call_time)
            PERIOD_PREMIER_SWEEP else
        PERIOD_MERCENARY_SWEEP
    }

    inline fun pool_id_and_immutable_reference<
        BaseType,
        QuoteType
    >(creator: address): (
        LockstreamPoolID,
        &LockstreamPool<BaseType, QuoteType>
    ) acquires LockstreamPool {
        let pool_id_option = pool_id<BaseType, QuoteType>(creator);
        assert!(option::is_some(&pool_id_option), E_NO_LOCKSTREAM_POOL);
        (
            option::destroy_some(pool_id_option),
            borrow_global<LockstreamPool<BaseType, QuoteType>>(creator),
        )
    }

    inline fun pool_id_and_mutable_reference<
        BaseType,
        QuoteType
    >(creator: address): (
        LockstreamPoolID,
        &mut LockstreamPool<BaseType, QuoteType>
    ) acquires LockstreamPool {
        let pool_id_option = pool_id<BaseType, QuoteType>(creator);
        assert!(option::is_some(&pool_id_option), E_NO_LOCKSTREAM_POOL);
        (
            option::destroy_some(pool_id_option),
            borrow_global_mut<LockstreamPool<BaseType, QuoteType>>(creator),
        )
    }

}