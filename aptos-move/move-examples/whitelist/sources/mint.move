module whitelist_example::mint {
    use std::error;
    use std::signer;
    use std::object;
    use std::option;
    use std::string::{Self, String};
    use std::string_utils;
    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::resource_account;
    use aptos_token_objects::aptos_token::{Self, AptosToken};
    use aptos_token_objects::collection::{Self, Collection};
    use whitelist_example::whitelist::{Self};

    struct MintConfiguration has key {
        signer_capability: SignerCapability,
    }

    /// Action not authorized because the signer is not the owner of this module
    const ENOT_AUTHORIZED: u64 = 1;

    const COLLECTION_NAME: vector<u8> = b"Krazy Kangaroos";
    const COLLECTION_DESCRIPTION: vector<u8> = b"A bunch of krazy kangaroos.";
    const COLLECTION_URI: vector<u8> = b"https://krazykangaroos.io/collection-image-of-a-kangaroo";
    const TOKEN_URI: vector<u8> = b"https://krazykangaroos.io/token-image-of-a-kangaroo";
    const TOKEN_DESCRIPTION: vector<u8> = b"A single krazy kangaroo.";
    const BASE_TOKEN_NAME: vector<u8> = b"Krazy Kangaroo";
    const MAXIMUM_SUPPLY: u64 = 1000;
    const MUTABLE_COLLECTION_DESCRIPTION: bool = false;
    const MUTABLE_ROYALTY: bool = false;
    const MUTABLE_URI: bool = false;
    const MUTABLE_TOKEN_DESCRIPTION: bool = false;
    const MUTABLE_TOKEN_NAME: bool = false;
    const MUTABLE_TOKEN_PROPERTIES: bool = true;
    const MUTABLE_TOKEN_URI: bool = false;
    const TOKENS_BURNABLE_BY_CREATOR: bool = false;
    const TOKENS_FREEZABLE_BY_CREATOR: bool = false;
    const ROYALTY_NUMERATOR: u64 = 5;
    const ROYALTY_DENOMINATOR: u64 = 100;
    const U64_MAX: u64 = 18446744073709551615;
    const WHITELIST_PRICE: u64 = 0;
    const WHITELIST_START_TIME: u64 = 0;
    const WHITELIST_END_TIME: u64 = 18446744073709551615;
    const WHITELIST_PER_USER_LIMIT: u64 = 1;
    const PUBLIC_PRICE: u64 = 1;
    const PUBLIC_START_TIME: u64 = 0;
    const PUBLIC_END_TIME: u64 = 18446744073709551615;
    const PUBLIC_PER_USER_LIMIT: u64 = 2;

    fun init_module(resource_signer: &signer) {
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @owner);
        move_to(resource_signer, MintConfiguration {
            signer_capability: resource_signer_cap,
        });
    }

    public entry fun initialize_collection(
        owner: &signer,
    ) acquires MintConfiguration {
        assert!(signer::address_of(owner) == @owner, error::permission_denied(ENOT_AUTHORIZED));

        let mint_configuration = borrow_global<MintConfiguration>(@whitelist_example);
        let resource_signer = &account::create_signer_with_capability(&mint_configuration.signer_capability);

        aptos_token::create_collection(
            resource_signer,
            string::utf8(COLLECTION_DESCRIPTION),
            MAXIMUM_SUPPLY,
            string::utf8(COLLECTION_NAME),
            string::utf8(COLLECTION_URI),
            MUTABLE_COLLECTION_DESCRIPTION,
            MUTABLE_ROYALTY,
            MUTABLE_URI,
            MUTABLE_TOKEN_DESCRIPTION,
            MUTABLE_TOKEN_NAME,
            MUTABLE_TOKEN_PROPERTIES,
            MUTABLE_TOKEN_URI,
            TOKENS_BURNABLE_BY_CREATOR,
            TOKENS_FREEZABLE_BY_CREATOR,
            ROYALTY_NUMERATOR,
            ROYALTY_DENOMINATOR,
        );

        whitelist::init_tiers(resource_signer);

        whitelist::upsert_tier_config(
            resource_signer,
            string::utf8(b"public"),
            true, // open_to_public, users don't need to be registered in the list
            PUBLIC_PRICE,
            PUBLIC_START_TIME,
            PUBLIC_END_TIME,
            PUBLIC_PER_USER_LIMIT,
        );

        whitelist::upsert_tier_config(
            resource_signer,
            string::utf8(b"whitelist"),
            false, // open_to_public, users need to be registered in the whitelist
            WHITELIST_PRICE,
            WHITELIST_START_TIME,
            WHITELIST_END_TIME,
            WHITELIST_PER_USER_LIMIT,
        );
    }

    /// simple mint function to demonstrate how to call the whitelist functions
    public entry fun mint(receiver: &signer, tier_name: String) acquires MintConfiguration {
        // access the configuration resources stored on-chain at @whitelist_example's address
        let mint_configuration = borrow_global<MintConfiguration>(@whitelist_example);
        let signer_cap = &mint_configuration.signer_capability;
        let resource_signer: &signer = &account::create_signer_with_capability(signer_cap);

        whitelist::deduct_one_from_tier(receiver, resource_signer, tier_name);

        // store next GUID to derive object address later
        let token_creation_num = account::get_guid_next_creation_num(@whitelist_example);

        // generate next token name based on current collection supply
        let token_name = next_token_name_from_supply(
            signer::address_of(resource_signer),
            string::utf8(BASE_TOKEN_NAME),
            string::utf8(COLLECTION_NAME),
        );

        // mint token and send it to the receiver
        aptos_token::mint(
            resource_signer,
            string::utf8(COLLECTION_NAME),
            string::utf8(TOKEN_DESCRIPTION),
            token_name,
            string::utf8(TOKEN_URI),
            vector<String> [ ],
            vector<String> [ ],
            vector<vector<u8>> [ ],
        );
        let token_object = object::address_to_object<AptosToken>(object::create_guid_object_address(@whitelist_example, token_creation_num));
        object::transfer(resource_signer, token_object, signer::address_of(receiver));
    }

    /// generates the next token name by concatenating the supply onto the base token name
    fun next_token_name_from_supply(
        creator_address: address,
        base_token_name: String,
        collection_name: String,
    ): String {
        let collection_addr = collection::create_collection_address(&creator_address, &collection_name);
        let collection_object = object::address_to_object<Collection>(collection_addr);
        let current_supply = option::borrow(&collection::count(collection_object));
        let format_string = base_token_name;
        // if base_token_name == Token Name
        string::append_utf8(&mut format_string, b" #{}");
        // 'Token Name #1' when supply == 0
        string_utils::format1(string::bytes(&format_string), *current_supply + 1)
    }

    public entry fun add_addresses_to_tier(
        owner: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires MintConfiguration {
        assert!(signer::address_of(owner) == @owner, error::permission_denied(ENOT_AUTHORIZED));
        let mint_configuration = borrow_global_mut<MintConfiguration>(@whitelist_example);
        let resource_signer = &account::create_signer_with_capability(&mint_configuration.signer_capability);
        whitelist::add_addresses_to_tier(resource_signer, tier_name, addresses);
    }

    public entry fun remove_addresses_from_tier(
        owner: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires MintConfiguration {
        assert!(signer::address_of(owner) == @owner, error::permission_denied(ENOT_AUTHORIZED));
        let mint_configuration = borrow_global_mut<MintConfiguration>(@whitelist_example);
        let resource_signer = &account::create_signer_with_capability(&mint_configuration.signer_capability);
        whitelist::remove_addresses_from_tier(resource_signer, tier_name, addresses);
    }

    // dependencies only used in test, if we link without #[test_only], the compiler will warn us
    #[test_only]
    use std::coin::{Self, MintCapability};
    #[test_only]
    use std::vector;
    #[test_only]
    use std::aptos_coin::{AptosCoin};
    #[test_only]
    use std::timestamp;

    #[test_only]
    public fun setup_account<CoinType>(
        acc: &signer,
        num_coins: u64,
        mint: &MintCapability<CoinType>,
    ) {
        let addr = signer::address_of(acc);
        account::create_account_for_test(addr);
        coin::register<CoinType>(acc);
        coin::deposit<CoinType>(addr, coin::mint<CoinType>(num_coins, mint));
    }

    #[test_only]
    public fun setup_test(
        owner: &signer,
        resource_account: &signer,
        nft_receiver: &signer,
        nft_receiver2: &signer,
        aptos_framework: &signer,
        timestamp: u64,
    ) acquires MintConfiguration {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);
        let (burn, mint) = aptos_framework::aptos_coin::initialize_for_test(aptos_framework);

        account::create_account_for_test(signer::address_of(owner));
        resource_account::create_resource_account(owner, vector::empty<u8>(), vector::empty<u8>());
        init_module(resource_account);

        setup_account<AptosCoin>(nft_receiver, 2, &mint);
        setup_account<AptosCoin>(nft_receiver2, 2, &mint);
        coin::destroy_burn_cap(burn);
        coin::destroy_mint_cap(mint);

        initialize_collection(owner);
    }

    // The whitelist.move unit tests are more important for this Move example, but we display a happy path test here to convey the intended flow.
    #[test(owner = @owner, resource_account = @whitelist_example, nft_receiver = @0xFB, nft_receiver2 = @0xFC, aptos_framework = @0x1)]
    public fun test_happy_path(
        owner: &signer,
        resource_account: &signer,
        nft_receiver: &signer,
        nft_receiver2: &signer,
        aptos_framework: &signer,
    ) acquires MintConfiguration {
        setup_test(owner, resource_account, nft_receiver, nft_receiver2, aptos_framework, 1000000000);
        let collection_object_addr = collection::create_collection_address(&@whitelist_example, &string::utf8(COLLECTION_NAME));
        let collection_object = object::address_to_object<Collection>(collection_object_addr);

        assert!(collection::creator(collection_object) == @whitelist_example, 1);
        assert!(object::owner(collection_object) == @whitelist_example, 2);
        assert!(collection::name(collection_object) == string::utf8(COLLECTION_NAME), 3);
        assert!(collection::uri(collection_object) == string::utf8(COLLECTION_URI), 4);

        let nft_receiver_addr = signer::address_of(nft_receiver);
        let nft_receiver2_addr = signer::address_of(nft_receiver2);

        // add both accounts to whitelist, then remove nft_receiver2
        add_addresses_to_tier(owner, string::utf8(b"whitelist"), vector<address> [nft_receiver_addr, nft_receiver2_addr]);
        remove_addresses_from_tier(owner, string::utf8(b"whitelist"), vector<address> [nft_receiver2_addr]);

        // mint one token to nft_receiver through the whitelist
        let token_creation_num = account::get_guid_next_creation_num(@whitelist_example);
        mint(nft_receiver, string::utf8(b"whitelist"));
        let token_object_addr = object::create_guid_object_address(@whitelist_example, token_creation_num);
        let token_object = object::address_to_object<AptosToken>(token_object_addr);

        // mint one token to nft_receiver2 through the public list
        let token_creation_num2 = account::get_guid_next_creation_num(@whitelist_example);
        mint(nft_receiver2, string::utf8(b"public"));
        let token_object_addr2 = object::create_guid_object_address(@whitelist_example, token_creation_num2);
        let token_object2 = object::address_to_object<AptosToken>(token_object_addr2);

        mint(nft_receiver, string::utf8(b"public"));
        mint(nft_receiver, string::utf8(b"public"));

        mint(nft_receiver2, string::utf8(b"public"));

        assert!(object::owner(token_object) == nft_receiver_addr, 5);
        assert!(object::owner(token_object2) == nft_receiver2_addr, 6);
        assert!(coin::balance<AptosCoin>(nft_receiver_addr) == 0, 7);
        assert!(coin::balance<AptosCoin>(nft_receiver2_addr) == 0, 8);
        assert!(coin::balance<AptosCoin>(@whitelist_example) == 4, 9);
    }
}
