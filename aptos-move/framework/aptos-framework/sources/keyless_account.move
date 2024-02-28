/// This module is responsible for configuring keyless blockchain accounts which were introduced in
/// [AIP-61](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-61.md).
module aptos_framework::keyless_account {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_framework::system_addresses;

    /// The training wheels PK needs to be 32 bytes long.
    const E_TRAINING_WHEELS_PK_WRONG_SIZE : u64 = 1;

    #[resource_group(scope = global)]
    struct Group {}

    #[resource_group_member(group = aptos_framework::keyless_account::Group)]
    /// The 288-byte Groth16 verification key (VK) for the ZK relation that implements keyless accounts
    struct Groth16VerificationKey has key, store {
        /// 32-byte serialization of `alpha * G`, where `G` is the generator of `G1`.
        alpha_g1: vector<u8>,
        /// 64-byte serialization of `alpha * H`, where `H` is the generator of `G2`.
        beta_g2: vector<u8>,
        /// 64-byte serialization of `gamma * H`, where `H` is the generator of `G2`.
        gamma_g2: vector<u8>,
        /// 64-byte serialization of `delta * H`, where `H` is the generator of `G2`.
        delta_g2: vector<u8>,
        /// `\forall i \in {0, ..., \ell}, 64-byte serialization of gamma^{-1} * (beta * a_i + alpha * b_i + c_i) * H`, where
        /// `H` is the generator of `G1` and `\ell` is 1 for the ZK relation.
        gamma_abc_g1: vector<vector<u8>>,
    }

    #[resource_group_member(group = aptos_framework::keyless_account::Group)]
    struct Configuration has key, store {
        /// An override `aud` for the identity of a recovery service, which will help users recover their keyless accounts
        /// associated with dapps or wallets that have disappeared.
        /// IMPORTANT: This recovery service **cannot** on its own take over user accounts; a user must first sign in
        /// via OAuth in the recovery service in order to allow it to rotate any of that user's keyless accounts.
        override_aud_vals: vector<String>,
        /// No transaction can have more than this many keyless signatures.
        max_signatures_per_txn: u16,
        /// How far in the future from the JWT issued at time the EPK expiry can be set.
        max_exp_horizon_secs: u64,
        /// The training wheels PK, if training wheels are on
        training_wheels_pubkey: Option<vector<u8>>,
        /// The max length of an ephemeral public key supported in our circuit (93 bytes)
        max_commited_epk_bytes: u16,
        /// The max length of the value of the JWT's `iss` field supported in our circuit (e.g., `"https://accounts.google.com"`)
        max_iss_val_bytes: u16,
        /// The max length of the JWT field name and value (e.g., `"max_age":"18"`) supported in our circuit
        max_extra_field_bytes: u16,
        /// The max length of the base64url-encoded JWT header in bytes supported in our circuit
        max_jwt_header_b64_bytes: u32,
    }

    #[test_only]
    public fun initialize_for_test(fx: &signer, vk: Groth16VerificationKey, constants: Configuration) {
        system_addresses::assert_aptos_framework(fx);

        move_to(fx, vk);
        move_to(fx, constants);
    }

    public fun new_groth16_verification_key(alpha_g1: vector<u8>,
                                            beta_g2: vector<u8>,
                                            gamma_g2: vector<u8>,
                                            delta_g2: vector<u8>,
                                            gamma_abc_g1: vector<vector<u8>>
    ): Groth16VerificationKey {
        Groth16VerificationKey {
            alpha_g1,
            beta_g2,
            gamma_g2,
            delta_g2,
            gamma_abc_g1,
        }
    }

    public fun new_configuration(
        override_aud_val: vector<String>,
        max_signatures_per_txn: u16,
        max_exp_horizon_secs: u64,
        training_wheels_pubkey: Option<vector<u8>>,
        max_commited_epk_bytes: u16,
        max_iss_val_bytes: u16,
        max_extra_field_bytes: u16,
        max_jwt_header_b64_bytes: u32
    ): Configuration {
        Configuration {
            override_aud_vals: override_aud_val,
            max_signatures_per_txn,
            max_exp_horizon_secs,
            training_wheels_pubkey,
            max_commited_epk_bytes,
            max_iss_val_bytes,
            max_extra_field_bytes,
            max_jwt_header_b64_bytes,
        }
    }

    // Sets the Groth16 verification key, only callable via governance proposal.
    // WARNING: If a malicious key is set, this would lead to stolen funds.
    public fun update_groth16_verification_key(fx: &signer, vk: Groth16VerificationKey) acquires Groth16VerificationKey {
        system_addresses::assert_aptos_framework(fx);

        if (exists<Groth16VerificationKey>(signer::address_of(fx))) {
            let Groth16VerificationKey {
                alpha_g1: _,
                beta_g2: _,
                gamma_g2: _,
                delta_g2: _,
                gamma_abc_g1: _
            } = move_from<Groth16VerificationKey>(signer::address_of(fx));
        };

        move_to(fx, vk);
    }

    // Sets the keyless configuration, only callable via governance proposal.
    // WARNING: If a malicious key is set, this would lead to stolen funds.
    public fun update_configuration(fx: &signer, config: Configuration) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);

        if (exists<Configuration>(signer::address_of(fx))) {
            let Configuration {
                override_aud_vals: _,
                max_signatures_per_txn: _,
                max_exp_horizon_secs: _,
                training_wheels_pubkey: _,
                max_commited_epk_bytes: _,
                max_iss_val_bytes: _,
                max_extra_field_bytes: _,
                max_jwt_header_b64_bytes: _,
            } = move_from<Configuration>(signer::address_of(fx));
        };

        move_to(fx, config);
    }

    // Convenience method to [un]set the training wheels PK, only callable via governance proposal.
    // WARNING: If a malicious key is set, this would lead to stolen funds.
    public fun update_training_wheels(fx: &signer, pk: Option<vector<u8>>) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);
        if (option::is_some(&pk)) {
            assert!(vector::length(option::borrow(&pk)) == 32, E_TRAINING_WHEELS_PK_WRONG_SIZE)
        };

        let config = borrow_global_mut<Configuration>(signer::address_of(fx));
        config.training_wheels_pubkey = pk;
    }

    // Convenience method to set the max expiration horizon, only callable via governance proposal.
    public fun update_max_exp_horizon(fx: &signer, max_exp_horizon_secs: u64) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);

        let config = borrow_global_mut<Configuration>(signer::address_of(fx));
        config.max_exp_horizon_secs = max_exp_horizon_secs;
    }

    // Convenience method to clear the set of override `aud`'s, only callable via governance proposal.
    // WARNING: When no override `aud` is set, recovery of keyless accounts associated with applications that disappeared
    // is no longer possible.
    public fun remove_all_override_auds(fx: &signer) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);

        let config = borrow_global_mut<Configuration>(signer::address_of(fx));
        config.override_aud_vals = vector[];
    }

    // Convenience method to append to the set of override `aud`'s, only callable via governance proposal.
    // WARNING: If a malicious override `aud` is set, this would lead to stolen funds.
    public fun add_override_aud(fx: &signer, aud: String) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);

        let config = borrow_global_mut<Configuration>(signer::address_of(fx));
        vector::push_back(&mut config.override_aud_vals, aud);
    }
}
