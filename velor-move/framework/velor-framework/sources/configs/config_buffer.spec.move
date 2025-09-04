spec velor_framework::config_buffer {
    spec module {
        pragma verify = true;
    }

    spec does_exist<T: store>(): bool {
        aborts_if false;
        let type_name = type_info::type_name<T>();
        ensures result == spec_fun_does_exist<T>(type_name);
    }

    spec fun spec_fun_does_exist<T: store>(type_name: String): bool {
        if (exists<PendingConfigs>(@velor_framework)) {
            let config = global<PendingConfigs>(@velor_framework);
            simple_map::spec_contains_key(config.configs, type_name)
        } else {
            false
        }
    }

    spec upsert<T: drop + store>(config: T) {
        aborts_if !exists<PendingConfigs>(@velor_framework);
    }

    spec extract_v2<T: store>(): T {
        aborts_if !exists<PendingConfigs>(@velor_framework);
        include ExtractAbortsIf<T>;
    }

    spec schema ExtractAbortsIf<T> {
        let configs = global<PendingConfigs>(@velor_framework);
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
        aborts_if account_addr != @velor_framework;
        aborts_if len(config) == 0;
        aborts_if !exists<PendingConfigs>(@velor_framework);
    }

    spec schema OnNewEpochAbortsIf<T> {
        use velor_std::type_info;
        let type_name = type_info::type_name<T>();
        let configs = global<PendingConfigs>(@velor_framework);
        // TODO(#12015)
        include spec_fun_does_exist<T>(type_name) ==> any::UnpackAbortsIf<T> {
            self: simple_map::spec_get(configs.configs, type_name)
        };
    }

    spec schema OnNewEpochRequirement<T> {
        use velor_std::type_info;
        let type_name = type_info::type_name<T>();
        let configs = global<PendingConfigs>(@velor_framework);
        // TODO(#12015)
        include spec_fun_does_exist<T>(type_name) ==> any::UnpackRequirement<T> {
            self: simple_map::spec_get(configs.configs, type_name)
        };
    }

}
