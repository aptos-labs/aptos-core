module aptos_framework::permissioned_delegation {
    use std::error;
    use std::signer;
    use aptos_std::ed25519;
    use aptos_std::ed25519::{new_signature_from_bytes, new_unvalidated_public_key_from_bytes, UnvalidatedPublicKey};
    use aptos_std::table::{Self, Table};
    use aptos_framework::bcs_stream;
    use aptos_framework::bcs_stream::deserialize_u8;
    use aptos_framework::permissioned_signer::{
        Self,
        is_permissioned_signer,
        StorablePermissionedHandle, check_permission_capacity_above
    };
    #[test_only]
    use std::bcs;

    const ENOT_MASTER_SIGNER: u64 = 1;
    const EINVALID_PUBLIC_KEY: u64 = 2;
    const EPUBLIC_KEY_NOT_FOUND: u64 = 3;
    const EINVALID_SIGNATURE: u64 = 4;
    const EHANDLE_EXISTENCE: u64 = 5;

    struct Delegation has key {
        handles: Table<ed25519::UnvalidatedPublicKey, StorablePermissionedHandle>
    }

    public fun add_permissioned_handle(
        master: &signer,
        key: vector<u8>,
        expiration_time: u64,
    ): signer acquires Delegation {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        if (!exists<Delegation>(addr)) {
            move_to(master, Delegation {
                handles: table::new()
            });
        };
        let handles = &mut borrow_global_mut<Delegation>(addr).handles;
        assert!(!table::contains(handles, pubkey), error::already_exists(EHANDLE_EXISTENCE));
        let handle = permissioned_signer::create_storable_permissioned_handle(master, expiration_time);
        let permissioned_signer = permissioned_signer::signer_from_storable_permissioned(&handle);
        table::add(handles, pubkey, handle);
        permissioned_signer
    }

    public fun remove_permissioned_handle(
        master: &signer,
        key: vector<u8>,
    ) acquires Delegation {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        let handles = &mut borrow_global_mut<Delegation>(addr).handles;
        assert!(table::contains(handles, pubkey), error::not_found(EHANDLE_EXISTENCE));
        permissioned_signer::destroy_storable_permissioned_handle(table::remove(handles, pubkey));
    }

    public fun permissioned_signer_by_key(
        master: &signer,
        key: vector<u8>,
    ): signer acquires Delegation {
        assert!(!is_permissioned_signer(master), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(master);
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        get_permissioned_signer(addr, pubkey)
    }

    public fun remove_permissioned_handle_by_delegate(
        master: address,
        signature: vector<u8>,
    ): StorablePermissionedHandle acquires Delegation {
        let stream = bcs_stream::new(signature);
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
                vector[1, 2, 3],
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );
        let handles = &mut borrow_global_mut<Delegation>(master).handles;
        assert!(table::contains(handles, public_key), error::not_found(EHANDLE_EXISTENCE));
        table::remove(handles, public_key)
    }

    #[view]
    public fun handle_address_by_key(master: address, key: vector<u8>): address acquires Delegation {
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(key);
        let handles = &borrow_global<Delegation>(master).handles;
        assert!(table::contains(handles, pubkey), error::not_found(EHANDLE_EXISTENCE));
        permissioned_signer::permission_address(table::borrow(handles, pubkey))
    }

    /// Authorization function for account abstraction.
    public fun authenticate(account: signer, signature: vector<u8>): signer acquires Delegation {
        let addr = signer::address_of(&account);
        let stream = bcs_stream::new(signature);
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
                vector[1, 2, 3],
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );
        get_permissioned_signer(addr, public_key)
    }

    inline fun get_permissioned_signer(master: address, pubkey: UnvalidatedPublicKey): signer {
        if (exists<Delegation>(master)) {
            let handles = &borrow_global<Delegation>(master).handles;
            if (table::contains(handles, pubkey)) {
                let signer = permissioned_signer::signer_from_storable_permissioned(table::borrow(handles, pubkey));
                signer
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
    use aptos_framework::transaction_context;

    #[test_only]
    struct SignatureBundle has drop {
        pubkey: UnvalidatedPublicKey,
        signature: Signature,
    }

    #[test(account = @0x123, account_copy = @0x123)]
    fun test_basics(account: signer, account_copy: signer) acquires Delegation {
        let aptos_framework = create_signer_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        let (sk, vpk) = generate_keys();
        let txn_hash = transaction_context::get_transaction_hash();
        let signature = sign_arbitrary_bytes(&sk, txn_hash);
        let pubkey_bytes = validated_public_key_to_bytes(&vpk);
        let sig_bundle = SignatureBundle {
            pubkey: new_unvalidated_public_key_from_bytes(pubkey_bytes),
            signature,
        };
        let sudo_signer = authenticate(account, bcs::to_bytes(&sig_bundle));
        assert!(!is_permissioned_signer(&sudo_signer), 1);

        add_permissioned_handle(&sudo_signer, pubkey_bytes, 60);
        let permissioned_signer = authenticate(sudo_signer, bcs::to_bytes(&sig_bundle));
        assert!(is_permissioned_signer(&permissioned_signer), 2);
        remove_permissioned_handle(&account_copy, pubkey_bytes);
    }
}
