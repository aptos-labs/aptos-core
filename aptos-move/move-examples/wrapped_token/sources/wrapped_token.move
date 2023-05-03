/// This provides an example for how to wrap a Token V1 in a Token V2
module wrapped_token::wrapped_token {

    use std::string::String;
    use std::signer;
    use aptos_token_objects::token::{create_from_account, generate_burn_ref, generate_mutator_ref};
    use std::option;
    use aptos_framework::object;
    use std::option::Option;
    use aptos_framework::object::{generate_extend_ref, generate_transfer_ref, ExtendRef, transfer, object_from_constructor_ref};
    use aptos_token_objects::royalty;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Storage state for managing the no-code Token.
    struct WrappedToken has key {
        /// Token to wrap
        token: 0x3::token::Token,
        /// Used to burn.
        burn_ref: Option<0x4::token::BurnRef>,
        /// Used to control freeze.
        transfer_ref: Option<object::TransferRef>,
        /// Used to mutate fields
        mutator_ref: Option<0x4::token::MutatorRef>,
        /// Used to mutate fields
        extend_ref: Option<ExtendRef>,
    }

    /// Create a collection based on the previous collection
    ///
    /// TODO: what should we do with the original collection?
    public entry fun create_wrapped_collection(
        new_creator: &signer,
        creator_address: address,
        name: String,
    ) {
        // TODO: Relax later?
        assert!(signer::address_of(new_creator) == creator_address, 12345);
        let description = aptos_token::token::get_collection_description(creator_address, name);
        let uri = aptos_token::token::get_collection_uri(creator_address, name);
        let max_supply = aptos_token::token::get_collection_maximum(creator_address, name);

        // For now, allow mutable everything, TODO: Make this configurable / use previous state
        0x4::aptos_token::create_collection(
            new_creator,
            description,
            max_supply,
            name,
            uri,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            0,
            1
        );
    }

    /// This allows you to wrap based on the creator, though for consistency, you'd want the same collection and creator
    public fun wrap(
        creator: &signer,
        receiver: address,
        collection: String,
        name: String,
    ) {
        let creator_address = signer::address_of(creator);
        let data_id = aptos_token::token::create_token_data_id(creator_address, collection, name);
        let token_id = aptos_token::token::create_token_id(data_id, 0);
        // Load token TODO: handle fungible?
        let token = aptos_token::token::withdraw_token(creator, token_id, 1);

        let description = aptos_token::token::get_tokendata_description(data_id);
        let uri = aptos_token::token::get_tokendata_uri(creator_address, data_id);
        let royalty = aptos_token::token::get_tokendata_royalty(data_id);
        let numerator = aptos_token::token::get_royalty_numerator(&royalty);
        let denominator = aptos_token::token::get_royalty_denominator(&royalty);
        let payee = aptos_token::token::get_royalty_payee(&royalty);
        let royalty = royalty::create(numerator, denominator, payee);

        let token_constructor = create_from_account(
            creator,
            collection,
            description,
            name,
            option::none(),
            uri
        );
        royalty::init(&token_constructor, royalty);

        let signer = object::generate_signer(&token_constructor);
        let burn_ref = generate_burn_ref(&token_constructor);
        let mutator_ref = generate_mutator_ref(&token_constructor);
        let extend_ref = generate_extend_ref(&token_constructor);
        let transfer_ref = generate_transfer_ref(&token_constructor);
        let token = WrappedToken {
            token,
            burn_ref: option::some(burn_ref),
            transfer_ref: option::some(transfer_ref),
            mutator_ref: option::some(mutator_ref),
            extend_ref: option::some(extend_ref)
        };
        move_to(&signer, token);
        let obj = object_from_constructor_ref<WrappedToken>(&token_constructor);
        transfer(creator, obj, receiver);
    }
}