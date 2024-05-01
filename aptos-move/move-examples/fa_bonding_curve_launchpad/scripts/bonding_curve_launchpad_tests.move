script {
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_std::string;
    use aptos_std::signer;
    use resource_account::bonding_curve_launchpad::{Self};
    // use std::debug;


    const ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT: u64 = 1001;
    const EUSER_APT_BALANCE_INCORRECT: u64 = 10001;

    const STARTING_USER_BALANCE: u64 = 99547900; //* Changes depending on the setup to the account prior to testing.

    fun test_create_fa_liquidity_pair_with_swap(liquidity_pair_creator: &signer) {
        let user_address = signer::address_of(liquidity_pair_creator);
        let apt_resulting_balance = coin::balance<AptosCoin>(user_address);
        assert!(apt_resulting_balance == STARTING_USER_BALANCE, EUSER_APT_BALANCE_INCORRECT);
        // Create FA, LiquidityPair, and Initial Swap.
        let name =  string::utf8(b"SheepyCoin8");
        let symbol = string::utf8(b"SHEEP8");
        bonding_curve_launchpad::create_fa_pair(
            liquidity_pair_creator,
            1_000,
            name,
            symbol,
            803_000_000_000_000_000,
            8,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
        assert!(coin::balance<AptosCoin>(user_address) == STARTING_USER_BALANCE - 1000, EUSER_APT_BALANCE_INCORRECT);
        assert!(bonding_curve_launchpad::get_balance(name, symbol, user_address) == 16_060_000_321, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);

        // Normal Swap. APT -> FA
        bonding_curve_launchpad::swap_apt_to_fa(liquidity_pair_creator, name, symbol, 10000);
        assert!(coin::balance<AptosCoin>(user_address) == STARTING_USER_BALANCE - 11000, EUSER_APT_BALANCE_INCORRECT);
        assert!(bonding_curve_launchpad::get_balance(name, symbol, user_address) == 176_660_026_017, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);

        // Normal Swap. FA -> APT
        bonding_curve_launchpad::swap_fa_to_apt(liquidity_pair_creator, name, symbol, 176_660_026_017);
        assert!(coin::balance<AptosCoin>(user_address) == STARTING_USER_BALANCE, EUSER_APT_BALANCE_INCORRECT);
        assert!(bonding_curve_launchpad::get_balance(name, symbol, user_address) == 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);
    }

}
