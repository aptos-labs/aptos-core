/// This module is a helper module that's only intended to be used with the post_mint_reveal_nft::minting module.
/// In the minting module, we use `add_or_update_whitelist_stage` to add a new stage to the whitelist, and
/// `add_whitelist_addresses` to add addresses to each whitelist stage.
/// When a user tries to mint a token, we will check if the user's address is on the whitelist by calling
/// `is_user_currently_eligible_for_whitelisted_minting`.
/// After a whitelisted user successfully mint a token, we will call `deduct_user_minting_amount` to deduct the number
/// of minted token from the user's minting limit.
module post_mint_reveal_nft::whitelist {
    use velor_framework::timestamp;
    use post_mint_reveal_nft::bucket_table::{Self, BucketTable};
    use std::error;
    use std::signer;
    use std::vector;

    friend post_mint_reveal_nft::minting;

    /// WhitelistMintConfig stores information about all stages of whitelist.
    /// Most whitelists are one-stage, but we allow multiple stages to be added in case there are multiple rounds of whitelists.
    struct WhitelistMintConfig has key {
        whitelist_configs: vector<WhitelistStage>,
    }

    /// WhitelistMintConfigSingleStage stores information about one stage of whitelist.
    struct WhitelistStage has store {
        whitelisted_address: BucketTable<address, u64>,
        whitelist_mint_price: u64,
        whitelist_minting_start_time: u64,
        whitelist_minting_end_time: u64,
    }

    /// The whitelist start time is not strictly smaller than the whitelist end time.
    const EINVALID_WHITELIST_SETTING: u64 = 1;
    /// The whitelist stage should be added in order. If the whitelist_stage parameter is not equal to the length of the whitelist_configs vector,
    /// it means that the whitelist stage is not added in order and we need to abort.
    const EINVALID_STAGE: u64 = 2;
    /// The specified address does not exist.
    const EACCOUNT_DOES_NOT_EXIST: u64 = 3;
    /// The amount that the user wants to mint exceeds the amount that the user is allowed to mint.
    const EEXCEEDS_MINT_LIMIT: u64 = 4;
    /// Cannot add more addresses to the whitelist after this whitelist minting stage already ends.
    const EINVALID_UPDATE_AFTER_MINTING: u64 = 5;
    /// The given address is not on the whitelist.
    const EACCOUNT_NOT_WHITELISTED: u64 = 6;

    #[view]
    /// Checks if WhitelistMintConfig resource exists.
    public fun whitelist_config_exists(module_address: address): bool {
        exists<WhitelistMintConfig>(module_address)
    }

    #[view]
    /// Returns the number of total stages available.
    public fun get_num_of_stages(module_address: address): u64 acquires WhitelistMintConfig {
        vector::length(&borrow_global<WhitelistMintConfig>(module_address).whitelist_configs)
    }

    #[view]
    /// Checks if the given address is whitelisted in the specified whitelist stage.
    /// Return value first u64 is the minting price of the current stage.
    /// Return value second u64 is the current active whitelist stage.
    /// Return value bool is true if the given address is whitelisted.
    public fun is_user_currently_eligible_for_whitelisted_minting(module_address: address, minter_address: address): (u64, u64, bool) acquires WhitelistMintConfig {
        // If the project doesn't have a whitelist, return false.
        if (!whitelist_config_exists(module_address)) {
            return (0, 0, false)
        };
        let whitelist_mint_config = borrow_global<WhitelistMintConfig>(module_address);
        let now = timestamp::now_seconds();

        let i = 0;
        while (i < vector::length(&whitelist_mint_config.whitelist_configs)) {
            let whitelist_stage = vector::borrow(&whitelist_mint_config.whitelist_configs, i);
            if (whitelist_stage.whitelist_minting_start_time <= now && now < whitelist_stage.whitelist_minting_end_time) {
                let user_is_eligible_for_current_whitelisted_minting = bucket_table::contains(&whitelist_stage.whitelisted_address, &minter_address);
                return (whitelist_stage.whitelist_mint_price, i, user_is_eligible_for_current_whitelisted_minting)
                };
            i = i + 1;
        };

        (0, 0, false)
    }

