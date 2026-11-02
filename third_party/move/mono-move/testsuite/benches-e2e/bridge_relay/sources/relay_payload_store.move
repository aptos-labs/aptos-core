/// One table item per message payload: the write target this workload exists
/// to measure.
///
/// A slot key is the direction bit over the channel id and the message nonce
/// folded into the channel's ring, so an outbound payload and the delivered
/// copy of the same message land in different items, neither collides across
/// channels, and the store settles at two rings a channel rather than growing
/// for the length of a run.
module bench::relay_payload_store {
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};

    /// Only the package address may hold the store.
    const E_NOT_BENCH: u64 = 1;

    const DIRECTION_SHIFT: u8 = 63;

    struct Store has key {
        payloads: Table<u64, vector<u8>>,
    }

    public fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Store>(@bench)) {
            move_to(admin, Store { payloads: table::new() });
        }
    }

    public fun slot(guid_slot_bits: u64, inbound: bool): u64 {
        if (inbound) {
            guid_slot_bits | (1 << DIRECTION_SHIFT)
        } else {
            guid_slot_bits
        }
    }

    /// Store `payload`, replacing whatever the slot held. Replacing rather
    /// than adding is what lets a redelivery land without aborting.
    public fun put(slot: u64, payload: vector<u8>) acquires Store {
        if (!exists<Store>(@bench)) return;
        let payloads = &mut borrow_global_mut<Store>(@bench).payloads;
        if (table::contains(payloads, slot)) {
            *table::borrow_mut(payloads, slot) = payload;
        } else {
            table::add(payloads, slot, payload);
        }
    }

    /// Delete a slot if it holds anything. This is the only deletion write in
    /// the package.
    public fun drop_slot(slot: u64) acquires Store {
        if (!exists<Store>(@bench)) return;
        let payloads = &mut borrow_global_mut<Store>(@bench).payloads;
        if (table::contains(payloads, slot)) {
            table::remove(payloads, slot);
        }
    }

    #[view]
    public fun contains(slot: u64): bool acquires Store {
        if (!exists<Store>(@bench)) return false;
        table::contains(&borrow_global<Store>(@bench).payloads, slot)
    }

    #[view]
    /// Bytes held in `slot`, zero when it holds nothing.
    public fun payload_len(slot: u64): u64 acquires Store {
        if (!contains(slot)) return 0;
        vector::length(table::borrow(&borrow_global<Store>(@bench).payloads, slot))
    }
}
