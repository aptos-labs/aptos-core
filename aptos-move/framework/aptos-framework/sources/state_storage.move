module aptos_framework::state_storage {

    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use std::error;
    use aptos_framework::reconfiguration::Configuration;

    friend aptos_framework::genesis;

    const EUSAGE_ALREADY_EXISTS: u64 = 0;

    struct StateStorageUsage has copy, drop, key, store {
        items: u64,
        bytes: u64,
    }

    public(friend) fun initialize(account: &signer) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);
        assert!(
            !exists<StateStorageUsage>(@aptos_framework),
            error::already_exists(EUSAGE_ALREADY_EXISTS)
        );
        move_to(account, StateStorageUsage {
            items: 0,
            bytes: 0,
        });
    }

    public(friend) fun on_epoch_begin() {
        let cfg = borrow_global<Configuration>(@aptos_framework);
        *borrow_global_mut<StateStorageUsage>(@aptos_framework) = get_usage_at_epoch_ending()
    }

    native fun get_usage_at_epoch_ending(epoch: u64): StateStorageUsage;

}