    /// Initializes the WhitelistMintConfig resource.
    public fun init_whitelist_config(admin: &signer) {
        let config = WhitelistMintConfig {
            whitelist_configs: vector::empty<WhitelistStage>(),
        };
        move_to(admin, config);
    }

    /// Deducts the number of source certificate that the user wants to mint from the user's minting limit.
    public(friend) fun deduct_user_minting_amount(module_address: address, minter_address: address, stage: u64, user_minting_amount: u64) acquires WhitelistMintConfig {
        let whitelist_mint_config = borrow_global_mut<WhitelistMintConfig>(module_address);
        assert!(stage < vector::length(&whitelist_mint_config.whitelist_configs), error::invalid_argument(EINVALID_STAGE));
        let whitelist_stage = vector::borrow_mut(&mut whitelist_mint_config.whitelist_configs, stage);
        assert!(bucket_table::contains(&whitelist_stage.whitelisted_address, &minter_address), error::permission_denied(EACCOUNT_NOT_WHITELISTED));
        let remaining_minting_amount = bucket_table::borrow_mut(&mut whitelist_stage.whitelisted_address, minter_address);
        assert!(*remaining_minting_amount >= user_minting_amount, error::invalid_argument(EEXCEEDS_MINT_LIMIT));
        *remaining_minting_amount = *remaining_minting_amount - user_minting_amount;
    }

    /// Adds a new whitelist stage.
    public fun add_or_update_whitelist_stage(admin: &signer, whitelist_start_time: u64, whitelist_end_time: u64, whitelist_price: u64, whitelist_stage: u64) acquires WhitelistMintConfig {
        assert!(whitelist_start_time < whitelist_end_time, error::invalid_argument(EINVALID_WHITELIST_SETTING));
        if (!whitelist_config_exists(signer::address_of(admin))) {
            init_whitelist_config(admin);
        };
        let num_stages = get_num_of_stages(signer::address_of(admin));
        assert!(whitelist_stage <= num_stages, error::invalid_argument(EINVALID_STAGE));
        let config = borrow_global_mut<WhitelistMintConfig>(signer::address_of(admin));

        // If whitelist_stage equals num_stages, it means that the user wants to add a new stage at the end of the whitelist stages.
        if (whitelist_stage == num_stages) {
            let whitelist_stage = WhitelistStage {
                whitelisted_address: bucket_table::new<address, u64>(4),
                whitelist_mint_price: whitelist_price,
                whitelist_minting_start_time: whitelist_start_time,
                whitelist_minting_end_time: whitelist_end_time,
            };
            vector::push_back(&mut config.whitelist_configs, whitelist_stage);
        } else {
            let whitelist_stage_to_be_updated = vector::borrow_mut(&mut config.whitelist_configs, whitelist_stage);
            whitelist_stage_to_be_updated.whitelist_mint_price = whitelist_price;
            whitelist_stage_to_be_updated.whitelist_minting_start_time = whitelist_start_time;
            whitelist_stage_to_be_updated.whitelist_minting_end_time = whitelist_end_time;
        };
    }

    /// Adds addresses to a specified whitelist stage.
    public fun add_whitelist_addresses(admin: &signer, wl_addresses: vector<address>, mint_limit: u64, whitelist_stage: u64) acquires WhitelistMintConfig {
        let config = borrow_global_mut<WhitelistMintConfig>(signer::address_of(admin));
        assert!(whitelist_stage < vector::length(&config.whitelist_configs), error::invalid_argument(EINVALID_STAGE));
        let whitelist_stage = vector::borrow_mut(&mut config.whitelist_configs, whitelist_stage);
        let now = timestamp::now_seconds();
        assert!(now < whitelist_stage.whitelist_minting_end_time, error::invalid_argument(EINVALID_UPDATE_AFTER_MINTING));

        vector::for_each_ref(&wl_addresses, |wl_address| {
            bucket_table::add(&mut whitelist_stage.whitelisted_address, *wl_address, mint_limit);
        });
    }
}
