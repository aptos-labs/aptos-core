/// Test framework module for testing missing native function error handling.
/// This module should never be included in production builds.
module supra_framework::test_missing_native {
    /// Native function declaration without implementation - FOR TESTING ONLY
    native fun missing_native();
    /// Public native function declaration without implementation - FOR TESTING ONLY
    public native fun public_missing_native();

    /// Public wrapper function that calls the missing native
    /// This function is used to trigger the missing native function error during tests.
    public fun missing_native_function(framework: &signer) {
        missing_native();
    }
}
