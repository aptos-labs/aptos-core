/// Entry surface of the oracle benchmark, shaped after a Switchboard feed
/// update: verify a batch of signed reports against the authority set, decode
/// it, and publish a median and a time-weighted price per feed.
///
/// The mix splits that work apart so the signature natives can be priced by
/// subtraction. `bench_verify_only` runs the verification and nothing else,
/// `bench_write_only` runs the decode, the writes and the event and nothing
/// else, and `bench_update_ed25519` runs both. Each branch tolerates any input
/// it is handed: a failed verification is counted, a short record decodes as
/// zeros, and a feed id past the seeded range creates the feed.
module bench::orc_aggregator {
    use std::hash;
    use std::signer;
    use std::vector;
    use aptos_framework::event;
    use bench::orc_decode;
    use bench::orc_queue;
    use bench::orc_registry;
    use bench::orc_verifier;

    /// Recovery id a signature with none of its own falls back to. Every
    /// signature the generator sends carries one, so this only covers a
    /// hand-built call.
    const DEFAULT_RECOVERY_ID: u8 = 0;

    /// Ceiling on a read-only scan, so a caller cannot ask for an unbounded
    /// one.
    const MAX_READ: u64 = 256;

    /// Per-account state of an update publisher.
    struct Subscriber has key {
        feed_base: u64,
        batches: u64,
        verified: u64,
        rejected: u64,
        /// Recovered secp256k1 keys that were in the authority set.
        authorized: u64,
        last_aggregate: u128,
    }

    #[event]
    struct BatchApplied has drop, store {
        publisher: address,
        feed_base: u64,
        written: u64,
        verified: u64,
        rejected: u64,
    }

    /// Bring a fresh account to the state the mix assumes: a subscriber record
    /// pointing at the feed window the account writes. Re-running it repoints
    /// the window rather than aborting.
    public entry fun bench_onboard(user: &signer, feed_base: u64) acquires Subscriber {
        let addr = signer::address_of(user);
        if (exists<Subscriber>(addr)) {
            borrow_global_mut<Subscriber>(addr).feed_base = feed_base;
        } else {
            move_to(
                user,
                Subscriber {
                    feed_base,
                    batches: 0,
                    verified: 0,
                    rejected: 0,
                    authorized: 0,
                    last_aggregate: 0,
                },
            );
        }
    }

    /// Verify every signature over the batch, then publish it.
    public entry fun bench_update_ed25519(
        user: &signer,
        feed_base: u64,
        records: vector<u8>,
        signatures: vector<vector<u8>>,
        signer_idxs: vector<u64>,
        decode_depth: u64,
    ) acquires Subscriber {
        let (verified, rejected) =
            verify_ed25519_batch(&records, &signatures, &signer_idxs);
        publish(user, feed_base, &records, decode_depth, verified, rejected, 0);
    }

    /// Same, over secp256k1 recovery: the digest is `sha3_256` of the batch,
    /// which is what the signer's message hashes to. Each signature carries the
    /// recovery id it was produced under, so a recovery lands on the key that
    /// signed rather than on whichever one the id happens to select.
    public entry fun bench_update_secp256k1(
        user: &signer,
        feed_base: u64,
        records: vector<u8>,
        signatures: vector<vector<u8>>,
        recovery_ids: vector<u8>,
        decode_depth: u64,
    ) acquires Subscriber {
        let (authorized, rejected) =
            recover_secp256k1_batch(&records, &signatures, &recovery_ids);
        publish(
            user,
            feed_base,
            &records,
            decode_depth,
            authorized,
            rejected,
            authorized,
        );
    }

    /// The verification of an update with none of its storage work.
    public entry fun bench_verify_only(
        _user: &signer,
        records: vector<u8>,
        signatures: vector<vector<u8>>,
        signer_idxs: vector<u64>,
    ) {
        let (_verified, _rejected) =
            verify_ed25519_batch(&records, &signatures, &signer_idxs);
    }

