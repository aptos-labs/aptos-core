spec aptos_std::multi_key {
    spec module {
        pragma verify = true;
    }

    spec new_multi_key_from_single_keys(
        single_keys: vector<single_key::AnyPublicKey>,
        signatures_required: u8
    ): MultiKey {
        pragma opaque;
        aborts_if len(single_keys) == 0;
        aborts_if len(single_keys) > MAX_NUMBER_OF_PUBLIC_KEYS;
        aborts_if (signatures_required as u64) > len(single_keys);
        ensures result == MultiKey { public_keys: single_keys, signatures_required };
    }

    spec to_authentication_key(self: &MultiKey): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures len(result) == 32;
    }

}
