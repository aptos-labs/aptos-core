spec supra_framework::evm_config {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec set_for_next_epoch(account: &signer, config: vector<u8>) {
        include config_buffer::SetForNextEpochAbortsIf;
    }

    spec on_new_epoch(framework: &signer) {
        requires @supra_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<EvmConfig>;
        aborts_if false;
    }

    spec initialize(supra_framework: &signer, config: vector<u8>) {
        pragma aborts_if_is_strict = false;
    }
}
