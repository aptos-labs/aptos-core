/*
Provides a singleton wrapper around PropertyMap to allow for easy and dynamic configurability of contract options.
This includes things like the maximum number of years that a name can be registered for, etc.

Anyone can read, but only admins can write, as all write methods are gated via permissions checks
*/

module aptos_names::config {
    friend aptos_names::domains;

    use aptos_framework::account;
    use aptos_framework::aptos_account;
    use aptos_names::utf8_utils;
    use aptos_token::property_map::{Self, PropertyMap};
    use std::error;
    use std::signer;
    use std::string::{Self, String};

    const CONFIG_KEY_ENABLED: vector<u8> = b"enabled";
    const CONFIG_KEY_ADMIN_ADDRESS: vector<u8> = b"admin_address";
    const CONFIG_KEY_FOUNDATION_FUND_ADDRESS: vector<u8> = b"foundation_fund_address";
    const CONFIG_KEY_TYPE: vector<u8> = b"type";
    const CONFIG_KEY_CREATION_TIME_SEC: vector<u8> = b"creation_time_sec";
    const CONFIG_KEY_EXPIRATION_TIME_SEC: vector<u8> = b"expiration_time_sec";
    const CONFIG_KEY_MAX_NUMBER_OF_YEARS_REGISTERED: vector<u8> = b"max_number_of_years_registered";
    const CONFIG_KEY_MAX_DOMAIN_LENGTH: vector<u8> = b"max_domain_length";
    const CONFIG_KEY_TOKENDATA_DESCRIPTION: vector<u8> = b"tokendata_description";
    const CONFIG_KEY_TOKENDATA_URL_PREFIX: vector<u8> = b"tokendata_url_prefix";
    const CONFIG_KEY_DOMAIN_PRICE_PREFIX: vector<u8> = b"domain_price_";
    const CONFIG_KEY_SUBDOMAIN_PRICE: vector<u8> = b"subdomain_price";

    const DOMAIN_TYPE: vector<u8> = b"domain";
    const SUBDOMAIN_TYPE: vector<u8> = b"subdomain";

    const COLLECTION_NAME_V1: vector<u8> = b"Aptos Names V1";

    /// Raised if the signer is not authorized to perform an action
    const ENOT_AUTHORIZED: u64 = 1;
    /// Raised if there is an invalid value for a configuration
    const EINVALID_VALUE: u64 = 2;

    struct ConfigurationV1 has key, store {
        config: PropertyMap,
    }

    public(friend) fun initialize_v1(framework: &signer, admin_address: address, foundation_fund_address: address) acquires ConfigurationV1 {
        move_to(framework, ConfigurationV1 {
            config: property_map::empty(),
        });

        // Temporarily set this to framework to allow othet methods below to be set with framework signer
        set_v1(@aptos_names, config_key_admin_address(), &signer::address_of(framework));

        set_is_enabled(framework, true);

        set_max_number_of_years_registered(framework, 2u8);
        set_max_domain_length(framework, 63);

        // TODO: SET THIS TO SOMETHING REAL
        set_tokendata_description(framework, string::utf8(b"This is an official Aptos Foundation Name Service Name"));
        set_tokendata_url_prefix(framework, string::utf8(b"https://aptosnames.com/api/v1/metadata/"));

        // TODO: SET REAL VALUES FOR DOMAIN PRICES
        // 0.2 APT
        set_subdomain_price(framework, octas() / 5);
        set_domain_price_for_length(framework, (100 * octas()), 2);
        set_domain_price_for_length(framework, (60 * octas()), 3);
        set_domain_price_for_length(framework, (30 * octas()), 4);
        set_domain_price_for_length(framework, (15 * octas()), 5);
        set_domain_price_for_length(framework, (5 * octas()), 6);

        // We set it directly here to allow boostrapping the other values
        set_v1(@aptos_names, config_key_foundation_fund_address(), &foundation_fund_address);
        set_v1(@aptos_names, config_key_admin_address(), &admin_address);
    }


    //
    // Configuration Shortcuts
    //

    public fun octas(): u64 {
        100000000
    }

    public fun is_enabled(): bool acquires ConfigurationV1 {
        read_bool_v1(@aptos_names, &config_key_enabled())
    }

    public fun foundation_fund_address(): address acquires ConfigurationV1 {
        read_address_v1(@aptos_names, &config_key_foundation_fund_address())
    }

    public fun admin_address(): address acquires ConfigurationV1 {
        read_address_v1(@aptos_names, &config_key_admin_address())
    }

