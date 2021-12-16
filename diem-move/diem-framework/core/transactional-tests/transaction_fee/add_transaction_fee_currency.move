//# init --validators Vivian --parent-vasps Bob

//# block --proposer Vivian --time 3

// TODO: see if we can replace some of the admin scripts with script function calls.

// BEGIN: registration of a currency

//# publish
module DiemRoot::COIN {
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
use DiemRoot::COIN;
fun main(dr: signer, tc: signer) {
    COIN::initialize(&dr, &tc);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::TransactionFee;
use DiemFramework::Diem;
use DiemRoot::COIN::COIN;
fun main() {
    TransactionFee::pay_fee(Diem::zero<COIN>());
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::TransactionFee;
use DiemRoot::COIN::COIN;
fun main(_dr: signer, tc: signer) {
    TransactionFee::burn_fees<COIN>(&tc);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::TransactionFee;
use DiemRoot::COIN::COIN;
fun main(_dr: signer, tc: signer) {
    TransactionFee::add_txn_fee_currency<COIN>(&tc);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::TransactionFee;
use DiemRoot::COIN::COIN;
fun main(_dr: signer, tc: signer) {
    TransactionFee::add_txn_fee_currency<COIN>(&tc);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::TransactionFee;
use DiemFramework::XDX::XDX;
fun main(dr: signer, tc: signer) {
    TransactionFee::add_txn_fee_currency<XDX>(&tc);
    TransactionFee::burn_fees<XDX>(&tc);
}
}
