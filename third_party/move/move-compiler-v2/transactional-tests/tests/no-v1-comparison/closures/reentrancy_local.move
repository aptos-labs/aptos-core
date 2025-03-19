//# publish
module 0x42::tests {
    struct R(u64) has key;

    public fun local_access_via_lambda_fails(): bool acquires R {
        let add_one = || R[@0x42].0 += 1;
        let r = &mut R[@0x42];
        add_one();
        r.0 += 1;
        false
    }

    public fun init(s: &signer) {
        move_to(s, R(0))
    }
}

//# run 0x42::tests::init --signers 0x42

//# run 0x42::tests::local_access_via_lambda_fails --verbose
