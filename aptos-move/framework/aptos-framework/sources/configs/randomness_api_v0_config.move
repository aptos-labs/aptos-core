module aptos_framework::randomness_api_v0_config {
    use std::option::Option;
    use aptos_framework::chain_status;
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;
    friend aptos_framework::reconfiguration_with_dkg;

    struct RequiredGasDeposit has key, drop, store {
        gas_amount: Option<u64>,
    }

    /// Only used in genesis.
    fun initialize(framework: &signer, required_amount: RequiredGasDeposit) {
        system_addresses::assert_aptos_framework(framework);
        chain_status::assert_genesis();
        move_to(framework, required_amount)
    }

    /// This can be called by on-chain governance to update `RequiredGasDeposit` for the next epoch.
    public fun set_for_next_epoch(framework: &signer, gas_amount: Option<u64>) {
        system_addresses::assert_aptos_framework(framework);
        config_buffer::upsert(RequiredGasDeposit { gas_amount });
    }

    /// Only used in reconfigurations to apply the pending `RequiredGasDeposit`, if there is any.
    public fun on_new_epoch(framework: &signer) acquires RequiredGasDeposit {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<RequiredGasDeposit>()) {
            let new_config = config_buffer::extract<RequiredGasDeposit>();
            if (exists<RequiredGasDeposit>(@aptos_framework)) {
                *borrow_global_mut<RequiredGasDeposit>(@aptos_framework) = new_config;
            } else {
                move_to(framework, new_config);
            }
        }
    }
}
