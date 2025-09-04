module aa::single_key {
    use std::option;
    use std::signer;
    use velor_std::bls12381::{Self, PublicKey};
    use velor_framework::auth_data::{Self, AbstractionAuthData};

    /// Only fungible asset metadata owner can make changes.
    const EINVALID_PUBLIC_KEY: u64 = 1;
    const EPUBLIC_KEY_NOT_FOUND: u64 = 2;
    const EINVALID_SIGNATURE: u64 = 3;

    /// Store the BLS public key.
    struct BLSPublicKey has key, drop {
        key: PublicKey
    }

    /// Update the public key.
    public entry fun update_public_key(admin: &signer, key: vector<u8>) acquires BLSPublicKey {
        let addr = signer::address_of(admin);
        let pubkey_opt = bls12381::public_key_from_bytes(key);
        assert!(option::is_some(&pubkey_opt), EINVALID_PUBLIC_KEY);
        if (exists<BLSPublicKey>(addr)) {
            let pubkey = &mut borrow_global_mut<BLSPublicKey>(addr).key;
            *pubkey = option::destroy_some(pubkey_opt);
        } else {
            move_to(admin, BLSPublicKey {
                key: option::destroy_some(pubkey_opt)
            })
        };
    }

    /// Authorization function for account abstraction.
    public fun authenticate(
        account: signer,
        signing_data: AbstractionAuthData,
    ): signer acquires BLSPublicKey {
        let addr = signer::address_of(&account);
        assert!(exists<BLSPublicKey>(addr), EPUBLIC_KEY_NOT_FOUND);
        let pubkey = &borrow_global<BLSPublicKey>(addr).key;
        assert!(
            bls12381::verify_normal_signature(
                &bls12381::signature_from_bytes(*auth_data::authenticator(&signing_data)),
                pubkey,
                *auth_data::digest(&signing_data)
            ),
            EINVALID_SIGNATURE
        );
        account
    }

    /// cleanup storage footprint before transition to another authentication scheme.
    public entry fun cleanup(admin: &signer) acquires BLSPublicKey {
        let addr = signer::address_of(admin);
        if (exists<BLSPublicKey>(addr)) {
            move_from<BLSPublicKey>(addr);
        };
    }
}
