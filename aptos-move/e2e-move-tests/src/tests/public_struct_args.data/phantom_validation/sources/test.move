/// Test module for phantom type parameter - should compile successfully.
/// Object<T> has phantom T, so private Hero is allowed.
module 0xcafe::phantom_validation {
    use std::object::Object;
    use std::signer;

    /// Private struct - not public, has key ability for use with Object<Hero>
    struct Hero has key {
        health: u64,
        level: u64,
    }

    /// Public enum with phantom type parameter - similar to Object<T>
    /// This represents a reference/handle to some type T
    public enum Wrapper<phantom T> has copy, drop {
        Some { id: u64 },
        None,
    }

    /// Struct for storing test results
    struct TestResult has key {
        success: bool,
        value: u64,
    }

    /// Initialize test result resource
    public entry fun initialize(sender: &signer) {
        move_to(sender, TestResult { success: false, value: 0 });
    }

    /// Entry function that takes Object<Hero> as parameter.
    /// This should COMPILE even though Hero is private, because Object<T>'s
    /// type parameter is phantom (not actually stored, just metadata).
    public entry fun test_object_hero(sender: &signer, _hero_obj: Object<Hero>) acquires TestResult {
        let result = borrow_global_mut<TestResult>(signer::address_of(sender));
        result.success = true;
        result.value = 42; // Arbitrary success value
    }

    /// Entry function that takes Object<u64> as parameter.
    /// This should COMPILE because Object accepts any type parameter (phantom).
    public entry fun test_object_u64(sender: &signer, _obj: Object<u64>) acquires TestResult {
        let result = borrow_global_mut<TestResult>(signer::address_of(sender));
        result.success = true;
        result.value = 123;
    }

    /// Entry function that takes user-defined Wrapper<Hero> enum as parameter.
    /// This should COMPILE even though Hero is private, because Wrapper<T>'s
    /// type parameter is phantom (not actually stored, just metadata).
    public entry fun test_wrapper_hero(sender: &signer, _wrapper: Wrapper<Hero>) acquires TestResult {
        let result = borrow_global_mut<TestResult>(signer::address_of(sender));
        result.success = true;
        result.value = 77; // Arbitrary success value
    }

    /// Entry function that takes user-defined Wrapper<u64> enum as parameter.
    /// This should COMPILE because Wrapper accepts any type parameter (phantom).
    public entry fun test_wrapper_u64(sender: &signer, _wrapper: Wrapper<u64>) acquires TestResult {
        let result = borrow_global_mut<TestResult>(signer::address_of(sender));
        result.success = true;
        result.value = 88;
    }

    #[view]
    public fun get_result(addr: address): (bool, u64) acquires TestResult {
        let result = borrow_global<TestResult>(addr);
        (result.success, result.value)
    }
}
