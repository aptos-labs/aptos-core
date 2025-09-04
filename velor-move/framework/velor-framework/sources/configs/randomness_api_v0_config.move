module velor_framework::randomness_api_v0_config {
    use std::option::Option;
    use velor_framework::chain_status;
    use velor_framework::config_buffer;
    use velor_framework::system_addresses;
    friend velor_framework::reconfiguration_with_dkg;

    struct RequiredGasDeposit has key, drop, store {
        gas_amount: Option<u64>,
    }

    /// If this flag is set, `max_gas` specified inside `#[randomness()]` will be used as the required deposit.
    struct AllowCustomMaxGasFlag has key, drop, store {
        value: bool,
    }

    /// Only used in genesis.
    fun initialize(framework: &signer, required_amount: RequiredGasDeposit, allow_custom_max_gas_flag: AllowCustomMaxGasFlag) {
        system_addresses::assert_velor_framework(framework);
        chain_status::assert_genesis();
        move_to(framework, required_amount);
        move_to(framework, allow_custom_max_gas_flag);
    }

    /// This can be called by on-chain governance to update `RequiredGasDeposit` for the next epoch.
    public fun set_for_next_epoch(framework: &signer, gas_amount: Option<u64>) {
        system_addresses::assert_velor_framework(framework);
        config_buffer::upsert(RequiredGasDeposit { gas_amount });
    }

    /// This can be called by on-chain governance to update `AllowCustomMaxGasFlag` for the next epoch.
    public fun set_allow_max_gas_flag_for_next_epoch(framework: &signer, value: bool) {
        system_addresses::assert_velor_framework(framework);
        config_buffer::upsert(AllowCustomMaxGasFlag { value } );
    }

    /// Only used in reconfigurations to apply the pending `RequiredGasDeposit`, if there is any.
    public fun on_new_epoch(framework: &signer) acquires RequiredGasDeposit, AllowCustomMaxGasFlag {
        system_addresses::assert_velor_framework(framework);
        if (config_buffer::does_exist<RequiredGasDeposit>()) {
            let new_config = config_buffer::extract_v2<RequiredGasDeposit>();
            if (exists<RequiredGasDeposit>(@velor_framework)) {
                *borrow_global_mut<RequiredGasDeposit>(@velor_framework) = new_config;
            } else {
                move_to(framework, new_config);
            }
        };
        if (config_buffer::does_exist<AllowCustomMaxGasFlag>()) {
            let new_config = config_buffer::extract_v2<AllowCustomMaxGasFlag>();
            if (exists<AllowCustomMaxGasFlag>(@velor_framework)) {
                *borrow_global_mut<AllowCustomMaxGasFlag>(@velor_framework) = new_config;
            } else {
                move_to(framework, new_config);
            }
        }
    }
}
