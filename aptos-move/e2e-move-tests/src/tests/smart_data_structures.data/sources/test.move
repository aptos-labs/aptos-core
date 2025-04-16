module publisher::test {
    use aptos_std::smart_vector::{Self, SmartVector};
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::table::{Self, Table};
    use std::vector;
    use std::signer;

    struct SmartVectorStore has key {
        v: SmartVector<u64>
    }

    struct VectorStore has key {
        v: vector<u64>
    }

    struct SmartTableStore has key {
        t: SmartTable<u64, u64>
    }

    struct TableStore has key {
        t: Table<u64, u64>
    }

    public entry fun create_smart_vector(acct: &signer) {
        let v = smart_vector::empty();
        let i: u64 = 0;
        while (i < 5000) {
            smart_vector::push_back(&mut v, i);
            i = i + 1;
        };
        move_to(acct, SmartVectorStore { v });
    }

    public entry fun update_smart_vector(acct: &signer) acquires SmartVectorStore {
        let v = &mut borrow_global_mut<SmartVectorStore>(signer::address_of(acct)).v;
        smart_vector::push_back(v, 5000);
    }

    public entry fun read_smart_vector(acct: &signer) acquires SmartVectorStore {
        let v = &borrow_global<SmartVectorStore>(signer::address_of(acct)).v;
        smart_vector::borrow(v, 2000);
    }

    public entry fun create_vector(acct: &signer) {
        let v = vector::empty();
        let i: u64 = 0;
        while (i < 5000) {
            vector::push_back(&mut v, i);
            i = i + 1;
        };
        move_to(acct, VectorStore { v });
    }

    public entry fun update_vector(acct: &signer) acquires VectorStore {
        let v = &mut borrow_global_mut<VectorStore>(signer::address_of(acct)).v;
        vector::push_back(v, 5000);
    }

    public entry fun read_vector(acct: &signer) acquires VectorStore {
        let v = &borrow_global<VectorStore>(signer::address_of(acct)).v;
        vector::borrow(v, 2000);
    }

    public entry fun create_smart_table(acct: &signer) {
        let t = smart_table::new();
        let i: u64 = 0;
        while (i < 1000) {
            smart_table::add(&mut t, i, i);
            i = i + 1;
        };
        move_to(acct, SmartTableStore { t });
    }

    public entry fun update_smart_table(acct: &signer) acquires SmartTableStore {
        let t = &mut borrow_global_mut<SmartTableStore>(signer::address_of(acct)).t;
        smart_table::add(t, 1001, 1001);
    }

    public entry fun read_smart_table(acct: &signer) acquires SmartTableStore {
        let t = &borrow_global<SmartTableStore>(signer::address_of(acct)).t;
        smart_table::borrow(t, 500);
    }

    public entry fun create_table(acct: &signer) {
        let t = table::new();
        let i: u64 = 0;
        while (i < 1000) {
            table::add(&mut t, i, i);
            i = i + 1;
        };
        move_to(acct, TableStore { t });
    }

    public entry fun update_table(acct: &signer) acquires TableStore {
        let t = &mut borrow_global_mut<TableStore>(signer::address_of(acct)).t;
        table::add(t, 1001, 1001);
    }

    public entry fun read_table(acct: &signer) acquires TableStore {
        let t = &borrow_global<TableStore>(signer::address_of(acct)).t;
        table::borrow(t, 500);
    }
}