    /// The storage work of an update with none of its verification. Matched
    /// against `bench_update_ed25519` on everything else, so the difference
    /// between the two is the signature natives.
    public entry fun bench_write_only(
        user: &signer, feed_base: u64, records: vector<u8>, decode_depth: u64
    ) acquires Subscriber {
        publish(user, feed_base, &records, decode_depth, 0, 0, 0);
    }

    /// k-of-n acceptance across both schemes. Only a signature that belongs to
    /// the authority set counts towards the quorum, and a batch that misses it
    /// is counted and dropped, which is how a real feed treats one.
    public entry fun bench_quorum(
        user: &signer,
        feed_base: u64,
        records: vector<u8>,
        ed_signatures: vector<vector<u8>>,
        signer_idxs: vector<u64>,
        secp_signatures: vector<vector<u8>>,
        recovery_ids: vector<u8>,
        decode_depth: u64,
    ) acquires Subscriber {
        let (ed_ok, ed_failed) =
            verify_ed25519_batch(&records, &ed_signatures, &signer_idxs);
        let (authorized, secp_failed) =
            recover_secp256k1_batch(&records, &secp_signatures, &recovery_ids);
        let verified = ed_ok + authorized;
        let rejected = ed_failed + secp_failed;
        if (verified >= orc_queue::quorum()) {
            publish(
                user,
                feed_base,
                &records,
                decode_depth,
                verified,
                rejected,
                authorized,
            );
        } else {
            note_rejected(signer::address_of(user), verified, rejected);
        }
    }

    /// Read-only consumer scan over a feed window.
    public entry fun bench_read_aggregate(
        _user: &signer, feed_base: u64, count: u64
    ) {
        read_aggregate(feed_base, count);
    }

    /// Fold the published aggregate of `count` feeds from `feed_base`.
    public fun read_aggregate(feed_base: u64, count: u64): u128 {
        let feed_base = feed_base % orc_registry::feed_id_space();
        let n = if (count > MAX_READ) MAX_READ else count;
        let acc = 0u128;
        let i = 0;
        while (i < n) {
            let (_price, aggregate, _conf, _ts, _updates) =
                orc_registry::read(feed_base + i);
            acc = acc ^ aggregate;
            i = i + 1;
        };
        acc
    }

    /// Decode every record in the batch and publish it. The record's feed id
    /// is an offset into the account's window, so two accounts on different
    /// windows never touch the same feed.
    fun apply_batch(
        feed_base: u64, records: &vector<u8>, decode_depth: u64
    ): (u64, u128) {
        let n = orc_decode::n_records(records);
        if (n == 0) return (0, 0);
        let feed_base = feed_base % orc_registry::feed_id_space();
        let last = 0u128;
        let i = 0;
        while (i < n) {
            let (offset, price, conf, ts) =
                orc_decode::decode_record(records, i, decode_depth);
            let feed_id = feed_base + (offset % n);
            let window = orc_registry::history(feed_id);
            vector::push_back(&mut window, price);
            last = median(window);
            orc_registry::write(feed_id, price, last, conf, ts);
            i = i + 1;
        };
        (n, last)
    }

    fun publish(
        user: &signer,
        feed_base: u64,
        records: &vector<u8>,
        decode_depth: u64,
        verified: u64,
        rejected: u64,
        authorized: u64,
    ) acquires Subscriber {
        let (written, last) = apply_batch(feed_base, records, decode_depth);
        let addr = signer::address_of(user);
        if (exists<Subscriber>(addr)) {
            let subscriber = borrow_global_mut<Subscriber>(addr);
            subscriber.batches = subscriber.batches + 1;
            subscriber.verified = subscriber.verified + verified;
            subscriber.rejected = subscriber.rejected + rejected;
            subscriber.authorized = subscriber.authorized + authorized;
            subscriber.last_aggregate = last;
        };
        event::emit(
            BatchApplied {
                publisher: addr,
                feed_base,
                written,
                verified,
                rejected,
            },
        );
    }

