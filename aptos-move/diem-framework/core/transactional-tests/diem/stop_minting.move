//# init --validators Vivian
//#      --addresses DD1=0xdc79c2a4e9500e144f90e65795fc6af3
//#                  DD2=0x1ac490d22ac9007121c234b149a788ce
//#      --private-keys DD1=23eae879e824c272c40035fd6794580e7ebff14701435ae4777f64d2412bc05c
//#                     DD2=0702662a6d1ccc4859ccd47663481764ec8a9452d787da2029fc20310b4f06d8

// BEGIN: registration of a currency

// Change option to CustomModule
//
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemTransactionPublishingOption;

fun main(dr: signer, _dr2: signer) {
    DiemTransactionPublishingOption::set_open_module(&dr, false)
}
}

//# block --proposer Vivian --time 3

// BEGIN: registration of a currency

//# publish --override-signer DiemRoot
module 0x1::COIN {
    use Std::FixedPoint32;
    use DiemFramework::Diem;

    struct COIN has store { }

    public fun initialize(dr_account: &signer, tc_account: &signer) {
        // Register the COIN currency.
        Diem::register_SCS_currency<COIN>(
            dr_account,
            tc_account,
            FixedPoint32::create_from_rational(1, 2), // exchange rate to XDX
            1000000, // scaling_factor = 10^6
            100,     // fractional_part = 10^2
            b"COIN",
        )
    }
}

//# block --proposer Vivian --time 4

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::TransactionFee;
    use 0x1::COIN::{Self, COIN};

    fun main(dr_account: signer, tc_account: signer) {
        COIN::initialize(&dr_account, &tc_account);
        TransactionFee::add_txn_fee_currency<COIN>(&tc_account);
    }
}

// END: registration of a currency

// TODO: see if we can replace some of these admin scripts with script function calls.

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;
    use 0x1::COIN::COIN;
    use DiemFramework::Diem;

    // register dd(1|2) as a preburner
    fun main(_dr: signer, account: signer) {
        let account = &account;
        let prev_mcap1 = Diem::market_cap<XUS>();
        let prev_mcap2 = Diem::market_cap<COIN>();
        DiemAccount::create_designated_dealer<XUS>(
            account,
            @DD1,
            x"1693bba1a6570b52e62ad9f7efb06185",
            x"",
            false,
        );
        DiemAccount::create_designated_dealer<COIN>(
            account,
            @DD2,
            x"50fe4400f7f5d305144ff6ec84780c06",
            x"",
            false,
        );
        DiemAccount::tiered_mint<XUS>(
            account,
            @DD1,
            10,
            0,
        );
        DiemAccount::tiered_mint<COIN>(
            account,
            @DD2,
            100,
            0,
        );
        assert!(Diem::market_cap<XUS>() - prev_mcap1 == 10, 7);
        assert!(Diem::market_cap<COIN>() - prev_mcap2 == 100, 8);
    }
}

// Do some preburning.
//# run --signers DD1 --type-args 0x1::XUS::XUS --args 10
//#     -- 0x1::TreasuryComplianceScripts::preburn

// Do some preburning.
//# run --signers DD2 --type-args 0x1::COIN::COIN --args 100
//#     -- 0x1::TreasuryComplianceScripts::preburn

// Do some burning.
//
//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;
    use 0x1::COIN::COIN;

    fun main(_dr: signer, account: signer) {
        let account = &account;
        let prev_mcap1 = Diem::market_cap<XUS>();
        let prev_mcap2 = Diem::market_cap<COIN>();
        Diem::burn<XUS>(account, @DD1, 10);
        Diem::burn<COIN>(account, @DD2, 100);
        assert!(prev_mcap1 - Diem::market_cap<XUS>() == 10, 9);
        assert!(prev_mcap2 - Diem::market_cap<COIN>() == 100, 10);
    }
}

// Disallow minting.
//
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args false
//#     -- 0x1::TreasuryComplianceScripts::update_minting_ability

// Check that stop minting works
//
//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;

    fun main(_dr: signer, account: signer) {
        let coin = Diem::mint<XUS>(&account, 10); // will abort here
        Diem::destroy_zero(coin);
    }
}
