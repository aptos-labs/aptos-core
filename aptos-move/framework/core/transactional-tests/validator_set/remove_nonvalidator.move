//# init --parent-vasps Alice --validators Bob

// Check that removing a non-existent validator aborts.

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::ValidatorSystem;
    fun main(_dr: signer, account: signer) {
        // alice cannot remove herself
        ValidatorSystem::remove_validator(&account, @Alice);
    }
}

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::ValidatorSystem;
    fun main(_dr: signer, account: signer) {
        // alice cannot remove bob
        ValidatorSystem::remove_validator(&account, @Bob);
    }
}

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::ValidatorSystem;
    fun main(_dr: signer, account: signer) {
        // bob cannot remove alice
        ValidatorSystem::remove_validator(&account, @Alice);
    }
}
