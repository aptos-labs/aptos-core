//# init --validators Alice Bob --parent-vasps NotValidator

//# block --proposer Bob --time 2

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Alice);
        assert!(!DiemSystem::is_validator(@Alice), 77);
        assert!(DiemSystem::is_validator(@Bob), 78);
    }
}

//# block --proposer Bob --time 3

//# run --admin-script --signers DiemRoot Bob --show-events
// bob cannot remove itself, only the diem root account can remove validators from the set
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Bob);
    }
}

//# block --proposer Bob --time 4

//# run --admin-script --signers DiemRoot NotValidator --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::add_validator(&account, @Alice);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::add_validator(&account, @NotValidator);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot --show-events

script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::add_validator(&account, @Alice);

        assert!(DiemSystem::is_validator(@Alice), 77);
        assert!(DiemSystem::is_validator(@Bob), 78);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::add_validator(&account, @Alice);
    }
}
