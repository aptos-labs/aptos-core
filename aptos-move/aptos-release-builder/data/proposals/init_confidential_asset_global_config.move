// Install `confidential_asset::GlobalConfig` on chains whose genesis skipped `init_module`.
//
// Genesis publishes the framework's module bytes directly, bypassing the VM flow that auto-invokes
// `init_module`, and `vm_genesis::initialize_confidential_asset` only compensates on chains that
// are neither mainnet nor testnet. A network genesis'd under the mainnet or testnet chain ID is
// therefore left with `confidential_asset` published but no `GlobalConfig`, which makes every entry
// point in that module abort with MISSING_DATA.
//
// `initialize_global_config` is a no-op when the config already exists, so this script is safe to
// run on a chain that is already initialized and safe to re-run after a failed release.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::confidential_asset;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        confidential_asset::initialize_global_config(&framework);
    }
}
