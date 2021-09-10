module 0x1::A {
    use Std::Vector;

    #[test]
    fun native_abort_unexpected_abort() {
        Vector::borrow(&Vector::empty<u64>(), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0)]
    fun native_abort_good_wrong_code() {
        Vector::borrow(&Vector::empty<u64>(), 1);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    fun native_abort_good_right_code() {
        Vector::borrow(&Vector::empty<u64>(), 1);
    }
}
