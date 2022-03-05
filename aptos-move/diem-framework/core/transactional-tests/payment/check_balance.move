//# init --parent-vasps Alice

// Check that Alice has no balance initially.
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 0, 42);
    }
}

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 1337 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Check that Alice has exactly 1337 XUS.
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 1337, 42);
    }
}
