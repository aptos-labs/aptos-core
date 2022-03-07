//# init --validators Vivian --parent-vasps Bob
//#      --addresses Dave=0xf42400810cda384c1966c472bfab11f7
//#      --private-keys Dave=f51472493bac725c7284a12c56df41aa3475d731ec289015782b0b9c741b24b5

//# publish --gas-currency COIN
module Bob::M {}

//# block --proposer Vivian --time 2

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

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use 0x1::COIN::COIN;

    fun main() {
        assert!(Diem::approx_xdx_for_value<COIN>(10) == 5, 1);
        assert!(Diem::scaling_factor<COIN>() == 1000000, 2);
        assert!(Diem::fractional_part<COIN>() == 100, 3);
    }
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::COIN::COIN
//#     --args 0 1 3
//#     --gas-currency COIN
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::update_exchange_rate

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use 0x1::COIN::COIN;

    fun main() {
        assert!(Diem::approx_xdx_for_value<COIN>(10) == 3, 4);
    }
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::COIN::COIN
//#     --args 0 @Dave x"4f52c9f095d4e46c0110c7360ae378a8" x"" false
//#     --show-events
//#     -- 0x1::AccountCreationScripts::create_designated_dealer

//# run --signers TreasuryCompliance
//#     --type-args 0x1::COIN::COIN
//#     --args 0 @Dave 10000 0
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::DiemAccount;
    use 0x1::COIN::COIN;

    fun main(_dr: signer, account: signer) {
        DiemAccount::add_currency_for_test<COIN>(&account);
    }
}

//# run --type-args 0x1::COIN::COIN --signers Dave --args @Bob 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish --override-signer DiemRoot
module 0x1::Test {
    public(script) fun nop() {}
}

//# run --signers Bob --gas-currency COIN --gas-price 1 -- 0x1::Test::nop
