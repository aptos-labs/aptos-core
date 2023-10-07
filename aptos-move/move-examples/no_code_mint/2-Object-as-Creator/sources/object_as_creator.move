module no_code_mint_p2::object_as_creator {
    use std::string;
    use std::bcs;
    use std::error;
    use std::object::{Self, ExtendRef};
    use std::signer;
    use std::vector;
    use std::option;
    use std::string::String;
    use aptos_token_objects::aptos_token::{Self};
    use aptos_token_objects::collection::{Self, Collection};

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct MintConfig has key {
        extend_ref: ExtendRef, // this is how we generate the Object's `&signer`
        collection_name: String,
        token_description: String,
        token_base_name: String,
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
        let deployer_address = signer::address_of(deployer);
        // ensure that the contract address itself is also the deployer and deployer of this module
        assert!(deployer_address == @no_code_mint_p2, error::permission_denied(ENOT_AUTHORIZED));

        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri = string::utf8(b"Collection uri");
        let token_base_name = string::utf8(b"Token #");
        let token_uri = string::utf8(b"Token uri");
        let maximum_supply = 1000;

        // Create an object that will be the collection creator
        // The object will be owned by the deployer account
        let constructor_ref = object::create_object(deployer_address);
        // generate its &signer to create the collection
        let obj_signer = object::generate_signer(&constructor_ref);

        aptos_token::create_collection(
            &obj_signer, // the object is now the creator of the collection
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

        // generate the ExtendRef with the returned ConstructorRef from the creation function
        let extend_ref = object::generate_extend_ref(&constructor_ref);

        let mint_config = MintConfig {
            extend_ref,
            collection_name,
            token_description: string::utf8(b""),
            token_base_name,
            token_uri,
            property_keys: vector<String>[string::utf8(b"color")],
            property_types: vector<String>[ string::utf8(b"0x1::string::String") ],
            property_values: vector<vector<u8>>[bcs::to_bytes(&string::utf8(b"BLUE"))],
        };

        // Move the MintConfig resource to the contract address itself, since deployer == @no_code_mint_p2
        move_to(deployer, mint_config);
    }

    // Mint a token and transfer it to the account that called this function.
    // Note that this time, we can require that the `receiver` is the signer of the request to mint,
    // since the object is the collection creator, meaning we can automate the minting process.
    public entry fun mint(receiver: &signer) acquires MintConfig {
        // get our contract data at the module address
        let mint_config = borrow_global_mut<MintConfig>(@no_code_mint_p2);

        // borrow the object's ExtendRef and use it to generate the object's &signer
        let extend_ref = &mint_config.extend_ref;
        let obj_signer = object::generate_signer_for_extending(extend_ref);

        let obj_creator_addr = object::address_from_extend_ref(extend_ref);
        let collection_supply = get_collection_supply(obj_creator_addr);
        let full_token_name = concat_u64(mint_config.token_base_name, collection_supply);

        // mint the token as the collection creator object
        let token_object = aptos_token::mint_token_object(
            &obj_signer,
            mint_config.collection_name,
            mint_config.token_description,
            full_token_name,
            mint_config.token_uri,
            mint_config.property_keys,
            mint_config.property_types,
            mint_config.property_values,
        );

        let receiver_address = signer::address_of(receiver);
        object::transfer(&obj_signer, token_object, receiver_address);

        // update "color" to "RED"
        aptos_token::update_property(
            &obj_signer,
            token_object,
            string::utf8(b"color"),
            string::utf8(b"0x1::string::String"),
            bcs::to_bytes(&string::utf8(b"RED")),
        );
    }

    inline fun get_collection_supply(creator_addr: address): u64 {
      option::extract(&mut collection::count(object::address_to_object<Collection>(creator_addr)))
    }

    inline fun u64_to_string(value: u64): String {
        if (value == 0) {
            string::utf8(b"0")
        } else {
            let buffer = vector::empty<u8>();
            while (value != 0) {
                vector::push_back(&mut buffer, ((48 + value % 10) as u8));
                value = value / 10;
            };
            vector::reverse(&mut buffer);
            string::utf8(buffer)
        }
    }

    inline fun concat_u64(s: String, n: u64): String {
        let n_str = u64_to_string(n);
        string::append(&mut s, n_str);
        s
    }
}
