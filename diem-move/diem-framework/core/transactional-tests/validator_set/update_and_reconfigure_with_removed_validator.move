//# init --validators Bob Carrol
//#      --addresses Alice=0x2e3a0b7a741dae873bf0f203a82dfd52
//#      --private-keys Alice=e1acb70a23dba96815db374b86c5ae96d6a9bc5fff072a7a8e55a1c27c1852d8

// Make bob a validator, set alice as bob's delegate.
// Test that alice can rotate bob's key and invoke reconfiguration.

//# run --signers DiemRoot --args 0 @Alice x"4ee1afe9d572c4eddc3e367e07bf756e" b"alice"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {
        // register alice as bob's delegate
        ValidatorConfig::set_operator(&account, @Alice);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Bob);
    }
}

//# block --proposer Carrol --time 2

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}
