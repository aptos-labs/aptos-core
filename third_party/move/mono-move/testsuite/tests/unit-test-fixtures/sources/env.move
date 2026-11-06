/// Values and effects that depend on the natives' environment.
module fixtures::env {
    use aptos_framework::event;
    use aptos_framework::transaction_context;
    use aptos_std::table::{Self, Table};

    struct Counter has key {
        value: u64,
    }

    #[event]
    struct Bumped has drop, store {
        value: u64,
    }

    #[test]
    fun transaction_hash(): vector<u8> {
        transaction_context::get_transaction_hash()
    }

    #[test]
    fun auids(): vector<address> {
        vector[
            transaction_context::generate_auid_address(),
            transaction_context::generate_auid_address(),
        ]
    }

    #[test]
    fun table_with_item(): Table<u64, u64> {
        let table = table::new();
        table.add(1, 2);
        table
    }

    #[test(account = @0xcafe)]
    fun resource_and_event(account: &signer) acquires Counter {
        move_to(account, Counter { value: 1 });
        let counter = &mut Counter[@0xcafe];
        counter.value += 1;
        event::emit(Bumped { value: counter.value });
    }
}
