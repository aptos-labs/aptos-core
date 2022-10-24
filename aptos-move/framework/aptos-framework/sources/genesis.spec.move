spec aptos_framework::genesis {
    spec module {
        // We are not proving each genesis step individually. Instead, we construct
        // and prove `initialize_for_verification` which is a "#[verify_only]" function that
        // simulates the genesis encoding process in `vm-genesis` (written in Rust).
        // So, we turn off the verification at the module level, but turn it on for
        // the verification-only function `initialize_for_verification`.
        pragma verify = false;
    }

    spec initialize_for_verification {
        pragma verify = true;
    }

    spec create_signer {
        // TODO: temporary mockup.
        pragma opaque;
    }
}
