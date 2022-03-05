//# init --validators Vivian

//# block --proposer Vivian --time 3

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemSystem;
    fun main() {
        DiemSystem::get_validator_config(@Vivian);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        let num_validators = DiemSystem::validator_set_size();
        // TODO: currently 10 test validators are part of the test genesis so we have to count them.
        assert!(num_validators == 1 + 10, 98);
        let index = 0 + 10;
        while (index < num_validators) {
            let addr = DiemSystem::get_ith_validator_address(index);
            DiemSystem::remove_validator(&account, addr);
            index = index + 1;
        };
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemSystem;
    fun main() {
        DiemSystem::get_validator_config(@Vivian);
    }
}
