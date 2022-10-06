#[test_only]
module aptos_names::test_helper {
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::timestamp;
    use aptos_names::config;
    use aptos_names::domains;
    use aptos_names::price_model;
    use aptos_names::test_utils;
    use aptos_names::time_helper;
    use aptos_names::token_helper;
    use aptos_token::token;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use std::vector;

    // Ammount to mint to test accounts during the e2e tests
    const MINT_AMOUNT_APT: u64 = 500;

    // 500 APT
    public fun mint_amount(): u64 {
        MINT_AMOUNT_APT * config::octas()
    }

    public fun domain_name(): String {
        string::utf8(b"test")
    }

    public fun subdomain_name(): String {
        string::utf8(b"sub")
    }

    public fun one_year_secs(): u64 {
        time_helper::years_to_seconds(1)
    }

    public fun two_hundred_year_secs(): u64 {
        time_helper::years_to_seconds(200)
    }

    public fun fq_domain_name(): String {
        string::utf8(b"test.apt")
    }

    public fun fq_subdomain_name(): String {
        string::utf8(b"sub.test.apt")
    }

    public fun e2e_test_setup(myself: &signer, user: signer, aptos: &signer, rando: signer, foundation: &signer): vector<signer> {
        account::create_account_for_test(@aptos_names);
        let new_accounts = setup_and_fund_accounts(aptos, foundation, vector[user, rando]);
        timestamp::set_time_has_started_for_testing(aptos);
        domains::init_module_for_test(myself, @aptos_names, @aptos_names);
        config::set_foundation_fund_address_test_only(signer::address_of(foundation));
        new_accounts
    }

    /// Register the domain, and verify the registration was done correctly
    public fun register_name(user: &signer, subdomain_name: Option<String>, domain_name: String, registration_duration_secs: u64, expected_fq_domain_name: String, expected_property_version: u64) {
        let user_addr = signer::address_of(user);

        let is_subdomain = option::is_some(&subdomain_name);

        let user_balance_before = coin::balance<AptosCoin>(user_addr);
        let register_name_event_v1_event_count_before = domains::get_register_name_event_v1_count();
        let set_name_address_event_v1_event_count_before = domains::get_set_name_address_event_v1_count();

        let years = (time_helper::seconds_to_years(registration_duration_secs) as u8);
        if (option::is_none(&subdomain_name)) {
            domains::register_domain(user, domain_name, years);
        } else {
            domains::register_subdomain(user, *option::borrow(&subdomain_name), domain_name, registration_duration_secs);
        };

        // It should now be: not expired, registered, and not registerable
        assert!(!domains::name_is_expired(subdomain_name, domain_name), 12);
        assert!(!domains::name_is_registerable(subdomain_name, domain_name), 13);
        assert!(domains::name_is_registered(subdomain_name, domain_name), 14);

        let (is_owner, token_id) = domains::is_owner_of_name(user_addr, subdomain_name, domain_name);
        let (tdi_creator, tdi_collection, tdi_name, tdi_property_version) = token::get_token_id_fields(&token_id);

        assert!(is_owner, 3);
        assert!(tdi_creator == token_helper::get_token_signer_address(), 4);
        assert!(tdi_collection == config::collection_name_v1(), 5);
        test_utils::print_actual_expected(b"tdi_name: ", tdi_name, expected_fq_domain_name, false);
        assert!(tdi_name == expected_fq_domain_name, 6);
        test_utils::print_actual_expected(b"tdi_property_version: ", tdi_property_version, expected_property_version, false);
        assert!(tdi_property_version == expected_property_version, tdi_property_version);

        let expected_user_balance_after;
        let user_balance_after = coin::balance<AptosCoin>(user_addr);
        if (is_subdomain) {
            // If it's a subdomain, we only charge a nomincal fee
            expected_user_balance_after = user_balance_before - price_model::price_for_subdomain_v1(registration_duration_secs);
        }else {
            let domain_price = price_model::price_for_domain_v1(string::length(&domain_name), years);
            assert!(domain_price / config::octas() == 30, domain_price / config::octas());
            expected_user_balance_after = user_balance_before - domain_price;
        };

        test_utils::print_actual_expected(b"user_balance_after: ", user_balance_after, expected_user_balance_after, false);
        assert!(user_balance_after == expected_user_balance_after, expected_user_balance_after);

        // Ensure the name was registered correctly, with an expiration timestamp one year in the future
        let (property_version, expiration_time_sec, target_address) = domains::get_name_record_v1_props_for_name(subdomain_name, domain_name);
        assert!(time_helper::seconds_to_days(expiration_time_sec - timestamp::now_seconds()) == 365, 10);

        if (is_subdomain) {
            // We haven't set a target address yet!
            assert!(target_address == option::none(), 11);
        } else {
            // Should automatically point to the users address
            assert!(target_address == option::some(user_addr), 11);
        };

        // And the property version is correct
        test_utils::print_actual_expected(b"property_version: ", property_version, expected_property_version, false);
        assert!(property_version == expected_property_version, 12);

        // Ensure the properties were set correctly
        let token_data_id = token_helper::build_tokendata_id(token_helper::get_token_signer_address(), subdomain_name, domain_name);
        let (creator, collection_name, token_name) = token::get_token_data_id_fields(&token_data_id);
        assert!(creator == token_helper::get_token_signer_address(), 20);
        assert!(collection_name == string::utf8(b"Aptos Names V1"), 21);
        assert!(token_name == token_name, 22);

        // Assert events have been correctly emmitted
        let register_name_event_v1_num_emitted = domains::get_register_name_event_v1_count() - register_name_event_v1_event_count_before;
        let set_name_address_event_v1_num_emitted = domains::get_set_name_address_event_v1_count() - set_name_address_event_v1_event_count_before;

        test_utils::print_actual_expected(b"register_name_event_v1_num_emitted: ", register_name_event_v1_num_emitted, 1, false);
        assert!(register_name_event_v1_num_emitted == 1, register_name_event_v1_num_emitted);

        if (is_subdomain) {
            // We haven't set a target address yet!
            test_utils::print_actual_expected(b"set_name_address_event_v1_num_emitted: ", set_name_address_event_v1_num_emitted, 0, false);
            assert!(set_name_address_event_v1_num_emitted == 0, set_name_address_event_v1_num_emitted);
        } else {
            // Should automatically point to the users address
            test_utils::print_actual_expected(b"set_name_address_event_v1_num_emitted: ", set_name_address_event_v1_num_emitted, 1, false);
            assert!(set_name_address_event_v1_num_emitted == 1, set_name_address_event_v1_num_emitted);
        };
    }

