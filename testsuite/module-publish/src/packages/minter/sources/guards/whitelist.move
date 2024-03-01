module token_minter::whitelist {

    use std::error;
    use std::vector;
    use aptos_std::smart_table;
    use aptos_std::smart_table::SmartTable;
    use aptos_framework::object;
    use aptos_framework::object::Object;

    friend token_minter::token_minter;

    /// Whitelist address arguments do not match in length.
    const EWHITELIST_ARGUMENT_MISMATCH: u64 = 1;
    /// User attempting to mint is not whitelisted.
    const EUSER_NOT_WHITELISTED: u64 = 2;
    /// Whitelist object does not exist at the given address.
    const EWHITELIST_DOES_NOT_EXIST: u64 = 3;
    /// Insufficient mint amount remaining.
    const EINSUFFICIENT_MINT_AMOUNT_REMAINING: u64 = 4;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Whitelist has key {
        minters: SmartTable<address, u64>,
    }

    public(friend) fun add_or_update_whitelist<T: key>(
        token_minter_signer: &signer,
        token_minter: Object<T>,
        whitelisted_addresses: vector<address>,
        max_mints_per_whitelist: vector<u64>,
    ) acquires Whitelist {
        let whitelist_length = vector::length(&whitelisted_addresses);
        assert!(
            whitelist_length == vector::length(&max_mints_per_whitelist),
            error::invalid_argument(EWHITELIST_ARGUMENT_MISMATCH),
        );

        if (!is_whitelist_enabled(token_minter)) {
            move_to(token_minter_signer, Whitelist { minters: smart_table::new() });
        };

        let whitelist = borrow_mut<T>(token_minter);
        let i = 0;
        while (i < whitelist_length) {
            smart_table::upsert(
                &mut whitelist.minters,
                *vector::borrow(&whitelisted_addresses, i),
                *vector::borrow(&max_mints_per_whitelist, i),
            );
            i = i + 1;
        };
    }

    public(friend) fun remove_whitelist<T: key>(token_minter: Object<T>) acquires Whitelist {
        let whitelist_address = whitelist_address(token_minter);
        let Whitelist { minters } = move_from<Whitelist>(whitelist_address);
        smart_table::destroy(minters);
    }

    public(friend) fun execute<T: key>(
        token_minter: Object<T>,
        amount: u64,
        to: address,
    ) acquires Whitelist {
        let remaining_amount = allowance_mut(token_minter, to);
        assert!(*remaining_amount >= amount, error::invalid_state(EINSUFFICIENT_MINT_AMOUNT_REMAINING));

        *remaining_amount = *remaining_amount - amount;
    }

    inline fun allowance_mut<T: key>(token_minter: Object<T>, addr: address): &mut u64 acquires Whitelist {
        let whitelist = borrow_mut<T>(token_minter);
        assert!(smart_table::contains(&whitelist.minters, addr), error::not_found(EUSER_NOT_WHITELISTED));

        smart_table::borrow_mut(&mut whitelist.minters, addr)
    }

    inline fun borrow<T: key>(token_minter: Object<T>): &mut Whitelist acquires Whitelist {
        borrow_global_mut<Whitelist>(whitelist_address(token_minter))
    }

    inline fun borrow_mut<T: key>(token_minter: Object<T>): &mut Whitelist acquires Whitelist {
        borrow_global_mut<Whitelist>(whitelist_address(token_minter))
    }

    fun whitelist_address<T: key>(token_minter: Object<T>): address {
        let whitelist_address = object::object_address(&token_minter);
        assert!(is_whitelist_enabled(token_minter), error::not_found(EWHITELIST_DOES_NOT_EXIST));

        whitelist_address
    }

    // ================================== View functions ================================== //

    #[view]
    public fun is_whitelist_enabled<T: key>(token_minter: Object<T>): bool {
        exists<Whitelist>(object::object_address(&token_minter))
    }

    #[view]
    public fun allowance<T: key>(token_minter: Object<T>, addr: address): u64 acquires Whitelist {
        *allowance_mut(token_minter, addr)
    }
}
