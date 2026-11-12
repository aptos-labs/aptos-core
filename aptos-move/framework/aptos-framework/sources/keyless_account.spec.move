spec aptos_framework::keyless_account {
    spec module {
        // TODO: verification is  disabled until this module is specified.
        pragma verify = false;
    }

    // Summary for `reconfiguration_with_dkg::finish`; the only verified spec in this module.
    spec on_new_epoch(fx: &signer) {
        use aptos_framework::config_buffer;
        pragma verify = true;
        pragma opaque;
        include config_buffer::OnNewEpochApplyBase<Groth16VerificationKey> { framework: fx };
        include config_buffer::OnNewEpochApplyBase<Configuration> { framework: fx };
        include config_buffer::OtherPendingConfigsUnchanged<Groth16VerificationKey, Configuration>;
    }
}
