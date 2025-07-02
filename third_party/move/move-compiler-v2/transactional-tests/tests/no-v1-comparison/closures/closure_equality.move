//# publish
module 0xc0ffee::m {
    public fun identity<T>(t: T): T {
        t
    }

    struct Wrapper<T> has copy, key, drop {
        value: T,
    }

    public fun init(s: &signer) {
        let f: |u64|u64 has copy + store + drop = identity<u64>;
        move_to(s, Wrapper { value: f });
    }

    public fun compare(other: |u64|u64 has copy + store + drop, s: &signer): bool {
        let this = Wrapper<|u64|u64 has copy + store + drop>[std::signer::address_of(s)].value;
        this == other
    }
}

//# run 0xc0ffee::m::init --signers 0xc0ffee

//# run --signers 0xc0ffee
script {
    fun main(s: &signer) {
        // compare resolved and unresolved closure
        let result = 0xc0ffee::m::compare(0xc0ffee::m::identity<u64>, s);
        assert!(result);
    }
}
