//# init --validators Bob Carrol
//#      --addresses Alice=0x05e7f5fc19d8a0a4717b3c182ce0b4c4
//#      --private-keys Alice=a1a26620fa5dda494bede49d416425a342bf7d3d868e197d43f07c7ad7f1d321

//# run --signers DiemRoot --args 0 @Alice x"4fe05e2099bdc4defb69ce6f59b7b082" b"alice"
//#     -- 0x1::AccountCreationScripts::create_validator_operator_account

//# block --proposer 0x0 --time 0

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::ValidatorConfig;

    fun main(_dr: signer, account: signer) {
        assert!(DiemTimestamp::now_microseconds() == 0, 999);
        // register alice as bob's delegate
        ValidatorConfig::set_operator(&account, @Alice);
    }
}

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {
        // set a new config locally
        ValidatorConfig::set_config(&account, @Bob,
                                    x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
                                    x"", x"");
    }
}

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        // update is too soon, will fail
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}

//# block --proposer Bob --time 300000000

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        // update is too soon, will not trigger the reconfiguration
        assert!(DiemTimestamp::now_microseconds() == 300000000, 999);
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}

//# block --proposer Bob --time 300000001

//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {

        // update is in exactly 5 minutes and 1 microsecond, so will succeed
        assert!(DiemTimestamp::now_microseconds() == 300000001, 999);
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}

//# block --proposer Bob --time 600000000

//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {

        // too soon to reconfig, but validator have not changed, should succeed but not reconfigure
        assert!(DiemTimestamp::now_microseconds() == 600000000, 999);
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}


//# block --proposer Bob --time 600000002

//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {

        // good to reconfig
        assert!(DiemTimestamp::now_microseconds() == 600000002, 999);
        ValidatorConfig::set_config(&account, @Bob,
                                    x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
                                    x"", x"");
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}

//# block --proposer Bob --time 600000003

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {

        DiemSystem::remove_validator(&account, @Bob);
        assert!(!DiemSystem::is_validator(@Bob), 77);
        assert!(DiemSystem::is_validator(@Carrol), 78);
    }
}

//# block --proposer Carrol --time 600000004

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemTimestamp;
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {

        // add validator back
        assert!(DiemTimestamp::now_microseconds() == 600000004, 999);
        DiemSystem::add_validator(&account, @Bob);
        assert!(DiemSystem::is_validator(@Bob), 79);
        assert!(DiemSystem::is_validator(@Carrol), 80);
    }
}

//# block --proposer Bob --time 900000004

//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {

        // update too soon
        assert!(DiemTimestamp::now_microseconds() == 900000004, 999);
        ValidatorConfig::set_config(&account, @Bob,
                                    x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
                                    x"", x"");
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}

//# block --proposer Bob --time 900000005

//# run --admin-script --signers DiemRoot Alice --show-events
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    fun main(_dr: signer, account: signer) {

        // good to reconfigure
        assert!(DiemTimestamp::now_microseconds() == 900000005, 999);
        ValidatorConfig::set_config(&account, @Bob,
                                    x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
                                    x"", x"");
        DiemSystem::update_config_and_reconfigure(&account, @Bob);
    }
}
