//# init --parent-vasps Alice Bob

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Bob some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::Test {
    public(script) fun less_work() {}

    public(script) fun more_work() {
        let x = 1;
        while (x < 2000) x = x + 1;
    }
}

//# run --signers Alice --gas-price 1 -- 0xA550C18::Test::less_work

//# run --signers Bob --gas-price 1 -- 0xA550C18::Test::more_work

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;
fun main() {
    // Alice did less work than bob so she should pay less gas.
    assert!(DiemAccount::balance<XUS>(@Bob) < DiemAccount::balance<XUS>(@Alice), 42);
}
}
