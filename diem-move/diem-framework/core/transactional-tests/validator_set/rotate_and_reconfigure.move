//# init --validators Bob
//#      --addresses Alice=0x2e3a0b7a741dae873bf0f203a82dfd52
//#      --private-keys Alice=e1acb70a23dba96815db374b86c5ae96d6a9bc5fff072a7a8e55a1c27c1852d8

// Make sure bob can rotate his key locally.
// The diem root account may trigger bulk update to incorporate
// bob's key key into the validator set.

//# run --signers DiemRoot --args 0 @Alice x"4ee1afe9d572c4eddc3e367e07bf756e" b"alice"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::ValidatorConfig;
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        // register alice as bob's delegate
        ValidatorConfig::set_operator(&account, @Alice);

        // assert bob is a validator
        assert!(ValidatorConfig::is_valid(@Bob) == true, 98);
        assert!(DiemSystem::is_validator(@Bob) == true, 98);
    }
}

//# block --proposer Bob --time 300000001

// Rotate bob's key.
//
//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {
        // assert bob is a validator
        assert!(ValidatorConfig::is_valid(@Bob) == true, 98);
        assert!(DiemSystem::is_validator(@Bob) == true, 98);

        assert!(ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Bob)) ==
               ValidatorConfig::get_consensus_pubkey(&ValidatorConfig::get_config(@Bob)), 99);

        // alice rotates bob's public key
        ValidatorConfig::set_config(&account, @Bob,
                                    x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
                                    x"", x"");
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
        // check bob's public key
        let validator_config = DiemSystem::get_validator_config(@Bob);
        assert!(*ValidatorConfig::get_consensus_pubkey(&validator_config) ==
               x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c", 99);
    }
}
