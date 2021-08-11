//# run --signers 0xA550C18
//#     --private-key 1b8c20cde2dbb43cd3c709b290ac50dcd2be2a87a3a24544b5a5109bc76ea7fb
script {
    use DiemFramework::DiemAccount::create_validator_account;

    fun main(s: signer) {
        create_validator_account(&s, @0x4d6ecd8b6ac8416825234605ba5d48ea, x"7b85f5f041e10d252aefddc9d41828d7", x"40");
    }
}

//# run --signers 0x4d6ecd8b6ac8416825234605ba5d48ea
//#     --private-key 42e777fa2a36434a318ec39b2d6833228078dd73a377442c46a90c0318090b3d

// correct private key: 42e777fa2a36434a318ec39b2d6833228078dd73a377442c46a90c0318090b3c

script {
    fun main() {}
}
