module AptosFramework::Aggregator {
    use Std::Errors;

    /// Aggregator type parameter is non-integral.
    const E_NON_INTEGRAL_TYPE: u64 = 0;

    // Raised by native code.
    const E_OVERFLOW: u64 = 1500;

    struct Aggregator<phantom IntTy> has drop, store {
        handle: u128,
    }

    /// Creates a new Aggregator instance, zero-initialized.
    public fun new<IntTy>(): Aggregator<IntTy> {
        assert!(is_integral<IntTy>(), Errors::invalid_argument(E_NON_INTEGRAL_TYPE));
        Aggregator {
            handle: new_handle<IntTy>(),
        }
    }

    /// Adds `value` to aggregator. Aborts on overflow as per `IntTy` semantics.
    native public fun add<IntTy>(aggregator: &mut Aggregator<IntTy>, value: IntTy);

    /// Private native functions for setting up a new Aggregator.
    native fun is_integral<IntTy>(): bool;
    native fun new_handle<IntTy>(): u128;

    #[test_only]
    struct FakeData {}

    #[test]
    #[expected_failure(abort_code = 7)]
    fun invalid_aggregator_test1() {
        let _aggregator = new<address>();
    }

    #[test]
    #[expected_failure(abort_code = 7)]
    fun invalid_aggregator_test2() {
        let _aggregator = new<FakeData>();
    }

    #[test]
    fun valid_aggregator_test() {
        let _a1 = new<u8>();
        let _a2 = new<u64>();
        let _a3 = new<u128>();
    }

    #[test]
    #[expected_failure(abort_code = 1500)]
    fun aggregator_add_overflow_test() {
        let a = new<u8>();
        add(&mut a, 100);
        add(&mut a, 200);
    }
}
