module inscriptions::immutable_collection {
    use std::option;
    use std::string::String;

    use aptos_framework::object::{Self, Object};

    use aptos_token_objects::collection::{Self, Collection};
    use aptos_token_objects::royalty::{Self, Royalty};
    use aptos_token_objects::token::{Self, Token};

    use inscriptions::inscriptions;

    public entry fun create_collection(
        creator: &signer,
        description: String,
        max_supply: u64,
        name: String,
        royalty_numerator: u64,
        royalty_denominator: u64,
        royalty_payee_address: address,
        uri: String,
    ) {
        let royalty = royalty::create(
            royalty_numerator,
            royalty_denominator,
            royalty_payee_address,
        );

        create_collection_object(
            creator,
            description,
            max_supply,
            name,
            royalty,
            uri,
        );
    }

    public fun create_collection_object(
        creator: &signer,
        description: String,
        max_supply: u64,
        name: String,
        royalty: Royalty,
        uri: String,
    ): Object<Collection> {
        let constructor_ref = collection::create_fixed_collection(
            creator,
            description,
            max_supply,
            name,
            option::some(royalty),
            uri,
        );

        object::object_from_constructor_ref(&constructor_ref)
    }

    public entry fun mint_token(
        creator: &signer,
        collection_name: String,
        data: vector<u8>,
        description: String,
        name: String,
        uri: String,
    ) {
        mint_token_object(
            creator,
            collection_name,
            data,
            description,
            name,
            uri,
        );
    }

    public entry fun mint_token_and_transfer(
        creator: &signer,
        collection_name: String,
        data: vector<u8>,
        description: String,
        name: String,
        uri: String,
        recipient: address,
    ) {
        let token = mint_token_object(
            creator,
            collection_name,
            data,
            description,
            name,
            uri,
        );

        object::transfer(creator, token, recipient);
    }

    public fun mint_token_object(
        creator: &signer,
        collection_name: String,
        data: vector<u8>,
        description: String,
        name: String,
        uri: String,
    ): Object<Token> {
        let constructor_ref = token::create(
            creator,
            collection_name,
            description,
            name,
            option::none(),
            uri,
        );

        let _inscription_id = inscriptions::add_inscription(&constructor_ref, data);

        object::object_from_constructor_ref(&constructor_ref)
    }

    #[test_only]
    use std::string;

    #[test(creator = @0x123, deployer = @inscriptions)]
    fun end_to_end_test(creator: &signer, deployer: &signer) {
        inscriptions::init_for_test(deployer);
        let collection = string::utf8(b"collection");

        create_collection(
            creator,
            string::utf8(b""),
            5,
            collection,
            0,
            1,
            @0x123,
            string::utf8(b""),
        );

        let data = b"00000000";
        let token_obj = mint_token_object(
            creator,
            collection,
            data,
            string::utf8(b""),
            string::utf8(b""),
            string::utf8(b""),
        );

        assert!(inscriptions::is_inscription(token_obj), 0);
        assert!(inscriptions::inscription_id(token_obj) == 0, 0);
    }
}
