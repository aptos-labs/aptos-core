//# init --parent-vasps Alice

//# publish
module DiemRoot::Test {
    public(script) fun will_succeed() {}
}

//# run --signers Alice --sequence-number 0 -- 0xA550C18::Test::will_succeed

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;

fun main() {
    assert!(DiemAccount::sequence_number(@Alice) == 1, 42);
}
}
