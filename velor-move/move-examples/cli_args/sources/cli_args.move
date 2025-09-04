// :!:>resource
module test_account::cli_args {
    use std::signer;
    use velor_std::type_info::{Self, TypeInfo};
    use std::string::String;

    struct Holder has key, drop {
        u8_solo: u8,
        bytes: vector<u8>,
        utf8_string: String,
        bool_vec: vector<bool>,
        address_vec_vec: vector<vector<address>>,
        type_info_1: TypeInfo,
        type_info_2: TypeInfo,
    } //<:!:resource


    // :!:>setter
    /// Set values in a `Holder` under `account`.
    public entry fun set_vals<T1, T2>(
        account: signer,
        u8_solo: u8,
        bytes: vector<u8>,
        utf8_string: String,
        bool_vec: vector<bool>,
        address_vec_vec: vector<vector<address>>,
    ) acquires Holder {
        let account_addr = signer::address_of(&account);
        if (exists<Holder>(account_addr)) {
            move_from<Holder>(account_addr);
        };
        move_to(&account, Holder {
            u8_solo,
            bytes,
            utf8_string,
            bool_vec,
            address_vec_vec,
            type_info_1: type_info::type_of<T1>(),
            type_info_2: type_info::type_of<T2>(),
        });
    } //<:!:setter

    // :!:>view
    struct RevealResult has drop {
        u8_solo: u8,
        bytes: vector<u8>,
        utf8_string: String,
        bool_vec: vector<bool>,
        address_vec_vec: vector<vector<address>>,
        type_info_1_match: bool,
        type_info_2_match: bool
    }

    #[view]
    /// Pack into a `RevealResult` the first three fields in host's
    /// `Holder`, as well as two `bool` flags denoting if `T1` & `T2`
    /// respectively match `Holder.type_info_1` & `Holder.type_info_2`,
    /// then return the `RevealResult`.
    public fun reveal<T1, T2>(host: address): RevealResult acquires Holder {
        let holder_ref = borrow_global<Holder>(host);
        RevealResult {
            u8_solo: holder_ref.u8_solo,
            bytes: holder_ref.bytes,
            utf8_string: holder_ref.utf8_string,
            bool_vec: holder_ref.bool_vec,
            address_vec_vec: holder_ref.address_vec_vec,
            type_info_1_match:
                type_info::type_of<T1>() == holder_ref.type_info_1,
            type_info_2_match:
                type_info::type_of<T2>() == holder_ref.type_info_2
        }
    }

} //<:!:view
