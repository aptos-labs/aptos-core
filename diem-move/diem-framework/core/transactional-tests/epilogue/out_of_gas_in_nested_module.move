//# init --parent-vasps Alice

// TODO: there is no guarantee that this will run out of gas in the Vector module.
// Is this something we're fine with?

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::Swapper {
    use Std::Vector;

    public(script) fun swap_it_up(vec_len: u64) {
        let v = Vector::empty();

        let i = 0;
        while (i < vec_len) {
          Vector::push_back(&mut v, i);
          i = i + 1;
        };

        i = 0;

        while (i < vec_len / 2) {
            Vector::swap(&mut v, i, vec_len - i - 1);
            i = i + 1;
        };
    }
}

//# run --signers Alice --gas-price 1 --gas-budget 700 --args 10000 -- 0xA550C18::Swapper::swap_it_up

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;
use DiemFramework::XUS::XUS;

fun main() {
    assert!(DiemAccount::balance<XUS>(@Alice) == 10000 - 700, 42);
}
}
