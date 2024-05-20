#[test_only]
module rewards::rewards_tests {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::stake;
    use rewards::rewards;

    #[test(admin = @0xcafe, claimer_1 = @0xdead, claimer_2 = @0xbeef, admin_2 = @0xface)]
    fun test_e2e(admin: &signer, claimer_1: &signer, claimer_2: &signer, admin_2: &signer) {
        stake::initialize_for_test(&account::create_signer_for_test(@0x1));
        rewards::init_for_test(admin);
        // Initialize the admin and admin_2 account with 1000 coins each.
        let apt = stake::mint_coins(1000);
        let apt_2 = stake::mint_coins(1000);
        aptos_account::deposit_coins(signer::address_of(admin), apt);
        aptos_account::deposit_coins(signer::address_of(admin_2), apt_2);

        // Add rewards
        let claimer_1_addr = signer::address_of(claimer_1);
        let claimer_2_addr = signer::address_of(claimer_2);
        rewards::add_rewards(admin, vector[claimer_1_addr, claimer_2_addr], vector[500, 500]);

        // // Cancel for claimer_2
        // assert!(rewards::pending_rewards(claimer_2_addr) == 500, 0);
        // rewards::cancel_rewards(admin, vector[claimer_2_addr]);
        // assert!(rewards::pending_rewards(claimer_2_addr) == 0, 0);

        // // Claim
        // assert!(rewards::pending_rewards(claimer_1_addr) == 500, 0);
        // rewards::claim_reward(claimer_1);
        // assert!(coin::balance<AptosCoin>(claimer_1_addr) == 500, 0);

        // // Transfer Admin Role and add more rewards
        // let admin_2_addr = signer::address_of(admin_2);
        // rewards::transfer_admin_role(admin, admin_2_addr);
        // rewards::add_rewards(admin_2, vector[claimer_1_addr, claimer_2_addr], vector[500, 500]);
    }
}
