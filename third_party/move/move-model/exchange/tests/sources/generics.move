module 0x42::generics {
    struct Box<T: copy + drop + store> has copy, drop, store { value: T }
    struct Marker<phantom T> has copy, drop, store { id: u64 }
    struct Vault<phantom T> has key { value: u64 }
    enum Choice<T: copy + drop> has copy, drop { None, Some(T) }

    fun identity<T: copy + drop>(value: T): T {
        value
    }

    fun choose<T: copy + drop>(left: T, right: T, use_left: bool): T {
        if (use_left) left else right
    }

    fun round_trip(value: u64): u64 {
        let boxed = Box<u64> { value: identity<u64>(value) };
        let Box { value } = boxed;
        choose<u64>(value, 0, true)
    }

    fun marker(id: u64): Marker<vector<u64>> {
        Marker<vector<u64>> { id }
    }

    fun inspect_choice(value: u64): u64 {
        let choice = Choice::Some<u64>(value);
        match (choice) {
            Choice::None => 0,
            Choice::Some(inner) => inner,
        }
    }

    fun publish<T>(account: &signer, value: u64) {
        move_to(account, Vault<T> { value })
    }

    fun read<T>(addr: address): u64 acquires Vault {
        borrow_global<Vault<T>>(addr).value
    }

    fun replace<T>(addr: address, value: u64) acquires Vault {
        borrow_global_mut<Vault<T>>(addr).value = value
    }

    fun take<T>(addr: address): u64 acquires Vault {
        let Vault { value } = move_from<Vault<T>>(addr);
        value
    }

    fun contains<T>(addr: address): bool {
        exists<Vault<T>>(addr)
    }
}
