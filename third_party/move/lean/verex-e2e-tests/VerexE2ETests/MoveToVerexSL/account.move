/// Balances under an address, with entry functions and contracts.
module 0x42::account {
    struct BalanceValue has copy, drop, store {
        value: u64,
    }

    /// The balance resource.
    struct Balance has key {
        balance: BalanceValue,
    }

    const E_INSUFFICIENT_BALANCE: u64 = 1;

    public entry fun deposit(addr: address, amount: u64) acquires Balance {
        let value = &mut Balance[addr].balance.value;
        *value = *value + amount;
    }
    spec deposit {
        requires exists<Balance>(addr);
        modifies global<Balance>(addr);
        ensures Balance[addr].balance.value == old(Balance[addr].balance.value) + amount;
        aborts_if Balance[addr].balance.value + amount > MAX_U64;
    }

    public entry fun withdraw(addr: address, amount: u64) acquires Balance {
        let value = &mut Balance[addr].balance.value;
        let current = *value;
        if (current < amount) {
            abort E_INSUFFICIENT_BALANCE
        };
        *value = *value - amount;
    }
    spec withdraw {
        requires exists<Balance>(addr);
        ensures Balance[addr].balance.value == old(Balance[addr].balance.value) - amount;
        aborts_if Balance[addr].balance.value < amount with E_INSUFFICIENT_BALANCE;
    }

    public entry fun publish(account: &signer, amount: u64) {
        move_to(account, Balance { balance: BalanceValue { value: amount } });
    }

    public fun is_published(addr: address): bool {
        exists<Balance>(addr)
    }

    public fun remove(addr: address): u64 acquires Balance {
        let Balance { balance } = move_from<Balance>(addr);
        balance.value
    }

    public fun balance_of(addr: address): u64 acquires Balance {
        Balance[addr].balance.value
    }
    spec balance_of {
        aborts_if !exists<Balance>(addr);
        ensures result == global<Balance>(addr).balance.value;
    }
}
