//# init --validators Alice Vivian Viola
//#      --addresses Bob=0x4b7653f6566a52c9b496f245628a69a0
//#                  Dave=0xeadf5eda5e7d5b9eea4a119df5dc9b26
//#      --private-keys Bob=f5fd1521bd82454a9834ef977c389a0201f9525b11520334842ab73d2dcbf8b7
//#                     Dave=80942c213a3ab47091dfb6979326784856f46aad26c4946aea4f9f0c5c041a79

//# block --proposer Vivian --time 2

//# run --signers DiemRoot --args 0 @Bob x"0441c145bc74bddaa00e098ca91e8435" b"bob"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --signers DiemRoot --args 0 @Dave x"042771c10e14f6436838e2c88a19873d" b"dave"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --signers Alice --args b"bob" @Bob
//#     -- 0x1::ValidatorAdministrationScripts::set_validator_operator

//# run --signers Viola --args b"dave" @Dave
//#     -- 0x1::ValidatorAdministrationScripts::set_validator_operator

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    // Decertify two validators to make sure we can remove both
    // from the set and trigger reconfiguration
    fun main(dr: signer, _dr2: signer) {
        assert!(DiemSystem::is_validator(@Alice) == true, 98);
        assert!(DiemSystem::is_validator(@Vivian) == true, 99);
        assert!(DiemSystem::is_validator(@Viola) == true, 100);
        DiemSystem::remove_validator(&dr, @Vivian);
        assert!(DiemSystem::is_validator(@Alice) == true, 101);
        assert!(DiemSystem::is_validator(@Vivian) == false, 102);
        assert!(DiemSystem::is_validator(@Viola) == true, 103);
    }
}

//# block --proposer Alice --time 300000001

//# run --admin-script --signers DiemRoot Dave --show-events
script {
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    // Two reconfigurations cannot happen in the same block
    fun main(_dr: signer, account: signer) {
        let account = &account;
        // the local validator's key was the same as the key in the validator set
        assert!(ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Viola)) ==
               ValidatorConfig::get_consensus_pubkey(&ValidatorConfig::get_config(@Viola)), 99);
        ValidatorConfig::set_config(account, @Viola,
                                    x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
                                    x"", x"");
        // the local validator's key is now different from the one in the validator set
        assert!(ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Viola)) !=
               ValidatorConfig::get_consensus_pubkey(&ValidatorConfig::get_config(@Viola)), 99);
        let old_num_validators = DiemSystem::validator_set_size();
        DiemSystem::update_config_and_reconfigure(account, @Viola);
        assert!(old_num_validators == DiemSystem::validator_set_size(), 98);
        // the local validator's key is now the same as the key in the validator set
        assert!(ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Viola)) ==
               ValidatorConfig::get_consensus_pubkey(&ValidatorConfig::get_config(@Viola)), 99);
    }
}

//# run --admin-script --signers DiemRoot Bob
script{
    use DiemFramework::DiemSystem;

    fun main(_dr: signer, account: signer) {
        DiemSystem::update_config_and_reconfigure(&account, @Viola);
    }
}

// Freezing does not cause changes to the set.
//
//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::DiemSystem;
    use DiemFramework::AccountFreezing;
    fun main(_dr: signer, tc_account: signer) {
        assert!(DiemSystem::is_validator(@Alice) == true, 101);
        AccountFreezing::freeze_account(&tc_account, @Alice);
        assert!(AccountFreezing::account_is_frozen(@Alice), 1);
        assert!(DiemSystem::is_validator(@Alice) == true, 102);
    }
}
