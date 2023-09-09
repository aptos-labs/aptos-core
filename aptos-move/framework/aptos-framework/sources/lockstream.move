module aptos_framework::lockstream {

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::math64;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::type_info::{Self, TypeInfo};
    use std::signer;

    /// All times in UNIX seconds.
    struct LockstreamPool<
        phantom BaseType,
        phantom QuoteType,
    > has key {
        base_locked: Coin<BaseType>,
        quote_locked: Coin<QuoteType>,
        lockers: SmartTable<address, LockerInfo>,
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
        move_to(creator, LockstreamPool<BaseType, QuoteType> {
            base_locked: coin::withdraw(creator, initial_base_locked),
            quote_locked: coin::zero(),
            lockers: smart_table::new(),
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
        assert!(quote_lock_amount > 0, E_NO_QUOTE_LOCK_AMOUNT);
        let pool_id = pool_id<BaseType, QuoteType>(creator);
        let pool_ref_mut =
            borrow_global_mut<LockstreamPool<BaseType, QuoteType>>(creator);
        let lock_time = timestamp::now_seconds();
        assert!(
            lock_time < pool_ref_mut.stream_start_time,
            E_TOO_LATE_TO_LOCK
        );
        coin::merge(
            &mut pool_ref_mut.quote_locked,
            coin::withdraw(locker, quote_lock_amount)
        );
        let total_quote_locked_for_pool =
            coin::value(&pool_ref_mut.quote_locked);
        let lockers_ref_mut = &mut pool_ref_mut.lockers;
        let locker_addr = signer::address_of(locker);
        let locking_more = smart_table::contains(lockers_ref_mut, locker_addr);
        let total_quote_locked_for_locker = if (locking_more) {
            let locker_info_ref_mut =
                smart_table::borrow_mut(lockers_ref_mut, locker_addr);
            let already_locked = locker_info_ref_mut.initial_quote_locked;
            let total_locked = already_locked + quote_lock_amount;
            locker_info_ref_mut.initial_quote_locked = total_locked;
            total_locked
        } else {
            smart_table::add(lockers_ref_mut, locker_addr, LockerInfo {
                initial_quote_locked: quote_lock_amount,
                base_claimed: 0,
                quote_claimed: 0,
            });
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
        let pool_id = pool_id<BaseType, QuoteType>(creator);
        let pool_ref_mut =
            borrow_global_mut<LockstreamPool<BaseType, QuoteType>>(creator);
        let lockers_ref_mut = &mut pool_ref_mut.lockers;
        let locker_addr = signer::address_of(locker);
        assert!(
            smart_table::contains(lockers_ref_mut, locker_addr),
            E_NOT_A_LOCKER
        );
        let claim_time = timestamp::now_seconds();
        assert!(
            claim_time > pool_ref_mut.stream_start_time,
            E_TOO_EARLY_TO_CLAIM
        );
        assert!(
            claim_time <= pool_ref_mut.claim_last_call_time,
            E_TOO_LATE_TO_CLAIM
        );
        let locker_info_ref_mut =
            smart_table::borrow_mut(lockers_ref_mut, locker_addr);
        let locker_initial_quote_locked =
            locker_info_ref_mut.initial_quote_locked;
        let pro_rata_base = math64::mul_div(
            pool_ref_mut.initial_base_locked,
            locker_initial_quote_locked,
            pool_ref_mut.initial_quote_locked
        );
        let stream_done = claim_time > pool_ref_mut.stream_end_time;
        let (base_claimed_ceiling, quote_claimed_ceiling) = if (stream_done) {
            (pro_rata_base, locker_initial_quote_locked)
        } else {
            let stream_start = pool_ref_mut.stream_start_time;
            let elapsed = claim_time - stream_start;
            let duration = pool_ref_mut.stream_end_time - stream_start;
            (
                math64::mul_div(pro_rata_base, elapsed, duration),
                math64::mul_div(locker_initial_quote_locked, elapsed, duration)
            )
        };
        let base_claimed =
            base_claimed_ceiling - locker_info_ref_mut.base_claimed;
        let quote_claimed =
            quote_claimed_ceiling - locker_info_ref_mut.quote_claimed;
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
        let pool_id = pool_id<BaseType, QuoteType>(creator);
        let pool_ref_mut =
            borrow_global_mut<LockstreamPool<BaseType, QuoteType>>(creator);
        let lockers_ref_mut = &mut pool_ref_mut.lockers;
        let locker_addr = signer::address_of(locker);
        assert!(
            smart_table::contains(lockers_ref_mut, locker_addr),
            E_NOT_A_LOCKER
        );
        let sweep_time = timestamp::now_seconds();
        if (locker_addr == pool_ref_mut.premier_locker) {
            assert!(
                sweep_time > pool_ref_mut.claim_last_call_time,
                E_TOO_EARLY_FOR_PREMIER_SWEEP
            );
            assert!(
                sweep_time <= pool_ref_mut.premier_sweep_last_call_time, E_TOO_LATE_FOR_PREMIER_SWEEP
            );
        } else {
            assert!(
                sweep_time > pool_ref_mut.premier_sweep_last_call_time,
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
    public fun pool_id<
        BaseType,
        QuoteType
    >(creator: address):
    LockstreamPoolID {
        assert!(
            exists<LockstreamPool<BaseType, QuoteType>>(creator),
            E_NO_LOCKSTREAM_POOL
        );
        LockstreamPoolID {
            creator,
            base_type: type_info::type_of<BaseType>(),
            quote_type: type_info::type_of<QuoteType>(),
        }
    }

}