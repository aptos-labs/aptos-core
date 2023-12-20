#[test_only]
module tournament::test_utils {
    use std::account;
    use std::coin;
    use std::signer;
    use std::timestamp;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::MintCapability;
    use aptos_framework::features;


    public fun fund_account(
        acc: &signer,
        amount: u64,
        mint: &MintCapability<AptosCoin>,
    ) {
        let addr = signer::address_of(acc);
        account::create_account_for_test(addr);
        coin::register<AptosCoin>(acc);
        coin::deposit<AptosCoin>(addr, coin::mint<AptosCoin>(amount, mint));
    }

    public fun init_test_framework(
        aptos_framework: &signer,
        init_timestamp: u64,
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(init_timestamp);
    }

    public fun enable_features_for_test(aptos_framework: &signer) {
        let auids = features::get_auids();
        let concurrent_assets = features::get_concurrent_assets_feature();
        let module_events = features::get_module_event_feature();
        let blake = features::get_blake2b_256_feature();
        features::change_feature_flags(aptos_framework, vector[auids, concurrent_assets, module_events, blake], vector[]);
    }

    public fun fast_forward_seconds(seconds: u64) {
        timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + seconds);
    }

    public fun fast_forward_microseconds(microseconds: u64) {
        timestamp::update_global_time_for_test(timestamp::now_microseconds() + microseconds);
    }

    // also destroys burn
    public fun get_mint_capabilities(
        aptos_framework: &signer,
    ): MintCapability<AptosCoin> {
        let (burn, mint) = aptos_framework::aptos_coin::initialize_for_test(aptos_framework);
        coin::destroy_burn_cap(burn);
        mint
    }

    public fun destroy_mint_capabilities(
        mint: MintCapability<AptosCoin>,
    ) {
        coin::destroy_mint_cap(mint);
    }
}
