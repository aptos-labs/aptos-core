module account::nested_values {
    use aptos_std::table::{Self, Table};

    /// A struct holding a function value.
    struct Wrapper has store, drop {
        function: |u64| u64 has store + drop,
    }

    struct Holder has key {
        items: Table<u64, vector<u8>>,
    }

    /// A public function taking a struct that holds a function value.
    public fun take(_wrapper: Wrapper) {}

    entry fun store(account: &signer, bytes: vector<u8>) {
        let items = table::new();
        items.add(0, bytes);
        move_to(account, Holder { items });
    }
}
