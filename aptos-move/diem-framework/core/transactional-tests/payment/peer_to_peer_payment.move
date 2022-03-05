//# init --parent-vasps Alice Bob

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Bob some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Check initial balances.
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 1000, 42);
        assert!(DiemAccount::balance<XUS>(@Bob) == 1000, 42);
    }
}

// Alice transfers 7 XUS to Bob.
//# run --type-args 0x1::XUS::XUS --signers Alice --args @Bob 7 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Check final balances.
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 993, 42);
        assert!(DiemAccount::balance<XUS>(@Bob) == 1007, 42);
    }
}
