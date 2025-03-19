//# publish
module 0x66::test {
    use 0x1::signer;

    struct Work(||u64) has store, key;

    #[persistent]
    fun should_be_storable(): u64 {
        3
    }

    entry fun store(s: &signer) {
        move_to(s, Work(should_be_storable))
    }

    entry fun exec(s: &signer) acquires Work {
        let work = move_from<Work>(signer::address_of(s));
        assert!(work() == 3)
    }
}

//# run 0x66::test::store --signers 0x66

//# run 0x66::test::exec --signers 0x66
