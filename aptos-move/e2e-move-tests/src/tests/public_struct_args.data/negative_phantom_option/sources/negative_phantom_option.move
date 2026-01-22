/// Test module for non-phantom type parameter in user-defined enum - should FAIL to compile.
/// Container<T> has non-phantom T (stored in the Value variant), so private Hero is rejected.
/// Unlike whitelisted Option<T>, user-defined public enums/structs require all field types to
/// be valid transaction arguments.
module 0xcafe::negative_phantom_option {
    use std::signer;

    /// Private struct - not public
    struct Hero has copy, drop {
        health: u64,
        level: u64,
    }

    /// Public enum with NON-phantom type parameter - the type T is actually stored
    /// This should be rejected when T is private
    public enum Container<T> has copy, drop {
        Value { data: T },
        Empty,
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

    /// Entry function that takes user-defined Container<Hero> enum as parameter.
    /// This should FAIL compilation because Container<T>'s type parameter is non-phantom
    /// (T is stored in the Value variant), and Hero is private — so Hero is not a valid
    /// transaction argument type for field-bearing structs/enums.
    public entry fun test_container_hero(sender: &signer, _container: Container<Hero>) acquires TestResult {
        let result = borrow_global_mut<TestResult>(signer::address_of(sender));
        result.success = true;
        result.value = 88; // This should never execute
    }

    #[view]
    public fun get_result(addr: address): (bool, u64) acquires TestResult {
        let result = borrow_global<TestResult>(addr);
        (result.success, result.value)
    }
}
