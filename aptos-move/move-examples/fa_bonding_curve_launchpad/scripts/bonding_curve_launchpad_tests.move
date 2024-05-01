script {
    use aptos_std::string;
    use aptos_std::signer;
    use resource_account::bonding_curve_launchpad::{Self, FAKey};
    use std::debug;


    const ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT: u64 = 1001;

    fun test_create_fa_liquidity_pair_with_swap(liquidity_pair_creator: &signer) {
        // Create FA, LiquidityPair, and Initial Swap.
        bonding_curve_launchpad::create_fa_pair(
            liquidity_pair_creator,
            1_000,
            string::utf8(b"SheepyCoin7"),
            string::utf8(b"SHEEP7"),
            803_000_000_000_000_000,
            8,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );

        let user_address = signer::address_of(liquidity_pair_creator);
        let name =  string::utf8(b"SheepyCoin7");
        let symbol = string::utf8(b"SHEEP7");
        let fa_resulting_balance = bonding_curve_launchpad::get_balance(name, symbol, user_address);
        assert!(fa_resulting_balance == 16_060_000_321, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);

        // Normal Swap. APT -> FA
        bonding_curve_launchpad::swap_apt_to_fa(liquidity_pair_creator, name, symbol, 10000);
        fa_resulting_balance = bonding_curve_launchpad::get_balance(name, symbol, user_address);
        assert!(fa_resulting_balance == 176_660_026_017, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INCORRECT);
    }

    // fun test_create_fa_liquidity_pair_with_swap(account: &signer) {
    //     bonding_curve_launchpad::create_fa_pair(
    //         account,
    //         1_000,
    //         string::utf8(b"SheepyCoin7"),
    //         string::utf8(b"SHEEP7"),
    //         803_000_000_000_000_000,
    //         8,
    //         string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
    //         string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
    //     );

    //     // let user_address = signer::address_of(account);
    //     // let name =  string::utf8(b"SheepyCoin");
    //     // let symbol = string::utf8(b"SHEEP");
    //     // let fa_resulting_balance = bonding_curve_launchpad::get_balance(name, symbol, user_address);
    //     // debug::print(&fa_resulting_balance);

    // }

}
