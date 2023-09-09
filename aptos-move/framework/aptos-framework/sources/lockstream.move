module aptos_framework::lockstream {

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::timestamp;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::type_info::{Self, TypeInfo};

    use std::signer;

    struct LockstreamPool<
        phantom BaseType,
        phantom QuoteType,
    > has key {
        base_coins: Coin<BaseType>,
        quote_coins: Coin<QuoteType>,
        participants: SmartTable<address, ParticipantInfo>,
        initial_base_locked: u64,
        initial_quote_locked: u64,
        premier_participant: address,
        premier_participant_initial_quote_locked: u64,
        pool_seed_time_seconds: u64,
        stream_start_time_seconds: u64,
        stream_end_time_seconds: u64,
        claim_window_end_time_seconds: u64,
        premier_sweep_window_end_time_seconds: u64,
        seed_event_handle: EventHandle<LockstreamSeedEvent>,
        lock_event_handle: EventHandle<LockstreamLockEvent>,
        new_premier_participant_event_handle:
            EventHandle<LockstreamNewPremierParticipantEvent>,
        claim_event_handle: EventHandle<LockstreamClaimEvent>,
        sweep_event_handle: EventHandle<LockstreamSweepEvent>,
    }

    struct ParticipantInfo has copy, drop, store {
        initial_quote_locked: u64,
        claimed_base: u64,
        claimed_quote: u64,
        last_claim_time_seconds: u64,
    }

    struct LockstreamPoolID has copy, drop, store {
        seeder: address,
        base_type: TypeInfo,
        quote_type: TypeInfo,
    }

    struct LockstreamSeedEvent has copy, drop, store {
        lockstream_pool_id: LockstreamPoolID,
        initial_base_locked: u64,
        pool_seed_time_seconds: u64,
        stream_start_time_seconds: u64,
        stream_end_time_seconds: u64,
        claim_window_end_time_seconds: u64,
        premier_sweep_window_end_time_seconds: u64,
    }

    struct LockstreamLockEvent has copy, drop, store {
        lockstream_pool_id: LockstreamPoolID,
        lock_time_seconds: u64,
        participant: address,
        quote_lock_amount: u64,
        total_quote_locked_for_participant: u64,
        total_quote_locked_for_pool: u64,
    }

    struct LockstreamNewPremierParticipantEvent has copy, drop, store {
        lockstream_pool_id: LockstreamPoolID,
        lock_time_seconds: u64,
        new_premier_participant: address,
        old_premier_participant: address,
        new_premier_participant_total_quote_locked: u64,
        old_premier_participant_total_quote_locked: u64,
        total_quote_locked_for_pool: u64,
    }

    struct LockstreamClaimEvent has copy, drop, store {
        lockstream_pool_id: LockstreamPoolID,
        claim_time_seconds: u64,
        participant: address,
        claimed_base: u64,
        claimed_quote: u64,
        total_claimed_base: u64,
        total_claimed_quote: u64,
    }

    struct LockstreamSweepEvent has copy, drop, store {
        lockstream_pool_id: LockstreamPoolID,
        sweep_time_seconds: u64,
        participant: address,
        base_sweep_amount: u64,
        quote_sweep_amount: u64,
    }

    struct LockstreamParticipantEventHandles has key {
        lock_event_handle: EventHandle<LockstreamLockEvent>,
        new_premier_participant_event_handle:
            EventHandle<LockstreamNewPremierParticipantEvent>,
        claim_event_handle: EventHandle<LockstreamClaimEvent>,
        sweep_event_handle: EventHandle<LockstreamSweepEvent>,
    }

    /// Time window bounds provided by seeder are invalid.
    const E_TIME_WINDOWS_INVALID: u64 = 0;
    /// Quote type provided by seeder is not a coin type.
    const E_QUOTE_NOT_COIN: u64 = 1;
    /// No lockstream pool for base type, quote type, and seeder.
    const E_NO_LOCKSTREAM_POOL: u64 = 2;
    /// Lockstream pool for base tye, quote type, and seeder exists.
    const E_LOCKSTREAM_POOL_EXISTS: u64 = 3;
    /// Too late to lock more quote into lockstream pool.
    const E_TOO_LATE_TO_LOCK: u64 = 4;
    /// No quote lock amount specified.
    const E_NO_QUOTE_LOCK_AMOUNT: u64 = 5;
    /// Signer is not a participant in the lockstream.
    const E_NOT_A_PARTICIPANT: u64 = 6;
    /// Too early to claim from lockstream.
    const E_TOO_EARLY_TO_CLAIM: u64 = 7;
    /// Too late to claim from lockstream.
    const E_TOO_LATE_TO_CLAIM: u64 = 8;
    /// Too early for premier participant to sweep lockstream pool.
    const E_TOO_EARLY_FOR_PREMIER_SWEEP: u64 = 9;
    /// Too late for premier participant to sweep lockstream pool.
    const E_TOO_LATE_FOR_PREMIER_SWEEP: u64 = 10;
    /// Too early for mercenary participant to sweep lockstream pool.
    const E_TOO_EARLY_FOR_MERCENARY_SWEEP: u64 = 11;
    /// No coins in lockstream pool left to sweep.
    const E_NOTHING_TO_SWEEP: u64 = 12;

    public entry fun seed<
        BaseType,
        QuoteType
    >(
        seeder: &signer,
        initial_base_locked: u64,
        stream_start_time_seconds: u64,
        stream_end_time_seconds: u64,
        claim_window_end_time_seconds: u64,
        premier_sweep_window_end_time_seconds: u64,
    ) {
        let seeder_address = signer::address_of(seeder);
        assert!(
            !exists<LockstreamPool<BaseType, QuoteType>>(seeder_address),
            E_LOCKSTREAM_POOL_EXISTS
        );
        let pool_seed_time_seconds = timestamp::now_seconds();
        assert!(
            pool_seed_time_seconds        < stream_start_time_seconds &&
            stream_start_time_seconds     < stream_end_time_seconds &&
            stream_end_time_seconds       < claim_window_end_time_seconds &&
            claim_window_end_time_seconds <
                    premier_sweep_window_end_time_seconds,
            E_TIME_WINDOWS_INVALID
        );
        assert!(coin::is_coin_initialized<QuoteType>(), E_QUOTE_NOT_COIN);
        let seed_event_handle = account::new_event_handle(seeder);
        event::emit_event(&mut seed_event_handle, LockstreamSeedEvent {
            lockstream_pool_id: LockstreamPoolID {
                seeder: seeder_address,
                base_type: type_info::type_of<BaseType>(),
                quote_type: type_info::type_of<QuoteType>(),
            },
            initial_base_locked,
            pool_seed_time_seconds,
            stream_start_time_seconds,
            stream_end_time_seconds,
            claim_window_end_time_seconds,
            premier_sweep_window_end_time_seconds,
        });
        move_to(seeder, LockstreamPool<BaseType, QuoteType> {
            base_coins: coin::withdraw(seeder, initial_base_locked),
            quote_coins: coin::zero(),
            participants: smart_table::new(),
            initial_base_locked,
            initial_quote_locked: 0,
            premier_participant: @0x0,
            premier_participant_initial_quote_locked: 0,
            pool_seed_time_seconds,
            stream_start_time_seconds,
            stream_end_time_seconds,
            claim_window_end_time_seconds,
            premier_sweep_window_end_time_seconds,
            seed_event_handle,
            lock_event_handle: account::new_event_handle(seeder),
            new_premier_participant_event_handle:
                account::new_event_handle(seeder),
            claim_event_handle: account::new_event_handle(seeder),
            sweep_event_handle: account::new_event_handle(seeder),
        });
    }

    public entry fun lock<
        BaseType,
        QuoteType
    >(
        participant: &signer,
        seeder: address,
        quote_lock_amount: u64,
    ) acquires
        LockstreamParticipantEventHandles,
        LockstreamPool
    {
        assert!(quote_lock_amount > 0, E_NO_QUOTE_LOCK_AMOUNT);
        let lockstream_pool_id =
            lockstream_pool_id<BaseType, QuoteType>(seeder);
        let pool_ref_mut =
            borrow_global_mut<LockstreamPool<BaseType, QuoteType>>(seeder);
        let lock_time_seconds = timestamp::now_seconds();
        assert!(
            lock_time_seconds < pool_ref_mut.stream_start_time_seconds,
            E_TOO_LATE_TO_LOCK
        );
        coin::merge(
            &mut pool_ref_mut.quote_coins,
            coin::withdraw(participant, quote_lock_amount)
        );
        let total_quote_locked_for_pool =
            coin::value(&pool_ref_mut.quote_coins);
        let participants_ref_mut = &mut pool_ref_mut.participants;
        let participant_address = signer::address_of(participant);
        let total_quote_locked_for_participant = quote_lock_amount;
        if (smart_table::contains(participants_ref_mut, participant_address)) {
            let participant_info_ref_mut = smart_table::borrow_mut(
                participants_ref_mut,
                participant_address
            );
            total_quote_locked_for_participant =
                total_quote_locked_for_participant +
                participant_info_ref_mut.initial_quote_locked;
            participant_info_ref_mut.initial_quote_locked =
                total_quote_locked_for_participant;
        } else {
            smart_table::add(
                participants_ref_mut, participant_address, ParticipantInfo {
                    initial_quote_locked: quote_lock_amount,
                    claimed_base: 0,
                    claimed_quote: 0,
                    last_claim_time_seconds: 0
                }
            );
        };
        let lock_event = LockstreamLockEvent {
            lockstream_pool_id,
            lock_time_seconds,
            participant: participant_address,
            quote_lock_amount,
            total_quote_locked_for_participant,
            total_quote_locked_for_pool
        };
        event::emit_event(&mut pool_ref_mut.lock_event_handle, lock_event);
        if (!exists<LockstreamParticipantEventHandles>(participant_address))
            move_to(participant, LockstreamParticipantEventHandles {
                lock_event_handle: account::new_event_handle(participant),
                new_premier_participant_event_handle:
                    account::new_event_handle(participant),
                claim_event_handle: account::new_event_handle(participant),
                sweep_event_handle: account::new_event_handle(participant),
            });
        let participant_handles_ref_mut =
            borrow_global_mut<LockstreamParticipantEventHandles>(
                participant_address);
        event::emit_event(
            &mut participant_handles_ref_mut.lock_event_handle,
            lock_event
        );
        let new_premier_participant =
            total_quote_locked_for_participant >
            pool_ref_mut.premier_participant_initial_quote_locked;
        if (new_premier_participant) {
            let event = LockstreamNewPremierParticipantEvent {
                lockstream_pool_id,
                lock_time_seconds,
                new_premier_participant: participant_address,
                old_premier_participant: pool_ref_mut.premier_participant,
                new_premier_participant_total_quote_locked:
                    total_quote_locked_for_participant,
                old_premier_participant_total_quote_locked:
                    pool_ref_mut.premier_participant_initial_quote_locked,
                total_quote_locked_for_pool,
            };
            event::emit_event(
                &mut pool_ref_mut.new_premier_participant_event_handle,
                event
            );
            let handles_ref_mut = participant_handles_ref_mut;
            event::emit_event(
                &mut handles_ref_mut.new_premier_participant_event_handle,
                event
            );
            pool_ref_mut.premier_participant = participant_address;
            pool_ref_mut.premier_participant_initial_quote_locked =
                total_quote_locked_for_participant;
        }
    }

    public entry fun claim<
        BaseType,
        QuoteType
    >(
        participant: &signer,
        seeder: address,
    ) acquires
        LockstreamParticipantEventHandles,
        LockstreamPool
    {
        let lockstream_pool_id =
            lockstream_pool_id<BaseType, QuoteType>(seeder);
        let pool_ref_mut =
            borrow_global_mut<LockstreamPool<BaseType, QuoteType>>(seeder);
        let participants_ref_mut = &mut pool_ref_mut.participants;
        let participant_address = signer::address_of(participant);
        assert!(
            smart_table::contains(participants_ref_mut, participant_address),
            E_NOT_A_PARTICIPANT
        );
        let claim_time_seconds = timestamp::now_seconds();
        assert!(
            claim_time_seconds > pool_ref_mut.stream_start_time_seconds,
            E_TOO_EARLY_TO_CLAIM
        );
        assert!(
            claim_time_seconds < pool_ref_mut.claim_window_end_time_seconds,
            E_TOO_LATE_TO_CLAIM
        );
        let participant_info_ref_mut =
            smart_table::borrow_mut(participants_ref_mut, participant_address);
        let participant_initial_quote_locked =
            participant_info_ref_mut.initial_quote_locked;
        let pro_rata_base = proportion(
            pool_ref_mut.initial_base_locked,
            participant_initial_quote_locked,
            pool_ref_mut.initial_quote_locked
        );
        let stream_done = claim_time_seconds >
            pool_ref_mut.stream_end_time_seconds;
        let (base_claim_ceiling, quote_claim_ceiling) = if (stream_done) {
            (pro_rata_base, participant_initial_quote_locked)
        } else {
            let stream_start = pool_ref_mut.stream_start_time_seconds;
            let elapsed = claim_time_seconds - stream_start;
            let duration = pool_ref_mut.stream_end_time_seconds - stream_start;
            (
                proportion(pro_rata_base, elapsed, duration),
                proportion(participant_initial_quote_locked, elapsed, duration)
            )
        };
        let claimed_base = base_claim_ceiling -
            participant_info_ref_mut.claimed_base;
        let claimed_quote = quote_claim_ceiling -
            participant_info_ref_mut.claimed_quote;
        if (claimed_base > 0) {
            coin::register<BaseType>(participant);
            coin::deposit(
                participant_address,
                coin::extract(&mut pool_ref_mut.base_coins, claimed_base)
            );
        };
        if (claimed_quote > 0) {
            coin::deposit(
                participant_address,
                coin::extract(&mut pool_ref_mut.quote_coins, claimed_quote)
            );
        };
        if (claimed_base > 0 || claimed_quote > 0) {
            let total_claimed_base =
                claimed_base + participant_info_ref_mut.claimed_base;
            let total_claimed_quote =
                claimed_quote + participant_info_ref_mut.claimed_quote;
            let event = LockstreamClaimEvent {
                lockstream_pool_id,
                claim_time_seconds,
                participant: participant_address,
                claimed_base,
                claimed_quote,
                total_claimed_base,
                total_claimed_quote,
            };
            participant_info_ref_mut.claimed_base = total_claimed_base;
            participant_info_ref_mut.claimed_quote = total_claimed_quote;
            event::emit_event(&mut pool_ref_mut.claim_event_handle, event);
            let participant_handles_ref_mut =
                borrow_global_mut<LockstreamParticipantEventHandles>(
                    participant_address);
            event::emit_event(
                &mut participant_handles_ref_mut.claim_event_handle,
                event
            );
        }
    }

    public entry fun sweep<
        BaseType,
        QuoteType
    >(
        participant: &signer,
        seeder: address,
    ) acquires
        LockstreamParticipantEventHandles,
        LockstreamPool
    {
        let lockstream_pool_id =
            lockstream_pool_id<BaseType, QuoteType>(seeder);
        let pool_ref_mut =
            borrow_global_mut<LockstreamPool<BaseType, QuoteType>>(seeder);
        let participants_ref_mut = &mut pool_ref_mut.participants;
        let participant_address = signer::address_of(participant);
        assert!(
            smart_table::contains(participants_ref_mut, participant_address),
            E_NOT_A_PARTICIPANT
        );
        let sweep_time_seconds = timestamp::now_seconds();
        if (participant_address == pool_ref_mut.premier_participant) {
            assert!(
                sweep_time_seconds >
                    pool_ref_mut.claim_window_end_time_seconds,
                E_TOO_EARLY_FOR_PREMIER_SWEEP
            );
            assert!(
                sweep_time_seconds <
                    pool_ref_mut.premier_sweep_window_end_time_seconds,
                E_TOO_LATE_FOR_PREMIER_SWEEP
            );
        } else {
            assert!(
                sweep_time_seconds >
                    pool_ref_mut.premier_sweep_window_end_time_seconds,
                E_TOO_EARLY_FOR_MERCENARY_SWEEP
            );
        };
        let base_to_sweep = coin::value(&pool_ref_mut.base_coins);
        let quote_to_sweep = coin::value(&pool_ref_mut.quote_coins);
        assert!(
            base_to_sweep > 0 || quote_to_sweep > 0,
            E_NOTHING_TO_SWEEP
        );
        if (base_to_sweep > 0) {
            coin::register<BaseType>(participant);
            coin::deposit(
                participant_address,
                coin::extract_all(&mut pool_ref_mut.base_coins)
            );
        };
        if (quote_to_sweep > 0) {
            coin::deposit(
                participant_address,
                coin::extract_all(&mut pool_ref_mut.quote_coins)
            );
        };
        let event = LockstreamSweepEvent {
            lockstream_pool_id,
            sweep_time_seconds,
            participant: participant_address,
            base_sweep_amount: base_to_sweep,
            quote_sweep_amount: quote_to_sweep,
        };
        event::emit_event(&mut pool_ref_mut.sweep_event_handle, event);
        let participant_handles_ref_mut =
            borrow_global_mut<LockstreamParticipantEventHandles>(
                participant_address);
        event::emit_event(
            &mut participant_handles_ref_mut.sweep_event_handle,
            event
        );
    }

    public inline fun proportion(
        scalar: u64,
        numerator: u64,
        denominator: u64
    ): u64 {
        ((
            (scalar as u128) *
            (numerator as u128) /
            (denominator as u128)
        ) as u64)
    }

    #[view]
    public fun lockstream_pool_id<
        BaseType,
        QuoteType
    >(seeder: address):
    LockstreamPoolID {
        assert!(
            exists<LockstreamPool<BaseType, QuoteType>>(seeder),
            E_NO_LOCKSTREAM_POOL
        );
        LockstreamPoolID {
            seeder,
            base_type: type_info::type_of<BaseType>(),
            quote_type: type_info::type_of<QuoteType>(),
        }
    }


}