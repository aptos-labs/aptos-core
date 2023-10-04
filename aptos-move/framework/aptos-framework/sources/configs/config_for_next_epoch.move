/// This wrapper helps store an on-chain config for the next version.
module aptos_framework::config_for_next_epoch {

    struct ForNextEpoch<T> has drop, key {
        payload: T,
    }

    public fun upsert<T: drop + store>(aptos_framework: &signer, config: T) acquires ForNextEpoch {
        let wrapped = ForNextEpoch { payload: config };
        if (config_for_next_epoch_exists<T>()) {
            *borrow_global_mut<ForNextEpoch<T>>(@aptos_framework) = wrapped;
        } else {
            move_to(aptos_framework, wrapped)
        }
    }

    public fun config_for_next_epoch_exists<T: store>(): bool {
        exists<ForNextEpoch<T>>(@aptos_framework)
    }

    public fun pop<T: store>(): T acquires ForNextEpoch {
        let ForNextEpoch { payload } = move_from<ForNextEpoch<T>>(@aptos_framework);
        payload
    }
}
