script {
    use aptos_std::string;
    use bonding_curve_launchpad_addr::bonding_curve_launchpad;


    fun test_create_fa_liquidity_pair_with_swap(account: &signer) {
        bonding_curve_launchpad::create_fa_pair(
            account,
            1_000,
            string::utf8(b"SheepyCoin"),
            string::utf8(b"SHEEP"),
            803_000_000_000_000_000,
            8,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
        bonding_curve_launchpad::create_fa_pair(
            account,
            1_000,
            string::utf8(b"PoggieCoin"),
            string::utf8(b"POGGIE"),
            803_000_000_000_000_000,
            8,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );


    }
}
