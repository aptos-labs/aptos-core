address 0x42 {

/// The documentation for my module.
module M {
    struct T has key, copy, drop { b: bool }

    public fun create(): T {
        // Return a T with a true value.
        T { b: true }
    }

    public fun publish(account: &signer, t: T) {
        move_to(account, t);
    }
}

#[test_only]
module MTest {
    use M;

    #[test, expected_failure(abort_code = 9)]
    fun create_test() {
        let ident: u128;
        ident = 0xaD1f;
    }

    #[test]
    #[expected_failure(abort_code = 11)]
    fun publish_test() {
        // ...
    }
}
}
