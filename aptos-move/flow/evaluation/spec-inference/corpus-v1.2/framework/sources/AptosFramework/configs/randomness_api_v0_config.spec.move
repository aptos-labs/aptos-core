spec aptos_framework::randomness_api_v0_config {
    spec module {
    }

    spec initialize(framework: &signer, required_amount: RequiredGasDeposit, allow_custom_max_gas_flag: AllowCustomMaxGasFlag) {
        pragma opaque;
        modifies global<RequiredGasDeposit>(@aptos_framework);
        modifies global<AllowCustomMaxGasFlag>(@aptos_framework);
        aborts_if aborts_of<system_addresses::assert_aptos_framework>(framework);
        aborts_if aborts_of<chain_status::assert_genesis>();
        aborts_if exists<RequiredGasDeposit>(@aptos_framework);
        aborts_if exists<AllowCustomMaxGasFlag>(@aptos_framework);
        ensures global<RequiredGasDeposit>(@aptos_framework) == required_amount;
        ensures global<AllowCustomMaxGasFlag>(@aptos_framework) == allow_custom_max_gas_flag;
    }

    spec set_for_next_epoch(framework: &signer, gas_amount: Option<u64>) {
        pragma opaque;
        include config_buffer::SetForNextEpoch<RequiredGasDeposit> {
            new_config: RequiredGasDeposit { gas_amount }
        };
    }

    spec set_allow_max_gas_flag_for_next_epoch(framework: &signer, value: bool) {
        pragma opaque;
        include config_buffer::SetForNextEpoch<AllowCustomMaxGasFlag> {
            new_config: AllowCustomMaxGasFlag { value }
        };
    }

    spec on_new_epoch(framework: &signer) {
        pragma opaque;
        include config_buffer::OnNewEpochApply<RequiredGasDeposit>;
        include config_buffer::OnNewEpochApply<AllowCustomMaxGasFlag>;
    }
}
