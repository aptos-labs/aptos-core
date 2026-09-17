/// OApp registration for the relay endpoint, shaped after LayerZero V2.
///
/// One OApp per channel, each pinned to a message library version and to a
/// peer address on the remote chain. Peers are derived from the channel id
/// rather than supplied, so a generator, a test, and the chain all agree on
/// them without exchanging addresses.
module bench::relay_registry {
    use std::bcs;
    use std::signer;
    use aptos_std::from_bcs;
    use aptos_std::table::{Self, Table};

    /// Only the package address may hold or configure the registry.
    const E_NOT_BENCH: u64 = 1;

    struct OApp has store, drop {
        remote_eid: u32,
        peer: address,
        msglib_version: u8,
    }

    struct Registry has key {
        oapps: Table<u64, OApp>,
        n_oapps: u64,
    }

    public fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Registry>(@bench)) {
            move_to(admin, Registry { oapps: table::new(), n_oapps: 0 });
        }
    }

    /// Peer address for `channel_id`: the id as a 32-byte little-endian
    /// integer, so the address is reproducible off chain from the id alone.
    public fun derive_peer(channel_id: u64): address {
        from_bcs::to_address(bcs::to_bytes(&(channel_id as u256)))
    }

    public fun register(
        admin: &signer, channel_id: u64, remote_eid: u32, msglib_version: u8
    ) acquires Registry {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let registry = borrow_global_mut<Registry>(@bench);
        let oapp = OApp {
            remote_eid,
            peer: derive_peer(channel_id),
            msglib_version,
        };
        if (table::contains(&registry.oapps, channel_id)) {
            *table::borrow_mut(&mut registry.oapps, channel_id) = oapp;
        } else {
            table::add(&mut registry.oapps, channel_id, oapp);
            registry.n_oapps = registry.n_oapps + 1;
        }
    }

    /// Repoint an OApp at another message library. Unpermissioned and a no-op
    /// on an unregistered channel, because the mix drives it from an ordinary
    /// account.
    public fun set_msglib(channel_id: u64, msglib_version: u8) acquires Registry {
        if (!exists<Registry>(@bench)) return;
        let registry = borrow_global_mut<Registry>(@bench);
        if (!table::contains(&registry.oapps, channel_id)) return;
        table::borrow_mut(&mut registry.oapps, channel_id).msglib_version =
            msglib_version;
    }

    #[view]
    public fun is_registered(channel_id: u64): bool acquires Registry {
        if (!exists<Registry>(@bench)) return false;
        table::contains(&borrow_global<Registry>(@bench).oapps, channel_id)
    }

    #[view]
    /// Remote peer of `channel_id`, or the zero address when it has none.
    public fun peer_of(channel_id: u64): address acquires Registry {
        if (!is_registered(channel_id)) return @0x0;
        table::borrow(&borrow_global<Registry>(@bench).oapps, channel_id).peer
    }

    #[view]
    public fun remote_eid_of(channel_id: u64): u32 acquires Registry {
        if (!is_registered(channel_id)) return 0;
        table::borrow(
            &borrow_global<Registry>(@bench).oapps, channel_id).remote_eid
    }

    #[view]
    public fun msglib_version_of(channel_id: u64): u8 acquires Registry {
        if (!is_registered(channel_id)) return 0;
        table::borrow(
            &borrow_global<Registry>(@bench).oapps, channel_id).msglib_version
    }

    #[view]
    public fun n_oapps(): u64 acquires Registry {
        if (!exists<Registry>(@bench)) return 0;
        borrow_global<Registry>(@bench).n_oapps
    }
}
