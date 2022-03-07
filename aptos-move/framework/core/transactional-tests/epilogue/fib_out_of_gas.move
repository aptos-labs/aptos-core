//# init --parent-vasps Alice

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::Test {
    fun fib(x: u64): u64 {
        if (x < 2) {
            1
        }
        else {
            fib(x - 1) + fib(x - 2)
        }
    }

    public(script) fun run_fib(x: u64) {
        fib(x);
    }
}

//# run --signers Alice --gas-price 1 --gas-budget 700 --args 20 -- 0xA550C18::Test::run_fib

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;

fun main() {
    assert!(DiemAccount::balance<XUS>(@Alice) == 10000 - 700, 42);
}
}