    fun note_rejected(addr: address, verified: u64, rejected: u64) acquires Subscriber {
        if (exists<Subscriber>(addr)) {
            let subscriber = borrow_global_mut<Subscriber>(addr);
            subscriber.verified = subscriber.verified + verified;
            subscriber.rejected = subscriber.rejected + rejected;
        }
    }

    /// `(verified, rejected)` over the batch. An index list shorter than the
    /// signature list falls back to the first authority.
    fun verify_ed25519_batch(
        message: &vector<u8>,
        signatures: &vector<vector<u8>>,
        signer_idxs: &vector<u64>,
    ): (u64, u64) {
        let n = vector::length(signatures);
        let idxs = vector::length(signer_idxs);
        let verified = 0;
        let rejected = 0;
        let i = 0;
        while (i < n) {
            let idx = if (i < idxs) *vector::borrow(signer_idxs, i) else 0;
            let ok = orc_verifier::verify_ed25519(
                orc_queue::ed_pubkey(idx),
                *vector::borrow(signatures, i),
                *message,
            );
            if (ok) verified = verified + 1 else rejected = rejected + 1;
            i = i + 1;
        };
        (verified, rejected)
    }

    /// `(authorized, rejected)` over the batch. A signature that fails to
    /// recover, or that recovers a key the authority set does not hold, is
    /// rejected: what a feed accepts is a signature from one of its
    /// authorities, not any signature at all. An id list shorter than the
    /// signature list falls back to `DEFAULT_RECOVERY_ID`.
    fun recover_secp256k1_batch(
        records: &vector<u8>,
        signatures: &vector<vector<u8>>,
        recovery_ids: &vector<u8>,
    ): (u64, u64) {
        let digest = hash::sha3_256(*records);
        let n = vector::length(signatures);
        let ids = vector::length(recovery_ids);
        let authorized = 0;
        let rejected = 0;
        let i = 0;
        while (i < n) {
            let id =
                if (i < ids) *vector::borrow(recovery_ids, i)
                else DEFAULT_RECOVERY_ID;
            let addr = orc_verifier::recover_secp256k1(
                digest, id, *vector::borrow(signatures, i));
            if (vector::length(&addr) > 0 && orc_queue::has_secp_addr(&addr)) {
                authorized = authorized + 1;
            } else {
                rejected = rejected + 1;
            };
            i = i + 1;
        };
        (authorized, rejected)
    }

    /// Median by insertion sort. The window is one longer than
    /// `orc_registry::history_len()`, short enough that anything smarter costs
    /// more than it saves.
    public fun median(prices: vector<u128>): u128 {
        let n = vector::length(&prices);
        if (n == 0) return 0;
        let i = 1;
        while (i < n) {
            let j = i;
            while (j > 0
                && *vector::borrow(&prices, j - 1) > *vector::borrow(&prices, j)) {
                vector::swap(&mut prices, j - 1, j);
                j = j - 1;
            };
            i = i + 1;
        };
        *vector::borrow(&prices, n / 2)
    }

    #[view]
    /// `(batches, verified, rejected, authorized)` for a publisher, all zero
    /// before it onboards.
    public fun subscriber_state(addr: address): (u64, u64, u64, u64) acquires Subscriber {
        if (!exists<Subscriber>(addr)) return (0, 0, 0, 0);
        let subscriber = borrow_global<Subscriber>(addr);
        (
            subscriber.batches,
            subscriber.verified,
            subscriber.rejected,
            subscriber.authorized,
        )
    }

    #[view]
    public fun last_aggregate(addr: address): u128 acquires Subscriber {
        if (!exists<Subscriber>(addr)) return 0;
        borrow_global<Subscriber>(addr).last_aggregate
    }
}
