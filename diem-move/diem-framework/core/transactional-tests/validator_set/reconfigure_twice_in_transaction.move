//# init --validators Alice Bob Carrol

// Checks that only two reconfigurations can be done within the same transaction and will only emit one reconfiguration
// event.

//# block --proposer Bob --time 2

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Alice);
        DiemSystem::remove_validator(&account, @Bob);
    }
}

//# block --proposer Carrol --time 3

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemSystem;
    fun main(_dr: signer, account: signer) {
        DiemSystem::remove_validator(&account, @Bob);
    }
}
