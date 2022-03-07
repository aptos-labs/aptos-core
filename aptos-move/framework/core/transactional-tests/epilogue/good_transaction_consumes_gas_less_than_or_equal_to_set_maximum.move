//# init --parent-vasps Alice Bob

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::Test {
    public(script) fun do_work() {}
}

//# run --signers Alice --gas-price 1 --gas-budget 5000 -- 0xA550C18::Test::do_work

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;

fun main() {
    // Ensures that the account was deducted for the gas fee.
    assert!(DiemAccount::balance<XUS>(@Alice) < 10000, 42);
    // Ensures that we are not just charging max_gas for the transaction.
    assert!(DiemAccount::balance<XUS>(@Alice) >= 5000, 43);
}
}
