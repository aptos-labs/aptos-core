//# init --validators Vivian Viola --parent-vasps Alice

//# block --proposer Viola --time 2

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::ValidatorSystem;
    fun main(_dr: signer, account: signer) {
        ValidatorSystem::remove_validator(&account, @Vivian);
    }
}

//# block --proposer Viola --time 4

//# run --admin-script --signers DiemRoot DiemRoot
script{
    use DiemFramework::ValidatorSystem;

    fun main() {
        assert!(!ValidatorSystem::is_validator(@Vivian), 70);
        assert!(!ValidatorSystem::is_validator(@Alice), 71);
        assert!(ValidatorSystem::is_validator(@Viola), 72);
    }
}
