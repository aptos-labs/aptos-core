module 0x42::generics {
    struct Pair<T, U> has copy, drop, store {
        first: T,
        second: U,
    }

    struct Vault<T: store> has key {
        value: T,
    }

    fun swap<T: copy + drop, U: copy + drop>(value: Pair<T, U>): Pair<U, T> {
        Pair { first: value.second, second: value.first }
    }
    spec swap {
        ensures result.first == value.second;
        ensures result.second == value.first;
    }

    fun publish_generic<T: store>(account: &signer, value: T) {
        move_to(account, Vault { value })
    }

    fun has_vault<T: store>(addr: address): bool {
        exists<Vault<T>>(addr)
    }

    fun swapped(value: u64): Pair<u64, u64> {
        swap(Pair { first: value, second: value + 1 })
    }
}
