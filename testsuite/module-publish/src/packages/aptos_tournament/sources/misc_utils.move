module tournament::misc_utils {
    use std::bcs;
    use std::string::{Self, String};
    use std::vector;
    use aptos_std::string_utils;
    use aptos_framework::timestamp;
    use aptos_framework::transaction_context;

    const NAME_SEPARATORS_DELIMITER: vector<u8> = b"::";

    public inline fun join_strings(strings: vector<String>, delimiter: String): String {
        let s = string::utf8(b"");
        let num_strings = vector::length(&strings);
        vector::enumerate_ref(&strings, |i, s1| {
            string::append(&mut s, *s1);
            if (i < num_strings - 1) {
                string::append(&mut s, delimiter);
            };
        });
        s
    }

    public inline fun concat_any_to_string<T: copy + drop>(s: String, any_value: &T): String {
        let any_value_as_string = string_utils::to_string<T>(any_value);
        string::append(&mut s, any_value_as_string);

        s
    }

    public inline fun concat_any_to_any<T1: copy + drop, T2: copy + drop>(s1: &T1, any_value: &T2): String {
        let any_value_as_string = string_utils::to_string<T2>(any_value);
        let s = string_utils::to_string<T1>(s1);
        string::append(&mut s, any_value_as_string);

        s
    }

    public inline fun split_fully_qualified_struct(struct_path: String): (String, String, String) {
        let sep = string::utf8(b"::");
        let first_index = string::index_of(&struct_path, &sep);

        let address = string::sub_string(&struct_path, 0, first_index);
        let struct_path = string::sub_string(&struct_path, first_index + 2, string::length(&struct_path));

        let second_index = string::index_of(&struct_path, &sep);
        let module_name = string::sub_string(&struct_path, 0, second_index);

        let struct_name = string::sub_string(&struct_path, second_index + 2, string::length(&struct_path));

        (address, module_name, struct_name)
    }


    /// pseudo-rng
    /// [min, max], inclusive
    public fun rand_range(min: u64, max: u64): u64 {
        let range = (max - min) + 1;
        let now = timestamp::now_microseconds();
        // this is to facilitate "randomness" in test without having to increment the timestamp
        let address_bytes = bcs::to_bytes(&transaction_context::generate_auid_address());
        let last_byte = (vector::pop_back(&mut address_bytes) as u64);
        let rand = (((now + last_byte) % range) as u64);
        rand + min
    }

    #[test]
    fun test_split_fully_qualified_struct() {
        let (a, b, c) = split_fully_qualified_struct(string::utf8(b"0x1::tournament::Tournament"));
        assert!(a == string::utf8(b"0x1"), 0);
        assert!(b == string::utf8(b"tournament"), 0);
        assert!(c == string::utf8(b"Tournament"), 0);
    }


    #[test]
    fun test_string_concats() {
        assert!(
            join_strings(vector<String>[
                string::utf8(b"hello"),
                string::utf8(b"world"),
            ], string::utf8(b" ")) == string::utf8(b"hello world"),
            0,
        );
        assert!(concat_any_to_string(string::utf8(b"hello"), &42) == string::utf8(b"hello42"), 0);
        assert!(concat_any_to_any(&string::utf8(b"hello"), &42) == string::utf8(b"\"hello\"42"), 0);
        assert!(
            concat_any_to_any(&string::utf8(b"hello"), &string::utf8(b"world")) == string::utf8(b"\"hello\"\"world\""),
            0
        );
        assert!(concat_any_to_any(&42, &string::utf8(b"world")) == string::utf8(b"42\"world\""), 0);
        assert!(concat_any_to_any(&42, &42) == string::utf8(b"4242"), 0);
    }


}