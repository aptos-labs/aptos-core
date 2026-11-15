/// A minimal coin module exercising declarations, bodies, and specs.
module 0x42::basic_coin {
    // An ordinary comment before the struct.
    /// The coin resource.
    struct Coin has key {
        /// The balance.
        value: u64,
    }

    const E_INSUFFICIENT: u64 = 1; // trailing comment

    /// Withdraws `amount` from `addr`.
    public fun withdraw(addr: address, amount: u64) acquires Coin {
        let balance = &mut Coin[addr].value;
        /* block comment */
        assert!(*balance >= amount, E_INSUFFICIENT);
        *balance = *balance - amount;
    }
    spec withdraw {
        pragma aborts_if_is_partial;
        aborts_if !exists<Coin>(addr);
        aborts_if Coin[addr].value < amount with E_INSUFFICIENT;
        ensures Coin[addr].value == old(Coin[addr].value) - amount;
    }

    public fun balance_of(addr: address): u64 acquires Coin {
        Coin[addr].value
    }
    spec balance_of {
        aborts_if !exists<Coin>(addr);
        ensures result == global<Coin>(addr).value;
    }

    spec fun total(a: address, b: address): num {
        global<Coin>(a).value + global<Coin>(b).value
    }
}
