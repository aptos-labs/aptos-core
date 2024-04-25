module aptos_framework::randomness_api_v0_config {
    use std::option::Option;
    use aptos_framework::chain_status;
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;
    friend aptos_framework::reconfiguration_with_dkg;

    struct RequiredDeposit has key, drop, store {
        amount: Option<u64>,
    }

    /// Only used in genesis.
    fun initialize(framework: &signer, required_amount: RequiredDeposit) {
        system_addresses::assert_aptos_framework(framework);
        chain_status::assert_genesis();
        move_to(framework, required_amount)
    }

    /// This can be called by on-chain governance to update `RequiredDeposit` for the next epoch.
    public fun set_for_next_epoch(framework: &signer, amount: Option<u64>) {
        system_addresses::assert_aptos_framework(framework);
        config_buffer::upsert(RequiredDeposit { amount });
    }

    /// Only used in reconfigurations to apply the pending `RequiredDeposit`, if there is any.
    public fun on_new_epoch(framework: &signer) acquires RequiredDeposit {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<RequiredDeposit>()) {
            let new_config = config_buffer::extract<RequiredDeposit>();
            if (exists<RequiredDeposit>(@aptos_framework)) {
                *borrow_global_mut<RequiredDeposit>(@aptos_framework) = new_config;
            } else {
                move_to(framework, new_config);
            }
        }
    }
}
