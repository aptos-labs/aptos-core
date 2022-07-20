/// This module provides test tables of various key / value types, for use in API tests
module TestAccount::TableTestData {
    use 0x1::table::{Self, Table};
    use 0x1::string;
    use 0x1::guid::{Self, ID};
    use 0x1::vector;

    struct TestTables has key {
        u8_table: Table<u8, u8>,
        u64_table: Table<u64, u64>,
        u128_table: Table<u128, u128>,
        bool_table: Table<bool, bool>,
        string_table: Table<string::String, string::String>,
        address_table: Table<address, address>,
        vector_u8_table: Table<vector<u8>, vector<u8>>,
        vector_string_table: Table<vector<string::String>, vector<string::String>>,
        id_table: Table<ID, ID>,
        id_table_id: ID,
        table_table: Table<u8, Table<u8, u8>>,
    }

    public entry fun make_test_tables(account: signer) {
        let id = guid::id(&guid::create(&account));
        let str = string::utf8(b"abc");
        let vec_u8 = vector::empty<u8>();
        vector::push_back(&mut vec_u8, 1);
        vector::push_back(&mut vec_u8, 2);
        let vec_str = vector::empty<string::String>();
        vector::push_back(&mut vec_str, str);
        vector::push_back(&mut vec_str, str);
        let table_u8 = table::new();
        table::add(&mut table_u8, 2, 3);

        let test_tables = TestTables {
            u8_table: table::new(),
            u64_table: table::new(),
            u128_table: table::new(),
            bool_table: table::new(),
            string_table: table::new(),
            address_table: table::new(),
            vector_u8_table: table::new(),
            vector_string_table: table::new(),
            id_table: table::new(),
            id_table_id: copy id,
            table_table: table::new(),
        };

        let t = &mut test_tables;

        table::add(&mut t.u8_table, 1, 1);
        table::add(&mut t.u64_table, 1, 1);
        table::add(&mut t.u128_table, 1, 1);
        table::add(&mut t.bool_table, true, true);
        table::add(&mut t.string_table, str, copy str);
        table::add(&mut t.address_table, @0x1, @0x1);
        table::add(&mut t.vector_u8_table, vec_u8, copy vec_u8);
        table::add(&mut t.vector_string_table, vec_str, copy vec_str);
        table::add(&mut t.id_table, id, copy id);
        table::add(&mut t.table_table, 1, table_u8);

        move_to(&account, test_tables);
    }
}
