//# init --parent-vasps Alice

// TODO: switch to unit test?

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::XUS::XUS;
    use DiemFramework::DiemAccount;
    use Std::Signer;

    fun main(_dr: signer, account: signer) {
        let sender = Signer::address_of(&account);

        // by default, an account has not delegated its withdrawal capability
        assert!(!DiemAccount::delegated_withdraw_capability(sender), 50);

        // make sure we report that the capability has been extracted
        let cap = DiemAccount::extract_withdraw_capability(&account);
        assert!(DiemAccount::delegated_withdraw_capability(sender), 51);

        // and the sender should be able to withdraw with this cap
        DiemAccount::pay_from<XUS>(&cap, sender, 100, x"", x"");

        // restoring the capability should flip the flag back
        DiemAccount::restore_withdraw_capability(cap);
        assert!(!DiemAccount::delegated_withdraw_capability(sender), 52);
    }
}
