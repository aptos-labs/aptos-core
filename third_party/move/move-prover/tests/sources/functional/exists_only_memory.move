// flag: --verify-only=generic_exists
module 0x42::exists_only_memory {
    // Keep the generic memory access in the specification: executable global
    // storage operations require a struct declared in this module.
    public fun generic_exists<T: key>(a: address, present: bool): bool {
        present
    }

    spec generic_exists {
        requires present == exists<T>(a);
        ensures result == exists<T>(a);
    }
}
