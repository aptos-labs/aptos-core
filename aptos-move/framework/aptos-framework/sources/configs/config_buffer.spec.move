spec aptos_framework::config_buffer {
    spec module {
        pragma verify = true;
    }

    spec initialize(aptos_framework: &signer) {
        use std::signer;
        aborts_if exists<PendingConfigs>(signer::address_of(aptos_framework));
    }

    spec does_exist<T: store>(): bool {
        aborts_if false;
    }

    spec upsert<T: drop + store>(config: T) {
        aborts_if !exists<PendingConfigs>(@aptos_framework);
    }

    spec extract<T: store>(): T {
        aborts_if !exists<PendingConfigs>(@aptos_framework);
        include ExtractAbortsIf<T>;
        // let configs = global<PendingConfigs>(@aptos_framework);
        // let key = type_info::type_name<T>();
        // aborts_if !exists<PendingConfigs>(@aptos_framework);
        // aborts_if !simple_map::spec_contains_key(configs.configs, key);
        // include any::UnpackAbortsIf<T> {
        //     x: simple_map::spec_get(configs.configs, key)
        // };
    }

    spec schema ExtractAbortsIf<T> {
        let configs = global<PendingConfigs>(@aptos_framework);
        let key = type_info::type_name<T>();
        aborts_if !simple_map::spec_contains_key(configs.configs, key);
        include any::UnpackAbortsIf<T> {
            x: simple_map::spec_get(configs.configs, key)
        };
    }

}
