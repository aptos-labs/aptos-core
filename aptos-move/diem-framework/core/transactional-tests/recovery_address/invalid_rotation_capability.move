//# init --parent-vasps Test Vasp1 Vasp2

// TODO: 1. Check if this test has the right name.
//       2, Consider rewriting this as a unit test.

//# publish
module Test::Holder {
    struct Holder<T> has key { x: T }
    public fun hold<T: store>(account: &signer, x: T) {
        move_to(account, Holder<T> { x });
    }

    public fun get<T: store>(addr: address): T
    acquires Holder {
        let Holder<T>{ x } = move_from<Holder<T>>(addr);
        x
    }
}

//# run --admin-script --signers DiemRoot Vasp1
script {
    use Test::Holder;
    use DiemFramework::DiemAccount;

    fun main(_dr: signer, account: signer) {
        Holder::hold(&account, DiemAccount::extract_key_rotation_capability(&account));
    }
}


// Try to create a recovery address with an invalid key rotation capability.
//
//# run --admin-script --signers DiemRoot Vasp2
script {
    use Test::Holder;
    use DiemFramework::DiemAccount;
    use DiemFramework::RecoveryAddress;

    fun main(_dr: signer, account: signer) {
        let cap = Holder::get<DiemAccount::KeyRotationCapability>(@Vasp1);
        RecoveryAddress::publish(&account, cap);
    }
}
