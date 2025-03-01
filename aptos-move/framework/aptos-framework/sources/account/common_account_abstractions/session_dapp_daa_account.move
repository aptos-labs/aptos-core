/// DApp-specific Account "owned" by the specific authentication scheme (parent account or x-chain private/public key pair),
/// which delegates full authorization tothe sub-account, via session keys.
///
/// Session key has full authorization over the DApp-specific Account.
module aptos_framework::session_dapp_daa_account {
    use std::error;
    use aptos_std::string_utils;
    use aptos_std::ed25519::{
        Self,
        new_signature_from_bytes,
        new_unvalidated_public_key_from_bytes,
    };
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::ordered_map::{Self, OrderedMap};
    use aptos_framework::bcs_stream::{Self, deserialize_u8};

    const EINVALID_SIGNATURE: u64 = 1;
    const EINVALID_ACCOUNT_IDENTITY: u64 = 2;

    enum AuthenticationType {
        Account,
        Ed25519_hex,
    }

    struct SessionKey {
        authentication_type: AuthenticationType,
        dapp_domain: String,
        authentication_identity: vector<u8>,
    }

    struct AuthorizedSession has key {
        public_keys: BigOrderedMap<SessionKey, UnvalidatedPublicKey>,
    }

    struct Ed25519HexProof {
        type_info: TypeInfo,
        dapp_domain: String,
        session_public_key: UnvalidatedPublicKey,

    }

    fun init_module(account: &signer) {
        move_to(account, AuthorizedSession {
            public_keys: big_ordered_map::new_with_config(0, 0, true),
        });
    }

    entry fun register_dapp_account_session_key(account: &signer, dapp_domain: String, session_public_key: vector<u8>, signature: vector<u8>) {
        let public_key = new_unvalidated_public_key_from_bytes(public_key);

        let session_key = SessionKey {
            dapp_domain,
            authentication_type: AuthenticationType::Account,
            authentication_identity: signer::address_of(account).bytes(),
        }

        borrow_global_mut<AuthorizedSession>(@aptos_framework).public_keys.upsert(session_key, session_public_key);
        // add event
    }

    public fun register_dapp_ed25519_hex_session_key(dapp_domain: String, session_public_key: vector<u8>, parent_public_key: vector<u8>, signature: vector<u8>) {
        let session_key = SessionKey {
            dapp_domain,
            authentication_type: AuthenticationType::Ed25519_hex,
            authentication_identity: parent_public_key,
        }

        let session_public_key = new_unvalidated_public_key_from_bytes(session_public_key);
        let parent_public_key = new_unvalidated_public_key_from_bytes(parent_public_key);

        let challenge = Ed25519HexProof {
            type_info: type_info::type_of<Ed25519HexProof>(),
            dapp_domain,
            session_public_key,
        };
        let hex_digest = string_utils::to_string(bcs::to_bytes(challenge));
        // cannot use signature_verify_strict_t, as we require hex.
        assert!(
            ed25519::signature_verify_strict(
                &new_signature_from_bytes(signature),
                &parent_public_key,
                *hex_digest.bytes(),
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );

        borrow_global_mut<AuthorizedSession>(@aptos_framework).public_keys.upsert(session_key, session_public_key);
    }

    /// Authorization function for domain account abstraction.
    public fun authenticate(account: signer, aa_auth_data: AbstractionAuthData): signer {
        let identity_stream = bcs_stream::new(*aa_auth_data.domain_account_identity());

        let enum_index = identity_stream.deserialize_uleb128();
        let dapp_domain = identity_stream.deserialize_string();

        let (authentication_type, authentication_identity) = if (enum_index == 0) { // AuthenticationType::Account
            let parent_account_address = identity_stream.deserialize_address();
            assert!(identity_stream.is_done(), error::permission_denied(EINVALID_ACCOUNT_IDENTITY));
            (AuthenticationType::Account, parent_account_address)
        } else if (enum_index == 1) { // AuthenticationType::Ed25519_hex
            let parent_public_key = identity_stream.deserialize_vector(|s| s.deserialize_u8());
            (AuthenticationType::Ed25519_hex, parent_public_key)
        } else {
            error::permission_denied(EINVALID_ACCOUNT_IDENTITY);
        };

        assert!(stream.is_done(), error::permission_denied(EINVALID_ACCOUNT_IDENTITY));

        let session_key = SessionKey {
            dapp_domain,
            authentication_type,
            authentication_identity,
        }

        let public_key = borrow_global<AuthorizedSession>(parent_account_address).get(session_key);
        let signature = new_signature_from_bytes(*aa_auth_data.domain_authenticator());
        assert!(
            ed25519::signature_verify_strict(
                &signature,
                &public_key,
                *aa_auth_data.digest(),
            ),
            error::permission_denied(EINVALID_SIGNATURE)
        );

        account
    }
}
