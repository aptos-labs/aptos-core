module aptos_framework::coin_example {
    use aptos_framework::object;
    use std::string::String;
    use aptos_framework::object::{generate_extend_ref, Object, disable_ungated_transfer, generate_transfer_ref};
    use fungible_asset::managed_fungible_asset::{initialize_managing_capabilities, mint_by_asset_owner, freeze_by_asset_owner, burn_by_asset_owner};
    use std::signer::address_of;
    use fungible_asset::fungible_asset;
    use std::error;
    use aptos_std::smart_table;
    use aptos_std::smart_table::SmartTable;
    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::account::create_account_for_test;

    struct Coin has key {
        name: String,
        /// Symbol of the coin, usually a shorter version of the name.
        /// For example, Singapore Dollar is SGD.
        symbol: String,
        /// Number of decimals used to get its user representation.
        /// For example, if `decimals` equals `2`, a balance of `505` coins should
        /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
        decimals: u8
    }

    struct CoinIndexer has key {
        index: smart_table::SmartTable<String, Object<Coin>>
    }

    public entry fun create_coin(creator: &signer, name: String, symbol: String, decimals: u8) acquires CoinIndexer {
        let creator_ref = object::create_object_from_account(creator);
        let object_signer = object::generate_signer(&creator_ref);
        disable_ungated_transfer(&generate_transfer_ref(&creator_ref));
        move_to(&object_signer, Coin {
            name,
            symbol,
            decimals
        });

        initialize_managing_capabilities(&generate_extend_ref(&creator_ref), 0 /* no max supply */);
        let coin_obj = object::object_from_constructor_ref<Coin>(&creator_ref);
        if (!exists<CoinIndexer>(address_of(creator))) {
            move_to(creator, CoinIndexer {
                index: smart_table::new()
            })
        };

        let index = borrow_coin_index_mut(address_of(creator));
        assert!(!smart_table::contains(index, name), 0);
        smart_table::add(index, name, coin_obj);
    }

    inline fun borrow_coin_index_mut(owner: address): &mut SmartTable<String, Object<Coin>> acquires CoinIndexer {
        assert!(exists<CoinIndexer>(owner), 0);
        &mut borrow_global_mut<CoinIndexer>(owner).index
    }

    inline fun get_coin_object(owner: address, name: String): Object<Coin> acquires CoinIndexer {
        let index = borrow_coin_index_mut(owner);
        assert!(smart_table::contains(index, name), 0);
        *smart_table::borrow(index, name)
    }

    public entry fun mint(owner: &signer, name: String, amount: u64, to: address) acquires CoinIndexer {
        let coin_obj = get_coin_object(address_of(owner), name);
        verify(&coin_obj);
        mint_by_asset_owner(owner, &coin_obj, amount, to);
    }

    public entry fun burn(owner: &signer, name: String, amount: u64, from: address) acquires CoinIndexer {
        let coin_obj = get_coin_object(address_of(owner), name);
        verify(&coin_obj);
        burn_by_asset_owner(owner, &coin_obj, amount, from);
    }

    public entry fun transfer(
        account: &signer,
        owner: address,
        name: String,
        amount: u64,
        to: address
    ) acquires CoinIndexer {
        let coin_obj = get_coin_object(owner, name);
        verify(&coin_obj);
        fungible_asset::transfer(account, &coin_obj, amount, to);
    }

    #[view]
    public fun is_account_frozen(owner: address, name: String, account: address): bool acquires CoinIndexer {
        let coin_obj = get_coin_object(owner, name);
        verify(&coin_obj);
        fungible_asset::is_frozen(account, &coin_obj)
    }

    #[view]
    public fun balance(owner: address, name: String, account: address): u64 acquires CoinIndexer {
        let coin_obj = get_coin_object(owner, name);
        verify(&coin_obj);
        fungible_asset::balance(account, &coin_obj)
    }

    public entry fun freeze_account(owner: &signer, name: String, account_to_freeze: address) acquires CoinIndexer {
        let coin_obj = get_coin_object(address_of(owner), name);
        verify(&coin_obj);
        freeze_by_asset_owner(owner, &coin_obj, account_to_freeze);
    }

    public entry fun unfreeze_account(owner: &signer, name: String, account_to_freeze: address) acquires CoinIndexer {
        let coin_obj = get_coin_object(address_of(owner), name);
        verify(&coin_obj);
        freeze_by_asset_owner(owner, &coin_obj, account_to_freeze);
    }

    inline fun verify(coin_obj: &Object<Coin>): address {
        let coin_address = object::object_address(coin_obj);
        assert!(
            exists<Coin>(coin_address),
            error::not_found(1),
        );
        coin_address
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    entry fun e2e_test(creator: &signer, aaron: &signer) acquires CoinIndexer {
        let usda = string::utf8(b"USDA");
        let creator_address = address_of(creator);
        create_account_for_test(creator_address);
        let aaron_address = address_of(aaron);
        create_coin(creator, usda, string::utf8(b"$"), 2);
        mint(creator, usda, 1000, aaron_address);
        transfer(aaron, creator_address, usda, 800, creator_address);
        burn(creator, usda, 100, aaron_address);
        assert!(balance(creator_address, usda, aaron_address) == 100, 1);
    }
}
