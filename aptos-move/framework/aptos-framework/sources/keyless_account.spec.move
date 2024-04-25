spec aptos_framework::keyless_account {
    spec on_new_epoch(fx: &signer) {
        requires @aptos_framework == std::signer::address_of(fx);
        include config_buffer::OnNewEpochRequirement<Configuration>;
        include config_buffer::OnNewEpochRequirement<Groth16VerificationKey>;
        aborts_if false;
    }
}
