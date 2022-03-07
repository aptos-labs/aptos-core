//# init --validators Vivian
//#      --addresses Sally=0x03cb4a2ce2fcfa4eadcdc08e10cee07b
//#                  Alice=0x9bbff670cb15aa5b24d958b5bb7d85c7
//#                  Bob=0xfa8ab8c8689b1bebfb725b7ff6077606
//#      --private-keys Sally=49fd8b5fa77fdb08ec2a8e1cab8d864ac353e4c013f191b3e6bb5e79d3e5a67d
//#                     Alice=e2eab54aca142743682627820c9857f33130c503f16a2d0f4f02c1230c903862
//#                     Bob=41269558314e48a65d54f1da20fb43464242cf9971e693211b6e37432e1530b3

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

//# run --signers TreasuryCompliance
//#     --type-args 0x1::COIN::COIN
//#     --args 0 @Sally x"344ec1f704209a9a1901321df20da8db" b"sally" false
//#     -- 0x1::AccountCreationScripts::create_designated_dealer

//# run --signers TreasuryCompliance
//#     --type-args 0x1::COIN::COIN
//#     --args 0 @Sally 10000 3
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0
//#            @Alice
//#            x"597ed686d322accf742f899bf9eb5abb"
//#            b"alice"
//#            false
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# run --signers TreasuryCompliance
//#     --type-args 0x1::COIN::COIN
//#     --args 0
//#            @Bob
//#            x"466024245b87d3bf210b97af5423b8cf"
//#            b"bob"
//#            false
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

// Give Alice XUS from DD
//
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Bob COIN from Sally
//
//# run --type-args 0x1::COIN::COIN --signers Sally --args @Bob 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemAccount;
    use 0x1::COIN::COIN;

    fun main(_dr: signer, account: signer) {
        DiemAccount::add_currency_for_test<COIN>(&account);
    }
}

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main(_dr: signer, account: signer) {
        DiemAccount::add_currency_for_test<XUS>(&account);
    }
}

// Alice pays Bob 10 XUS.
//
//# run --type-args 0x1::XUS::XUS --signers Alice --args @Bob 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 0, 0);
        assert!(DiemAccount::balance<XUS>(@Bob) == 10, 1);
    }
}

// Bob pays Alice 10 COIN.
//
//# run --type-args 0x1::COIN::COIN --signers Bob --args @Alice 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Bob pays Alice 10 XUS.
//
//# run --type-args 0x1::XUS::XUS --signers Bob --args @Alice 10 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
    use 0x1::COIN::COIN;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Bob) == 0, 2);
        assert!(DiemAccount::balance<COIN>(@Bob) == 0, 3);
        assert!(DiemAccount::balance<XUS>(@Alice) == 10, 4);
        assert!(DiemAccount::balance<COIN>(@Alice) == 10, 5);
    }
}
