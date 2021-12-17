//# init --validators Viola
//#      --addresses Vasp=0xd29183499aee9bb4e733e25f13e30fe5
//#                  Child=0x61b15294f8ea4acbce5c335e0a9238b2
//#                  Vivian=0x7d21827b5347125f90e5b0d1e156ac85
//#                  Otto=0x8387230c77aba855df074f3d60b152dc
//#      --private-keys Vasp=7d8a4711ce306575ab55ad14071c4952a3d2d9a06c80842f3258dadb389f69a9
//#                     Child=a8e96dbfec8902f52ec54495136e0c0406c78ff664f45e6fd9d3de63a3594c02
//#                     Vivian=8a448876ad9538148699aee1b2073a86c4cc56a240203b91309c314aaaecc92f
//#                     Otto=288be08d74e4eb2856ebcea4749c316612501c7bce5729052fce5f27c2684f8e

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

//# block --proposer Viola --time 3

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

//# block --proposer Viola --time 4

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



// DiemRoot should not be able to add a balance.
//
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DiemAccount::add_currency_for_test<XUS>(&account);
}
}


// TreasuryCompliance should not be able to add a balance.
//
//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DiemAccount::add_currency_for_test<XUS>(&account);
}
}


// Validators and ValidatorOperators should not be able to add a balance.
//
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
fun main(_dr: signer, account: signer) {
    DiemAccount::create_validator_account(&account, @Vivian, x"67230339dcf6aed33ff47ba7dc568127", b"owner_name");
    DiemAccount::create_validator_operator_account(&account, @Otto, x"a568b56c848ac6690171a74c4d278d5f", b"operator_name")
}
}

// Check validator case.
//
//# run --admin-script --signers DiemRoot Vivian
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DiemAccount::add_currency_for_test<XUS>(&account);
}
}

// Check validator operator case.
//
//# run --admin-script --signers DiemRoot Otto
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DiemAccount::add_currency_for_test<XUS>(&account);
}
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0
//#            @Vasp
//#            x"5c7053188041eeb694b489b1559d9393"
//#            b"vasp"
//#            false
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# run --signers Vasp
//#     --type-args 0x1::COIN::COIN
//#     -- 0x1::AccountAdministrationScripts::add_currency_to_account

//# run --signers Vasp
//#     --type-args 0x1::COIN::COIN
//#     --args @Child x"1c83b62021564659770f15ce5ae73031" false 0
//#     -- 0x1::AccountCreationScripts::create_child_vasp_account

//# run --signers Child
//#     --type-args 0x1::XDX::XDX
//#     -- 0x1::AccountAdministrationScripts::add_currency_to_account
