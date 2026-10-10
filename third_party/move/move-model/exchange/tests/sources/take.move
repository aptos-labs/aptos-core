module 0x42::account {
    struct Account has key { balance: u64 }

    fun take(addr: address): u64 acquires Account {
        let Account { balance } = move_from<Account>(addr);
        balance
    }
    spec take {
        aborts_if !exists<Account>(addr);
        ensures result == old(global<Account>(addr).balance);
        ensures !exists<Account>(addr);
        modifies global<Account>(addr);
    }
}
