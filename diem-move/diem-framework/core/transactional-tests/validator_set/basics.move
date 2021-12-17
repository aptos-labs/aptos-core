//# init --validators Bob
//#      --addresses Alex=0x4b7653f6566a52c9b496f245628a69a0
//#                  Alice=0xeadf5eda5e7d5b9eea4a119df5dc9b26
//#      --private-keys Alex=f5fd1521bd82454a9834ef977c389a0201f9525b11520334842ab73d2dcbf8b7
//#                     Alice=80942c213a3ab47091dfb6979326784856f46aad26c4946aea4f9f0c5c041a79
//#      --parent-vasps Carol

// TODO: rewrite as script function calls or unit tests?

//# run --admin-script --signers DiemRoot Carol
script {
    use DiemFramework::DiemSystem;

    fun main(_dr: signer, account: signer) {
        DiemSystem::initialize_validator_set(&account);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemSystem;

    fun main() {
        let len = DiemSystem::validator_set_size();
        DiemSystem::get_ith_validator_address(len);
    }
}

//# run --admin-script --signers DiemRoot Carol
script {
    use DiemFramework::DiemSystem;

    fun main(_dr: signer, account: signer) {
        let account = &account;
        DiemSystem::update_config_and_reconfigure(account, @Bob);
    }
}

//# run --signers DiemRoot --args 0 @Alice x"0441c145bc74bddaa00e098ca91e8435" b"alice"
//#     -- 0x1::AccountCreationScripts::create_validator_account

//# run --signers DiemRoot --args 0 @Alex x"042771c10e14f6436838e2c88a19873d" b"alex"
//#     -- 0x1::AccountCreationScripts::create_validator_account
