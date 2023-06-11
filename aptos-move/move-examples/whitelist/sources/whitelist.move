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
    use std::object::{Self, Object, DeleteRef};

    /// Resource moved into the creator's account, used to find the object easily.
    /// Destroyed with the object when `destroy(...)` is called
    struct ObjectInfo has key {
        whitelist_obj: Object<Whitelist>,
        whitelist_addr: address,
        delete_ref: DeleteRef,
    } 

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents all the mint tiers available as a map<key: String, value: MintTier>
    /// Used as an Object<Whitelist>
    struct Whitelist has key {
        map: SimpleMap<String, MintTier>,
    }

    /// The price, times, and per user limit for a specific tier; e.g. public, whitelist
    /// the `open_to_public` field indicates there is no restrictions for a requesting address. it is a public mint- it still tracks # of mints though
    struct MintTier has store {
        open_to_public: bool,
        addresses: SmartTable<address, u64>, // used as a set
        price: u64,
        start_time: u64,
        end_time: u64,
        per_user_limit: u64,
    }

    /// The whitelist MintTier with name "tier_name" was not found
    const ETIER_NOT_FOUND: u64 = 0;
    /// The account requesting to mint is not in that whitelist tier
    const EACCOUNT_NOT_IN_TIER: u64 = 1;
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
    /// There is no whitelist for the given account
    const EWHITELIST_NOT_FOUND: u64 = 7;

    public entry fun init_tiers(
        creator: &signer,
    ) {
        coin::register<AptosCoin>(creator);
        let constructor_ref = object::create_object_from_account(creator);
        let whitelist_obj_signer = &object::generate_signer(&constructor_ref);
        let delete_ref = object::generate_delete_ref(&constructor_ref);
        // make whitelist object soulbound
        object::disable_ungated_transfer(&object::generate_transfer_ref(&constructor_ref));
        move_to(
            whitelist_obj_signer,
            Whitelist {
                map: simple_map::create<String, MintTier>(),
            },
        );
        move_to(
            creator,
            ObjectInfo {
                whitelist_obj: object::object_from_constructor_ref(&constructor_ref),
                whitelist_addr: signer::address_of(whitelist_obj_signer),
                delete_ref,
            }
        );
    }

    /// Facilitates adding or updating tiers. If the whitelist tier already exists, update its values- keep the addresses the same
    public entry fun upsert_tier_config(
        creator: &signer,
        tier_name: String,
        open_to_public: bool,
        price: u64,
        start_time: u64,
        end_time: u64,
        per_user_limit: u64,
    ) acquires Whitelist, ObjectInfo {
        assert!(start_time < end_time, error::invalid_argument(ESTART_TIME_AFTER_END_TIME));
        let creator_addr = signer::address_of(creator);

        if (!whitelist_exists(creator_addr)) {
            init_tiers(creator);
        };

        let (_, whitelist_addr) = get_whitelist_info(creator_addr);
        let whitelist_config = borrow_global_mut<Whitelist>(whitelist_addr);

        if (simple_map::contains_key(&whitelist_config.map, &tier_name)) {
            let tier = simple_map::borrow_mut(&mut whitelist_config.map, &tier_name);
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
            simple_map::add(&mut whitelist_config.map, tier_name, mint_tier);
        };
    }
    

    public entry fun add_addresses_to_tier(
        creator: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires Whitelist, ObjectInfo {
        let creator_addr = signer::address_of(creator);
        let (_, whitelist_addr) = get_whitelist_info(creator_addr);

        let map = &mut borrow_global_mut<Whitelist>(whitelist_addr).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);
        vector::for_each(addresses, |user_addr| {
            // note that this will abort in `table` if the address exists already- use `upsert` to ignore this
            smart_table::add(&mut mint_tier.addresses, user_addr, 0);
        });
    }

    public entry fun remove_addresses_from_tier(
        creator: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires Whitelist, ObjectInfo {
        let creator_addr = signer::address_of(creator);
        let (_, whitelist_addr) = get_whitelist_info(creator_addr);

        let map = &mut borrow_global_mut<Whitelist>(whitelist_addr).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);
        vector::for_each(addresses, |user_addr| {
            // note that this will abort in `table` if the address is not found
            smart_table::remove(&mut mint_tier.addresses, user_addr);
        });
    }

    public fun deduct_one_from_tier(
        creator: &signer,
        minter: &signer,
        tier_name: String,
    ) acquires Whitelist, ObjectInfo {
        let creator_addr = signer::address_of(creator);
        let (_, whitelist_addr) = get_whitelist_info(creator_addr);

        let map = &mut borrow_global_mut<Whitelist>(whitelist_addr).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);

        // assert not too early and not too late
        let now = timestamp::now_seconds();
        assert!(now > mint_tier.start_time, error::permission_denied(EMINT_NOT_STARTED));
        assert!(now < mint_tier.end_time, error::permission_denied(EMINT_ENDED));

        let minter_addr = signer::address_of(minter);
        // if `addresses` doesn't contain the minter address, abort if the tier is not open to the public, otherwise add it
        if (!smart_table::contains(&mint_tier.addresses, minter_addr)) {
            if (mint_tier.open_to_public) {
                // open to public but address not in whitelist, add it to list with 0 mints
                smart_table::add(&mut mint_tier.addresses, minter_addr, 0);
            } else {
                // not open to public and address not in whitelist, abort
                abort error::permission_denied(EACCOUNT_NOT_IN_TIER)
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

    /// destroys everything related to the contract:
    /// 1. the ObjectInfo resource on the owner's account
    /// 2. ObjectCore on the Object<Whitelist>
    /// 3. Whitelist at the object's address
    public entry fun destroy(
        owner: &signer,
    ) acquires ObjectInfo, Whitelist {
        let owner_addr = signer::address_of(owner);
        let (_, whitelist_addr) = get_whitelist_info(owner_addr);
        let ObjectInfo {
            whitelist_obj: _,
            whitelist_addr: _,
            delete_ref,
        } = move_from<ObjectInfo>(owner_addr);

        object::delete(delete_ref);

        let Whitelist {
            map
        } = move_from<Whitelist>(whitelist_addr);

        simple_map::destroy(map, |_k| { }, |v| {
            let MintTier {
                open_to_public: _,
                addresses,
                price: _,
                start_time: _,
                end_time: _,
                per_user_limit: _,
            } = v;
            smart_table::destroy(addresses);
        });
    }

    /// Since the ObjectInfo will never exist without the Object, we only need to check if ObjectInfo exists
    /// if in some edge case the object is somehow not found, it will still error, so this is mostly just an informative error message
    /// init_tiers(...) creates both, destroy(...) removes both, they never exist independently
    fun whitelist_exists(
        creator_addr: address,
    ): bool {
        exists<ObjectInfo>(creator_addr)
    }


    #[view]
   public fun get_whitelist_info(
        creator_addr: address,
    ): (Object<Whitelist>, address) acquires ObjectInfo {
        assert!(whitelist_exists(creator_addr), error::not_found(EWHITELIST_NOT_FOUND));
        let whitelist_obj_info = borrow_global<ObjectInfo>(creator_addr);
        let obj = whitelist_obj_info.whitelist_obj;
        let obj_addr = whitelist_obj_info.whitelist_addr;
        (obj, obj_addr)
    }

    #[view]
    public fun address_in_tier(
        creator_addr: address,
        account_addr: address,
        tier_name: String,
    ): bool acquires Whitelist, ObjectInfo {
        let (_, whitelist_addr) = get_whitelist_info(creator_addr);
        
        let map = &borrow_global<Whitelist>(whitelist_addr).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow(map, &tier_name);

        smart_table::contains(&mint_tier.addresses, account_addr)
    }

    #[view]
    public fun num_used(
        creator_addr: address,
        account_addr: address,
        tier_name: String,
    ): u64 acquires Whitelist, ObjectInfo {
        let (_, whitelist_addr) = get_whitelist_info(creator_addr);

        assert!(address_in_tier(creator_addr, account_addr, tier_name), error::permission_denied(EACCOUNT_NOT_IN_TIER));
        let map = &borrow_global<Whitelist>(whitelist_addr).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow(map, &tier_name);

        *smart_table::borrow(&mint_tier.addresses, account_addr)
    }

    #[view]
    public fun get_tier_info(
        creator_addr: address,
        tier_name: String,
    ): (bool, u64, u64, u64, u64) acquires Whitelist, ObjectInfo {
        let (_, whitelist_addr) = get_whitelist_info(creator_addr);

        let map = &borrow_global<Whitelist>(whitelist_addr).map;
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow(map, &tier_name);
        (
            mint_tier.open_to_public,
            mint_tier.price,
            mint_tier.start_time,
            mint_tier.end_time,
            mint_tier.per_user_limit,
        )
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
    ) acquires Whitelist, ObjectInfo {

        // 1.  Initialize account a, b, c with 4, 3, and 2 APT each.
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        let creator_addr = signer::address_of(creator);
        let address_a = signer::address_of(account_a);
        let address_b = signer::address_of(account_b);
        let address_c = signer::address_of(account_c);

        // 2.  Creator creates 3 tiers
        init_tiers(creator);

        let (whitelist_obj, whitelist_addr) = get_whitelist_info(creator_addr);
        assert!(!object::ungated_transfer_allowed(whitelist_obj), 0);

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

        let (open_to_public_tier1, price_tier1, start_tier1, end_tier1, num_tier1) = get_tier_info(creator_addr, string::utf8(b"tier1"));
        assert!(
            open_to_public_tier1 == !open_to_public &&
            price_tier1 == 0 &&
            start_tier1 == DEFAULT_START_TIME &&
            end_tier1 == DEFAULT_END_TIME &&
            num_tier1 == 3,
            1
        );

        assert!( address_in_tier(creator_addr, address_a, string::utf8(b"tier1")) &&
                !address_in_tier(creator_addr, address_b, string::utf8(b"tier1")) &&
                 address_in_tier(creator_addr, address_a, string::utf8(b"tier2")) &&
                 address_in_tier(creator_addr, address_b, string::utf8(b"tier2")) &&
                !address_in_tier(creator_addr, address_c, string::utf8(b"tier3")),
                2
        );

        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier2"));
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier2"));
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier3"));

        deduct_one_from_tier(creator, account_b, string::utf8(b"tier2"));
        deduct_one_from_tier(creator, account_b, string::utf8(b"tier2"));
        deduct_one_from_tier(creator, account_b, string::utf8(b"tier3"));

        deduct_one_from_tier(creator, account_c, string::utf8(b"tier3"));

        assert!(
            num_used(creator_addr, address_a, string::utf8(b"tier1")) == 3 &&
            num_used(creator_addr, address_a, string::utf8(b"tier2")) == 2 &&
            num_used(creator_addr, address_a, string::utf8(b"tier3")) == 1 &&
            num_used(creator_addr, address_b, string::utf8(b"tier2")) == 2 &&
            num_used(creator_addr, address_b, string::utf8(b"tier3")) == 1 &&
            num_used(creator_addr, address_c, string::utf8(b"tier3")) == 1,
            3
        );

        assert!(coin::balance<AptosCoin>(address_a) == 0, 4);
        assert!(coin::balance<AptosCoin>(address_b) == 0, 5);
        assert!(coin::balance<AptosCoin>(address_c) == 0, 6);

        assert!(coin::balance<AptosCoin>(creator_addr) == 10, 7);

        destroy(creator);
        assert!(!whitelist_exists(creator_addr), 8);
        assert!(!exists<Whitelist>(whitelist_addr), 9);
        assert!(!object::is_object(whitelist_addr), 10);
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x60000, location = whitelist_example::whitelist)]
    public fun test_tier_not_found(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
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
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), !open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 3);
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50002, location = whitelist_example::whitelist)]
    public fun test_no_mints_left(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50003, location = whitelist_example::whitelist)]
    public fun test_mint_not_started(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
        upsert_tier_config(creator, string::utf8(b"tier2"), open_to_public, 0, DEFAULT_CURRENT_TIME + 1, DEFAULT_END_TIME + 1, 1);
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier2"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50004, location = whitelist_example::whitelist)]
    public fun test_mint_ended(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
        upsert_tier_config(creator, string::utf8(b"tier2"), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME - 1, 1);
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier2"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50005, location = whitelist_example::whitelist)]
    public fun test_not_enough_coins(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 100000000, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        deduct_one_from_tier(creator, account_a, string::utf8(b"tier1"));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x10006, location = whitelist_example::whitelist)]
    public fun test_start_time_after_end_time(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        let open_to_public = true;
        upsert_tier_config(creator, string::utf8(b"tier1"), open_to_public, 100000000, DEFAULT_START_TIME + 1, DEFAULT_START_TIME, 1);
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x60007, location = whitelist_example::whitelist)]
    public fun test_whitelist_obj_doesnt_exist(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        init_tiers(creator);
        destroy(creator);
        get_whitelist_info(signer::address_of(creator));
    }

    
    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    public fun test_destroy(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Whitelist, ObjectInfo {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME);
        let creator_addr = signer::address_of(creator);
        init_tiers(creator);
        let (_, whitelist_addr) = get_whitelist_info(creator_addr);
        upsert_tier_config(creator, string::utf8(b"tier1"), false, 9001, 0, 1, 1337);
        upsert_tier_config(creator, string::utf8(b"tier2"), false, 9001, 0, 1, 1337);
        add_addresses_to_tier(creator, string::utf8(b"tier1"), vector<address> [@0x0]);
        add_addresses_to_tier(creator, string::utf8(b"tier2"), vector<address> [@0x0]);
        destroy(creator);
        init_tiers(creator);
        destroy(creator);
        assert!(!whitelist_exists(creator_addr), 0);
        assert!(!exists<Whitelist>(whitelist_addr), 1);
        assert!(!object::is_object(whitelist_addr), 2);
    }
}