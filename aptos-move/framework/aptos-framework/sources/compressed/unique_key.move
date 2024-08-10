
module aptos_framework::unique_key {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_framework::transaction_context;

    /// The index is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 1;

    /// Unauthorized
    const EUNAUTHORIZED: u64 = 1;

    friend aptos_framework::compressed_state;

    struct Counters has key {
        values: TableWithLength<u16, u64>,
    }

    entry fun initialize_at_address(owner: &signer) {
        assert!(signer::address_of(owner) == @aptos_framework, EUNAUTHORIZED);

        let counter = Counters {
            values: table_with_length::new(),
        };

        let i = 0;
        while (i < 256) {
            table_with_length::add(&mut counter.values, i, 0);
            i = i + 1;
        };

        move_to(owner, counter);
    }

    public(friend) fun generate_unique_key(counter_addr: address): u64 acquires Counters {
        let index = (*vector::borrow(&transaction_context::get_transaction_hash(), 0) as u16);
        assert!(index < 256, error::invalid_argument(EINDEX_OUT_OF_BOUNDS));

        let counters = borrow_global_mut<Counters>(counter_addr);
        let value = table_with_length::borrow_mut(&mut counters.values, index);
        *value = *value + 1;

        *value * 256 + (index as u64)
    }
}
