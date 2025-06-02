//# publish
module 0x66::registry {
    use 0x1::signer;
    struct R<T>(T) has key;
    public fun store<T: store>(s: &signer, x: T) {
        move_to(s, R(x))
    }
    public fun remove<T: store>(s: &signer): T acquires R {
        let R(x) = move_from<R<T>>(signer::address_of(s));
        x
    }
}

//# publish
module 0x66::delayed_work {
    use 0x66::registry;

    struct Work(|u64|u64) has store;

    entry fun initialize(s: &signer) {
        registry::store(s, Work(id_fun))
    }

    entry fun add(s: &signer, amount: u64) {
        let current = registry::remove<Work>(s);
        registry::store(s, Work(|x| more_work(current, amount, x)))
    }

    entry fun eval(s: &signer, amount: u64, expected: u64) {
        let todo = registry::remove<Work>(s);
        assert!(todo(amount) == expected)
    }

    public fun more_work(old: Work, x: u64, y: u64): u64 {
        old(x) + y
    }

    public fun id_fun(x: u64): u64 {
        x
    }
}

//# run 0x66::delayed_work::initialize --signers 0x66

//# run 0x66::delayed_work::add --signers 0x66 --args 5

//# run 0x66::delayed_work::add --signers 0x66 --args 7

//# run 0x66::delayed_work::eval --signers 0x66 --args 3 15
