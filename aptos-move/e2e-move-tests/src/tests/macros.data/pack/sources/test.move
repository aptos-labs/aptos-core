/// Module for testing the assert! macro
module 0x99::assert {
    entry fun test_assert_returns() {
        assert!(true);
        assert!(true, b"custom error message");
        assert!(true, b"custom error message with arg: {}", 42);
    }

    entry fun test_assert_aborts() {
        assert!(false);
    }

    entry fun test_assert_aborts_with_code() {
        assert!(false, 42);
    }

    entry fun test_assert_aborts_with_message() {
        assert!(false, b"custom error message");
    }

    entry fun test_assert_aborts_with_formatted_message() {
        assert!(false, b"custom error message with arg: {}", 42);
    }
}

/// Module for testing the assert_eq! macro
module 0x99::assert_eq {
    entry fun test_assert_eq_returns() {
        assert_eq!(1, 1);
        assert_eq!(1, 1, b"custom error message");
        assert_eq!(1, 1, b"custom error message with arg: {}", 42);
    }

    entry fun test_assert_eq_aborts() {
        assert_eq!(1, 2);
    }

    entry fun test_assert_eq_aborts_with_message() {
        assert_eq!(1, 2, b"custom error message");
    }

    entry fun test_assert_eq_aborts_with_formatted_message() {
        assert_eq!(1, 2, b"custom error message with arg: {}", 42);
    }
}

/// Module for testing the assert_ne! macro
module 0x99::assert_ne {
    entry fun test_assert_ne_returns() {
        assert_ne!(1, 2);
        assert_ne!(1, 2, b"custom error message");
        assert_ne!(1, 2, b"custom error message with arg: {}", 42);
    }

    entry fun test_assert_ne_aborts() {
        assert_ne!(1, 1);
    }

    entry fun test_assert_ne_aborts_with_message() {
        assert_ne!(1, 1, b"custom error message");
    }

    entry fun test_assert_ne_aborts_with_formatted_message() {
        assert_ne!(1, 1, b"custom error message with arg: {}", 42);
    }
}
