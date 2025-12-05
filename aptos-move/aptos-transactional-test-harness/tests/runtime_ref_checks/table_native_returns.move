//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f
//#      --initial-coins 1000000000

//# publish --private-key Alice
module Alice::table_native_returns {
    use std::signer;
    use aptos_std::table;

    struct Holder has key {
        table: table::Table<u64, u64>,
    }

    fun borrow_read_internal(addr: address, key: u64): u64 acquires Holder {
        let table_ref = &borrow_global<Holder>(addr).table;

        let value = {
            let entry_ref = table::borrow(table_ref, key);
            *entry_ref
        };

        value
    }

    fun borrow_mut_update_internal(addr: address, key: u64, delta: u64): u64 acquires Holder {
        let updated_value = {
            let holder = borrow_global_mut<Holder>(addr);
            let table_ref = &mut holder.table;
            let entry_ref = table::borrow_mut(table_ref, key);
            *entry_ref = *entry_ref + delta;
            *entry_ref
        };

        updated_value
    }

    public entry fun init(account: signer, key: u64, value: u64) {
        let addr = signer::address_of(&account);
        assert!(!exists<Holder>(addr), 0);

        let values = table::new<u64, u64>();
        table::add(&mut values, key, value);

        move_to(&account, Holder { table: values });
    }

    public entry fun borrow_read_expect(account: signer, key: u64, expected: u64) acquires Holder {
        let addr = signer::address_of(&account);
        let value = borrow_read_internal(addr, key);
        assert!(value == expected, expected);
    }

    public entry fun borrow_mut_update_expect(
        account: signer,
        key: u64,
        delta: u64,
        expected: u64,
    ) acquires Holder {
        let addr = signer::address_of(&account);
        let updated = borrow_mut_update_internal(addr, key, delta);
        assert!(updated == expected, expected);
    }
}

// Run sequence: initialize, borrow immutably, then mutably update and verify results

//# run --signers Alice --args 0 42 -- 0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6::table_native_returns::init

//# run --signers Alice --args 0 42 -- 0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6::table_native_returns::borrow_read_expect

//# run --signers Alice --args 0 1 43 -- 0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6::table_native_returns::borrow_mut_update_expect
