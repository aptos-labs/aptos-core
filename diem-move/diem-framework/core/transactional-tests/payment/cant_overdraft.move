//# init --parent-vasps Alice Bob

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 5000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Check that Alice has exactly the amount she got from DD.
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 5000, 42);
    }
}

// Alice transfers 5000 XUS to Bob.
//# run --type-args 0x1::XUS::XUS --signers Alice --args @Bob 5000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Alice should no balance after the transaction.
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 0, 42);
    }
}

// Check that Alice cannot overdraft and send money to Bob.
//# run --type-args 0x1::XUS::XUS --signers Alice --args @Bob 1 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata
