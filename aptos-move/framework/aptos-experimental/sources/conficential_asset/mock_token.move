module aptos_experimental::mock_token {
    use std::option;
    use std::signer;
    use std::string::utf8;
    use aptos_framework::fungible_asset;
    use aptos_framework::fungible_asset::{Metadata, MintRef};
    use aptos_framework::object;
    use aptos_framework::object::Object;
    use aptos_framework::primary_fungible_store;

    struct TokenStore has key {
        mint_ref: MintRef,
    }

    fun init_module(deployer: &signer) {
        let ctor_ref = &object::create_sticky_object(signer::address_of(deployer));

        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            ctor_ref,
            option::none(),
            utf8(b"MockToken"),
            utf8(b"MT"),
            0,
            utf8(b"https://"),
            utf8(b"https://"),
        );

        let mint_ref = fungible_asset::generate_mint_ref(ctor_ref);

        move_to(deployer, TokenStore { mint_ref });
    }

    public entry fun mint_to(to: &signer, amount: u64) acquires TokenStore {
        let store = primary_fungible_store::ensure_primary_store_exists(signer::address_of(to), get_token_metadata());
        let mint_ref = &borrow_global<TokenStore>(@aptos_experimental).mint_ref;

        fungible_asset::mint_to(mint_ref, store, amount);
    }

    #[view]
    public fun get_token_metadata(): Object<Metadata> acquires TokenStore {
        let token_store = borrow_global<TokenStore>(@aptos_experimental);

        fungible_asset::mint_ref_metadata(&token_store.mint_ref)
    }
}
