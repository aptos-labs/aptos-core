#[test_only]
module parallel_accounts::parallel_accounts_demo_test {
    use aptos_framework::account::{create_account_for_test, create_signer_for_test};
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use aptos_framework::coin::{Self, destroy_mint_cap, destroy_burn_cap};
    use aptos_framework::aptos_account::assert_account_exists;
    use parallel_accounts::parallel_accounts_demo::{create_resource_account_and_store_cap, resource_account_address, transfer_coins, delegate_transfer_cap, revoke_transfer_cap};
    use std::signer::address_of;

    /// 1 APT
    const APT_1: u64 = 100000000;

    inline fun setup_cap_test(framework: &signer, admin: &signer, payer: &signer, seed: vector<u8>) {
        let admin_address = address_of(admin);
        let payer_address = address_of(payer);

        let (burn, mint) = aptos_coin::initialize_for_test(framework);
        let admin_coins = coin::mint(APT_1 * 3, &mint);
        destroy_burn_cap(burn);
        destroy_mint_cap(mint);
        create_account_for_test(admin_address);
        create_account_for_test(payer_address);

        create_resource_account_and_store_cap(admin, seed);
        let resource_address = resource_account_address(address_of(admin), seed);
        let resource_signer = create_signer_for_test(resource_address);

        let payer_coins = coin::extract(&mut admin_coins, APT_1);
        let sender_coins = coin::extract(&mut admin_coins, APT_1);
        coin::register<AptosCoin>(admin);
        coin::register<AptosCoin>(payer);
        coin::register<AptosCoin>(&resource_signer);
        coin::deposit(admin_address, admin_coins);
        coin::deposit(payer_address, payer_coins);
        coin::deposit(resource_address, sender_coins);
    }

    #[test(framework = @0x1, admin = @0x22, payer = @0x33, receiver = @0x44)]
    fun cap_transfer_coins_not_created_test(
        framework: &signer,
        admin: &signer,
        payer: &signer,
        receiver: &signer
    ) {
        let seed = b"seed1";
        let receiver_address = address_of(receiver);

        setup_cap_test(framework, admin, payer, seed);

        delegate_transfer_cap(admin, payer);
        transfer_coins<AptosCoin>(payer, receiver_address, APT_1);

        assert_account_exists(receiver_address);
        let balance = coin::balance<AptosCoin>(receiver_address);
        assert!(balance == APT_1, 99);
    }

    #[test(framework = @0x1, admin = @0x22, payer = @0x33, receiver = @0x44)]
    #[expected_failure(abort_code = 2, location = parallel_accounts::parallel_accounts_demo)]
    fun cap_transfer_coins_no_delegate(
        framework: &signer,
        admin: &signer,
        payer: &signer,
        receiver: &signer
    ) {
        let seed = b"seed1";
        let receiver_address = address_of(receiver);

        setup_cap_test(framework, admin, payer, seed);

        transfer_coins<AptosCoin>(payer, receiver_address, APT_1);
    }

    #[test(admin = @0x22, payer = @0x33)]
    #[expected_failure(abort_code = 1, location = parallel_accounts::parallel_accounts_demo)]
    fun no_admin_cap_delegate(
        admin: &signer,
        payer: &signer,
    ) {
        create_account_for_test(address_of(admin));
        create_account_for_test(address_of(payer));
        delegate_transfer_cap(admin, payer);
    }

    #[test(admin = @0x22, payer = @0x33)]
    #[expected_failure(abort_code = 1, location = parallel_accounts::parallel_accounts_demo)]
    fun no_admin_cap_revoke(
        admin: &signer,
        payer: &signer,
    ) {
        create_account_for_test(address_of(admin));
        create_account_for_test(address_of(payer));
        revoke_transfer_cap(admin, address_of(payer));
    }

    #[test(admin = @0x22, payer = @0x33)]
    #[expected_failure(abort_code = 2, location = parallel_accounts::parallel_accounts_demo)]
    fun no_delegated_transfer_cap_to_revoke(
        admin: &signer,
        payer: &signer,
    ) {
        create_account_for_test(address_of(admin));
        create_account_for_test(address_of(payer));
        create_resource_account_and_store_cap(admin, b"fun");
        revoke_transfer_cap(admin, address_of(payer))
    }

    #[test(admin = @0x22, payer = @0x33)]
    fun revoke_delegated_transfer_cap(
        admin: &signer,
        payer: &signer,
    ) {
        create_account_for_test(address_of(admin));
        create_account_for_test(address_of(payer));
        create_resource_account_and_store_cap(admin, b"fun");
        delegate_transfer_cap(admin, payer);
        revoke_transfer_cap(admin, address_of(payer))
    }
}