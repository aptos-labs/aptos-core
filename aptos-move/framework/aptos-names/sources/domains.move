module aptos_names::domains {
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::timestamp;
    use aptos_names::config;
    use aptos_names::price_model;
    use aptos_names::time_helper;
    use aptos_names::token_helper;
    use aptos_names::utf8_utils;
    use aptos_std::event;
    use aptos_std::table::{Self, Table};
    use aptos_token::property_map::Self;
    use aptos_token::token::{Self, TokenId};
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;


    /// The Naming Service contract is not enabled
    const ENOT_ENABLED: u64 = 1;
    /// The caller is not authorized to perform this operation
    const ENOT_AUTHORIZED: u64 = 2;
    /// The name is not available, as it has already been registered
    const ENAME_NOT_AVAILABLE: u64 = 3;
    /// The number of years the caller attempted to register the domain or subdomain for is invalid
    const EINVALID_NUMBER_YEARS: u64 = 4;
    /// The domain does not exist- it is not registered
    const ENAME_NOT_EXIST: u64 = 5;
    /// The caller is not the owner of the domain, and is not authorized to perform the action
    const ENOT_OWNER_OF_DOMAIN: u64 = 6;
    /// The caller is not the owner of the name, and is not authorized to perform the action
    const ENOT_OWNER_OF_NAME: u64 = 9;
    /// The domain name is too long- it exceeds the configured maximum number of utf8 glyphs
    const EDOMAIN_TOO_LONG: u64 = 10;
    /// The subdomain name is too long- it exceeds the configured maximum number of utf8 glyphs
    const ESUBDOMAIN_TOO_LONG: u64 = 11;
    /// The domain name contains invalid characters: it is not a valid domain name
    const EDOMAIN_HAS_INVALID_CHARACTERS: u64 = 12;
    /// The subdomain name contains invalid characters: it is not a valid domain name
    const ESUBDOMAIN_HAS_INVALID_CHARACTERS: u64 = 13;
    /// The subdomain registration duration can not be longer than its parent domain
    const ESUBDOMAIN_CAN_NOT_EXCEED_DOMAIN_REGISTRATION: u64 = 14;
    /// The subdomain name is too short (must be >= 2)
    const ESUBDOMAIN_TOO_SHORT: u64 = 15;

    struct NameRecordKeyV1 has copy, drop, store {
        subdomain_name: Option<String>,
        domain_name: String,
    }

    struct NameRecordV1 has copy, drop, store {
        // This is the property version of the NFT that this name record represents.
        // This is required to tell expired vs current NFTs apart.
        property_version: u64,
        // The time, in seconds, when the name is considered expired
        expiration_time_sec: u64,
        // The address this name is set to point to
        target_address: Option<address>,
    }

    /// The main registry: keeps a mapping of NameRecordKeyV1 (domain_name & optional subdomain) to NameRecord
    struct NameRegistryV1 has key, store {
        // A mapping from domain name to an address
        registry: Table<NameRecordKeyV1, NameRecordV1>,
    }

    /// Holder for `SetNameAddressEventV1` events
    struct SetNameAddressEventsV1 has key, store {
        set_name_events: event::EventHandle<SetNameAddressEventV1>,
    }

    /// Holder for `RegisterNameEventV1` events
    struct RegisterNameEventsV1 has key, store {
        register_name_events: event::EventHandle<RegisterNameEventV1>,
    }

    /// A name (potentially subdomain) has had it's address changed
    /// This could be to a new address, or it could have been cleared
    struct SetNameAddressEventV1 has drop, store {
        subdomain_name: Option<String>,
        domain_name: String,
        property_version: u64,
        expiration_time_secs: u64,
        new_address: Option<address>,
    }

    /// A name (potentially subdomain) has been registered on chain
    /// Includes the the fee paid for the registration, and the expiration time
    /// Also includes the property_version, so we can tell which version of a given domain NFT is the latest
    struct RegisterNameEventV1 has drop, store {
        subdomain_name: Option<String>,
        domain_name: String,
        registration_fee_octas: u64,
        property_version: u64,
        expiration_time_secs: u64,
    }

    /// This is only callable during genesis or framework upgrades
    public entry fun initialize(framework: &signer, funds_address: address, admin_address: address) {
        use aptos_framework::aptos_account;

        assert!(signer::address_of(framework) == @0x4, error::permission_denied(ENOT_AUTHORIZED));

        if (!account::exists_at(funds_address)) {
            aptos_account::create_account(funds_address);
        };

        if (!account::exists_at(admin_address)) {
            aptos_account::create_account(admin_address);
        };

        config::initialize_v1(framework, admin_address, funds_address);

        move_to(
            framework,
            NameRegistryV1 {
                registry: table::new(),
            }
        );

        move_to(framework, SetNameAddressEventsV1 {
            set_name_events: account::new_event_handle<SetNameAddressEventV1>(framework),
        });

        move_to(framework, RegisterNameEventsV1 {
            register_name_events: account::new_event_handle<RegisterNameEventV1>(framework),
        });

        token_helper::initialize(framework);
    }

    /// A wrapper around `register_name` as an entry function.
    /// Option<String> is not currently serializable, so we have these convenience methods
    public entry fun register_domain(sign: &signer, domain_name: String, num_years: u8) acquires NameRegistryV1, RegisterNameEventsV1, SetNameAddressEventsV1 {
        assert!(config::is_enabled(), error::unavailable(ENOT_ENABLED));
        assert!(num_years > 0 && num_years <= config::max_number_of_years_registered(), error::out_of_range(EINVALID_NUMBER_YEARS));

        let subdomain_name = option::none<String>();

        assert!(name_is_registerable(subdomain_name, domain_name), error::invalid_state(ENAME_NOT_AVAILABLE));

        // Conver the num_years to its seconds representation for the inner method
        let registration_duration_secs: u64 = time_helper::years_to_seconds((num_years as u64));

        let (is_valid, length) = utf8_utils::string_is_allowed(&domain_name);
        assert!(is_valid, error::invalid_argument(EDOMAIN_HAS_INVALID_CHARACTERS));
        assert!(length <= config::max_domain_length(), error::out_of_range(EDOMAIN_TOO_LONG));


        let price = price_model::price_for_domain_v1(length, num_years);
        coin::transfer<AptosCoin>(sign, config::foundation_fund_address(), price);

        register_name_internal(sign, subdomain_name, domain_name, registration_duration_secs, price);
        // Automatically set the name to point to the sender's address
        set_name_address_internal(subdomain_name, domain_name, signer::address_of(sign));
    }

    /// A wrapper around `register_name` as an entry function.
    /// Option<String> is not currently serializable, so we have these convenience method
    public entry fun register_subdomain(sign: &signer, subdomain_name: String, domain_name: String, registration_duration_secs: u64) acquires NameRegistryV1, RegisterNameEventsV1 {
        assert!(config::is_enabled(), error::unavailable(ENOT_ENABLED));

        assert!(name_is_registerable(option::some(subdomain_name), domain_name), error::invalid_state(ENAME_NOT_AVAILABLE));

        // We are registering a subdomain name: this has no cost, but is only doable by the owner of the domain
        let (is_valid, length) = utf8_utils::string_is_allowed(&subdomain_name);
        assert!(is_valid, error::invalid_argument(ESUBDOMAIN_HAS_INVALID_CHARACTERS));
        assert!(length <= config::max_domain_length(), error::out_of_range(ESUBDOMAIN_TOO_LONG));
        assert!(length >= 2, error::out_of_range(ESUBDOMAIN_TOO_SHORT));

        // Ensure signer owns the domain we're registering a subdomain for
        let signer_addr = signer::address_of(sign);
        let (is_owner, _token_id) = is_owner_of_name(signer_addr, option::none(), domain_name);
        assert!(is_owner, error::permission_denied(ENOT_OWNER_OF_DOMAIN));

        let price = price_model::price_for_subdomain_v1(registration_duration_secs);
        coin::transfer<AptosCoin>(sign, config::foundation_fund_address(), price);

        register_name_internal(sign, option::some(subdomain_name), domain_name, registration_duration_secs, price);
    }

    /// Register a nane. Accepts an optional subdomain name, a required domain name, and a registration duration in seconds.
    /// For domains, the registration duration is only allowed to be in increments of 1 year, for now
    /// Since the owner of the domain is the only one that can create the subdomain, we allow them to decide how long they want the underlying registration to be
    /// The maximum subdomain registration duration is limited to the duration of its parent domain registration
    fun register_name_internal(sign: &signer, subdomain_name: Option<String>, domain_name: String, registration_duration_secs: u64, price: u64) acquires NameRegistryV1, RegisterNameEventsV1 {
        let aptos_names = borrow_global_mut<NameRegistryV1>(@aptos_names);

        let name_expiration_time_secs = timestamp::now_seconds() + registration_duration_secs;

        // if it is a subdomain, and it expires later than its domain, throw an error
        // This is done here so that any governance moderation activities must abide by the same invariant
        if (option::is_some(&subdomain_name)) {
            let domain_name_record_key = create_name_record_key_v1(option::none(), domain_name);
            let (_property_version, domain_expiration_time_sec, _target_address) = get_name_record_v1_props(table::borrow(&aptos_names.registry, domain_name_record_key));
            assert!(name_expiration_time_secs <= domain_expiration_time_sec, error::out_of_range(ESUBDOMAIN_CAN_NOT_EXCEED_DOMAIN_REGISTRATION));
        };

        // Create the token, and transfer it to the user
        let tokendata_id = token_helper::ensure_token_data(subdomain_name, domain_name, config::domain_type());
        let token_id = token_helper::create_token(tokendata_id);


        let (property_keys, property_values, property_types) = get_name_property_map(subdomain_name, name_expiration_time_secs);
        token_id = token_helper::set_token_props(token_helper::get_token_signer_address(), property_keys, property_values, property_types, token_id);
        token_helper::transfer_token_to(sign, token_id);

        // Add this domain to the registry
        let (_creator, _collection, _name, property_version) = token::get_token_id_fields(&token_id);
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        let name_record = create_name_record_v1(property_version, name_expiration_time_secs, option::none());

        table::upsert(&mut aptos_names.registry, name_record_key, name_record);

        event::emit_event<RegisterNameEventV1>(
            &mut borrow_global_mut<RegisterNameEventsV1>(@aptos_names).register_name_events,
            RegisterNameEventV1 {
                subdomain_name,
                domain_name,
                registration_fee_octas: price,
                property_version,
                expiration_time_secs: name_expiration_time_secs,
            },
        );
    }

    /// Forcefully set the name of a domain.
    /// This is a privileged operation, used via governance, to forcefully set a domain address
    /// This can be used, for example, to forcefully set the domain for a system address domain
    public entry fun force_set_domain_address(sign: &signer, domain_name: String, new_owner: address) acquires NameRegistryV1, SetNameAddressEventsV1 {
        force_set_name_address(sign, option::none(), domain_name, new_owner);
    }

    public entry fun force_set_subdomain_address(sign: &signer, subdomain_name: String, domain_name: String, new_owner: address) acquires NameRegistryV1, SetNameAddressEventsV1 {
        force_set_name_address(sign, option::some(subdomain_name), domain_name, new_owner);
    }

    fun force_set_name_address(sign: &signer, subdomain_name: Option<String>, domain_name: String, new_owner: address) acquires NameRegistryV1, SetNameAddressEventsV1 {
        config::assert_signer_is_admin(sign);
        set_name_address_internal(subdomain_name, domain_name, new_owner);
    }

    /// Forcefully create or seize a domain name. This is a privileged operation, used via governance.
    /// This can be used, for example, to forcefully create a domain for a system address domain, or to seize a domain from a malicious user.
    /// The `registration_duration_secs` parameter is the number of seconds to register the domain for, but is not limited to the maximum set in the config for domains registered normally.
    /// This allows, for example, to create a domain for the system address for 100 years so we don't need to worry about expiry
    /// Or for moderation purposes, it allows us to seize a racist/harassing domain for 100 years, and park it somewhere safe
    public entry fun force_create_or_seize_domain_name(sign: &signer, domain_name: String, registration_duration_secs: u64) acquires NameRegistryV1, RegisterNameEventsV1 {
        force_create_or_seize_name(sign, option::none(), domain_name, registration_duration_secs);
    }

    public entry fun force_create_or_seize_subdomain_name(sign: &signer, subdomain_name: String, domain_name: String, registration_duration_secs: u64) acquires NameRegistryV1, RegisterNameEventsV1 {
        force_create_or_seize_name(sign, option::some(subdomain_name), domain_name, registration_duration_secs);
    }

    public fun force_create_or_seize_name(sign: &signer, subdomain_name: Option<String>, domain_name: String, registration_duration_secs: u64) acquires NameRegistryV1, RegisterNameEventsV1 {
        config::assert_signer_is_admin(sign);
        register_name_internal(sign, subdomain_name, domain_name, registration_duration_secs, 0);
    }

    /// Checks for the name not existing, or being expired
    /// Returns true if the name is available for registration
    /// if this is a subdomain, and the domain doesn't exist, returns false
    /// Doesn't use the `name_is_expired` or `name_is_registered` internally to share the borrow
    public fun name_is_registerable(subdomain_name: Option<String>, domain_name: String): bool acquires NameRegistryV1 {
        // If this is a subdomain, ensure the domain also exists, and is not expired: i.e not registerable
        // So if the domain name is registerable, we return false, as the subdomain is not registerable
        if (option::is_some(&subdomain_name) && name_is_registerable(option::none(), domain_name)) {
            return false
        };
        // Check to see if the domain is registered, or expired
        let aptos_names = borrow_global<NameRegistryV1>(@aptos_names);
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        !table::contains(&aptos_names.registry, name_record_key) || name_is_expired(subdomain_name, domain_name)
    }

    /// Returns true if the name is registered, and is expired.
    /// If the name does not exist, raises an error
    public fun name_is_expired(subdomain_name: Option<String>, domain_name: String): bool acquires NameRegistryV1 {
        let aptos_names = borrow_global<NameRegistryV1>(@aptos_names);
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        assert!(table::contains(&aptos_names.registry, name_record_key), error::not_found(ENAME_NOT_EXIST));

        let name_record = table::borrow(&aptos_names.registry, name_record_key);
        let (_property_version, expiration_time_sec, _target_address) = get_name_record_v1_props(name_record);
        time_is_expired(expiration_time_sec)
    }

    /// Returns true if the name is registered
    /// If the name does not exist, returns false
    public fun name_is_registered(subdomain_name: Option<String>, domain_name: String): bool acquires NameRegistryV1 {
        let aptos_names = borrow_global<NameRegistryV1>(@aptos_names);
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        table::contains(&aptos_names.registry, name_record_key)
    }

    /// Given a domain and/or a subdomain, returns the name record
    public fun get_name_record_v1(subdomain_name: Option<String>, domain_name: String): NameRecordV1 acquires NameRegistryV1 {
        assert!(name_is_registered(subdomain_name, domain_name), error::not_found(ENAME_NOT_EXIST));
        let aptos_names = borrow_global<NameRegistryV1>(@aptos_names);
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        *table::borrow(&aptos_names.registry, name_record_key)
    }

    /// Given a domain and/or a subdomain, returns the name record properties
    public fun get_name_record_v1_props_for_name(subdomain_name: Option<String>, domain_name: String): (u64, u64, Option<address>) acquires NameRegistryV1 {
        assert!(name_is_registered(subdomain_name, domain_name), error::not_found(ENAME_NOT_EXIST));
        let aptos_names = borrow_global<NameRegistryV1>(@aptos_names);
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        get_name_record_v1_props(table::borrow(&aptos_names.registry, name_record_key))
    }

    /// Check if the address is the owner of the given aptos_name
    /// If the name does not exist, returns false
    public fun is_owner_of_name(owner_address: address, subdomain_name: Option<String>, domain_name: String): (bool, TokenId) {
        let token_data_id = token_helper::build_tokendata_id(token_helper::get_token_signer_address(), subdomain_name, domain_name);
        let token_id = token_helper::latest_token_id(&token_data_id);
        (token::balance_of(owner_address, token_id) > 0, token_id)
    }

    /// gets the address pointed to by a given name
    /// Is `Option<address>` because the name may not be registered, or it may not have an address associated with it
    public fun name_resolved_address(subdomain_name: Option<String>, domain_name: String): Option<address> acquires NameRegistryV1 {
        let aptos_names = borrow_global<NameRegistryV1>(@aptos_names);
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        if (table::contains(&aptos_names.registry, name_record_key)) {
            let name_record = table::borrow(&aptos_names.registry, name_record_key);
            let (_property_version, _expiration_time_sec, target_address) = get_name_record_v1_props(name_record);
            target_address
        } else {
            option::none<address>()
        }
    }

    public entry fun set_domain_address(sign: &signer, domain_name: String, new_address: address) acquires NameRegistryV1, SetNameAddressEventsV1 {
        set_name_address(sign, option::none(), domain_name, new_address);
    }


    public entry fun set_subdomain_address(sign: &signer, subdomain_name: String, domain_name: String, new_address: address) acquires NameRegistryV1, SetNameAddressEventsV1 {
        set_name_address(sign, option::some(subdomain_name), domain_name, new_address);
    }

    public fun set_name_address(sign: &signer, subdomain_name: Option<String>, domain_name: String, new_address: address) acquires NameRegistryV1, SetNameAddressEventsV1 {
        let signer_addr = signer::address_of(sign);
        let (is_owner, token_id) = is_owner_of_name(signer_addr, subdomain_name, domain_name);
        assert!(is_owner, error::permission_denied(ENOT_OWNER_OF_NAME));

        let name_record = set_name_address_internal(subdomain_name, domain_name, new_address);
        let (_property_version, expiration_time_sec, _target_address) = get_name_record_v1_props(&name_record);
        let (property_keys, property_values, property_types) = get_name_property_map(subdomain_name, expiration_time_sec);
        token_helper::set_token_props(signer_addr, property_keys, property_values, property_types, token_id);
    }

    fun set_name_address_internal(subdomain_name: Option<String>, domain_name: String, new_address: address): NameRecordV1 acquires NameRegistryV1, SetNameAddressEventsV1 {
        assert!(name_is_registered(subdomain_name, domain_name), error::not_found(ENAME_NOT_EXIST));
        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        let aptos_names = borrow_global_mut<NameRegistryV1>(@aptos_names);
        let name_record = table::borrow_mut(&mut aptos_names.registry, name_record_key);
        let (property_version, expiration_time_sec, _target_address) = get_name_record_v1_props(name_record);
        name_record.target_address = option::some(new_address);
        emit_set_name_address_event_v1(
            subdomain_name,
            domain_name,
            property_version,
            expiration_time_sec,
            option::some(new_address),
        );
        *name_record
    }

    public entry fun clear_domain_address(sign: &signer, domain_name: String) acquires NameRegistryV1, SetNameAddressEventsV1 {
        clear_name_address(sign, option::none(), domain_name);
    }

    public entry fun clear_subdomain_address(sign: &signer, subdomain_name: String, domain_name: String) acquires NameRegistryV1, SetNameAddressEventsV1 {
        clear_name_address(sign, option::some(subdomain_name), domain_name);
    }

    /// This is a shared entry point for clearing the address of a domain or subdomain
    /// It enforces owner permissions
    fun clear_name_address(sign: &signer, subdomain_name: Option<String>, domain_name: String) acquires NameRegistryV1, SetNameAddressEventsV1 {
        assert!(name_is_registered(subdomain_name, domain_name), error::not_found(ENAME_NOT_EXIST));

        let signer_addr = signer::address_of(sign);
        // Only the owner or the registered address can clear the address
        let (is_owner, token_id) = is_owner_of_name(signer_addr, subdomain_name, domain_name);
        let is_name_resolved_address = name_resolved_address(subdomain_name, domain_name) == option::some<address>(signer_addr);

        assert!(is_owner || is_name_resolved_address, error::permission_denied(ENOT_AUTHORIZED));

        let name_record_key = create_name_record_key_v1(subdomain_name, domain_name);
        let aptos_names = borrow_global_mut<NameRegistryV1>(@aptos_names);
        let name_record = table::borrow_mut(&mut aptos_names.registry, name_record_key);
        let (property_version, expiration_time_sec, _target_address) = get_name_record_v1_props(name_record);
        name_record.target_address = option::none();
        emit_set_name_address_event_v1(
            subdomain_name,
            domain_name,
            property_version,
            expiration_time_sec,
            option::none(),
        );

        if (is_owner) {
            let (_property_version, expiration_time_sec, _target_address) = get_name_record_v1_props(name_record);
            let (property_keys, property_values, property_types) = get_name_property_map(subdomain_name, expiration_time_sec);
            token_helper::set_token_props(signer_addr, property_keys, property_values, property_types, token_id);
        };
    }

    fun emit_set_name_address_event_v1(subdomain_name: Option<String>, domain_name: String, property_version: u64, expiration_time_secs: u64, new_address: Option<address>) acquires SetNameAddressEventsV1 {
        let event = SetNameAddressEventV1 {
            subdomain_name,
            domain_name,
            property_version,
            expiration_time_secs,
            new_address,
        };

        event::emit_event<SetNameAddressEventV1>(
            &mut borrow_global_mut<SetNameAddressEventsV1>(@aptos_names).set_name_events,
            event,
        );
    }

    public fun get_name_property_map(subdomain_name: Option<String>, expiration_time_sec: u64): (vector<String>, vector<vector<u8>>, vector<String>) {
        let type;
        if (option::is_some(&subdomain_name)) {
            type = property_map::create_property_value(&config::subdomain_type());
        } else {
            type = property_map::create_property_value(&config::domain_type());
        };
        let expiration_time_sec = property_map::create_property_value(&expiration_time_sec);

        let property_keys: vector<String> = vector[config::config_key_type(), config::config_key_expiration_time_sec()];
        let property_values: vector<vector<u8>> = vector[ property_map::borrow_value(&type), property_map::borrow_value(&expiration_time_sec)];
        let property_types: vector<String> = vector[property_map::borrow_type(&type), property_map::borrow_type(&expiration_time_sec)];
        (property_keys, property_values, property_types)
    }

    public fun create_name_record_v1(property_version: u64, expiration_time_sec: u64, target_address: Option<address>): NameRecordV1 {
        NameRecordV1 {
            property_version,
            expiration_time_sec,
            target_address,
        }
    }

    public fun get_name_record_v1_props(name_record: &NameRecordV1): (u64, u64, Option<address>) {
        (name_record.property_version, name_record.expiration_time_sec, name_record.target_address)
    }

    public fun create_name_record_key_v1(subdomain_name: Option<String>, domain_name: String): NameRecordKeyV1 {
        NameRecordKeyV1 {
            subdomain_name,
            domain_name,
        }
    }

    /// Given a time, returns true if that time is in the past, false otherwise
    public fun time_is_expired(expiration_time_sec: u64): bool {
        timestamp::now_seconds() >= expiration_time_sec
    }

    public fun get_name_record_key_v1_props(name_record_key: &NameRecordKeyV1): (Option<String>, String) {
        (name_record_key.subdomain_name, name_record_key.domain_name)
    }

    #[test_only]
    public fun init_module_for_test(framework: &signer, funds_address: address, admin_address: address) {
        initialize(framework, funds_address, admin_address)
    }

    #[test_only]
    public fun get_set_name_address_event_v1_count(): u64 acquires SetNameAddressEventsV1 {
        event::counter(&borrow_global<SetNameAddressEventsV1>(@aptos_names).set_name_events)
    }

    #[test_only]
    public fun get_register_name_event_v1_count(): u64 acquires RegisterNameEventsV1 {
        event::counter(&borrow_global<RegisterNameEventsV1>(@aptos_names).register_name_events)
    }

    #[test(aptos = @0x1)]
    fun test_time_is_expired(aptos: &signer) {
        timestamp::set_time_has_started_for_testing(aptos);
        // Set the time to a nonzero value to avoid subtraction overflow.
        timestamp::update_global_time_for_test_secs(100);

        // If the expiration time is after the current time, we should return not expired
        assert!(!time_is_expired(timestamp::now_seconds() + 1), 1);

        // If the current time is equal to expiration time, consider it expired
        assert!(time_is_expired(timestamp::now_seconds()), 2);

        // If the expiration time is earlier than the current time, we should return expired
        assert!(time_is_expired(timestamp::now_seconds() - 1), 3);
    }
}
