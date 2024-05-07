#[test_only]
module resource_account::test_bonding_curve_launchpad {
    use aptos_std::string;
    use aptos_std::signer;
    use aptos_std::vector;
    use aptos_std::math64;
    use aptos_framework::account;
    use aptos_framework::resource_account;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use aptos_framework::fungible_asset;
    use aptos_framework::primary_fungible_store;
    use std::debug; //! DEBUG
    use resource_account::bonding_curve_launchpad;
    use resource_account::resource_signer_holder;
    use swap::test_helpers;


    const ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT: u64 = 1001;
    const EUSER_APT_BALANCE_INCORRECT: u64 = 10001;
    const EUSER_APT_BALANCE_IS_ZERO: u64 = 10002;
    const EINCORRECT_FROZEN_STATUS: u64 = 10003;

    fun test_setup_accounts(aptos_framework: &signer, bcl_owner_signer: &signer, resource_signer: &signer, bonding_curve_creator: &signer) {
        account::create_account_for_test(@0x1);
        account::create_account_for_test(@0x922a028b0dbd8ff206074977ae4c5f9fb003ce384242b6253c67192cd2a45ee1);
        account::create_account_for_test(@0x52ddc290f7be79b2583472217af88a8500bdcb16d865e9c2bf4d3c995df0825f);
        resource_account::create_resource_account(bcl_owner_signer, b"random4", vector::empty());
        account::create_account_for_test(@0x803);
        coin::register<AptosCoin>(bonding_curve_creator);

        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        let bcc_coins = coin::mint(1_000_000_000_000_000, &mint_cap);
        let bcc_address = signer::address_of(bonding_curve_creator);
        coin::deposit(bcc_address, bcc_coins);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, bcl_owner_signer = @0x922a028b0dbd8ff206074977ae4c5f9fb003ce384242b6253c67192cd2a45ee1, resource_signer = @0x52ddc290f7be79b2583472217af88a8500bdcb16d865e9c2bf4d3c995df0825f, bonding_curve_creator = @0x803)]
    fun test_bonding_curve_creation(aptos_framework: &signer, bcl_owner_signer: &signer, resource_signer: &signer, bonding_curve_creator: &signer){
        // timestamp::set_time_has_started_for_testing(aptos_framework);
        test_setup_accounts(aptos_framework, bcl_owner_signer, resource_signer, bonding_curve_creator);
        test_helpers::set_up(resource_signer);

        resource_signer_holder::initialize_for_test(resource_signer);
        bonding_curve_launchpad::initialize_for_test(resource_signer);

        let user_address = signer::address_of(bonding_curve_creator);
        let apt_resulting_balance = coin::balance<AptosCoin>(user_address);
        let starting_user_balance = apt_resulting_balance; //* Changes depending on the setup to the account prior to testing.
        assert!(apt_resulting_balance != 0, EUSER_APT_BALANCE_IS_ZERO);
        assert!(apt_resulting_balance == starting_user_balance, EUSER_APT_BALANCE_INCORRECT);
        // Create FA, LiquidityPair, and Initial Swap.
        let name =  string::utf8(b"SheepyCoin8");
        let symbol = string::utf8(b"SHEEP8");
        bonding_curve_launchpad::create_fa_pair(
            bonding_curve_creator,
            1_000,
            name,
            symbol,
            803_000_000,
            8,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
        assert!(coin::balance<AptosCoin>(user_address) == starting_user_balance - 1000, EUSER_APT_BALANCE_INCORRECT);
        assert!(bonding_curve_launchpad::get_balance(name, symbol, user_address) == 16, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);
    }

    #[test(aptos_framework = @0x1, bcl_owner_signer = @0x922a028b0dbd8ff206074977ae4c5f9fb003ce384242b6253c67192cd2a45ee1, resource_signer = @0x52ddc290f7be79b2583472217af88a8500bdcb16d865e9c2bf4d3c995df0825f, bonding_curve_creator = @0x803)]
    fun test_dual_swaps(aptos_framework: &signer, bcl_owner_signer: &signer, resource_signer: &signer, bonding_curve_creator: &signer){
        test_bonding_curve_creation(aptos_framework, bcl_owner_signer, resource_signer, bonding_curve_creator);

        let user_address = signer::address_of(bonding_curve_creator);
        let name =  string::utf8(b"SheepyCoin8");
        let symbol = string::utf8(b"SHEEP8");
        let apt_resulting_balance = coin::balance<AptosCoin>(user_address);
        let starting_user_balance = apt_resulting_balance; //* Changes depending on the setup to the account prior to testing.
        // Normal Swap. APT -> FA
        bonding_curve_launchpad::swap_apt_to_fa(bonding_curve_creator, name, symbol, 10_000);
        assert!(coin::balance<AptosCoin>(user_address) == starting_user_balance - 10_000, EUSER_APT_BALANCE_INCORRECT);
        assert!(bonding_curve_launchpad::get_balance(name, symbol, user_address) == 176_660_026_017, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);

        // Normal Swap. FA -> APT
        bonding_curve_launchpad::swap_fa_to_apt(bonding_curve_creator, name, symbol, 176_660_026_017);
        assert!(coin::balance<AptosCoin>(user_address) == starting_user_balance+1000, EUSER_APT_BALANCE_INCORRECT);
        assert!(bonding_curve_launchpad::get_balance(name, symbol, user_address) == 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);
    }


    #[test(aptos_framework = @0x1, bcl_owner_signer = @0x922a028b0dbd8ff206074977ae4c5f9fb003ce384242b6253c67192cd2a45ee1, resource_signer = @0x52ddc290f7be79b2583472217af88a8500bdcb16d865e9c2bf4d3c995df0825f, bonding_curve_creator = @0x803)]
    fun test_graduation(aptos_framework: &signer, bcl_owner_signer: &signer, resource_signer: &signer, bonding_curve_creator: &signer){
        test_bonding_curve_creation(aptos_framework, bcl_owner_signer, resource_signer, bonding_curve_creator);

        let user_address = signer::address_of(bonding_curve_creator);
        let name =  string::utf8(b"SheepyCoin8");
        let symbol = string::utf8(b"SHEEP8");
        let apt_resulting_balance = coin::balance<AptosCoin>(user_address);
        let starting_user_balance = apt_resulting_balance; //* Changes depending on the setup to the account prior to testing.

        // Over-threshold Swap. APT -> FA
        let grad_apt: u64 = 6_000 * math64::pow(10, (8 as u64));
        assert!(bonding_curve_launchpad::get_is_frozen(name, symbol) == true, EINCORRECT_FROZEN_STATUS);
        bonding_curve_launchpad::swap_apt_to_fa(bonding_curve_creator, name, symbol, grad_apt);
        assert!(bonding_curve_launchpad::get_is_frozen(name, symbol) == false, EINCORRECT_FROZEN_STATUS);

    }

}
