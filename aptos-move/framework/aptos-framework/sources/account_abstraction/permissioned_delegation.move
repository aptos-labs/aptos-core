module aptos_framework::permissioned_delegation {
    use std::error;
    use std::option::Option;
    use std::signer;
    use aptos_std::ed25519;
    use aptos_std::ed25519::{new_signature_from_bytes, new_unvalidated_public_key_from_bytes, UnvalidatedPublicKey};
    use aptos_std::table::{Self, Table};
    use aptos_framework::auth_data;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::bcs_stream;
    use aptos_framework::bcs_stream::deserialize_u8;
    use aptos_framework::permissioned_signer::{Self, is_permissioned_signer, StorablePermissionedHandle};
    use aptos_framework::token_bucket;
    #[test_only]
    use std::bcs;
    #[test_only]
    use std::option;

    const ENOT_MASTER_SIGNER: u64 = 1;
    const EINVALID_PUBLIC_KEY: u64 = 2;
    const EPUBLIC_KEY_NOT_FOUND: u64 = 3;
    const EINVALID_SIGNATURE: u64 = 4;
    const EHANDLE_EXISTENCE: u64 = 5;
    const ERATE_LIMITED: u64 = 6;

    enum HandleBundle has store {
        V1 { handle: StorablePermissionedHandle, bucket: Option<token_bucket::Bucket> }
    }

    struct Delegation has key {
        handle_bundles: Table<ed25519::UnvalidatedPublicKey, HandleBundle>
    }

    inline fun fetch_handle(bundle: &mut HandleBundle, check_rate_limit: bool): &StorablePermissionedHandle {
        let token_bucket = &mut bundle.bucket;
        if (check_rate_limit && token_bucket.is_some()) {
            assert!(token_bucket::request(token_bucket.borrow_mut(), 1), std::error::permission_denied(ERATE_LIMITED));
        };
        &bundle.handle
    }

    public fun add_permissioned_handle(
        master: &signer,
        key: vector<u8>,
        max_txn_per_minute: Option<u64>,
        expiration_time: u64,
    ): signer acquires Delegation {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        if (!exists<Delegation>(addr)) {
            move_to(master, Delegation {
                handle_bundles: table::new()
            });
        };
        let handles = &mut borrow_global_mut<Delegation>(addr).handle_bundles;
        assert!(!handles.contains(pubkey), error::already_exists(EHANDLE_EXISTENCE));
        let handle = permissioned_signer::create_storable_permissioned_handle(master, expiration_time);
        let bucket = max_txn_per_minute.map(|capacity|token_bucket::initialize_bucket(capacity));
        let permissioned_signer = permissioned_signer::signer_from_storable_permissioned_handle(&handle);
        handles.add(pubkey, HandleBundle::V1 { bucket, handle });
        permissioned_signer
    }

    public fun remove_permissioned_handle(
        master: &signer,
        key: vector<u8>,
    ) acquires Delegation {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        let handle_bundles = &mut borrow_global_mut<Delegation>(addr).handle_bundles;
        assert!(handle_bundles.contains(pubkey), error::not_found(EHANDLE_EXISTENCE));
        let bundle = handle_bundles.remove(pubkey);
        match (bundle) {
            HandleBundle::V1 { handle, bucket: _ } => {
                permissioned_signer::destroy_storable_permissioned_handle(handle);
            }
        };
    }

    public fun permissioned_signer_by_key(
        master: &signer,
        key: vector<u8>,
    ): signer acquires Delegation {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        let handle = get_storable_permissioned_handle(addr, pubkey, false);
        permissioned_signer::signer_from_storable_permissioned_handle(handle)
    }

    #[view]
    public fun handle_address_by_key(master: address, key: vector<u8>): address acquires Delegation {
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        let handle = get_storable_permissioned_handle(master, pubkey, false);
        permissioned_signer::permissions_storage_address(handle)
    }

    /// Authorization function for account abstraction.
    public fun authenticate(account: signer, abstraction_auth_data: AbstractionAuthData): signer acquires Delegation {
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
        let handle = get_storable_permissioned_handle(addr, public_key, true);
        permissioned_signer::signer_from_storable_permissioned_handle(handle)
    }

    inline fun get_storable_permissioned_handle(
        master: address,
        pubkey: UnvalidatedPublicKey,
        count_rate: bool
    ): &StorablePermissionedHandle {
        if (exists<Delegation>(master)) {
            let bundles = &mut borrow_global_mut<Delegation>(master).handle_bundles;
            if (bundles.contains(pubkey)) {
                fetch_handle(bundles.borrow_mut(pubkey), count_rate)
            } else {
                abort error::permission_denied(EINVALID_SIGNATURE)
            }
        } else {
            abort error::permission_denied(EINVALID_SIGNATURE)
        }
    }

    #[test_only]
    use aptos_std::ed25519::{sign_arbitrary_bytes, generate_keys, validated_public_key_to_bytes, Signature};
    #[test_only]
    use aptos_framework::account::create_signer_for_test;
    #[test_only]
    use aptos_framework::timestamp;

    #[test_only]
    struct SignatureBundle has drop {
        pubkey: UnvalidatedPublicKey,
        signature: Signature,
    }

    #[test(account = @0xcafe, account_copy = @0xcafe)]
    fun test_basics(account: signer, account_copy: signer) acquires Delegation {
        let aptos_framework = create_signer_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        let (sk, vpk) = generate_keys();
        let signature = sign_arbitrary_bytes(&sk, vector[1, 2, 3]);
        let pubkey_bytes = validated_public_key_to_bytes(&vpk);
        let sig_bundle = SignatureBundle {
            pubkey: new_unvalidated_public_key_from_bytes(pubkey_bytes),
            signature,
        };
        let auth_data = auth_data::create_auth_data(vector[1, 2, 3], bcs::to_bytes(&sig_bundle));
        assert!(!is_permissioned_signer(&account), 1);
        add_permissioned_handle(&account, pubkey_bytes, option::none(), 60);
        let permissioned_signer = authenticate(account, auth_data);
        assert!(is_permissioned_signer(&permissioned_signer), 2);
        remove_permissioned_handle(&account_copy, pubkey_bytes);
    }

    #[test(account = @0xcafe, account_copy = @0xcafe, account_copy_2 = @0xcafe)]
    #[expected_failure(abort_code = 0x50006, location = Self)]
    fun test_rate_limit(account: signer, account_copy: signer, account_copy_2: signer) acquires Delegation {
        let aptos_framework = create_signer_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        let (sk, vpk) = generate_keys();
        let signature = sign_arbitrary_bytes(&sk, vector[1, 2, 3]);
        let pubkey_bytes = validated_public_key_to_bytes(&vpk);
        let sig_bundle = SignatureBundle {
            pubkey: new_unvalidated_public_key_from_bytes(pubkey_bytes),
            signature,
        };
        let auth_data = auth_data::create_auth_data(vector[1, 2, 3], bcs::to_bytes(&sig_bundle));
        assert!(!is_permissioned_signer(&account), 1);
        add_permissioned_handle(&account, pubkey_bytes, option::some(1), 60);
        authenticate(account, auth_data);
        authenticate(account_copy, auth_data);
        remove_permissioned_handle(&account_copy_2, pubkey_bytes);
    }
}
