//# init --validators Vivian Viola --parent-vasps Alice

//# block --proposer Viola --time 2

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Vivian);
    }
}

//# block --proposer Viola --time 4

//# run --admin-script --signers DiemRoot DiemRoot
script{
    use DiemFramework::DiemSystem;

    fun main() {
        assert!(!DiemSystem::is_validator(@Vivian), 70);
        assert!(!DiemSystem::is_validator(@Alice), 71);
        assert!(DiemSystem::is_validator(@Viola), 72);
    }
}