    public fun max_number_of_years_registered(): u8 acquires ConfigurationV1 {
        read_u8_v1(@aptos_names, &config_key_max_number_of_years_registered())
    }

    public fun max_domain_length(): u64 acquires ConfigurationV1 {
        read_u64_v1(@aptos_names, &config_key_max_domain_length())
    }

    /// Admins will be able to intervene when necessary.
    /// The account will be used to manage names that are being used in a way that is harmful to others.
    /// Alternatively, the on-chain governance can be used to get the 0x4 signer, and perform admin actions.
    public fun signer_is_admin(sign: &signer): bool acquires ConfigurationV1 {
        signer::address_of(sign) == admin_address() || signer::address_of(sign) == @aptos_names
    }

    public fun assert_signer_is_admin(sign: &signer) acquires ConfigurationV1 {
        assert!(signer_is_admin(sign), error::permission_denied(ENOT_AUTHORIZED));
    }

    public fun tokendata_description(): String acquires ConfigurationV1 {
        read_string_v1(@aptos_names, &config_key_tokendata_description())
    }

    public fun tokendata_url_prefix(): String acquires ConfigurationV1 {
        read_string_v1(@aptos_names, &config_key_tokendata_url_prefix())
    }

    public fun domain_type(): String {
        return string::utf8(DOMAIN_TYPE)
    }

    public fun subdomain_type(): String {
        return string::utf8(SUBDOMAIN_TYPE)
    }

    public fun collection_name_v1(): String {
        return string::utf8(COLLECTION_NAME_V1)
    }

    public fun domain_price_for_length(domain_length: u64): u64 acquires ConfigurationV1 {
        read_u64_v1(@aptos_names, &config_key_domain_price(domain_length))
    }

    public fun subdomain_price(): u64 acquires ConfigurationV1 {
        read_u64_v1(@aptos_names, &config_key_subdomain_price())
    }


    //
    // Setters
    //

