//# init --parent-vasps Alice Bob

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Bob some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::Test {
    use DiemFramework::XUS::XUS;
    use DiemFramework::DiemAccount;

    public(script) fun pay_bob_bad(account: signer) {
        let with_cap = DiemAccount::extract_withdraw_capability(&account);
        DiemAccount::pay_from<XUS>(&with_cap, @Bob, 514, x"", x"");
        DiemAccount::restore_withdraw_capability(with_cap);
        assert!(false, 1337);
    }
}

//# run --signers Alice -- 0xA550C18::Test::pay_bob_bad

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::XUS::XUS;
use DiemFramework::DiemAccount;

fun main() {
    assert!(DiemAccount::balance<XUS>(@Bob) == 10000, 1338);
}
}
