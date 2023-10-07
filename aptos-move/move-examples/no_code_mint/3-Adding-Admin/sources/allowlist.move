module no_code_mint::allowlist {
    use std::vector;
    use std::string::{String};
    use std::timestamp;
    use std::signer;
    use aptos_std::simple_map::{Self, SimpleMap};
    use std::smart_table::{Self, SmartTable};
    use std::coin;
    use std::error;
    use std::option::{Self, Option};
    use std::aptos_account;
    use std::aptos_coin::{AptosCoin};

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents all the mint tiers available as a map<key: String, value: MintTier>
    /// The `sorted_tiers` field is a vector of the map `keys` sorted by
    /// the corresponding MintTier's start time, ascending.
    /// The mint price for a tier that comes after another must be higher; that is, both the
    /// times and the prices of the MintTiers are in ascending order.
    /// This module does not manage the owning object, it just manages the allowlist resource in it.
    struct Allowlist has key {
        map: SimpleMap<String, MintTier>,
        sorted_tiers: vector<String>,
    }

    /// The price, times, and per user limit for a specific tier; e.g. public, allowlist
    /// the `open_to_public` field indicates there is no restrictions for a requesting address. it is a public mint- it still tracks # of mints though
    struct MintTier has store {
        open_to_public: bool,
        addresses: SmartTable<address, u64>, // used as a set
        price: u64,
        start_time: u64,
        end_time: u64,
        per_user_limit: u64,
    }

    /// The allowlist MintTier with name "tier_name" was not found
    const ETIER_NOT_FOUND: u64 = 0;
    /// The account requesting to mint is not eligible to do so.
    const EACCOUNT_NOT_ELIGIBLE: u64 = 1;
    /// The account requesting to mint doesn't have enough coins to mint
    const ENOT_ENOUGH_COINS: u64 = 2;
    /// The requested start time is not before the end time
    const ESTART_TIME_AFTER_END_TIME: u64 = 3;
    /// There is no allowlist at the given address
    const EWHITELIST_NOT_FOUND: u64 = 4;
    /// The mint tiers must increase in price and time.
    const ETIERS_MUST_INCREASE_IN_PRICE_AND_TIME: u64 = 5;
    /// The per user limit must be greater than zero.
    const EINVALID_PER_USER_LIMIT: u64 = 6;
    /// The tier has not begun.
    const ETIER_NOT_STARTED: u64 = 7;
    /// The tier has already ended.
    const ETIER_HAS_ENDED: u64 = 8;
    /// The user has exceeded their per user limit.
    const ENONE_LEFT: u64 = 9;
    /// The requested end time is not after the present timestamp
    const ENOW_AFTER_END_TIME: u64 = 10;

    public entry fun init_allowlist(
        creator: &signer,
    ) {
        move_to(
            creator,
            Allowlist {
                map: simple_map::create<String, MintTier>(),
                sorted_tiers: vector<String> [],
            },
        );
    }

    /// Facilitates adding or updating tiers. If the allowlist tier already exists, update its values- keep the addresses the same
    public entry fun upsert_tier_config(
        creator: &signer,
        tier_name: String,
        open_to_public: bool,
        price: u64,
        start_time: u64,
        end_time: u64,
        per_user_limit: u64,
    ) acquires Allowlist {
        assert!(per_user_limit > 0, error::invalid_argument(EINVALID_PER_USER_LIMIT));
        assert!(start_time < end_time, error::invalid_argument(ESTART_TIME_AFTER_END_TIME));
        assert!(end_time > timestamp::now_seconds(), error::invalid_argument(ENOW_AFTER_END_TIME));
        let creator_addr = signer::address_of(creator);

        if (!exists_at(creator_addr)) {
            init_allowlist(creator);
        };

        let map = borrow_mut_map(creator_addr);

        if (simple_map::contains_key(map, &tier_name)) {
            let tier = simple_map::borrow_mut(map, &tier_name);
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
            simple_map::add(map, tier_name, mint_tier);
        };

        sort_tiers(creator_addr);
    }

    public entry fun add_to_tier(
        creator: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires Allowlist {
        let creator_addr = signer::address_of(creator);
        let map = borrow_mut_map(creator_addr);
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);
        vector::for_each(addresses, |user_addr| {
            // note that this will abort in `table` if the address exists already- use `upsert` to ignore this
            smart_table::add(&mut mint_tier.addresses, user_addr, 0);
        });
    }

    public entry fun remove_from_tier(
        creator: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires Allowlist {
        let creator_addr = signer::address_of(creator);
        let map = borrow_mut_map(creator_addr);
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        let mint_tier = simple_map::borrow_mut(map, &tier_name);
        vector::for_each(addresses, |user_addr| {
            // note that this will abort in `table` if the address is not found
            smart_table::remove(&mut mint_tier.addresses, user_addr);
        });
    }

    #[view]
    /// Check if the allowlist has at least one tier open to the public or at least one address in it.
    public fun has_valid_tier(creator_addr: address): bool acquires Allowlist {
        let keys = borrow_allowlist(creator_addr).sorted_tiers;
        let (any_valid_tiers, _) = vector::find(&keys, |k| {
            let is_open_to_public = open_to_public(creator_addr, *k);
            let tier = borrow_tier(creator_addr, *k);
            is_open_to_public || smart_table::length(&tier.addresses) >= 1
        });
        any_valid_tiers
    }

    #[view]
    public fun exists_at(creator_addr: address): bool {
        exists<Allowlist>(creator_addr)
    }

    #[view]
    /// This is not only how `try_increment` selects the earliest tier for the minter,
    /// but it is also how you'd query what tier to display for the user on the frontend.
    /// Note that this only returns the tier name.
    public fun get_earliest_tier_available(
        creator_addr: address,
        minter_addr: address
    ): Option<String> acquires Allowlist {
        let keys = borrow_allowlist(creator_addr).sorted_tiers;

        // iterate over all mint tiers to see if any have a valid time and the minter_addr is eligible to mint from that tier
        let (any_valid_tiers, index) = vector::find(&keys, |k| {
            let now = timestamp::now_seconds();
            let tier = borrow_mut_tier(creator_addr, *k);
            let tier_is_active = now > tier.start_time && now < tier.end_time;

            if (tier_is_active) {
                // the user is not in the allowlist but the tier is active
                if (!smart_table::contains(&tier.addresses, minter_addr)) {
                    if (tier.open_to_public) {
                        // if it's open to the public, active, and user is not in it
                        // we can pre-emptively add them to the allowlist to track their mints
                        smart_table::add(&mut tier.addresses, minter_addr, 0);
                        // tier is open to public and user is not in it, so we can assume they have at least 1 mint left
                        true
                    } else {
                        false
                    }
                } else {
                    // user is in tier, check if the user has mints left
                    let count = smart_table::borrow(&tier.addresses, minter_addr);
                    // return: user # of mints thus far < limit
                    (*count < tier.per_user_limit)
                }
            } else {
                // tier is not active
                false
            }
        });

        if (any_valid_tiers) {
            option::some(*vector::borrow(&keys, index))
        } else {
            option::none()
        }
    }

    /// Attempts to add 1 to the auto-selected tier count, selected by the minter address being in the
    /// allowlist and the tier being the earliest time + lowest price, ensured from using the sorted
    /// vector of tiers.
    public fun try_increment(
        creator: &signer,
        minter: &signer,
    ) acquires Allowlist {
        let creator_addr = signer::address_of(creator);
        let minter_addr = signer::address_of(minter);

        let tier_name_option = &mut get_earliest_tier_available(creator_addr, minter_addr);
        assert!(option::is_some(tier_name_option), error::permission_denied(EACCOUNT_NOT_ELIGIBLE));

        let tier_name = option::extract(tier_name_option);

        let mint_tier = borrow_mut_tier(creator_addr, tier_name);

        // ensure minter has enough coins
        assert!(coin::balance<AptosCoin>(minter_addr) >= mint_tier.price, error::permission_denied(ENOT_ENOUGH_COINS));

        // transfer `price` # of AptosCoin from minter to creator
        // redundant safe transfer, registers coins if unregistered
        aptos_account::transfer_coins<AptosCoin>(minter, creator_addr, mint_tier.price);

        // get_earliest_tier_available already checks that the user has mints left, just update the value of the addr in the table now
        let count = smart_table::borrow_mut(&mut mint_tier.addresses, minter_addr);
        *count = *count + 1;
    }

    /// removes the allowlist resource from the creator
    public entry fun destroy(creator: &signer) acquires Allowlist {
        let creator_addr = signer::address_of(creator);
        let Allowlist {
            map,
            sorted_tiers: _,
        } = move_from<Allowlist>(creator_addr);

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

    /// Insertion sort, not intended to be used for more than a ~dozen tiers
    fun sort_tiers(
        creator_addr: address,
    ) acquires Allowlist {
        let sorted_keys = borrow_allowlist(creator_addr).sorted_tiers;
        let keys = if (vector::length(&sorted_keys) != simple_map::length(borrow_map(creator_addr))) {
            // if the keys aren't sorted yet, use the simple map keys
            simple_map::keys(borrow_map(creator_addr))
        } else {
            sorted_keys
        };

        let i = 1;
        let length = vector::length(&keys);
        // compares start_time first, then price as secondary comparator
        while(i < length) {
            let j = i;
            while(true) {
                if (j > 0) {
                    let (tier_1, tier_2) = (*vector::borrow(&keys, j - 1), *vector::borrow(&keys, j));
                    let (time_1, time_2) = (start_time(creator_addr, tier_1), start_time(creator_addr, tier_2));
                    let price_1_gt_2 = price(creator_addr, tier_1) > price(creator_addr, tier_2);
                    // compare start_time first, then price if tiebreaker
                    if (time_1 > time_2 || (time_1 == time_2 && price_1_gt_2)) {
                        vector::swap(&mut keys, j, j - 1);
                        j = j - 1;
                    } else {
                        break
                    };
                } else {
                    break
                };
            };

            i = i + 1;
        };

        // copy the sorted keys to sorted_tiers
        *&mut borrow_mut_allowlist(creator_addr).sorted_tiers = keys;
    }

    public fun assert_exists(creator_addr: address) {
        assert!(exists_at(creator_addr), error::not_found(EWHITELIST_NOT_FOUND));
    }

    inline fun borrow_allowlist(creator_addr: address): &Allowlist acquires Allowlist {
        assert!(exists_at(creator_addr), error::not_found(EWHITELIST_NOT_FOUND));
        borrow_global<Allowlist>(creator_addr)
    }

    inline fun borrow_mut_allowlist(creator_addr: address): &mut Allowlist acquires Allowlist {
        assert!(exists_at(creator_addr), error::not_found(EWHITELIST_NOT_FOUND));
        borrow_global_mut<Allowlist>(creator_addr)
    }

    inline fun borrow_map(creator_addr: address,): &SimpleMap<String, MintTier> acquires Allowlist {
        &borrow_allowlist(creator_addr).map
    }

    inline fun borrow_mut_map(creator_addr: address,): &mut SimpleMap<String, MintTier> acquires Allowlist {
        &mut borrow_mut_allowlist(creator_addr).map
    }

    inline fun borrow_tier(creator_addr: address, tier_name: String): &MintTier acquires Allowlist {
        let map = borrow_map(creator_addr);
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        simple_map::borrow(map, &tier_name)
    }

    inline fun borrow_mut_tier(creator_addr: address, tier_name: String): &mut MintTier acquires Allowlist {
        let map = borrow_mut_map(creator_addr);
        assert!(simple_map::contains_key(map, &tier_name), error::not_found(ETIER_NOT_FOUND));
        simple_map::borrow_mut(map, &tier_name)
    }

    #[view]
    public fun address_in_tier(
        creator_addr: address,
        account_addr: address,
        tier_name: String,
    ): bool acquires Allowlist {
        let tier = borrow_tier(creator_addr, tier_name);
        smart_table::contains(&tier.addresses, account_addr)
    }

    #[view]
    public fun address_eligible_for_tier(
        creator_addr: address,
        account_addr: address,
        tier_name: String,
    ): (bool, bool, bool, bool, bool) acquires Allowlist {
        let in_tier = {
            let tier = borrow_tier(creator_addr, tier_name);
            smart_table::contains(&tier.addresses, account_addr) || tier.open_to_public
        };

        if (!in_tier) {
            (false, false, false, false, false)
        } else {
            let num_used = num_used(creator_addr, account_addr, tier_name);
            let tier = borrow_tier(creator_addr, tier_name); // must be after num_used because of acquires
            let has_any_left = num_used < tier.per_user_limit;
            let now = timestamp::now_seconds();
            let not_too_early = now > tier.start_time;
            let not_too_late = now < tier.end_time;
            let has_enough_coins = coin::balance<AptosCoin>(account_addr) >= tier.price;

            (in_tier, has_any_left, not_too_early, not_too_late, has_enough_coins)
        }
    }

    #[view]
    public fun num_used(
        creator_addr: address,
        account_addr: address,
        tier_name: String,
    ): u64 acquires Allowlist {
        let address_in_tier = address_in_tier(creator_addr, account_addr, tier_name);
        let open_to_public = open_to_public(creator_addr, tier_name);

        if (address_in_tier) {
            let tier = borrow_tier(creator_addr, tier_name);
            *smart_table::borrow(&tier.addresses, account_addr)
        } else {
            if (open_to_public) {
                0
            } else {
                abort error::permission_denied(EACCOUNT_NOT_ELIGIBLE)
            }
        }
    }

    #[view]
    public fun open_to_public(creator_addr: address, tier_name: String): bool acquires Allowlist {
        borrow_tier(creator_addr, tier_name).open_to_public
    }

    #[view]
    public fun price(creator_addr: address, tier_name: String): u64 acquires Allowlist {
        borrow_tier(creator_addr, tier_name).price
    }

    #[view]
    public fun start_time(creator_addr: address, tier_name: String): u64 acquires Allowlist {
        borrow_tier(creator_addr, tier_name).start_time
    }

    #[view]
    public fun end_time(creator_addr: address, tier_name: String): u64 acquires Allowlist {
        borrow_tier(creator_addr, tier_name).end_time
    }

    #[view]
    public fun per_user_limit(creator_addr: address, tier_name: String): u64 acquires Allowlist {
        borrow_tier(creator_addr, tier_name).per_user_limit
    }

    #[view]
    public fun tier_info(
        creator_addr: address,
        tier_name: String
    ): (bool, u64, u64, u64, u64) acquires Allowlist {
        let tier = borrow_tier(creator_addr, tier_name);
        (
            tier.open_to_public,
            tier.price,
            tier.start_time,
            tier.end_time,
            tier.per_user_limit,
        )
    }

    #[test_only]
    use aptos_std::string_utils;

    #[test_only]
    inline fun tier_n(i: u64): String {
        string_utils::format1(&b"tier_{}", i)
    }

    #[test_only]
    /// assert that the tier times in the allowlist are increasing in price and time
    inline fun assert_ascending_tiers(creator_addr: address) acquires Allowlist {
        let keys = &borrow_allowlist(creator_addr).sorted_tiers;

        let ascending = true;
        vector::enumerate_ref(keys, |i, k| {
            let _ = k;
            if (i < vector::length(keys) - 1) {
                let current_tier = borrow_tier(creator_addr, *vector::borrow(keys, i));
                let next_tier = borrow_tier(creator_addr, *vector::borrow(keys, i + 1));
                let increasing_time = current_tier.start_time <= next_tier.start_time;
                let increasing_price = current_tier.price <= next_tier.price;
                ascending = increasing_time && increasing_price;
            };
        });
        assert!(ascending, error::invalid_argument(ETIERS_MUST_INCREASE_IN_PRICE_AND_TIME));
    }

    #[test_only]
    public fun assert_eligible_for_tier(
        creator_addr: address,
        account_addr: address,
        tier_name: String,
    ) acquires Allowlist {
        let (in_tier, has_any_left, not_too_early, not_too_late, has_enough_coins) =
            address_eligible_for_tier(creator_addr, account_addr, tier_name);

        assert!(in_tier, error::permission_denied(EACCOUNT_NOT_ELIGIBLE));
        assert!(has_any_left, error::permission_denied(ENONE_LEFT));
        assert!(not_too_early, error::permission_denied(ETIER_NOT_STARTED));
        assert!(not_too_late, error::permission_denied(ETIER_HAS_ENDED));
        assert!(has_enough_coins, error::permission_denied(ENOT_ENOUGH_COINS));
    }

    // dependencies only used in test, if we link without #[test_only], the compiler will warn us
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
    public fun fast_forward_secs(seconds: u64) {
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + seconds);
    }

    #[test_only]
    public fun setup_test(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
        timestamp: u64,
        coin_multiplier: u64,
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);
        account::create_account_for_test(signer::address_of(creator));
        let (burn, mint) = aptos_framework::aptos_coin::initialize_for_test(aptos_framework);
        setup_account<AptosCoin>(account_a, 4 * coin_multiplier, &mint);
        setup_account<AptosCoin>(account_b, 4 * coin_multiplier, &mint);
        setup_account<AptosCoin>(account_c, 2 * coin_multiplier, &mint);
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
    ) acquires Allowlist {

        // Initialize account a, b, c with 4, 3, and 2 APT each.
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        let creator_addr = signer::address_of(creator);
        let address_a = signer::address_of(account_a);
        let address_b = signer::address_of(account_b);
        let address_c = signer::address_of(account_c);

        // Initialize allowlist
        init_allowlist(creator);

        // tier1: 0 APT, allowlist, DEFAULT_START_TIME, DEFAULT_END_TIME, per_user_limit = 3
        // tier2: 1 APT, allowlist, DEFAULT_START_TIME, DEFAULT_END_TIME, per_user_limit = 2
        // tier3: 2 APT, public, DEFAULT_START_TIME, DEFAULT_END_TIME, per_user_limit = 1
        let open_to_public = true;

        // Creator creates 3 tiers
        upsert_tier_config(creator, tier_n(1), !open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 3);
        upsert_tier_config(creator, tier_n(2), !open_to_public, 1, DEFAULT_START_TIME, DEFAULT_END_TIME, 2);
        upsert_tier_config(creator, tier_n(3),  open_to_public, 2, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);

        add_to_tier(creator, tier_n(1), vector<address> [address_a, address_b]);
        remove_from_tier(creator, tier_n(1), vector<address> [address_b]);
        add_to_tier(creator, tier_n(2), vector<address> [address_a, address_b, address_c]);
        remove_from_tier(creator, tier_n(2), vector<address> [address_c]);

        // ensure upsert works correctly with addresses added, update tier 1 and update it back to normal
        upsert_tier_config(creator, tier_n(1), !open_to_public, 1, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        upsert_tier_config(creator, tier_n(1), !open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 3);

        assert!(
            open_to_public(creator_addr, tier_n(1)) == !open_to_public &&
            price(creator_addr, tier_n(1)) == 0 &&
            start_time(creator_addr, tier_n(1)) == DEFAULT_START_TIME &&
            end_time(creator_addr, tier_n(1)) == DEFAULT_END_TIME &&
            per_user_limit(creator_addr, tier_n(1)) == 3,
            1
        );

        assert!( address_in_tier(creator_addr, address_a, tier_n(1)) &&
                !address_in_tier(creator_addr, address_b, tier_n(1)) &&
                 address_in_tier(creator_addr, address_a, tier_n(2)) &&
                 address_in_tier(creator_addr, address_b, tier_n(2)) &&
                !address_in_tier(creator_addr, address_c, tier_n(3)),
                2
        );

        try_increment(creator, account_a); //, tier_n(1));
        try_increment(creator, account_a); //, tier_n(1));
        try_increment(creator, account_a); //, tier_n(1));
        try_increment(creator, account_a); //, tier_n(2));
        try_increment(creator, account_a); //, tier_n(2));
        try_increment(creator, account_a); //, tier_n(3));

        try_increment(creator, account_b); //, tier_n(2));
        try_increment(creator, account_b); //, tier_n(2));
        try_increment(creator, account_b); //, tier_n(3));

        try_increment(creator, account_c);//, tier_n(3));

        assert!(
            num_used(creator_addr, address_a, tier_n(1)) == 3 &&
            num_used(creator_addr, address_a, tier_n(2)) == 2 &&
            num_used(creator_addr, address_a, tier_n(3)) == 1 &&
            num_used(creator_addr, address_b, tier_n(2)) == 2 &&
            num_used(creator_addr, address_b, tier_n(3)) == 1 &&
            num_used(creator_addr, address_c, tier_n(3)) == 1,
            3
        );

        assert!(coin::balance<AptosCoin>(address_a) == 0, 4);
        assert!(coin::balance<AptosCoin>(address_b) == 0, 5);
        assert!(coin::balance<AptosCoin>(address_c) == 0, 6);

        assert!(coin::balance<AptosCoin>(creator_addr) == 10, 7);

        assert_ascending_tiers(creator_addr);

        destroy(creator);
        assert!(!exists_at(creator_addr), 8);
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    /// Intention here is to test the sorting algorithm
    public fun convoluted_happy_path(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 5);
        let address_a = signer::address_of(account_a);
        let address_b = signer::address_of(account_b);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(0), !open_to_public, 1, DEFAULT_START_TIME + 0, DEFAULT_END_TIME + 0, 1); // 1 APT, mint order: 1
        upsert_tier_config(creator, tier_n(1), !open_to_public, 2, DEFAULT_START_TIME + 1, DEFAULT_END_TIME + 1, 1); // 2 APT, mint order: 2
        upsert_tier_config(creator, tier_n(2), !open_to_public, 5, DEFAULT_START_TIME + 2, DEFAULT_END_TIME + 2, 1); // 5 APT, mint order: 5
        upsert_tier_config(creator, tier_n(3), !open_to_public, 6, DEFAULT_START_TIME + 3, DEFAULT_END_TIME + 3, 1); // 6 APT, mint order: 6
        upsert_tier_config(creator, tier_n(4), !open_to_public, 9, DEFAULT_START_TIME + 4, DEFAULT_END_TIME + 4, 1); // 9 APT, mint order: 9
        upsert_tier_config(creator, tier_n(5), !open_to_public, 8, DEFAULT_START_TIME + 4, DEFAULT_END_TIME + 4, 1); // 8 APT, mint order: 8
        upsert_tier_config(creator, tier_n(6), !open_to_public, 7, DEFAULT_START_TIME + 3, DEFAULT_END_TIME + 3, 1); // 7 APT, mint order: 7
        upsert_tier_config(creator, tier_n(7), !open_to_public, 4, DEFAULT_START_TIME + 2, DEFAULT_END_TIME + 2, 1); // 4 APT, mint order: 4
        upsert_tier_config(creator, tier_n(8), !open_to_public, 3, DEFAULT_START_TIME + 1, DEFAULT_END_TIME + 1, 1); // 3 APT, mint order: 3
        upsert_tier_config(creator, tier_n(9), !open_to_public, 0, DEFAULT_START_TIME + 0, DEFAULT_END_TIME + 0, 1); // 0 APT, mint order: 0
        let order_idx = vector<u64> [9, 0, 1, 8, 7, 2, 3, 6, 5, 4];
        let order = vector::map(order_idx, |v| { tier_n(v) });
        let creator_addr = signer::address_of(creator);
        // check sorted tier order
        assert!(borrow_allowlist(creator_addr).sorted_tiers == order, 0);
        vector::for_each(order_idx, |v| {
            add_to_tier(creator, tier_n(v), vector<address> [address_a]);
        });
        vector::for_each(vector<u64> [9, 0, 8, 2, 6, 4], |v| {
            add_to_tier(creator, tier_n(v), vector<address> [address_b]);
        });

        assert_ascending_tiers(creator_addr);

        // account a starts with 20 apt
        // account b starts with 20 apt
        try_increment(creator, account_a); // mint from tier_9, costs 0
        assert!(coin::balance<AptosCoin>(address_a) == 20, 1);
        try_increment(creator, account_a); // mint from tier_0, costs 1
        assert!(coin::balance<AptosCoin>(address_a) == 19, 2);
        fast_forward_secs(1);
        try_increment(creator, account_b); // mint from tier_8, costs 3
        assert!(coin::balance<AptosCoin>(address_b) == 17, 3);
        try_increment(creator, account_a); // mint from tier_1, costs 2
        assert!(coin::balance<AptosCoin>(address_a) == 17, 4);
        fast_forward_secs(1);
        try_increment(creator, account_b); // mint from tier_2, costs 5
        assert!(coin::balance<AptosCoin>(address_b) == 12, 5);
        try_increment(creator, account_a); // mint from tier_7, costs 4
        assert!(coin::balance<AptosCoin>(address_a) == 13, 6);
        fast_forward_secs(1);
        try_increment(creator, account_a); // mint from tier_3, costs 6
        assert!(coin::balance<AptosCoin>(address_a) == 7, 7);
        fast_forward_secs(1);
        try_increment(creator, account_b); // mint from tier_4, costs 9
        assert!(coin::balance<AptosCoin>(address_b) == 3, 8);
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x60000, location = no_code_mint::allowlist)]
    public fun test_tier_not_found(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        let address_a = signer::address_of(account_a);
        init_allowlist(creator);
        add_to_tier(creator, tier_n(1), vector<address> [address_a]);
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50001, location = no_code_mint::allowlist)]
    public fun test_account_not_allowlisted(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(1), !open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 3);
        try_increment(creator, account_a);//, tier_n(1));
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50001, location = no_code_mint::allowlist)]
    public fun test_no_mints_left(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(1), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        try_increment(creator, account_a);//, tier_n(1));
        try_increment(creator, account_a);//, tier_n(1));
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50001, location = no_code_mint::allowlist)]
    public fun test_mint_not_started(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(1), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        try_increment(creator, account_a);//, tier_n(1));
        upsert_tier_config(creator, tier_n(2), open_to_public, 0, DEFAULT_CURRENT_TIME + 1, DEFAULT_END_TIME + 1, 1);
        try_increment(creator, account_a);//, tier_n(2));
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50001, location = no_code_mint::allowlist)]
    public fun test_mint_ended(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(1), open_to_public, 0, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        try_increment(creator, account_a);//, tier_n(1));
        fast_forward_secs(1);
        try_increment(creator, account_a);//, tier_n(2));
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x50002, location = no_code_mint::allowlist)]
    public fun test_not_enough_coins(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(1), open_to_public, 100000000, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        try_increment(creator, account_a);//, tier_n(1));
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x10003, location = no_code_mint::allowlist)]
    public fun test_start_time_after_end_time(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(1), open_to_public, 100000000, DEFAULT_START_TIME + 1, DEFAULT_START_TIME, 1);
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x1000a, location = no_code_mint::allowlist)]
    public fun test_end_time_after_now(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        let open_to_public = true;
        upsert_tier_config(creator, tier_n(1), open_to_public, 100000000, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        // Fast forward N seconds where N = DEFAULT_END_TIME - now
        fast_forward_secs(DEFAULT_END_TIME - timestamp::now_seconds());
        upsert_tier_config(creator, tier_n(1), open_to_public, 100000000, DEFAULT_START_TIME, DEFAULT_END_TIME, 1);
        assert_ascending_tiers(signer::address_of(creator));
    }

    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 0x60004, location = no_code_mint::allowlist)]
    public fun test_allowlist_obj_doesnt_exist(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        init_allowlist(creator);
        assert_ascending_tiers(signer::address_of(creator));
        destroy(creator);
        borrow_allowlist(signer::address_of(creator));
    }


    #[test(creator = @0xAA, account_a = @0xFA, account_b = @0xFB, account_c = @0xFC, aptos_framework = @0x1)]
    public fun test_destroy(
        creator: &signer,
        account_a: &signer,
        account_b: &signer,
        account_c: &signer,
        aptos_framework: &signer,
    ) acquires Allowlist {
        setup_test(creator, account_a, account_b, account_c, aptos_framework, DEFAULT_CURRENT_TIME, 1);
        let creator_addr = signer::address_of(creator);
        init_allowlist(creator);
        upsert_tier_config(creator, tier_n(1), false, 9001, DEFAULT_START_TIME, DEFAULT_END_TIME, 1337);
        upsert_tier_config(creator, tier_n(2), false, 9001, DEFAULT_START_TIME, DEFAULT_END_TIME, 1337);
        add_to_tier(creator, tier_n(1), vector<address> [@0x0]);
        add_to_tier(creator, tier_n(2), vector<address> [@0x0]);
        assert_ascending_tiers(creator_addr);
        destroy(creator);
        init_allowlist(creator);
        destroy(creator);
        assert!(!exists_at(creator_addr), 0);
    }
}
