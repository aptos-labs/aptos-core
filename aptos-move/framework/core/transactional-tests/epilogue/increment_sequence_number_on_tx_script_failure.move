//# init --parent-vasps Alice

//# publish
module DiemRoot::Test {
    public(script) fun will_fail() {
        assert!(false, 77);
    }
}

//# run --signers Alice --sequence-number 0 -- 0xA550C18::Test::will_fail

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemAccount;

fun main() {
    assert!(DiemAccount::sequence_number(@Alice) == 1, 42);
}
}
