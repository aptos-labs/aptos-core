spec aptos_framework::config_buffer {
    spec module {
        pragma verify = true;
    }

    spec initialize(aptos_framework: &signer) {
        use std::signer;
        pragma opaque;
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        modifies global<PendingConfigs>(@aptos_framework);
        ensures exists<PendingConfigs>(@aptos_framework);
        ensures !old(exists<PendingConfigs>(@aptos_framework)) ==>
            simple_map::spec_len(global<PendingConfigs>(@aptos_framework).configs) == 0;
        ensures old(exists<PendingConfigs>(@aptos_framework)) ==>
            global<PendingConfigs>(@aptos_framework) == old(global<PendingConfigs>(@aptos_framework));
    }

    spec does_exist<T: store>(): bool {
        pragma opaque;
        aborts_if false;
        let type_name = type_info::type_name<T>();
        ensures result == spec_fun_does_exist<T>(type_name);
    }

    spec fun spec_fun_does_exist<T: store>(type_name: String): bool {
        if (exists<PendingConfigs>(@aptos_framework)) {
            let config = global<PendingConfigs>(@aptos_framework);
            simple_map::spec_contains_key(config.configs, type_name)
        } else {
            false
        }
    }

    spec upsert<T: drop + store>(config: T) {
        pragma opaque;
        aborts_if !exists<PendingConfigs>(@aptos_framework);
        modifies global<PendingConfigs>(@aptos_framework);

        let key = type_info::type_name<T>();
        let post configs_post = global<PendingConfigs>(@aptos_framework).configs;
        ensures simple_map::spec_contains_key(configs_post, key);
        ensures simple_map::spec_get(configs_post, key) == any::pack(config);
    }

    spec extract_v2<T: store>(): T {
        use aptos_std::from_bcs;
        aborts_if !exists<PendingConfigs>(@aptos_framework);
        include ExtractAbortsIf<T>;
        modifies global<PendingConfigs>(@aptos_framework);
        let key = type_info::type_name<T>();
        let pre_configs = global<PendingConfigs>(@aptos_framework).configs;
        let stored = simple_map::spec_get(pre_configs, key);
        let post post_configs = global<PendingConfigs>(@aptos_framework).configs;
        ensures result == from_bcs::deserialize<T>(stored.data);
        ensures !simple_map::spec_contains_key(post_configs, key);
    }

    spec schema ExtractAbortsIf<T> {
        let configs = global<PendingConfigs>(@aptos_framework);
        let key = type_info::type_name<T>();
        aborts_if !simple_map::spec_contains_key(configs.configs, key);
        include any::UnpackAbortsIf<T> {
            self: simple_map::spec_get(configs.configs, key)
        };
    }

    spec schema SetForNextEpochAbortsIf {
        account: &signer;
        config: vector<u8>;
        let account_addr = std::signer::address_of(account);
        aborts_if account_addr != @aptos_framework;
        aborts_if len(config) == 0;
        aborts_if !exists<PendingConfigs>(@aptos_framework);
    }

    spec schema OnNewEpochRequirement<T> {
        use aptos_std::type_info;
        let type_name = type_info::type_name<T>();
        let configs = global<PendingConfigs>(@aptos_framework);
        include spec_fun_does_exist<T>(type_name) ==> any::UnpackRequirement<T> {
            self: simple_map::spec_get(configs.configs, type_name)
        };
    }

    spec schema OnNewEpochApply<T> {
        use aptos_std::from_bcs;
        use aptos_std::simple_map;
        use aptos_std::type_info;
        framework: &signer;

        requires @aptos_framework == std::signer::address_of(framework);
        include OnNewEpochRequirement<T>;
        aborts_if false;
        modifies global<PendingConfigs>(@aptos_framework);
        modifies global<T>(@aptos_framework);

        let pending_configs = global<PendingConfigs>(@aptos_framework);
        let pending_configs_exists = exists<PendingConfigs>(@aptos_framework);
        let key = type_info::type_name<T>();
        let had = pending_configs_exists
            && simple_map::spec_contains_key(pending_configs.configs, key);
        let extracted = from_bcs::deserialize<T>(
            simple_map::spec_get(pending_configs.configs, key).data);
        ensures had ==> global<T>(@aptos_framework) == extracted;
    }

    spec schema SetForNextEpoch<T: drop + store> {
        framework: &signer;
        new_config: T;

        modifies global<PendingConfigs>(@aptos_framework);
        aborts_if aborts_of<system_addresses::assert_aptos_framework>(framework);
        aborts_if aborts_of<upsert<T>>(new_config);
        ensures ensures_of<upsert<T>>(new_config);
    }

    spec schema InitializeResource<T> {
        framework: &signer;
        config: T;

        modifies global<T>(@aptos_framework);
        aborts_if aborts_of<system_addresses::assert_aptos_framework>(framework);
        ensures !old(exists<T>(@aptos_framework)) ==>
            global<T>(@aptos_framework) == config;
    }
}
