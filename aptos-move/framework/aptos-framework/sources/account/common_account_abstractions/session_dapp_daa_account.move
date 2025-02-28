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

    struct Ed25519HexProof {
        type_info: TypeInfo,
        dapp_domain: String,
        session_public_key: UnvalidatedPublicKey,

    }

    // Account authorized sessions are stored onchain
    // For rest, proof is always passed in the authenticator

    struct AccountAuthorizedSession has key {
        dapp_domain_to_public_key: OrderedMap<String, UnvalidatedPublicKey>,
    }

    entry fun register_dapp_account_session_key(account: &signer, dapp_domain: String, public_key: vector<u8>) {
        let public_key = new_unvalidated_public_key_from_bytes(public_key);

        if (!exists<AccountAuthorizedSession>()) {
            move_to<AccountAuthorizedSession>(account) = AccountAuthorizedSession {
                dapp_domain_to_public_key: ordered_map::new(),
            };
        };

        borrow_global_mut<AccountAuthorizedSession>(account_addr).dapp_domain_to_public_key.upsert(dapp_domain, public_key);
        // add event
    }

    /// Authorization function for domain account abstraction.
    ///
    /// Format:
    /// account_identity:
    ///  - enum_index from AuthenticationType
    ///  - dapp_domain
    ///  - account address if AuthenticationType::Account, otherwise parent (i.e. external) public key
    /// authenticator:
    ///  - for AuthenticationType::Account:
    ///     - signature prooving digest was signed by session private key
    ///  - for AuthenticationType::Ed25519_hex:
    ///     - session public key
    ///     - signature prooving session public key has been authorized by parent (external) private key
    ///     - signature prooving digest was signed by session private key
    public fun authenticate(account: signer, aa_auth_data: AbstractionAuthData): signer {
        let identity_stream = bcs_stream::new(*aa_auth_data.domain_account_identity());

        let enum_index = identity_stream.deserialize_uleb128();
        let dapp_domain = identity_stream.deserialize_string();

        if (enum_index == 0) { // AuthenticationType::Account
            let parent_account_address = identity_stream.deserialize_address();
            assert!(identity_stream.is_done(), error::permission_denied(EINVALID_ACCOUNT_IDENTITY));

            let public_key = borrow_global<AuthorizedSession>(parent_account_address).get(dapp_domain);

            let signature = new_signature_from_bytes(*aa_auth_data.domain_authenticator());
            assert!(
                ed25519::signature_verify_strict(
                    &signature,
                    &public_key,
                    *aa_auth_data.digest(),
                ),
                error::permission_denied(EINVALID_SIGNATURE)
            );
        } else if (enum_index == 1) { // AuthenticationType::Ed25519_hex
            // first we get proof of session key via Ed25519_hex
            // then we get proof of digest signed by the session key

            let parent_public_key = new_unvalidated_public_key_from_bytes(identity_stream.deserialize_vector(|s| s.deserialize_u8()));

            let authenticator_stream = bcs_stream::new(*aa_auth_data.domain_authenticator());

            let session_public_key = authenticator_stream.deserialize_vector(|s| s.deserialize_u8());
            let session_key_authorization_signature = new_signature_from_bytes(authenticator_stream.deserialize_vector(|s| s.deserialize_u8()));
            let digest_authorization_signature = new_signature_from_bytes(authenticator_stream.deserialize_vector(|s| s.deserialize_u8()));

            let challenge = Ed25519HexProof {
                type_info: type_info::type_of<Ed25519HexProof>(),
                dapp_domain,
                session_public_key,
            };
            let hex_digest = string_utils::to_string(bcs::to_bytes(challenge));
            // cannot use signature_verify_strict_t, as we require hex.
            assert!(
                ed25519::signature_verify_strict(
                    &session_key_authorization_signature,
                    &parent_public_key,
                    *hex_digest.bytes(),
                ),
                error::permission_denied(EINVALID_SIGNATURE)
            );

            assert!(
                ed25519::signature_verify_strict(
                    &digest_authorization_signature,
                    &session_public_key,
                    *aa_auth_data.digest(),
                ),
                error::permission_denied(EINVALID_SIGNATURE)
            );

        } else {
            error::permission_denied(EINVALID_ACCOUNT_IDENTITY);
        };


        account
    }
}
