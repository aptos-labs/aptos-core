module aptos_framework::sched_txns_sender_seqno {
    /// We need this module outside of scheduled_txns to prevent cyclical dependency issues between
    /// `scheduled_txns module and account module` during `key rotation handling`
    use std::error;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::system_addresses;

    friend aptos_framework::account;
    friend aptos_framework::scheduled_txns;

    /// Sender sequence number not found - must be initialized first via get_sender_seqno
    const ESENDER_SEQNO_NOT_FOUND: u64 = 1;

    /// Invalid signer - only framework can call this
    const EINVALID_SIGNER: u64 = 2;

    /// Stores the sender sequence number mapping
    struct SenderSeqnoData has key {
        /// BigOrderedMap to track sender address -> current sequence number for authorization
        sender_seqno_map: BigOrderedMap<address, u64>
    }

    /// Initialize the sender sequence number map - called from scheduled_txns::initialize
    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);

        move_to(
            framework,
            SenderSeqnoData { sender_seqno_map: big_ordered_map::new_with_reusable() }
        );
    }

    /// Returns the current authorization sequence number for a sender address
    /// Lazy initialization: starts from 1 and stores in map upon first use
    public(friend) fun get_sender_seqno(sender_addr: address): u64 acquires SenderSeqnoData {
        let seqno_data = borrow_global_mut<SenderSeqnoData>(@aptos_framework);
        if (seqno_data.sender_seqno_map.contains(&sender_addr)) {
            *seqno_data.sender_seqno_map.borrow(&sender_addr)
        } else {
            // Lazy initialization: start from 1
            let initial_seqno = 1;
            seqno_data.sender_seqno_map.add(sender_addr, initial_seqno);
            initial_seqno
        }
    }

    /// Returns the current authorization sequence number for a sender address (read-only)
    /// Requires that the sender already exists in sender_seqno_map (initialized via get_sender_seqno)
    public(friend) fun get_sender_seqno_readonly(sender_addr: address): u64 acquires SenderSeqnoData {
        let seqno_data = borrow_global<SenderSeqnoData>(@aptos_framework);
        assert!(
            seqno_data.sender_seqno_map.contains(&sender_addr),
            error::invalid_state(ESENDER_SEQNO_NOT_FOUND)
        );
        *seqno_data.sender_seqno_map.borrow(&sender_addr)
    }

    /// Increments the sequence number for a sender address
    /// Requires that the sender already exists in sender_seqno_map (initialized via get_sender_seqno)
    public(friend) fun increment_sender_seqno(sender_addr: address) acquires SenderSeqnoData {
        let seqno_data = borrow_global_mut<SenderSeqnoData>(@aptos_framework);

        // Assert that sender exists in map - must be initialized first via get_sender_seqno
        assert!(
            seqno_data.sender_seqno_map.contains(&sender_addr),
            error::invalid_state(ESENDER_SEQNO_NOT_FOUND)
        );

        let current_seqno = *seqno_data.sender_seqno_map.borrow(&sender_addr);
        let new_seqno = current_seqno + 1;
        *seqno_data.sender_seqno_map.borrow_mut(&sender_addr) = new_seqno;
    }

    /// Handles key rotation by incrementing the sender sequence number
    /// Only increments if the sender already exists in the sender_seqno_map
    public(friend) fun handle_key_rotation(sender_addr: address) acquires SenderSeqnoData {
        if (contains_sender(sender_addr)) {
            increment_sender_seqno(sender_addr);
        }
        // If sender doesn't exist, do nothing
    }

    public(friend) fun destroy_sender_seqno_map() acquires SenderSeqnoData {
        let SenderSeqnoData { sender_seqno_map } =
            move_from<SenderSeqnoData>(@aptos_framework);
        // Clear all elements from the map before dropping it
        sender_seqno_map.for_each(
            |_key, _value| {
                // Do nothing - just consume the elements
            }
        );
    }

    /// Sets a specific sequence number for a sender (useful for testing or migration)
    public(friend) fun set_sender_seqno(sender_addr: address, seqno: u64) acquires SenderSeqnoData {
        let seqno_data = borrow_global_mut<SenderSeqnoData>(@aptos_framework);
        if (seqno_data.sender_seqno_map.contains(&sender_addr)) {
            *seqno_data.sender_seqno_map.borrow_mut(&sender_addr) = seqno;
        } else {
            seqno_data.sender_seqno_map.add(sender_addr, seqno);
        }
    }

    /// Checks if a sender exists in the sequence number map
    fun contains_sender(sender_addr: address): bool acquires SenderSeqnoData {
        let seqno_data = borrow_global<SenderSeqnoData>(@aptos_framework);
        seqno_data.sender_seqno_map.contains(&sender_addr)
    }
}
