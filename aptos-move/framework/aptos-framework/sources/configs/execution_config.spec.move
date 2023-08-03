spec aptos_framework::execution_config {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Ensure the caller is admin
    /// When setting now time must be later than last_reconfiguration_time.
    spec set(account: &signer, config: vector<u8>) {
        use aptos_framework::timestamp;
        use std::signer;

        pragma verify = false; // TODO: set to false because of timeout

        let addr = signer::address_of(account);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if !(len(config) > 0);

        requires timestamp::spec_now_microseconds() >= reconfiguration::last_reconfiguration_time();
    }
}
