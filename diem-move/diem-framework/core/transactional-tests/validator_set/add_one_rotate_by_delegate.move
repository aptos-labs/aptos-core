//# init --validators Bob Carol
//#      --addresses Alice=0x2e3a0b7a741dae873bf0f203a82dfd52
//#      --private-keys Alice=e1acb70a23dba96815db374b86c5ae96d6a9bc5fff072a7a8e55a1c27c1852d8

// Make bob a validator, set alice as bob's delegate.
// Test that alice can rotate bob's key and invoke reconfiguration.

//# run --signers DiemRoot --args 0 @Alice x"4ee1afe9d572c4eddc3e367e07bf756e" b"alice"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --signers Bob --args b"alice" @Alice
//#     -- 0x1::ValidatorAdministrationScripts::set_validator_operator

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::ValidatorConfig;
    // test alice can rotate bob's consensus public key
    fun main(_dr: signer, account: signer) {
    let account = &account;
        assert!(ValidatorConfig::get_operator(@Bob) == @Alice, 44);
        ValidatorConfig::set_config(account, @Bob, x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c", x"", x"");

        // check new key is "20"
        let config = ValidatorConfig::get_config(@Bob);
        assert!(*ValidatorConfig::get_consensus_pubkey(&config) == x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c", 99);
    }
}

//# block --proposer Carol --time 300000001

//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;

    fun main(_dr: signer, account: signer) {
    let account = &account;
        ValidatorConfig::set_config(account, @Bob, x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", x"", x"");
        // the local validator's key is now different from the one in the validator set
        assert!(ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Bob)) !=
               ValidatorConfig::get_consensus_pubkey(&ValidatorConfig::get_config(@Bob)), 99);
        DiemSystem::update_config_and_reconfigure(account, @Bob);
        // the local validator's key is now the same as the key in the validator set
        assert!(ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Bob)) ==
               ValidatorConfig::get_consensus_pubkey(&ValidatorConfig::get_config(@Bob)), 99);
        // check bob's public key is updated
        let validator_config = DiemSystem::get_validator_config(@Bob);
        assert!(*ValidatorConfig::get_consensus_pubkey(&validator_config) == x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", 99);
    }
}
