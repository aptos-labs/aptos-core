spec aptos_std::single_key {
    spec module {
        pragma verify = true;
    }

    spec is_keyless_or_federated_keyless_public_key(pk: &AnyPublicKey): bool {
        pragma opaque;
        aborts_if false;
    }

    spec from_ed25519_public_key_unvalidated(pk: ed25519::UnvalidatedPublicKey): AnyPublicKey {
        pragma opaque;
        aborts_if false;
        ensures result == AnyPublicKey::Ed25519 { pk };
    }

    spec to_authentication_key(self: &AnyPublicKey): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures len(result) == 32;
    }
}
