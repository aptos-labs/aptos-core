module 0x1::bcs_function_values_test {
    use std::bcs;

    public fun public_function(x: u64): u64 {
        x
    }

    #[persistent]
    public fun public_persistent_function(x: u64): u64 {
        x
    }

    public(friend) fun friend_function(x: u64): u64 {
        x
    }

    #[persistent]
    public(friend) fun friend_persistent_function(x: u64): u64 {
        x
    }

    fun private_function(x: u64): u64 {
        x
    }

    #[persistent]
    fun private_persistent_function(x: u64): u64 {
        x
    }

    fun check_bcs<T>(x: &T, abort_code: u64) {
        let bytes = bcs::to_bytes(x);
        let size = bcs::serialized_size(x);
        assert!(bytes.length() == size, abort_code);
    }

    public entry fun successful_bcs_tests() {
        let f1: |u64|u64 has drop = public_function;
        check_bcs(&f1, 1);

        let f2: |u64|u64 has drop = public_persistent_function;
        check_bcs(&f2, 2);

        let f3: |u64|u64 has drop = friend_persistent_function;
        check_bcs(&f3, 3);

        let f4: |u64|u64 has drop = private_persistent_function;
        check_bcs(&f4, 4);
    }

    public entry fun failure_bcs_test_friend_function() {
        let f: |u64|u64 has drop = friend_function;
        check_bcs(&f, 404);
    }

    public entry fun failure_bcs_test_friend_function_with_capturing() {
        let f: ||u64 has drop = || friend_function(3);
        check_bcs(&f, 404);
    }

    public entry fun failure_bcs_test_private_function() {
        let f: |u64|u64 has drop = private_function;
        check_bcs(&f, 404);
    }

    public entry fun failure_bcs_test_private_function_with_capturing() {
        let f: ||u64 has drop = || private_function(4);
        check_bcs(&f, 404);
    }

    public entry fun failure_bcs_test_anonymous() {
        let f: |u64|u64 has drop = |x| { x };
        check_bcs(&f, 404);
    }

    public entry fun failure_bcs_test_anonymous_with_capturing() {
        let y: u64 = 2;
        let f: |u64|u64 has drop = |x| { x + y };
        check_bcs(&f, 404);
    }
}
