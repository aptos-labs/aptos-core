/// Module that publishes non-private constants for use by other modules.
module 0xCAFE::constants {
    /// A public u64 constant accessible from any module.
    public const MAX_VALUE: u64 = 100;

    /// A public u8 constant.
    public const VERSION: u8 = 42;

    /// A public bool constant.
    public const ENABLED: bool = true;

    /// A private constant — must not be accessible cross-module.
    const PRIVATE_SECRET: u64 = 999;

    /// Returns the private constant (same-module access is always allowed).
    public fun get_private(): u64 {
        PRIVATE_SECRET
    }
}

/// Module that consumes public constants from 0xCAFE::constants.
module 0xCAFE::consumer {
    use 0xCAFE::constants;

    struct Result has key {
        max_value: u64,
        version: u8,
        enabled: bool,
    }

    /// Entry function that stores the public constant values into a resource
    /// so the test can verify them.
    public entry fun store_constants(account: &signer) {
        move_to(account, Result {
            max_value: constants::MAX_VALUE,
            version: constants::VERSION,
            enabled: constants::ENABLED,
        });
    }
}
