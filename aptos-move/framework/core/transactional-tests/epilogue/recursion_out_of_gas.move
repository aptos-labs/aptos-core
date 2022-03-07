//# init --parent-vasps Alice

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::Test {
    public(script) fun rec() {
        rec()
    }
}

//# run --signers Alice --gas-price 1 --gas-budget 700 -- 0xA550C18::Test::rec

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;

fun main() {
    assert!(DiemAccount::balance<XUS>(@Alice) == 10000 - 700, 42);
}
}
