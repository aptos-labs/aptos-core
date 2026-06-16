spec aptos_framework::permissioned_signer {
    // The permissioned signer feature was never enabled and has been removed. The module is
    // retained for upgrade compatibility but its functions are neutralized, so there is nothing
    // meaningful to verify here.
    spec module {
        pragma verify = false;
    }
}
