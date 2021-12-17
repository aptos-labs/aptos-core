//# init --validators Vivian --parent-vasps Alice

//# block --proposer Vivian --time 3

// Remove_validator cannot be called on a non-validator.
//
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Alice);
    }
}

// Remove_validator can only be called by the Association.
//
//# run --admin-script --signers DiemRoot Alice --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Vivian);
    }
}

// check: "Keep(ABORTED { code: 2,"

// Should work because Vivian is a validator.
//
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Vivian);
    }
}

// Double-removing Vivian should fail.
//
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Vivian);
    }
}
