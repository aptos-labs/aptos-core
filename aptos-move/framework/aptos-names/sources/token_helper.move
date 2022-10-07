module aptos_names::token_helper {
    friend aptos_names::domains;

    use aptos_framework::timestamp;
    use aptos_names::config;
    use aptos_names::utf8_utils;
    use aptos_token::token::{Self, TokenDataId, TokenId};
    use aptos_token::property_map;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use aptos_framework::account::{Self, SignerCapability};

    const DOMAIN_SUFFIX: vector<u8> = b".apt";

    /// The collection does not exist. This should never happen.
    const ECOLLECTION_NOT_EXISTS: u64 = 1;


    /// Tokens require a signer to create, so this is the signer for the collection
    struct CollectionCapabilityV1 has key, drop {
        capability: SignerCapability,
    }

    public fun get_token_signer_address(): address acquires CollectionCapabilityV1 {
        account::get_signer_capability_address(&borrow_global<CollectionCapabilityV1>(@aptos_names).capability)
    }

    fun get_token_signer(): signer acquires CollectionCapabilityV1 {
        account::create_signer_with_capability(&borrow_global<CollectionCapabilityV1>(@aptos_names).capability)
    }

    /// In the event of requiring operations via script, this allows root to get the registry signer
    public fun break_token_registry_glass(sign: &signer): signer acquires CollectionCapabilityV1 {
        config::assert_signer_is_admin(sign);
        get_token_signer()
    }

    public(friend) fun initialize(framework: &signer) {
        // Create the resource account for token creation, so we can get it as a signer later
        let registry_seed = utf8_utils::u128_to_string((timestamp::now_microseconds() as u128));
        string::append(&mut registry_seed, string::utf8(b"registry_seed"));
        let (token_resource, token_signer_cap) = account::create_resource_account(framework, *string::bytes(&registry_seed));

        move_to(framework, CollectionCapabilityV1 {
            capability: token_signer_cap,
        });

        // Set up NFT collection
        let description = string::utf8(b".apt names from the Aptos Foundation");
        let collection_uri = string::utf8(b"https://aptosnames.com");
        // This turns off supply tracking, which allows for parallel execution
        let maximum_supply = 0;
        // collection description mutable: true
        // collection URI mutable: true
        // collection max mutable: false
        let mutate_setting = vector<bool>[ true, true, false ];
        token::create_collection(&token_resource, config::collection_name_v1(), description, collection_uri, maximum_supply, mutate_setting);
    }

    public fun get_fully_qualified_domain_name(subdomain_name: Option<String>, domain_name: String): String {
        let combined = combine_sub_and_domain_str(subdomain_name, domain_name);
        string::append_utf8(&mut combined, DOMAIN_SUFFIX);
        combined
    }

    public fun tokendata_exists(token_data_id: &TokenDataId): bool {
        let (creator, collection_name, token_name) = token::get_token_data_id_fields(token_data_id);
        token::check_tokendata_exists(creator, collection_name, token_name)
    }

    public fun build_tokendata_id(token_resource_address: address, subdomain_name: Option<String>, domain_name: String): TokenDataId {
        let collection_name = config::collection_name_v1();
        let fq_domain_name = get_fully_qualified_domain_name(subdomain_name, domain_name);
        token::create_token_data_id(token_resource_address, collection_name, fq_domain_name)
    }

    public fun latest_token_id(token_data_id: &TokenDataId): TokenId {
        let (creator, _collection_name, _token_name) = token::get_token_data_id_fields(token_data_id);
        let largest_tokendata_property_version = token::get_tokendata_largest_property_version(creator, *token_data_id);
        token::create_token_id(*token_data_id, largest_tokendata_property_version)
    }

    /// Combines a subdomain and domain into a new string, separated by a `.`
    /// Used for building fully qualified domain names (Ex: `{subdomain_name}.{domain_name}.apt`)
    /// If there is no subdomain, just returns the domain name
    public fun combine_sub_and_domain_str(subdomain_name: Option<String>, domain_name: String): String {
        if (option::is_none(&subdomain_name)) {
            return domain_name
        };

        let combined = option::extract(&mut copy subdomain_name);
        string::append_utf8(&mut combined, b".");
        string::append(&mut combined, domain_name);
        combined
    }

    /// gets or creates the token data for the given domain name
    public(friend) fun ensure_token_data(subdomain_name: Option<String>, domain_name: String, type: String): TokenDataId acquires CollectionCapabilityV1 {
        let token_resource = &get_token_signer();

        let token_data_id = build_tokendata_id(signer::address_of(token_resource), subdomain_name, domain_name);
        if (tokendata_exists(&token_data_id)) {
            token_data_id
        } else {
            create_token_data(token_resource, subdomain_name, domain_name, type)
        }
    }

    fun create_token_data(token_resource: &signer, subdomain_name: Option<String>, domain_name: String, type: String): TokenDataId {
        // Set up the NFT
        let collection_name = config::collection_name_v1();
        assert!(token::check_collection_exists(signer::address_of(token_resource), collection_name), ECOLLECTION_NOT_EXISTS);

        let fq_domain_name = get_fully_qualified_domain_name(subdomain_name, domain_name);

        let nft_maximum: u64 = 0;
        let description = config::tokendata_description();
        let token_uri: string::String = config::tokendata_url_prefix();
        string::append(&mut token_uri, fq_domain_name);
        let royalty_payee_address: address = @aptos_names;
        let royalty_points_denominator: u64 = 0;
        let royalty_points_numerator: u64 = 0;
        // tokan max mutable: false
        // token URI mutable: true
        // token description mutable: true
        // token royalty mutable: false
        // token properties mutable: true
        let token_mutate_config = token::create_token_mutability_config(&vector<bool>[ false, true, true, false, true ]);

        let type = property_map::create_property_value(&type);
        let now = property_map::create_property_value(&timestamp::now_seconds());
        let property_keys: vector<String> = vector[config::config_key_creation_time_sec(), config::config_key_type()];
        let property_values: vector<vector<u8>> = vector[property_map::borrow_value(&now), property_map::borrow_value(&type)];
        let property_types: vector<String> = vector[property_map::borrow_type(&now), property_map::borrow_type(&type)];


        token::create_tokendata(
            token_resource,
            collection_name,
            fq_domain_name,
            description,
            nft_maximum,
            token_uri,
            royalty_payee_address,
            royalty_points_denominator,
            royalty_points_numerator,
            token_mutate_config,
            property_keys,
            property_values,
            property_types
        )
    }

    public(friend) fun create_token(tokendata_id: TokenDataId): TokenId acquires CollectionCapabilityV1 {
        let token_resource = get_token_signer();

        // At this point, property_version is 0
        let (_creator, collection_name, _name) = token::get_token_data_id_fields(&tokendata_id);
        assert!(token::check_collection_exists(signer::address_of(&token_resource), collection_name), 125);

        token::mint_token(&token_resource, tokendata_id, 1)
    }

    public(friend) fun set_token_props(token_owner: address, property_keys: vector<String>, property_values: vector<vector<u8>>, property_types: vector<String>, token_id: TokenId): TokenId acquires CollectionCapabilityV1 {
        let token_resource = get_token_signer();

        // At this point, property_version is 0
        // This will create a _new_ token with property_version == max_property_version of the tokendata, and with the properties we just set
        token::mutate_one_token(
            &token_resource,
            token_owner,
            token_id,
            property_keys,
            property_values,
            property_types
        )
    }

    public(friend) fun transfer_token_to(sign: &signer, token_id: TokenId) acquires CollectionCapabilityV1 {
        token::initialize_token_store(sign);
        token::opt_in_direct_transfer(sign, true);

        let token_resource = get_token_signer();
        token::transfer(&token_resource, token_id, signer::address_of(sign), 1);
    }

    #[test]
    fun test_get_fully_qualified_domain_name() {
        assert!(get_fully_qualified_domain_name(option::none(), string::utf8(b"test")) == string::utf8(b"test.apt"), 1);
        assert!(get_fully_qualified_domain_name(option::none(), string::utf8(b"wow_this_is_long")) == string::utf8(b"wow_this_is_long.apt"), 2);
        assert!(get_fully_qualified_domain_name(option::none(), string::utf8(b"123")) == string::utf8(b"123.apt"), 2);
        assert!(get_fully_qualified_domain_name(option::some(string::utf8(b"sub")), string::utf8(b"test")) == string::utf8(b"sub.test.apt"), 2);
    }

    #[test]
    fun test_combine_sub_and_domain_str() {
        let subdomain_name = string::utf8(b"sub");
        let domain_name = string::utf8(b"dom");
        let combined = combine_sub_and_domain_str(option::some(subdomain_name), domain_name);
        assert!(combined == string::utf8(b"sub.dom"), 1);
    }

    #[test]
    fun test_combine_sub_and_domain_str_dom_only() {
        let domain_name = string::utf8(b"dom");
        let combined = combine_sub_and_domain_str(option::none(), domain_name);
        assert!(combined == string::utf8(b"dom"), 1);
    }
}
