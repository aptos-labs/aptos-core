module no_code_mint_p1::create_nft {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::object;
    use std::string::{Self, String};
    use aptos_token_objects::aptos_token::{Self};

    // This struct stores all the relevant NFT collection and token's mint_config
    struct MintConfig has key {
        collection_name: String,
        creator: address,
        token_description: String,
        token_name: String,
        token_uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
    }

    /// Action not authorized because the signer is not the admin of this module
    const ENOT_AUTHORIZED: u64 = 1;

    /// `init_module` is automatically called when publishing the module.
    /// In this function, we create an example NFT collection and an example token.
    fun init_module(deployer: &signer) {
        // ensure that the contract address itself is also the deployer and admin of this module
        assert!(deployer_address == @no_code_mint_p1, error::permission_denied(ENOT_AUTHORIZED));

        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri = string::utf8(b"Collection uri");
        let token_name = string::utf8(b"Token name");
        let token_uri = string::utf8(b"Token uri");
        let maximum_supply = 1000;

        aptos_token::create_collection(
            deployer,
            description,
            maximum_supply,
            collection_name,
            collection_uri,
            false, // mutable_description
            false, // mutable_royalty
            false, // mutable_uri
            false, // mutable_token_description
            false, // mutable_token_name
            true, // mutable_token_properties
            false, // mutable_token_uri
            false, // tokens_burnable_by_creator
            false, // tokens_freezable_by_creator
            5, // royalty_numerator
            100, // royalty_denominator
        );

        let deployer_address = signer::address_of(deployer);

        let mint_config = MintConfig {
            collection_name,
            creator: deployer_address,
            token_description: string::utf8(b""),
            token_name,
            token_uri,
            property_keys: vector<String>[string::utf8(b"color")],
            property_types: vector<String>[ string::utf8(b"0x1::string::String") ],
            property_values: vector<vector<u8>>[bcs::to_bytes(&string::utf8(b"BLUE"))],
        };

        // Move the MintConfig resource to the contract address itself, since deployer == @no_code_mint_p1
        move_to(deployer, mint_config);
    }

    /// Mint an NFT to the receiver. Note that we don't need the receiver to sign to receive a token/object,
    /// you only need to pass the `receiver_address` to the entry function.
    public entry fun mint_to(
        deployer: &signer,
        receiver_address: address
    ) acquires MintConfig {
        let deployer_address = signer::address_of(deployer);
        assert!(deployer_address == @no_code_mint_p1, error::permission_denied(ENOT_AUTHORIZED));

        // borrow the MintConfig resource and store it locally as mint_config
        let mint_config = borrow_global_mut<MintConfig>(@no_code_mint_p1);

        // mint the token object
        let token_object = aptos_token::mint_token_object(
            deployer,
            mint_config.collection_name,
            mint_config.token_description,
            mint_config.token_name,
            mint_config.token_uri,
            mint_config.property_keys,
            mint_config.property_types,
            mint_config.property_values,
        );
        object::transfer(deployer, token_object, receiver_address);

        // update "color" to "RED"
        aptos_token::update_property(
            deployer,
            token_object,
            string::utf8(b"color"),
            string::utf8(b"0x1::string::String"),
            bcs::to_bytes(&string::utf8(b"RED")),
        );
    }
}
