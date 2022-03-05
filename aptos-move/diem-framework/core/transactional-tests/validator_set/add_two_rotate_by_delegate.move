//# init --validators Bob Carol
//#      --addresses Alice=0x2e3a0b7a741dae873bf0f203a82dfd52
//#      --private-keys Alice=e1acb70a23dba96815db374b86c5ae96d6a9bc5fff072a7a8e55a1c27c1852d8

// Register alice as bob's delegate
// test all possible key rotations:
// bob's key by bob - aborts
// bob's key by alice - executes
// alice's key by bob - aborts
// alice's key by alice - executes

//# run --signers DiemRoot --args 0 @Alice x"4ee1afe9d572c4eddc3e367e07bf756e" b"alice"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --signers Bob --args b"alice" @Alice
//#     -- 0x1::ValidatorAdministrationScripts::set_validator_operator

// Check alice can rotate bob's consensus key.
//
//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {
        ValidatorConfig::set_config(&account, @Bob, x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", x"", x"");
        assert!(*ValidatorConfig::get_consensus_pubkey(&ValidatorConfig::get_config(@Bob)) == x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", 99);
    }
}
