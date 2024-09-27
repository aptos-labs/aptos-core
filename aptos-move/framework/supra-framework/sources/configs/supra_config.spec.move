spec supra_framework::supra_config {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During genesis, the Supra framework account should be assigned the supra config resource.
    /// Criticality: Medium
    /// Implementation: The supra_config::initialize function calls the assert_supra_framework function to ensure
    /// that the signer is the supra_framework and then assigns the SupraConfig resource to it.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: Only aptos framework account is allowed to update the supra protocol configuration.
    /// Criticality: Medium
    /// Implementation: The supra_config::set function ensures that the signer is supra_framework.
    /// Enforcement: Formally verified via [high-level-req-2](set).
    ///
    /// No.: 3
    /// Requirement: Only a valid configuration can be used during initialization and update.
    /// Criticality: Medium
    /// Implementation: Both the initialize and set functions validate the config by ensuring its length to be greater
    /// than 0.
    /// Enforcement: Formally verified via [high-level-req-3.1](initialize) and [high-level-req-3.2](set).
    /// </high-level-req>
    ///
    spec module {
        use supra_framework::chain_status;
        pragma verify = true;
        pragma aborts_if_is_strict;
        invariant [suspendable] chain_status::is_operating() ==> exists<SupraConfig>(@supra_framework);
    }

    /// Ensure caller is admin.
    /// Aborts if StateStorageUsage already exists.
    spec initialize(supra_framework: &signer, config: vector<u8>) {
        use std::signer;
        let addr = signer::address_of(supra_framework);
        /// [high-level-req-1]
        aborts_if !system_addresses::is_supra_framework_address(addr);
        aborts_if exists<SupraConfig>(@supra_framework);
        /// [high-level-req-3.1]
        aborts_if !(len(config) > 0);
        ensures global<SupraConfig>(addr) == SupraConfig { config };
    }

    spec set_for_next_epoch(account: &signer, config: vector<u8>) {
        include config_buffer::SetForNextEpochAbortsIf;
    }

    spec on_new_epoch(framework: &signer) {
        requires @supra_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<SupraConfig>;
        aborts_if false;
    }
}
