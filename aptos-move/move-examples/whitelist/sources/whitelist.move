module whitelist_example::whitelist {
    use std::vector;
    use std::string::{String};
    use std::timestamp;
    use std::signer;
    use aptos_std::simple_map::{Self, SimpleMap};
    use std::smart_table::{Self, SmartTable};
    use std::coin;
    use std::error;
    use std::aptos_coin::{AptosCoin};

    /// Represents all the mint tiers available as a map<key: String, value: MintTier>
    /// Stored in the creator's account resources
    struct Tiers has key {
        map: SimpleMap<String, MintTier>,
    }

    /// The price, times, and per user limit for a specific tier; e.g. public, whitelist
    /// the `open_to_public` field indicates there is no restrictions for a requesting address. it is a public mint- it still tracks # of mints though
    struct MintTier has store {
        open_to_public: bool,
        addresses: SmartTable<address, u64>,
        price: u64,
        start_time: u64,
        end_time: u64,
        per_user_limit: u64,
    }

    /// The whitelist MintTier with name "tier_name" was not found
    const ETIER_NOT_FOUND: u64 = 0;
    /// The account requesting to mint is not in that whitelist tier
    const EACCOUNT_NOT_WHITELISTED: u64 = 1;
    /// The account requesting to mint has no mints left for that whitelist tier
    const EACCOUNT_HAS_NO_MINTS_LEFT: u64 = 2;
    /// The mint tier requested has not started yet
    const EMINT_NOT_STARTED: u64 = 3;
    /// The mint tier requested has already ended
    const EMINT_ENDED: u64 = 4;
    /// The account requesting to mint doesn't have enough coins to mint
    const ENOT_ENOUGH_COINS: u64 = 5;
    /// The requested start time is not before the end time
    const ESTART_TIME_AFTER_END_TIME: u64 = 6;

    public entry fun init_tiers(
        creator: &signer,
    ) {
        coin::register<AptosCoin>(creator);
        move_to(
            creator,
            Tiers {
                map: simple_map::create<String, MintTier>(),
            },
        );
    }

    /// Facilitates adding or updating tiers. If the whitelist tier already exists, update it's values- keep the addresses the same
    public entry fun upsert_tier_config(
        creator: &signer,
        tier_name: String,
        open_to_public: bool,
        price: u64,
        start_time: u64,
        end_time: u64,
        per_user_limit: u64,
    ) acquires Tiers {
        assert!(start_time < end_time, error::invalid_argument(ESTART_TIME_AFTER_END_TIME));
        let creator_addr = signer::address_of(creator);
        if (!exists<Tiers>(creator_addr)) {
            init_tiers(creator);
        };
        let tiers = borrow_global_mut<Tiers>(creator_addr);

        if (simple_map::contains_key(&tiers.map, &tier_name)) {
            let tier = simple_map::borrow_mut(&mut tiers.map, &tier_name);
            tier.open_to_public = open_to_public;
            tier.price = price;
            tier.start_time = start_time;
            tier.end_time = end_time;
            tier.per_user_limit = per_user_limit;
        } else {
            let mint_tier = MintTier {
                open_to_public,
                addresses: smart_table::new_with_config<address, u64>(4, 0, 0),
                price,
                start_time,
                end_time,
                per_user_limit,
            };
            simple_map::add(&mut tiers.map, tier_name, mint_tier);
        };
    }

    // Note that this module is agnostic to the existence of an 'admin', that is managed from the calling module.
    // we assume that the caller has gated access to this function correctly
    public entry fun add_addresses_to_tier(
        creator: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires Tiers {
        let map = &mut borrow_global_mut<Tiers>(signer::address_of(creator)).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);
        vector::for_each(addresses, |user_addr| {
            // note that this will abort in `table` if the address exists already- use `upsert` to ignore this
            smart_table::add(&mut mint_tier.addresses, user_addr, 0);
        });
    }

    // Note that this module is agnostic to the existence of an 'admin', that is managed from the calling module.
    // we assume that the caller has gated access to this function correctly
    public entry fun remove_addresses_from_tier(
        creator: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires Tiers {
        let map = &mut borrow_global_mut<Tiers>(signer::address_of(creator)).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);
        vector::for_each(addresses, |user_addr| {
            // note that this will abort in `table` if the address is not found
            smart_table::remove(&mut mint_tier.addresses, user_addr);
        });
    }

    public fun deduct_one_from_tier(
        minter: &signer,
        creator: &signer,
        tier_name: String,
    ) acquires Tiers {
        let minter_addr = signer::address_of(minter);
        let creator_addr = signer::address_of(creator);

        let map = &mut borrow_global_mut<Tiers>(creator_addr).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);

        // assert not too early and not too late
        let now = timestamp::now_seconds();
        assert!(now > mint_tier.start_time, error::permission_denied(EMINT_NOT_STARTED));
        assert!(now < mint_tier.end_time, error::permission_denied(EMINT_ENDED));

        // if `addresses` doesn't contain the minter address, abort if the tier is not open to the public, otherwise add it
        if (!smart_table::contains(&mint_tier.addresses, minter_addr)) {
            if (mint_tier.open_to_public) {
                // open to public but address not in whitelist, add it to list with 0 mints
                smart_table::add(&mut mint_tier.addresses, minter_addr, 0);
            } else {
                // not open to public and address not in whitelist, abort
                abort error::permission_denied(EACCOUNT_NOT_WHITELISTED)
            };
        };

        // assert that the user has mints left
        let count = smart_table::borrow_mut(&mut mint_tier.addresses, minter_addr);
        assert!(*count < mint_tier.per_user_limit, error::permission_denied(EACCOUNT_HAS_NO_MINTS_LEFT));

        // mint the token and transfer `price` AptosCoin from minter to
        assert!(coin::balance<AptosCoin>(minter_addr) >= mint_tier.price, error::permission_denied(ENOT_ENOUGH_COINS));
        coin::transfer<AptosCoin>(minter, creator_addr, mint_tier.price);

        // update the value at the user's address in the smart table
        *count = *count + 1;
    }

    // dependencies only used in test, if we link without #[test_only], the compiler will warn us
    #[test_only]
    use std::string::{Self};
    #[test_only]
    use std::account;
    #[test_only]
    use std::coin::{MintCapability};

    #[test_only]
    const DEFAULT_START_TIME: u64 = 1;
    #[test_only]
    const DEFAULT_CURRENT_TIME: u64 = 2;
    #[test_only]
    const DEFAULT_END_TIME: u64 = 3;

    #[test_only]
    public fun setup_test(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
        timestamp: u64,
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);
        account::create_account_for_test(signer::address_of(creator));
        let (burn, mint) = aptos_framework::aptos_coin::initialize_for_test(aptos_framework);
        setup_account<AptosCoin>(account_a, 4, &mint);
        setup_account<AptosCoin>(account_b, 4, &mint);
        setup_account<AptosCoin>(account_c, 2, &mint);
        coin::destroy_burn_cap(burn);
        coin::destroy_mint_cap(mint);
    }

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

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    public fun test_happy_path(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {

        // 1.  Initialize account a, b, c with 4, 3, and 2 APT each.
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        let creator_addr = signer::address_of(creator);
        let address_a = signer::address_of(account_a);
        let address_b = signer::address_of(account_b);
        let address_c = signer::address_of(account_c);

        // 2.  Creator creates 3 tiers
        init_tiers(creator);
        // tier1: 0 APT, whitelist, per_user_limit = 3, DEFAULT_START_TIME, DEFAULT_END_TIME
        // tier2: 1 APT, whitelist, per_user_limit = 2, DEFAULT_START_TIME, DEFAULT_END_TIME
        // tier3: 2 APT, public, per_user_limit = 1, DEFAULT_START_TIME, DEFAULT_END_TIME
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), !open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 3);
        upsert_tier_config(creator, string::utf8(b"tier2"), !open_to_public, 1, DEFAULT_START_TIME, DEFAULT_END_TIME, 2);
        upsert_tier_config(creator, string::utf8(b"tier3"),  open_to_public, 2, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);

        add_addresses_to_tier(creator, string::utf8(b"tier1"), vector<address> [address_a, address_b]);
        remove_addresses_from_tier(creator, string::utf8(b"tier1"), vector<address> [address_b]);
        add_addresses_to_tier(creator, string::utf8(b"tier2"), vector<address> [address_a, address_b, address_c]);
        remove_addresses_from_tier(creator, string::utf8(b"tier2"), vector<address> [address_c]);

        // ensure upsert works correctly with addresses added, update tier 1 and update it back to normal
        upsert_tier_config(creator, string::utf8(b"tier1"), !open_to_public, 1, DEFAULT_START_TIME, DEFAULT_END_TIME, 0);
        upsert_tier_config(creator, string::utf8(b"tier1"), !open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 3);

        {
            let map = &(borrow_global<Tiers>(creator_addr).map);
            let mint_tier1 = simple_map::borrow(map, &string::utf8(b"tier1"));
            let mint_tier2 = simple_map::borrow(map, &string::utf8(b"tier2"));
            let mint_tier3 = simple_map::borrow(map, &string::utf8(b"tier3"));
            assert!( smart_table::contains(&mint_tier1.addresses, address_a) &&
                    !smart_table::contains(&mint_tier1.addresses, address_b) &&
                     smart_table::contains(&mint_tier2.addresses, address_a) &&
                     smart_table::contains(&mint_tier2.addresses, address_b) &&
                    !smart_table::contains(&mint_tier3.addresses, address_c), 0);
        };

        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier2"));
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier2"));
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier3"));

        deduct_one_from_tier(account_b, creator, string::utf8(b"tier2"));
        deduct_one_from_tier(account_b, creator, string::utf8(b"tier2"));
        deduct_one_from_tier(account_b, creator, string::utf8(b"tier3"));

        deduct_one_from_tier(account_c, creator, string::utf8(b"tier3"));

        {
            let map = &(borrow_global<Tiers>(creator_addr).map);
            let mint_tier1 = simple_map::borrow(map, &string::utf8(b"tier1"));
            let mint_tier2 = simple_map::borrow(map, &string::utf8(b"tier2"));
            let mint_tier3 = simple_map::borrow(map, &string::utf8(b"tier3"));
            assert!(*smart_table::borrow(&mint_tier1.addresses, address_a) == 3 &&
                    *smart_table::borrow(&mint_tier2.addresses, address_a) == 2 &&
                    *smart_table::borrow(&mint_tier3.addresses, address_a) == 1 &&
                    *smart_table::borrow(&mint_tier2.addresses, address_b) == 2 &&
                    *smart_table::borrow(&mint_tier3.addresses, address_b) == 1 &&
                    *smart_table::borrow(&mint_tier3.addresses, address_c) == 1, 1);
        };

        assert!(coin::balance<AptosCoin>(address_a) == 0, 0);
        assert!(coin::balance<AptosCoin>(address_b) == 0, 0);
        assert!(coin::balance<AptosCoin>(address_c) == 0, 0);

        assert!(coin::balance<AptosCoin>(creator_addr) == 10, 0);
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x60000, location = whitelist_example::whitelist)]
    public fun test_tier_not_found(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        let address_a = signer::address_of(account_a);
        init_tiers(creator);
        add_addresses_to_tier(creator, string::utf8(b"tier1"), vector<address> [address_a]);
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50001, location = whitelist_example::whitelist)]
    public fun test_account_not_whitelisted(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), !open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 3);
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50002, location = whitelist_example::whitelist)]
    public fun test_no_mints_left(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50003, location = whitelist_example::whitelist)]
    public fun test_mint_not_started(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
        upsert_tier_config(creator, string::utf8(b"tier2"), open_to_public, 0, DEFAULT_CURRENT_TIME + 1, DEFAULT_END_TIME + 1, 1);
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier2"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50004, location = whitelist_example::whitelist)]
    public fun test_mint_ended(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
        upsert_tier_config(creator, string::utf8(b"tier2"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME - 1, 1);
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier2"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50005, location = whitelist_example::whitelist)]
    public fun test_not_enough_coins(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 100000000, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(account_a, creator, string::utf8(b"tier1"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x10006, location = whitelist_example::whitelist)]
    public fun test_start_time_after_end_time(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Tiers {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 100000000, DEFAULT_START_TIME + 1, DEFAULT_START_TIME, 1);
    }
}
