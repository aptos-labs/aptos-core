/// Test module for gas profiling that exercises:
/// - Dependencies (this module + aptos_framework::event)
/// - Events
/// - Storage writes and deletions (refunds)
module 0xCAFE::gas_profiling_test {
    use std::signer;
    use aptos_framework::event;

    struct Small has key { value: u64 }
    struct Large has key { a: u64, b: u64, c: u64, d: u64 }

    #[event]
    struct TestEvent has drop, store { value: u64 }

    /// Setup: create a small resource
    public entry fun setup(account: &signer) {
        move_to(account, Small { value: 42 });
    }

    /// Profiled tx: delete small (refund), create large (storage fee), emit event
    public entry fun replace(account: &signer) acquires Small {
        let addr = signer::address_of(account);
        let Small { value } = move_from<Small>(addr);
        move_to(account, Large { a: value, b: value, c: value, d: value });
        event::emit(TestEvent { value });
    }
}
