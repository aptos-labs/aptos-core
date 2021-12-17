//# init --parent-vasps Alice --validators Bob

// Check that removing a non-existent validator aborts.

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        // alice cannot remove herself
        DiemSystem::remove_validator(&account, @Alice);
    }
}

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        // alice cannot remove bob
        DiemSystem::remove_validator(&account, @Bob);
    }
}

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        // bob cannot remove alice
        DiemSystem::remove_validator(&account, @Alice);
    }
}
