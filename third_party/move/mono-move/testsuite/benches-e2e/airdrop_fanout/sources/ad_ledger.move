/// Campaign counters and the per-batch event.
///
/// Mainnet batch distributors keep a running total and emit one event per
/// call rather than one per recipient, so the ledger is a single resource at
/// `@bench` that every batch updates.
module bench::ad_ledger {
    use std::signer;
    use aptos_framework::event;

    /// Only the package address can hold the ledger.
    const E_NOT_BENCH: u64 = 1;

    /// Branch that produced a batch, so a report can be attributed back to it.
    const KIND_DISTRIBUTE: u8 = 0;
    const KIND_FA_FANOUT: u8 = 1;
    const KIND_TOUCH: u8 = 2;
    const KIND_CLAIM: u8 = 3;

    #[event]
    struct BatchDistributed has drop, store {
        shard: u64,
        kind: u8,
        recipients: u64,
        /// Slots the batch created, as opposed to modified.
        fresh: u64,
        amount: u64,
    }

    struct Ledger has key {
        batches: u64,
        recipients: u64,
        fresh_slots: u64,
        claims: u64,
        /// Widened past the per-batch `u64` so that a run long enough to
        /// overflow it cannot abort the benchmark.
        amount: u128,
    }

    /// Calling this again is a no-op, so a re-run of the setup does not abort.
    public fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (exists<Ledger>(@bench)) { return };
        move_to(
            admin,
            Ledger {
                batches: 0,
                recipients: 0,
                fresh_slots: 0,
                claims: 0,
                amount: 0,
            },
        );
    }

    public fun record_batch(
        shard: u64, kind: u8, recipients: u64, fresh: u64, amount: u64
    ) acquires Ledger {
        let ledger = borrow_global_mut<Ledger>(@bench);
        ledger.batches = ledger.batches + 1;
        ledger.recipients = ledger.recipients + recipients;
        ledger.fresh_slots = ledger.fresh_slots + fresh;
        ledger.amount = ledger.amount + (amount as u128);
        event::emit(BatchDistributed { shard, kind, recipients, fresh, amount });
    }

    public fun record_claim(shard: u64, amount: u64) acquires Ledger {
        let ledger = borrow_global_mut<Ledger>(@bench);
        ledger.batches = ledger.batches + 1;
        ledger.claims = ledger.claims + 1;
        event::emit(
            BatchDistributed {
                shard,
                kind: KIND_CLAIM,
                recipients: 1,
                fresh: 0,
                amount,
            },
        );
    }

    public fun kind_distribute(): u8 { KIND_DISTRIBUTE }

    public fun kind_fa_fanout(): u8 { KIND_FA_FANOUT }

    public fun kind_touch(): u8 { KIND_TOUCH }

    public fun kind_claim(): u8 { KIND_CLAIM }

    #[view]
    public fun batches(): u64 acquires Ledger {
        borrow_global<Ledger>(@bench).batches
    }

    #[view]
    public fun recipients(): u64 acquires Ledger {
        borrow_global<Ledger>(@bench).recipients
    }

    #[view]
    public fun fresh_slots(): u64 acquires Ledger {
        borrow_global<Ledger>(@bench).fresh_slots
    }

    #[view]
    public fun claims(): u64 acquires Ledger {
        borrow_global<Ledger>(@bench).claims
    }

    #[view]
    public fun amount(): u128 acquires Ledger {
        borrow_global<Ledger>(@bench).amount
    }
}
