script {
    fun unsafe_public_entry(_attacker: &signer) {
        0x1::some_randapp::unsafe_public_call();
    }
}
