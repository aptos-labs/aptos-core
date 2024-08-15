#[test_only]
module unmanaged_launchpad::unmanaged_launchpad_tests {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::utf8;
    use aptos_std::crypto_algebra::enable_cryptography_algebra_natives;
    use aptos_framework::account;
    use aptos_framework::aptos_coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::object::{Object, owner, root_owner};
    use aptos_framework::randomness;
    use aptos_framework::signer::address_of;
    use aptos_framework::timestamp;

    use aptos_token_objects::collection::Collection;

    use unmanaged_launchpad::unmanaged_launchpad::Self;

    fun setup_test(aptos_framework: &signer) {
        let now: u64 = 10000000;
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(now);
        enable_cryptography_algebra_natives(aptos_framework);
        randomness::initialize_for_testing(aptos_framework);
        randomness::set_seed(x"0000000000000000000000000000000000000000000000000000000000000000");
    }

    fun create_test_collection(creator: &signer, aptos_framework: &signer): Object<Collection> {
        setup_test(aptos_framework);

        unmanaged_launchpad::create_collection_impl(
            creator,
            utf8(b"Test Description"),
            utf8(b"Test Name"),
            utf8(b"Test URI"),
            option::none(),
            option::none(),
            option::none(),
        )
    }

    fun pre_mint_test_tokens(creator: &signer, collection: Object<Collection>, num_tokens: Option<u64>) {
        unmanaged_launchpad::pre_mint_tokens_impl(
            creator,
            collection,
            utf8(b"Token Name"),
            utf8(b"Token URI"),
            utf8(b"Token Description"),
            num_tokens,
            vector[],
            vector[],
            vector[],
        );
    }

    #[test(creator = @0x2, aptos_framework = @0x1)]
    fun test_create_collection(creator: &signer, aptos_framework: &signer) {
        let creator_addr = address_of(creator);
        let collection = create_test_collection(creator, aptos_framework);

        assert!(root_owner(collection) == creator_addr, 1);
    }

    #[test(creator = @0x2, aptos_framework = @0x1)]
    fun test_pre_mint_tokens(creator: &signer, aptos_framework: &signer) {
        let collection = create_test_collection(creator, aptos_framework);

        pre_mint_test_tokens(creator, collection, option::some(10));
    }

    #[test(creator = @0x2, aptos_framework = @0x1)]
    fun test_set_minting_status(creator: &signer, aptos_framework: &signer) {
        let collection = create_test_collection(creator, aptos_framework);

        unmanaged_launchpad::set_minting_status(creator, collection, true);

        assert!(unmanaged_launchpad::ready_to_mint(collection) == true, 1);
    }

    #[test(creator = @0x2, user = @0x3, aptos_framework = @0x1)]
    fun test_mint(creator: &signer, user: &signer, aptos_framework: &signer) {
        let user_addr = address_of(user);
        let collection = create_test_collection(creator, aptos_framework);

        pre_mint_test_tokens(creator, collection, option::some(10));

        unmanaged_launchpad::set_minting_status(creator, collection, true);

        let token = unmanaged_launchpad::mint_impl(user, collection);

        assert!(owner(token) == user_addr, 1);
    }

    #[test(creator = @0x2, user = @0x3, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 327683, location = unmanaged_launchpad::unmanaged_launchpad)]
    fun test_mint_without_ready_status_fails(creator: &signer, user: &signer, aptos_framework: &signer) {
        let collection = create_test_collection(creator, aptos_framework);

        pre_mint_test_tokens(creator, collection, option::some(10));

        unmanaged_launchpad::mint_impl(user, collection);
    }

    #[test(creator = @0x2, user = @0x3, aptos_framework = @0x1)]
    #[expected_failure(abort_code = 458756, location = unmanaged_launchpad::unmanaged_launchpad)]
    fun test_mint_out_tokens(creator: &signer, user: &signer, aptos_framework: &signer) {
        let collection = create_test_collection(creator, aptos_framework);

        pre_mint_test_tokens(creator, collection, option::some(1));

        unmanaged_launchpad::set_minting_status(creator, collection, true);

        // First mint should succeed
        unmanaged_launchpad::mint_impl(user, collection);

        // Second mint should fail since all tokens are minted
        unmanaged_launchpad::mint_impl(user, collection);
    }

    #[test(creator = @0x2, user = @0x3, aptos_framework = @0x1)]
    fun test_mint_with_fee(creator: &signer, user: &signer, aptos_framework: &signer) {
        let user_addr = address_of(user);
        let creator_addr = address_of(creator);
        let mint_fee = 100;

        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        account::create_account_for_test(user_addr);
        account::create_account_for_test(creator_addr);
        coin::register<AptosCoin>(creator);
        coin::register<AptosCoin>(user);
        coin::deposit(user_addr, coin::mint<AptosCoin>(mint_fee, &mint_cap));

        let collection = create_test_collection(creator, aptos_framework);

        pre_mint_test_tokens(creator, collection, option::some(1));

        let mint_fee_category = utf8(b"Secondary Mint Fee");
        unmanaged_launchpad::add_mint_fee(
            creator,
            collection,
            mint_fee,
            mint_fee_category,
            signer::address_of(creator)
        );

        unmanaged_launchpad::set_minting_status(creator, collection, true);
        let token = unmanaged_launchpad::mint_impl(user, collection);
        assert!(owner(token) == user_addr, 1);

        coin::destroy_burn_cap<AptosCoin>(burn_cap);
        coin::destroy_mint_cap<AptosCoin>(mint_cap);
    }
}
