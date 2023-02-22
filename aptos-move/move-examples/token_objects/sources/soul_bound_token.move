module token_objects::soul_bound_token {
    use std::string::String;
    use token_objects::token::{MutabilityConfig, Token};
    use std::signer;
    use aptos_framework::object::{generate_transfer_ref, Object, disable_ungated_transfer, generate_linear_transfer_ref, object_from_constructor_ref, transfer};
    use token_objects::token;
    use aptos_framework::object;
    use std::string;
    use token_objects::collection;
    use std::option;

    struct OnChainConfig has key {
        collection: String,
        mutability_config: MutabilityConfig,
    }

    fun init_module(account: &signer) {
        let collection = string::utf8(b"Mixed Tokens");
        collection::create_aggregable_collection(
            account,
            string::utf8(b"collection description"),
            collection::create_mutability_config(false, false),
            *&collection,
            option::none(),
            string::utf8(b"collection uri"),
        );

        let on_chain_config = OnChainConfig {
            collection: string::utf8(b"Souler"),
            mutability_config: token::create_mutability_config(true, true, true),
        };
        move_to(account, on_chain_config);
    }

    public fun mint_soul_bound_token(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
        to: address,
    ): Object<Token> acquires OnChainConfig {
        let on_chain_config = borrow_global<OnChainConfig>(signer::address_of(creator));
        let root_ref = token::create_token(
            creator,
            on_chain_config.collection,
            description,
            on_chain_config.mutability_config,
            name,
            option::none(),
            uri,
        );
        let transfer_ref = generate_transfer_ref(&root_ref);
        disable_ungated_transfer(&transfer_ref);
        let linear_transfer_ref = generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, to);
        object_from_constructor_ref<Token>(&root_ref)
    }

    public fun mint_normal_token(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
        to: address,
    ):Object<Token> acquires OnChainConfig {
        let on_chain_config = borrow_global<OnChainConfig>(signer::address_of(creator));
        let root_ref = token::create_token(
            creator,
            on_chain_config.collection,
            description,
            on_chain_config.mutability_config,
            name,
            option::none(),
            uri,
        );
        transfer(creator, object_from_constructor_ref<Token>(&root_ref), to);
        object_from_constructor_ref<Token>(&root_ref)
    }


    #[test(creator = @0x123, account1 = @0x456, account2 = @0x789)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::object)]
    fun test_soul_bound(creator: &signer, account1: &signer, account2: &signer) acquires OnChainConfig {
        init_module(creator);

        let sbt = mint_soul_bound_token(
            creator,
            string::utf8(b"Soul bound token for test"),
            string::utf8(b"Aptos hackathon Tokyo"),
            string::utf8(b"ipfs:://aptos/tokyo"),
            signer::address_of(account1)
        );

        let t = mint_normal_token(
            creator,
            string::utf8(b"Normal token for test"),
            string::utf8(b"Aptos hackathon HQ"),
            string::utf8(b"ipfs:://aptos/hq"),
            signer::address_of(account2)
        );

        assert!(object::is_owner(sbt, signer::address_of(account1)), 0);
        assert!(object::is_owner(t, signer::address_of(account2)), 1);
        transfer(account2, t, signer::address_of(account1));
        assert!(object::is_owner(t, signer::address_of(account1)), 2);
        transfer(account1, t, signer::address_of(account2));
        assert!(object::is_owner(t, signer::address_of(account2)), 3);
        transfer(account1, sbt, signer::address_of(account2));
    }
}
