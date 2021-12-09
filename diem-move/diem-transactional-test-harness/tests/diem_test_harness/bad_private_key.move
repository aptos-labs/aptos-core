//# init --addresses A=0x4d6ecd8b6ac8416825234605ba5d48ea


//# run --signers 0xA550C18
//#     --admin-script
script {
    use DiemFramework::DiemAccount::create_validator_account;

    fun main(dr: signer, _s: signer) {
        create_validator_account(&dr, @0x4d6ecd8b6ac8416825234605ba5d48ea, x"7b85f5f041e10d252aefddc9d41828d7", x"40");
    }
}

//# publish --private-key 42e777fa2a36434a318ec39b2d6833228078dd73a377442c46a90c0318090b3c
module A::M {
    fun foo() {}
}


//# run --signers 0x4d6ecd8b6ac8416825234605ba5d48ea
//#     --private-key 42e777fa2a36434a318ec39b2d6833228078dd73a377442c46a90c0318090b3d
//#     -- 0x4d6ecd8b6ac8416825234605ba5d48ea::M::foo
// correct private key: 42e777fa2a36434a318ec39b2d6833228078dd73a377442c46a90c0318090b3c
