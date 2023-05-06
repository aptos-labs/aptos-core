// :!:>resource
module test_account::cli_args {
    use std::signer;
    use aptos_std::type_info::{Self, TypeInfo};


    struct Holder has key, drop {
        u8_solo: u8,
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
        bool_vec: vector<bool>,
        address_vec_vec: vector<vector<address>>,
    ) acquires Holder {
        let account_addr = signer::address_of(&account);
        if (exists<Holder>(account_addr)) {
            move_from<Holder>(account_addr);
        };
        move_to(&account, Holder {
            u8_solo,
            bool_vec,
            address_vec_vec,
            type_info_1: type_info::type_of<T1>(),
            type_info_2: type_info::type_of<T2>(),
        });
    } //<:!:setter

    // :!:>view
    #[view]
    /// Reveal first three fields in host's `Holder`, as well as two
    /// `bool` flags denoting if `T1` and `T2` respectively match
    /// `Holder.type_info_1` and `Holder.type_info_2`.
    public fun reveal<T1, T2>(host: address): (
        u8,
        vector<bool>,
        vector<vector<address>>,
        bool,
        bool
    ) acquires Holder {
        let holder_ref = borrow_global<Holder>(host);
        (holder_ref.u8_solo,
         holder_ref.bool_vec,
         holder_ref.address_vec_vec,
         type_info::type_of<T1>() == holder_ref.type_info_1,
         type_info::type_of<T2>() == holder_ref.type_info_2)
    }

} //<:!:view
