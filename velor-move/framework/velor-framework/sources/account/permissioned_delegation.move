module velor_framework::permissioned_delegation {
    use std::error;
    use std::option::Option;
    use std::signer;
    use velor_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_unvalidated_public_key_from_bytes,
        UnvalidatedPublicKey
    };
    use velor_std::big_ordered_map::{Self, BigOrderedMap};
    use velor_framework::auth_data::{Self, AbstractionAuthData};
    use velor_framework::bcs_stream::{Self, deserialize_u8};
    use velor_framework::permissioned_signer::{Self, is_permissioned_signer, StorablePermissionedHandle};
    use velor_framework::rate_limiter;
    use velor_framework::rate_limiter::RateLimiter;
    #[test_only]
    use std::bcs;
    #[test_only]
    use std::option;

    const ENOT_MASTER_SIGNER: u64 = 1;
    const EINVALID_PUBLIC_KEY: u64 = 2;
    const EPUBLIC_KEY_NOT_FOUND: u64 = 3;
    const EINVALID_SIGNATURE: u64 = 4;
    const EDELEGATION_EXISTENCE: u64 = 5;
    const ERATE_LIMITED: u64 = 6;

    enum AccountDelegation has store {
        V1 { handle: StorablePermissionedHandle, rate_limiter: Option<rate_limiter::RateLimiter> }
    }

    enum DelegationKey has copy, store, drop {
        Ed25519PublicKey(UnvalidatedPublicKey)
    }

    public fun gen_ed25519_key(key: UnvalidatedPublicKey): DelegationKey {
        DelegationKey::Ed25519PublicKey(key)
    }

    struct RegisteredDelegations has key {
        delegations: BigOrderedMap<DelegationKey, AccountDelegation>
    }

    inline fun check_txn_rate(bundle: &mut AccountDelegation, check_rate_limit: bool) {
        let token_bucket = &mut bundle.rate_limiter;
        if (check_rate_limit && token_bucket.is_some()) {
            assert!(rate_limiter::request(token_bucket.borrow_mut(), 1), std::error::permission_denied(ERATE_LIMITED));
        };
    }

    public fun add_permissioned_handle(
        master: &signer,
        key: DelegationKey,
        rate_limiter: Option<RateLimiter>,
        expiration_time: u64,
    ): signer acquires RegisteredDelegations {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        if (!exists<RegisteredDelegations>(addr)) {
            move_to(master, RegisteredDelegations {
                delegations: big_ordered_map::new_with_config(50, 20, false)
            });
        };
        let handles = &mut RegisteredDelegations[addr].delegations;
        assert!(!handles.contains(&key), error::already_exists(EDELEGATION_EXISTENCE));
        let handle = permissioned_signer::create_storable_permissioned_handle(master, expiration_time);
        let permissioned_signer = permissioned_signer::signer_from_storable_permissioned_handle(&handle);
        handles.add(key, AccountDelegation::V1 { handle, rate_limiter });
        permissioned_signer
    }

    public fun remove_permissioned_handle(
        master: &signer,
        key: DelegationKey,
    ) acquires RegisteredDelegations {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let delegations = &mut RegisteredDelegations[addr].delegations;
        assert!(delegations.contains(&key), error::not_found(EDELEGATION_EXISTENCE));
        let delegation = delegations.remove(&key);
        match (delegation) {
            AccountDelegation::V1 { handle, rate_limiter: _ } => {
                permissioned_signer::destroy_storable_permissioned_handle(handle);
            }
        };
    }

    public fun permissioned_signer_by_key(
        master: &signer,
        key: DelegationKey,
    ): signer acquires RegisteredDelegations {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let handle = get_storable_permissioned_handle(addr, key, false);
        permissioned_signer::signer_from_storable_permissioned_handle(handle)
    }

    public fun handle_address_by_key(master: address, key: DelegationKey): address acquires RegisteredDelegations {
        let handle = get_storable_permissioned_handle(master, key, false);
        permissioned_signer::permissions_storage_address(handle)
    }

    /// Authorization function for account abstraction.
    public fun authenticate(
        account: signer,
        abstraction_auth_data: AbstractionAuthData
    ): signer acquires RegisteredDelegations {
        let addr = signer::address_of(&account);
        let stream = bcs_stream::new(*auth_data::authenticator(&abstraction_auth_data));
        let public_key = new_unvalidated_public_key_from_bytes(
            bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x))
        );
        let signature = new_signature_from_bytes(
            bcs_stream::deserialize_vector<u8>(&mut stream, |x| deserialize_u8(x))
        );
        assert!(
            ed25519::signature_verify_strict(
                &signature,
                &public_key,
                *auth_data::digest(&abstraction_auth_data),
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );
        let handle = get_storable_permissioned_handle(addr, DelegationKey::Ed25519PublicKey(public_key), true);
        permissioned_signer::signer_from_storable_permissioned_handle(handle)
    }

    inline fun get_storable_permissioned_handle(
        master: address,
        key: DelegationKey,
        count_rate: bool
    ): &StorablePermissionedHandle {
        if (exists<RegisteredDelegations>(master)) {
            let delegations = &mut RegisteredDelegations[master].delegations;
            if (delegations.contains(&key)) {
                let delegation = delegations.remove(&key);
                check_txn_rate(&mut delegation, count_rate);
                delegations.add(key, delegation);
                &delegations.borrow(&key).handle
            } else {
                abort error::permission_denied(EINVALID_SIGNATURE)
            }
        } else {
            abort error::permission_denied(EINVALID_SIGNATURE)
        }
    }

    ///
    spec module {
        // TODO: fix verification
        pragma verify = false;
    }

    #[test_only]
    use velor_std::ed25519::{sign_arbitrary_bytes, generate_keys, validated_public_key_to_bytes, Signature,
        public_key_into_unvalidated
    };
    #[test_only]
    use velor_framework::account::create_signer_for_test;
    #[test_only]
    use velor_framework::timestamp;

    #[test_only]
    struct SignatureBundle has drop {
        pubkey: UnvalidatedPublicKey,
        signature: Signature,
    }

    #[test(account = @0xcafe, account_copy = @0xcafe)]
    fun test_basics(account: signer, account_copy: signer) acquires RegisteredDelegations {
        let velor_framework = create_signer_for_test(@velor_framework);
        timestamp::set_time_has_started_for_testing(&velor_framework);
        let (sk, vpk) = generate_keys();
        let signature = sign_arbitrary_bytes(&sk, vector[1, 2, 3]);
        let pubkey_bytes = validated_public_key_to_bytes(&vpk);
        let key = DelegationKey::Ed25519PublicKey(public_key_into_unvalidated(vpk));
        let sig_bundle = SignatureBundle {
            pubkey: new_unvalidated_public_key_from_bytes(pubkey_bytes),
            signature,
        };
        let auth_data = auth_data::create_auth_data(vector[1, 2, 3], bcs::to_bytes(&sig_bundle));
        assert!(!is_permissioned_signer(&account));
        add_permissioned_handle(&account, key, option::none(), 60);
        let permissioned_signer = authenticate(account, auth_data);
        assert!(is_permissioned_signer(&permissioned_signer));
        remove_permissioned_handle(&account_copy, key);
    }

    #[test(account = @0xcafe, account_copy = @0xcafe, account_copy_2 = @0xcafe)]
    #[expected_failure(abort_code = 0x50006, location = Self)]
    fun test_rate_limit(account: signer, account_copy: signer, account_copy_2: signer) acquires RegisteredDelegations {
        let velor_framework = create_signer_for_test(@velor_framework);
        timestamp::set_time_has_started_for_testing(&velor_framework);
        let (sk, vpk) = generate_keys();
        let signature = sign_arbitrary_bytes(&sk, vector[1, 2, 3]);
        let pubkey_bytes = validated_public_key_to_bytes(&vpk);
        let key = DelegationKey::Ed25519PublicKey(public_key_into_unvalidated(vpk));
        let sig_bundle = SignatureBundle {
            pubkey: new_unvalidated_public_key_from_bytes(pubkey_bytes),
            signature,
        };
        let auth_data = auth_data::create_auth_data(vector[1, 2, 3], bcs::to_bytes(&sig_bundle));
        assert!(!is_permissioned_signer(&account));
        add_permissioned_handle(&account, key, option::some(rate_limiter::initialize(1, 10)), 60);
        authenticate(account, auth_data);
        authenticate(account_copy, auth_data);
        remove_permissioned_handle(&account_copy_2, key);
    }
}
