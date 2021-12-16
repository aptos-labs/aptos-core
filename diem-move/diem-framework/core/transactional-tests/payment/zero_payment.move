//# init --parent-vasps Alice Bob

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Alice tries to transfer 0 XUS to Bob.
//# run --type-args 0x1::XUS::XUS --signers Alice --args @Bob 0 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Check that Alice's balance remains unchanged.
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(DiemAccount::balance<XUS>(@Alice) == 1000, 42);
    }
}
