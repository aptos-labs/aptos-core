/// Authority set an oracle update is checked against, shaped after the
/// Switchboard queue on Aptos: a list of ed25519 public keys, a list of
/// secp256k1 authority addresses, and how many of them a quorum needs.
///
/// The two lists arrive as one argument and are split by length, since an
/// ed25519 public key is 32 bytes and a raw secp256k1 one is 64.
module bench::orc_queue {
    use std::hash;
    use std::signer;
    use std::vector;

    /// Only the package address configures the queue.
    const E_NOT_BENCH: u64 = 1;

    /// Length that marks an entry of the authority list as an ed25519 key.
    const ED_PUBKEY_LEN: u64 = 32;

    struct Queue has key {
        ed_pubkeys: vector<vector<u8>>,
        /// `sha3_256` of each raw secp256k1 public key, which is what a
        /// recovered key can be compared against.
        secp_addrs: vector<vector<u8>>,
        quorum: u64,
    }

    /// Install the authority set. Re-running it replaces the set rather than
    /// aborting, so a run can be re-initialized.
    public entry fun initialize(
        admin: &signer, pubkeys: vector<vector<u8>>, quorum: u64
    ) acquires Queue {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let ed_pubkeys = vector::empty<vector<u8>>();
        let secp_addrs = vector::empty<vector<u8>>();
        let n = vector::length(&pubkeys);
        let i = 0;
        while (i < n) {
            let key = *vector::borrow(&pubkeys, i);
            if (vector::length(&key) == ED_PUBKEY_LEN) {
                vector::push_back(&mut ed_pubkeys, key);
            } else {
                vector::push_back(&mut secp_addrs, hash::sha3_256(key));
            };
            i = i + 1;
        };
        if (exists<Queue>(@bench)) {
            let queue = borrow_global_mut<Queue>(@bench);
            queue.ed_pubkeys = ed_pubkeys;
            queue.secp_addrs = secp_addrs;
            queue.quorum = quorum;
        } else {
            move_to(admin, Queue { ed_pubkeys, secp_addrs, quorum });
        }
    }

    #[view]
    public fun is_initialized(): bool {
        exists<Queue>(@bench)
    }

    #[view]
    public fun num_ed_pubkeys(): u64 acquires Queue {
        if (!exists<Queue>(@bench)) return 0;
        vector::length(&borrow_global<Queue>(@bench).ed_pubkeys)
    }

    #[view]
    public fun num_secp_addrs(): u64 acquires Queue {
        if (!exists<Queue>(@bench)) return 0;
        vector::length(&borrow_global<Queue>(@bench).secp_addrs)
    }

    #[view]
    public fun quorum(): u64 acquires Queue {
        if (!exists<Queue>(@bench)) return 0;
        borrow_global<Queue>(@bench).quorum
    }

    /// The `index`th ed25519 key, wrapping the index and handing back an empty
    /// key on an empty set, so a caller can never index out of bounds.
    public fun ed_pubkey(index: u64): vector<u8> acquires Queue {
        if (!exists<Queue>(@bench)) return vector::empty();
        let keys = &borrow_global<Queue>(@bench).ed_pubkeys;
        let n = vector::length(keys);
        if (n == 0) vector::empty() else *vector::borrow(keys, index % n)
    }

    /// Whether `addr` is one of the configured secp256k1 authority addresses.
    public fun has_secp_addr(addr: &vector<u8>): bool acquires Queue {
        if (!exists<Queue>(@bench)) return false;
        vector::contains(&borrow_global<Queue>(@bench).secp_addrs, addr)
    }
}
