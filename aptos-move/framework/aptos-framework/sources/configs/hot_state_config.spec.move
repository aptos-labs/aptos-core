spec aptos_framework::hot_state_config {
    spec initialize(framework: &signer, config: vector<u8>) {
        pragma opaque;
        include config_buffer::InitializeResource<HotStateConfig> {
            config: HotStateConfig { config }
        };
        aborts_if len(config) == 0;
    }

    spec set_for_next_epoch(framework: &signer, config: vector<u8>) {
        pragma opaque;
        modifies global<config_buffer::PendingConfigs>(@aptos_framework);
        include config_buffer::SetForNextEpochAbortsIf { account: framework };
        let key = std::type_info::type_name<HotStateConfig>();
        let post configs_post = global<config_buffer::PendingConfigs>(@aptos_framework).configs;
        ensures std::simple_map::spec_contains_key(configs_post, key);
        ensures std::simple_map::spec_get(configs_post, key) == std::any::pack(HotStateConfig { config });
    }

    spec on_new_epoch(framework: &signer) {
        pragma opaque;
        include config_buffer::OnNewEpochApply<HotStateConfig>;
    }
}