    /// Set the domain address, and verify the address was set correctly
    public fun set_name_address(user: &signer, subdomain_name: Option<String>, domain_name: String, expected_target_address: address) {
        let register_name_event_v1_event_count_before = domains::get_register_name_event_v1_count();
        let set_name_address_event_v1_event_count_before = domains::get_set_name_address_event_v1_count();

        domains::set_name_address(user, subdomain_name, domain_name, expected_target_address);
        let (_property_version, _expiration_time_sec, target_address) = domains::get_name_record_v1_props_for_name(subdomain_name, domain_name);
        test_utils::print_actual_expected(b"set_domain_address: ", target_address, option::some(expected_target_address), false);
        assert!(target_address == option::some(expected_target_address), 33);

        // Assert events have been correctly emmitted
        let register_name_event_v1_num_emitted = domains::get_register_name_event_v1_count() - register_name_event_v1_event_count_before;
        let set_name_address_event_v1_num_emitted = domains::get_set_name_address_event_v1_count() - set_name_address_event_v1_event_count_before;

        test_utils::print_actual_expected(b"register_name_event_v1_num_emitted: ", register_name_event_v1_num_emitted, 0, false);
        assert!(register_name_event_v1_num_emitted == 0, register_name_event_v1_num_emitted);

        test_utils::print_actual_expected(b"set_name_address_event_v1_num_emitted: ", set_name_address_event_v1_num_emitted, 1, false);
        assert!(set_name_address_event_v1_num_emitted == 1, set_name_address_event_v1_num_emitted);
    }

    /// Clear the domain address, and verify the address was cleared
    public fun clear_name_address(user: &signer, subdomain_name: Option<String>, domain_name: String) {
        let register_name_event_v1_event_count_before = domains::get_register_name_event_v1_count();
        let set_name_address_event_v1_event_count_before = domains::get_set_name_address_event_v1_count();

        // And also can clear if is registered address, but not owner
        if (option::is_none(&subdomain_name)) {
            domains::clear_domain_address(user, domain_name);
        } else {
            domains::clear_subdomain_address(user, *option::borrow(&subdomain_name), domain_name);
        };
        let (_property_version, _expiration_time_sec, target_address) = domains::get_name_record_v1_props_for_name(subdomain_name, domain_name);
        test_utils::print_actual_expected(b"clear_domain_address: ", target_address, option::none(), false);
        assert!(target_address == option::none(), 32);

        // Assert events have been correctly emmitted
        let register_name_event_v1_num_emitted = domains::get_register_name_event_v1_count() - register_name_event_v1_event_count_before;
        let set_name_address_event_v1_num_emitted = domains::get_set_name_address_event_v1_count() - set_name_address_event_v1_event_count_before;

        test_utils::print_actual_expected(b"register_name_event_v1_num_emitted: ", register_name_event_v1_num_emitted, 0, false);
        assert!(register_name_event_v1_num_emitted == 0, register_name_event_v1_num_emitted);

        test_utils::print_actual_expected(b"set_name_address_event_v1_num_emitted: ", set_name_address_event_v1_num_emitted, 1, false);
        assert!(set_name_address_event_v1_num_emitted == 1, set_name_address_event_v1_num_emitted);
    }

    public fun setup_and_fund_accounts(aptos: &signer, foundation: &signer, users: vector<signer>): vector<signer> {
        let (burn_cap, mint_cap) = aptos_framework::aptos_coin::initialize_for_test(aptos);

        let len = vector::length(&users);
        let i = 0;
        while (i < len) {
            let user = vector::borrow(&users, i);
            let user_addr = signer::address_of(user);
            account::create_account_for_test(user_addr);
            coin::register<AptosCoin>(user);
            coin::deposit(user_addr, coin::mint<AptosCoin>(mint_amount(), &mint_cap));
            assert!(coin::balance<AptosCoin>(user_addr) == mint_amount(), 1);
            i = i + 1;
        };

        account::create_account_for_test(signer::address_of(foundation));
        coin::register<AptosCoin>(foundation);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
        users
    }
}
