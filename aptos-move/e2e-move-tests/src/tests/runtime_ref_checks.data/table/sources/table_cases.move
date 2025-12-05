module runtime_ref_table::cases {
    use std::signer;
    use aptos_std::table;

    struct Holder has key {
        table: table::Table<u64, u64>,
    }

    public entry fun init(account: signer) {
        let addr = signer::address_of(&account);
        assert!(!exists<Holder>(addr), 0);

        let table_value = table::new<u64, u64>();
        table::add(&mut table_value, 0, 41);
        table::add(&mut table_value, 1, 7);

        move_to(&account, Holder { table: table_value });
    }

    public entry fun borrow_read(account: signer, key: u64, expected: u64) acquires Holder {
        let addr = signer::address_of(&account);
        let holder = &borrow_global<Holder>(addr).table;

        let value = {
            let entry_ref = table::borrow(holder, key);
            *entry_ref
        };

        assert!(value == expected, 0);
    }

    public entry fun borrow_with_default(account: signer, key: u64, default: u64, expected: u64) acquires Holder {
        let addr = signer::address_of(&account);
        let holder = &borrow_global<Holder>(addr).table;

        let borrowed = table::borrow_with_default(holder, key, &default);
        assert!(*borrowed == expected, 0);
    }

    public entry fun borrow_mut_update(account: signer, key: u64, delta: u64, expected: u64) acquires Holder {
        let addr = signer::address_of(&account);
        let holder = borrow_global_mut<Holder>(addr);
        let table_ref = &mut holder.table;

        let new_value = {
            let entry_ref = table::borrow_mut(table_ref, key);
            *entry_ref = *entry_ref + delta;
            *entry_ref
        };
        assert!(new_value == expected, 0);
    }

    public entry fun upsert_value(account: signer, key: u64, value: u64) acquires Holder {
        let addr = signer::address_of(&account);
        let holder = borrow_global_mut<Holder>(addr);
        let table_ref = &mut holder.table;

        table::upsert(table_ref, key, value);
    }

}
