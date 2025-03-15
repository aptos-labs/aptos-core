/// DApp-specific sub-Account "owned" by the parent account, which delegates full authorization to
/// the sub-account, via session keys.
/// Session key has full authorization over the DApp-specific sub-Account.
module aptos_framework::session_dapp_subaccount {
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

    struct AuthorizedSession has key {
        dapp_domain_to_public_key: OrderedMap<String, UnvalidatedPublicKey>,
    }

    entry fun register_dapp_account_session_key(account: &signer, dapp_domain: String, public_key: vector<u8>) {
        let public_key = new_unvalidated_public_key_from_bytes(public_key);

        if (!exists<AuthorizedSession>()) {
            move_to<AuthorizedSession>(account) = AuthorizedSession {
                dapp_domain_to_public_key: ordered_map::new(),
            };
        };

        borrow_global_mut<AuthorizedSession>(account_addr).dapp_domain_to_public_key.upsert(dapp_domain, public_key);
        // add event
    }

    /// Authorization function for domain account abstraction.
    public fun authenticate(account: signer, aa_auth_data: AbstractionAuthData): signer {
        let stream = bcs_stream::new(*aa_auth_data.domain_account_identity());

        let dapp_domain = stream.deserialize_string();
        let parent_account_address = stream.deserialize_address();
        assert!(stream.is_done(), error::permission_denied(EINVALID_ACCOUNT_IDENTITY));

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

        account
    }
}
