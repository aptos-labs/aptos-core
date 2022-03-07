//# init --validators Alice Vivian Viola
//#      --addresses Bob=0x4b7653f6566a52c9b496f245628a69a0
//#                  Dave=0xeadf5eda5e7d5b9eea4a119df5dc9b26
//#      --private-keys Bob=f5fd1521bd82454a9834ef977c389a0201f9525b11520334842ab73d2dcbf8b7
//#                     Dave=80942c213a3ab47091dfb6979326784856f46aad26c4946aea4f9f0c5c041a79

// TODO: switch to script functions?

//# run --signers DiemRoot --args 0 @Bob x"0441c145bc74bddaa00e098ca91e8435" b"bob"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --signers DiemRoot --args 0 @Dave x"042771c10e14f6436838e2c88a19873d" b"dave"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {
        // set bob to change alice's key
        ValidatorConfig::set_operator(&account, @Bob);
    }
}

//# run --admin-script --signers DiemRoot Vivian --show-events
script {
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {
        // set dave to change vivian's key
        ValidatorConfig::set_operator(&account, @Dave);
    }
}

//# run --admin-script --signers DiemRoot Bob --show-events

script{
    use DiemFramework::ValidatorConfig;
    // rotate alice's pubkey
    fun main(_dr: signer, account: signer) {
        ValidatorConfig::set_config(&account, @Alice, x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", x"", x"");
    }
}

//# block --proposer Vivian --time 300000001

//# run --admin-script --signers DiemRoot Dave --show-events
script{
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    // rotate vivian's pubkey and then run the block prologue. Now, reconfiguration should be triggered.
    fun main(_dr: signer, account: signer) {
        assert!(*ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Vivian)) !=
               x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", 98);
        ValidatorConfig::set_config(&account, @Vivian, x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", x"", x"");
        DiemSystem::update_config_and_reconfigure(&account, @Vivian);
        // check that the validator set contains Vivian's new key after reconfiguration
        assert!(*ValidatorConfig::get_consensus_pubkey(&DiemSystem::get_validator_config(@Vivian)) ==
               x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", 99);
    }
}

//# block --proposer Vivian --time 600000002

//# run --admin-script --signers DiemRoot Dave --show-events
script{
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    // rotate vivian's pubkey to the same value does not trigger the reconfiguration.
    fun main(_dr: signer, account: signer) {
        ValidatorConfig::set_config(&account, @Vivian, x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", x"", x"");
        DiemSystem::update_config_and_reconfigure(&account, @Vivian);
    }
}
