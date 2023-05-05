module deploy_address::cli_args {
    use std::signer;

    struct Holder has key {
        u8_solo: u8,
        bool_vec: vector<bool>,
        address_vec_vec: vector<vector<address>>,
    }


    #[view]
    public fun reveal(host: address): (u8, vector<bool>, vector<vector<address>>) acquires Holder {
        let holder_ref = borrow_global<Holder>(host);
        (holder_ref.u8_solo, holder_ref.bool_vec, holder_ref.address_vec_vec)
    }

    public entry fun set_vals(
        account: signer,
        u8_solo: u8,
        bool_vec: vector<bool>,
        address_vec_vec: vector<vector<address>>,
    ) acquires Holder {
        let account_addr = signer::address_of(&account);
        if (!exists<Holder>(account_addr)) {
            move_to(&account, Holder {
                u8_solo,
                bool_vec,
                address_vec_vec,
            })
        } else {
            let old_holder = borrow_global_mut<Holder>(account_addr);
            old_holder.u8_solo = u8_solo;
            old_holder.bool_vec = bool_vec;
            old_holder.address_vec_vec = address_vec_vec;
        }
    }
}
