spec aptos_framework::jwks {
    spec on_new_epoch() {
        pragma verify = false;
    }

    spec on_new_epoch_v2(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<SupportedOIDCProviders>;
        aborts_if false;
    }
}
