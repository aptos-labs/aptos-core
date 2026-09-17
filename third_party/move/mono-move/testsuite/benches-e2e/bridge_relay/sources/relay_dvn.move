/// Decentralised verifier network: the per-channel verifier set and the
/// attestations it has produced, shaped after a LayerZero V2 DVN.
///
/// An attestation is a count keyed by GUID, so re-attesting overwrites instead
/// of failing. Delivery reads the count but never requires it, which is what
/// lets the mix run verify and deliver in any order.
module bench::relay_dvn {
    use std::bcs;
    use std::signer;
    use std::vector;
    use aptos_std::from_bcs;
    use aptos_std::table::{Self, Table};

    /// Only the package address may configure a verifier set.
    const E_NOT_BENCH: u64 = 1;

    /// Verifiers a channel may hold. A caller-supplied count is clamped to
    /// this rather than rejected, since the mix drives it from an ordinary
    /// account.
    const MAX_VERIFIERS: u64 = 16;

    struct Dvn has key {
        verifiers: Table<u64, vector<address>>,
        attested: Table<u128, u64>,
    }

    public fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Dvn>(@bench)) {
            move_to(
                admin,
                Dvn { verifiers: table::new(), attested: table::new() },
            );
        }
    }

    public entry fun set_verifiers(
        admin: &signer, channel_id: u64, n_verifiers: u64
    ) acquires Dvn {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        resize(channel_id, n_verifiers);
    }

    /// Rebuild the channel's verifier set at `n_verifiers`, clamped.
    public fun resize(channel_id: u64, n_verifiers: u64) acquires Dvn {
        if (!exists<Dvn>(@bench)) return;
        let n =
            if (n_verifiers > MAX_VERIFIERS) MAX_VERIFIERS else n_verifiers;
        let set = vector::empty<address>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut set, verifier_address(channel_id, i));
            i = i + 1;
        };
        let dvn = borrow_global_mut<Dvn>(@bench);
        if (table::contains(&dvn.verifiers, channel_id)) {
            *table::borrow_mut(&mut dvn.verifiers, channel_id) = set;
        } else {
            table::add(&mut dvn.verifiers, channel_id, set);
        }
    }

    /// Record `count` attestations against `guid`, replacing whatever was
    /// there.
    public fun attest(guid: u128, count: u64) acquires Dvn {
        if (!exists<Dvn>(@bench)) return;
        let dvn = borrow_global_mut<Dvn>(@bench);
        if (table::contains(&dvn.attested, guid)) {
            *table::borrow_mut(&mut dvn.attested, guid) = count;
        } else {
            table::add(&mut dvn.attested, guid, count);
        }
    }

    /// Verifier address `index` of `channel_id`, derived so a test and a
    /// generator agree on the set without it being written down.
    fun verifier_address(channel_id: u64, index: u64): address {
        let seed = (channel_id as u256) * (MAX_VERIFIERS as u256)
            + (index as u256) + 1;
        from_bcs::to_address(bcs::to_bytes(&seed))
    }

    #[view]
    public fun attestations(guid: u128): u64 acquires Dvn {
        if (!exists<Dvn>(@bench)) return 0;
        let dvn = borrow_global<Dvn>(@bench);
        if (!table::contains(&dvn.attested, guid)) return 0;
        *table::borrow(&dvn.attested, guid)
    }

    #[view]
    public fun verifier_count(channel_id: u64): u64 acquires Dvn {
        if (!exists<Dvn>(@bench)) return 0;
        let dvn = borrow_global<Dvn>(@bench);
        if (!table::contains(&dvn.verifiers, channel_id)) return 0;
        vector::length(table::borrow(&dvn.verifiers, channel_id))
    }

    #[view]
    public fun max_verifiers(): u64 { MAX_VERIFIERS }
}
