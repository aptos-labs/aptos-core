/// Sharded claim registry for the airdrop benchmark.
///
/// Every recipient owns one table item, so a distribution batch of `N`
/// recipients writes `N` independent storage slots and needs no signer from
/// any of them. Shards exist so that accounts assigned to different shards
/// write disjoint tables while accounts sharing one collide on purpose.
///
/// Shards sit in a vector rather than a table: a batch then reaches its claims
/// table through a vector index instead of a second table item lookup per
/// recipient, which would otherwise show up in the measurement.
///
/// A campaign pays each cohort once, so the recipients a run generates are
/// never seen twice and the claims table would grow for as long as the run
/// lasts. Each shard therefore records the slots a distribution created in
/// creation order and `trim` drops the oldest of them back to a cap, which
/// holds the table at a fixed size. Slots the setup creates stay out of that
/// order, so the warm recipients a batch is drawn from are never dropped.
module bench::ad_registry {
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};

    /// Only the package address can hold the registry.
    const E_NOT_BENCH: u64 = 1;
    /// A registry with no shards has nowhere to put a recipient.
    const E_NO_SHARDS: u64 = 2;

    /// Slots one `trim` may drop. A batch creates at most `MAX_RECIPIENTS` of
    /// them, so a trim per batch keeps up, and a caller that lowers the cap
    /// mid-run walks the backlog down over several batches instead of doing
    /// it all in one transaction.
    const MAX_EVICTIONS: u64 = 256;

    struct Claim has store, drop {
        /// Distributed but not yet claimed.
        amount: u64,
        /// Distributions credited to this slot.
        credits: u64,
        /// Claims made against this slot, paying or not.
        claims: u64,
        /// Mutations the touch probe has made to this slot.
        touches: u64,
        /// Campaign payload carried alongside the amount. Sized by the
        /// generator, so a run can widen the item without changing its shape.
        memo: vector<u8>,
    }

    struct Shard has store {
        claims: Table<address, Claim>,
        /// Slots ever created in this shard.
        n_slots: u64,
        /// Tracked slots in creation order, the oldest at `head` and the next
        /// one in at `tail`.
        order: Table<u64, address>,
        head: u64,
        tail: u64,
    }

    struct Registry has key {
        shards: vector<Shard>,
    }

    /// Create `n_shards` empty shards. Calling this again is a no-op, so a
    /// re-run of the setup does not abort.
    public fun initialize(admin: &signer, n_shards: u64) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        assert!(n_shards > 0, E_NO_SHARDS);
        if (exists<Registry>(@bench)) { return };
        let shards = vector::empty<Shard>();
        let i = 0;
        while (i < n_shards) {
            vector::push_back(
                &mut shards,
                Shard {
                    claims: table::new<address, Claim>(),
                    n_slots: 0,
                    order: table::new<u64, address>(),
                    head: 0,
                    tail: 0,
                },
            );
            i = i + 1;
        };
        move_to(admin, Registry { shards });
    }

    /// The id wraps, so a caller that names a shard past the end lands on a
    /// real one instead of aborting.
    fun shard_mut(registry: &mut Registry, shard_id: u64): &mut Shard {
        let id = shard_id % vector::length(&registry.shards);
        vector::borrow_mut(&mut registry.shards, id)
    }

    fun shard_ref(registry: &Registry, shard_id: u64): &Shard {
        let id = shard_id % vector::length(&registry.shards);
        vector::borrow(&registry.shards, id)
    }

    /// Credit `recipient`, creating the slot when nothing has paid it before.
    /// Returns whether the slot was created, which is what separates a batch
    /// of creation writes from a batch of modification writes. A created slot
    /// joins the shard's eviction order when `track` is set, which is how the
    /// one-shot recipients of the mix are held to a fixed population and the
    /// warm ones the setup creates are kept out of it.
    public fun credit(
        shard_id: u64,
        recipient: address,
        amount: u64,
        memo: vector<u8>,
        track: bool,
    ): bool acquires Registry {
        let shard = shard_mut(borrow_global_mut<Registry>(@bench), shard_id);
        if (table::contains(&shard.claims, recipient)) {
            let claim = table::borrow_mut(&mut shard.claims, recipient);
            claim.amount = claim.amount + amount;
            claim.credits = claim.credits + 1;
            claim.memo = memo;
            false
        } else {
            table::add(
                &mut shard.claims,
                recipient,
                Claim { amount, credits: 1, claims: 0, touches: 0, memo },
            );
            shard.n_slots = shard.n_slots + 1;
            if (track) {
                table::add(&mut shard.order, shard.tail, recipient);
                shard.tail = shard.tail + 1;
            };
            true
        }
    }

    /// Drop tracked slots, oldest first, until at most `cap` of them are left,
    /// and report how many went. A slot that is already gone costs a lookup
    /// and nothing else.
    public fun trim(shard_id: u64, cap: u64): u64 acquires Registry {
        let shard = shard_mut(borrow_global_mut<Registry>(@bench), shard_id);
        let dropped = 0;
        while (shard.tail - shard.head > cap && dropped < MAX_EVICTIONS) {
            let recipient = table::remove(&mut shard.order, shard.head);
            shard.head = shard.head + 1;
            if (table::contains(&shard.claims, recipient)) {
                table::remove(&mut shard.claims, recipient);
            };
            dropped = dropped + 1;
        };
        dropped
    }

    /// Borrow the slot for writing and change it only when `write` is set. The
    /// borrow happens either way, which is what makes the gap between reported
    /// and real writes observable. A recipient with no slot yet is skipped.
    /// Returns whether the slot really changed.
    public fun touch(
        shard_id: u64, recipient: address, write: bool
    ): bool acquires Registry {
        let shard = shard_mut(borrow_global_mut<Registry>(@bench), shard_id);
        if (!table::contains(&shard.claims, recipient)) { return false };
        let claim = table::borrow_mut(&mut shard.claims, recipient);
        if (write) {
            claim.touches = claim.touches + 1;
            true
        } else {
            false
        }
    }

    /// Take whatever the slot owes and return it. A second claim finds zero
    /// and only bumps the counter, and claiming against a slot that does not
    /// exist creates an empty one.
    public fun claim(shard_id: u64, recipient: address): u64 acquires Registry {
        let shard = shard_mut(borrow_global_mut<Registry>(@bench), shard_id);
        if (!table::contains(&shard.claims, recipient)) {
            table::add(
                &mut shard.claims,
                recipient,
                Claim {
                    amount: 0,
                    credits: 0,
                    claims: 1,
                    touches: 0,
                    memo: vector::empty<u8>(),
                },
            );
            shard.n_slots = shard.n_slots + 1;
            return 0
        };
        let claim = table::borrow_mut(&mut shard.claims, recipient);
        let amount = claim.amount;
        claim.amount = 0;
        claim.claims = claim.claims + 1;
        amount
    }

    /// Read one slot without writing anything. An absent slot reads as zero.
    public fun peek(shard_id: u64, recipient: address): u64 acquires Registry {
        let shard = shard_ref(borrow_global<Registry>(@bench), shard_id);
        if (table::contains(&shard.claims, recipient)) {
            table::borrow(&shard.claims, recipient).amount
        } else {
            0
        }
    }

    #[view]
    public fun n_shards(): u64 acquires Registry {
        vector::length(&borrow_global<Registry>(@bench).shards)
    }

    #[view]
    public fun shard_slots(shard_id: u64): u64 acquires Registry {
        shard_ref(borrow_global<Registry>(@bench), shard_id).n_slots
    }

    #[view]
    /// Slots of this shard that a trim can still drop, which is the part of
    /// the claims table that a run grows.
    public fun shard_tracked(shard_id: u64): u64 acquires Registry {
        let shard = shard_ref(borrow_global<Registry>(@bench), shard_id);
        shard.tail - shard.head
    }

    #[view]
    public fun has_slot(shard_id: u64, recipient: address): bool acquires Registry {
        let shard = shard_ref(borrow_global<Registry>(@bench), shard_id);
        table::contains(&shard.claims, recipient)
    }

    #[view]
    /// `(amount, credits, claims, touches)` of a slot, all zero if absent.
    public fun slot_state(
        shard_id: u64, recipient: address
    ): (u64, u64, u64, u64) acquires Registry {
        let shard = shard_ref(borrow_global<Registry>(@bench), shard_id);
        if (!table::contains(&shard.claims, recipient)) {
            return (0, 0, 0, 0)
        };
        let claim = table::borrow(&shard.claims, recipient);
        (claim.amount, claim.credits, claim.claims, claim.touches)
    }

    #[view]
    public fun memo_len(shard_id: u64, recipient: address): u64 acquires Registry {
        let shard = shard_ref(borrow_global<Registry>(@bench), shard_id);
        if (!table::contains(&shard.claims, recipient)) {
            return 0
        };
        vector::length(&table::borrow(&shard.claims, recipient).memo)
    }
}