    public entry fun set_is_enabled(sign: &signer, enabled: bool) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        set_v1(@aptos_names, config_key_enabled(), &enabled)
    }

    public entry fun set_foundation_fund_address(sign: &signer, addr: address) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        aptos_account::assert_account_is_registered_for_apt(addr);

        set_v1(@aptos_names, config_key_foundation_fund_address(), &addr)
    }

    public entry fun set_admin_address(sign: &signer, addr: address) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        assert!(account::exists_at(addr), error::invalid_argument(EINVALID_VALUE));
        set_v1(@aptos_names, config_key_admin_address(), &addr)
    }

    public entry fun set_max_number_of_years_registered(sign: &signer, max_years_registered: u8) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        assert!(max_years_registered > 0, error::invalid_argument(EINVALID_VALUE));
        set_v1(@aptos_names, config_key_max_number_of_years_registered(), &max_years_registered)
    }

    public entry fun set_max_domain_length(sign: &signer, domain_length: u64) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        assert!(domain_length > 0, error::invalid_argument(EINVALID_VALUE));
        set_v1(@aptos_names, config_key_max_domain_length(), &domain_length)
    }

    public entry fun set_tokendata_description(sign: &signer, description: String) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        set_v1(@aptos_names, config_key_tokendata_description(), &description)
    }

    public entry fun set_tokendata_url_prefix(sign: &signer, description: String) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        set_v1(@aptos_names, config_key_tokendata_url_prefix(), &description)
    }

    public entry fun set_subdomain_price(sign: &signer, price: u64) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        set_v1(@aptos_names, config_key_subdomain_price(), &price)
    }

    public entry fun set_domain_price_for_length(sign: &signer, price: u64, length: u64) acquires ConfigurationV1 {
        assert_signer_is_admin(sign);
        assert!(price > 0, error::invalid_argument(EINVALID_VALUE));
        assert!(length > 0, error::invalid_argument(EINVALID_VALUE));
        set_v1(@aptos_names, config_key_domain_price(length), &price)
    }


    //
    // Configuration Methods
    //

    public fun config_key_enabled(): String {
        string::utf8(CONFIG_KEY_ENABLED)
    }

    public fun config_key_admin_address(): String {
        string::utf8(CONFIG_KEY_ADMIN_ADDRESS)
    }

    public fun config_key_foundation_fund_address(): String {
        string::utf8(CONFIG_KEY_FOUNDATION_FUND_ADDRESS)
    }

    public fun config_key_type(): String {
        string::utf8(CONFIG_KEY_TYPE)
    }

    public fun config_key_creation_time_sec(): String {
        string::utf8(CONFIG_KEY_CREATION_TIME_SEC)
    }

    public fun config_key_expiration_time_sec(): String {
        string::utf8(CONFIG_KEY_EXPIRATION_TIME_SEC)
    }

    public fun config_key_max_number_of_years_registered(): String {
        string::utf8(CONFIG_KEY_MAX_NUMBER_OF_YEARS_REGISTERED)
    }

    public fun config_key_max_domain_length(): String {
        string::utf8(CONFIG_KEY_MAX_DOMAIN_LENGTH)
    }

    public fun config_key_tokendata_description(): String {
        string::utf8(CONFIG_KEY_TOKENDATA_DESCRIPTION)
    }

    public fun config_key_tokendata_url_prefix(): String {
        string::utf8(CONFIG_KEY_TOKENDATA_URL_PREFIX)
    }

    public fun config_key_domain_price(domain_length: u64): String {
        let key = string::utf8(CONFIG_KEY_DOMAIN_PRICE_PREFIX);
        string::append(&mut key, utf8_utils::u128_to_string((domain_length as u128)));
        key
    }

    public fun config_key_subdomain_price(): String {
        string::utf8(CONFIG_KEY_SUBDOMAIN_PRICE)
    }

    fun set_v1<T: copy>(addr: address, config_name: String, value: &T) acquires ConfigurationV1 {
        let map = &mut borrow_global_mut<ConfigurationV1>(addr).config;
        let value = property_map::create_property_value(value);
        if (property_map::contains_key(map, &config_name)) {
            property_map::update_property_value(map, &config_name, value);
        } else {
            property_map::add(map, config_name, value);
        };
    }

    public fun read_string_v1(addr: address, key: &String): String acquires ConfigurationV1 {
        property_map::read_string(&borrow_global<ConfigurationV1>(addr).config, key)
    }

    public fun read_u8_v1(addr: address, key: &String): u8 acquires ConfigurationV1 {
        property_map::read_u8(&borrow_global<ConfigurationV1>(addr).config, key)
    }

    public fun read_u64_v1(addr: address, key: &String): u64 acquires ConfigurationV1 {
        property_map::read_u64(&borrow_global<ConfigurationV1>(addr).config, key)
    }

    public fun read_address_v1(addr: address, key: &String): address acquires ConfigurationV1 {
        property_map::read_address(&borrow_global<ConfigurationV1>(addr).config, key)
    }

    public fun read_u128_v1(addr: address, key: &String): u128 acquires ConfigurationV1 {
        property_map::read_u128(&borrow_global<ConfigurationV1>(addr).config, key)
    }

    public fun read_bool_v1(addr: address, key: &String): bool acquires ConfigurationV1 {
        property_map::read_bool(&borrow_global<ConfigurationV1>(addr).config, key)
    }


    //
    // Tests
    //

    #[test_only]
    friend aptos_names::price_model;
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
    #[test_only]
    use aptos_framework::timestamp;

    #[test_only]
    public fun initialize_aptoscoin_for(framework: &signer) {
        let (burn_cap, mint_cap) = aptos_framework::aptos_coin::initialize_for_test(framework);
        coin::register<AptosCoin>(framework);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test_only]
    public fun set_foundation_fund_address_test_only(addr: address) acquires ConfigurationV1 {
        set_v1(@aptos_names, config_key_foundation_fund_address(), &addr)
    }

    #[test_only]
    public fun set_admin_address_test_only(addr: address) acquires ConfigurationV1 {
        set_v1(@aptos_names, config_key_admin_address(), &addr)
    }

    #[test_only]
    public fun initialize_for_test(aptos_names: &signer, aptos: &signer) acquires ConfigurationV1 {
        timestamp::set_time_has_started_for_testing(aptos);
        initialize_aptoscoin_for(aptos);
        initialize_v1(aptos_names, @aptos_names, @aptos_names);
        set_admin_address_test_only(signer::address_of(aptos_names));
    }

    #[test(myself = @aptos_names)]
    fun test_default_token_configs_are_set(myself: signer) acquires ConfigurationV1 {
        account::create_account_for_test(signer::address_of(&myself));

        initialize_v1(&myself, @aptos_names, @aptos_names);
        set_v1(@aptos_names, config_key_admin_address(), &@aptos_names);

        set_tokendata_description(&myself, string::utf8(b"test description"));
        assert!(tokendata_description() == string::utf8(b"test description"), 1);

        set_tokendata_url_prefix(&myself, string::utf8(b"test_prefix"));
        assert!(tokendata_url_prefix() == string::utf8(b"test_prefix"), 1);
    }

    #[test(myself = @aptos_names)]
    fun test_default_tokens_configs_are_set(myself: signer) acquires ConfigurationV1 {
        account::create_account_for_test(signer::address_of(&myself));

        initialize_v1(&myself, @aptos_names, @aptos_names);
        set_v1(@aptos_names, config_key_admin_address(), &@aptos_names);

        set_tokendata_description(&myself, string::utf8(b"test description"));
        assert!(tokendata_description() == string::utf8(b"test description"), 1);

        set_tokendata_url_prefix(&myself, string::utf8(b"test_prefix"));
        set_tokendata_description(&myself, string::utf8(b"test_desc"));
    }

    #[test(myself = @aptos_names, rando = @0x266f, aptos = @0x1)]
    fun test_configs_are_set(myself: &signer, rando: &signer, aptos: &signer) acquires ConfigurationV1 {
        account::create_account_for_test(signer::address_of(myself));
        account::create_account_for_test(signer::address_of(rando));
        account::create_account_for_test(signer::address_of(aptos));

        // initializes coin, which is required for transfers
        coin::register<AptosCoin>(myself);
        initialize_for_test(myself, aptos);

        assert!(is_enabled(), 0);
        set_is_enabled(myself, false);
        assert!(!is_enabled(), 0);

        assert!(max_domain_length() == 63, 3);
        set_max_domain_length(myself, 25);
        assert!(max_domain_length() == 25, 3);

        assert!(max_number_of_years_registered() == 2, 4);
        set_max_number_of_years_registered(myself, 5);
        assert!(max_number_of_years_registered() == 5, 4);

        assert!(domain_price_for_length(2) == 100 * octas(), 7);
        set_domain_price_for_length(myself, 500 * octas(), 2);
        assert!(domain_price_for_length(2) == 500 * octas(), 7);

        assert!(foundation_fund_address() == signer::address_of(myself), 5);
        coin::register<AptosCoin>(rando);
        set_foundation_fund_address(myself, signer::address_of(rando));
        assert!(foundation_fund_address() == signer::address_of(rando), 5);

        assert!(admin_address() == signer::address_of(myself), 6);
        set_admin_address(myself, signer::address_of(rando));
        assert!(admin_address() == signer::address_of(rando), 6);
    }


    #[test(myself = @aptos_names, rando = @0x266f, aptos = @0x1)]
    #[expected_failure(abort_code = 393218)]
    fun test_cant_set_foundation_address_without_coin(myself: &signer, rando: &signer, aptos: &signer) acquires ConfigurationV1 {
        account::create_account_for_test(signer::address_of(myself));
        account::create_account_for_test(signer::address_of(rando));
        account::create_account_for_test(signer::address_of(aptos));

        // initializes coin, which is required for transfers
        coin::register<AptosCoin>(myself);
        initialize_for_test(myself, aptos);

        assert!(foundation_fund_address() == signer::address_of(myself), 5);
        set_foundation_fund_address(myself, signer::address_of(rando));
        assert!(foundation_fund_address() == signer::address_of(rando), 5);
    }

    #[test(myself = @aptos_names, rando = @0x266f, aptos = @0x1)]
    #[expected_failure(abort_code = 327681)]
    fun test_foundation_config_requires_admin(myself: &signer, rando: &signer, aptos: &signer) acquires ConfigurationV1 {
        account::create_account_for_test(signer::address_of(myself));
        account::create_account_for_test(signer::address_of(rando));
        account::create_account_for_test(signer::address_of(aptos));

        coin::register<AptosCoin>(myself);
        initialize_for_test(myself, aptos);

        assert!(foundation_fund_address() == signer::address_of(myself), 5);
        set_foundation_fund_address(rando, signer::address_of(rando));
    }

    #[test(myself = @aptos_names, rando = @0x266f, aptos = @0x1)]
    #[expected_failure(abort_code = 327681)]
    fun test_admin_config_requires_admin(myself: &signer, rando: &signer, aptos: &signer) acquires ConfigurationV1 {
        account::create_account_for_test(signer::address_of(myself));
        account::create_account_for_test(signer::address_of(rando));
        account::create_account_for_test(signer::address_of(aptos));

        initialize_for_test(myself, aptos);
        coin::register<AptosCoin>(myself);

        assert!(admin_address() == signer::address_of(myself), 6);
        assert!(admin_address() != signer::address_of(rando), 7);

        set_admin_address(rando, signer::address_of(rando));
    }
}
