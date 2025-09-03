script {
    use aptos_framework::reconfiguration;
    use aptos_framework::aptos_governance;

    /// Script to update the epoch and timestamp in the Configuration resource
    /// and trigger a reconfiguration event.
    /// This script must be executed with the core resource signer
    /// 
    /// @param core_resources - The core resource signer 
    /// @param new_epoch - The new epoch number to set
    /// @param new_timestamp - The new timestamp to set (in microseconds)
    fun main(
        core_resources: &signer,
        new_epoch: u64,
        new_timestamp: u64,
    ) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        reconfiguration::update_configuration(&framework_signer, new_epoch, new_timestamp);
        aptos_governance::reconfigure(&framework_signer);
    }
}
