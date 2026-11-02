script {
    use aptos_framework::aptos_governance;

    /// Turns on TRADING_NATIVE, NATIVE_POSITION and COMPUTE_TRADING_NATIVE_STATE_ROOTS.
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        aptos_governance::toggle_features(&framework_signer, vector[118, 119, 122], vector[]);
    }
}
