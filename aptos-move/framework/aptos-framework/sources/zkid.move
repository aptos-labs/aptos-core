module aptos_framework::zkid {
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

    #[resource_group_member(group = aptos_framework::zkid::Group)]
    /// The 288-byte Groth16 verification key (VK) for the zkID relation.
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
        /// `H` is the generator of `G1` and `\ell` is 1 for the zkID relation.
        gamma_abc_g1: vector<vector<u8>>,
    }

    #[resource_group_member(group = aptos_framework::zkid::Group)]
    struct Configuration has key, store {
        /// An override `aud` for the identity of a recovery service, which will help users recover their zkID accounts
        /// associated with dapps or wallets that have disappeared.
        /// IMPORTANT: This recovery service **cannot** on its own take over user accounts; a user must first sign in
        /// via OAuth in the recovery service in order to allow it to rotate any of that user's zkID accounts.
        override_aud_vals: vector<String>,
        /// No transaction can have more than this many zkID signatures.
        max_zkid_signatures_per_txn: u16,
        /// How far in the future from the JWT issued at time the EPK expiry can be set.
        max_exp_horizon_secs: u64,
        /// The training wheels PK, if training wheels are on
        training_wheels_pubkey: Option<vector<u8>>,
        /// The size of the "nonce commitment (to the EPK and expiration date)" stored in the JWT's `nonce` field.
        nonce_commitment_num_bytes: u16,
        /// The max length of an ephemeral public key supported in our circuit (93 bytes)
        max_commited_epk_bytes: u16,
        /// The max length of the field name and value of the JWT's `iss` field supported in our circuit (e.g., `"iss":"aptos.com"`)
        max_iss_field_bytes: u16,
        /// The max length of the JWT field name and value (e.g., `"max_age":"18"`) supported in our circuit
        max_extra_field_bytes: u16,
        /// The max length of the base64url-encoded JWT header in bytes supported in our circuit
        max_jwt_header_b64_bytes: u32,
    }

    // genesis.move needs to initialize the devnet VK
    friend aptos_framework::genesis;

    public(friend) fun initialize(fx: &signer, vk: Groth16VerificationKey, constants: Configuration) {
        system_addresses::assert_aptos_framework(fx);

        move_to(fx, vk);
        move_to(fx, constants);
    }

    #[test_only]
    public fun initialize_for_test(fx: &signer, vk: Groth16VerificationKey, constants: Configuration) {
        initialize(fx, vk, constants)
    }

    public fun new_groth16_verification_key(alpha_g1: vector<u8>, beta_g2: vector<u8>, gamma_g2: vector<u8>, delta_g2: vector<u8>, gamma_abc_g1: vector<vector<u8>>): Groth16VerificationKey {
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
        max_zkid_signatures_per_txn: u16,
        max_exp_horizon_secs: u64,
        training_wheels_pubkey: Option<vector<u8>>,
        nonce_commitment_num_bytes: u16,
        max_commited_epk_bytes: u16,
        max_iss_field_bytes: u16,
        max_extra_field_bytes: u16,
        max_jwt_header_b64_bytes: u32
    ): Configuration {
        Configuration {
            override_aud_vals: override_aud_val,
            max_zkid_signatures_per_txn,
            max_exp_horizon_secs,
            training_wheels_pubkey,
            nonce_commitment_num_bytes,
            max_commited_epk_bytes,
            max_iss_field_bytes,
            max_extra_field_bytes,
            max_jwt_header_b64_bytes,
        }
    }

    /// Returns the Groth16 VK for our devnet deployment.
    public fun devnet_groth16_vk(): Groth16VerificationKey {
        Groth16VerificationKey {
            alpha_g1: x"6d1c152d2705e35fe7a07a66eb8a10a7f42f1e38c412fbbc3ac7f9affc25dc24",
            beta_g2: x"e20a834c55ae6e2fcbd66636e09322727f317aff8957dd342afa11f936ef7c02cfdc8c9862849a0442bcaa4e03f45343e8bf261ef4ab58cead2efc17100a3b16",
            gamma_g2: x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            delta_g2: x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            gamma_abc_g1: vector[
                x"9aae6580d6040e77969d70e748e861664228e3567e77aa99822f8a4a19c29101",
                x"e38ad8b845e3ef599232b43af2a64a73ada04d5f0e73f1848e6631e17a247415",
            ],
        }
    }

    /// Returns the configuration for our devnet deployment.
    public fun default_devnet_configuration(): Configuration {
        // TODO(zkid): Put reasonable defaults & circuit-specific constants here.
        Configuration {
            override_aud_vals: vector[],
            max_zkid_signatures_per_txn: 3,
            max_exp_horizon_secs: 100_255_944, // ~1160 days
            training_wheels_pubkey: option::some(x"aa"),
            // The commitment is using the Poseidon-BN254 hash function, hence the 254-bit (32 byte) size.
            nonce_commitment_num_bytes: 32,
            max_commited_epk_bytes: 3 * 31,
            max_iss_field_bytes: 126,
            max_extra_field_bytes:  350,
            max_jwt_header_b64_bytes: 300,
        }
    }

    // Sets the zkID Groth16 verification key, only callable via governance proposal.
    // WARNING: If a malicious key is set, this would lead to stolen funds.
    public fun update_groth16_verification_key(fx: &signer,
                                               alpha_g1: vector<u8>,
                                               beta_g2: vector<u8>,
                                               gamma_g2: vector<u8>,
                                               delta_g2: vector<u8>,
                                               gamma_abc_g1: vector<vector<u8>>,
    ) acquires Groth16VerificationKey {
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

        let vk = new_groth16_verification_key(alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1);
        move_to(fx, vk);
    }

    // Sets the zkID configuration, only callable via governance proposal.
    // WARNING: If a malicious key is set, this would lead to stolen funds.
    public fun update_configuration(fx: &signer, config: Configuration) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);

        if (exists<Configuration>(signer::address_of(fx))) {
            let Configuration {
                override_aud_vals: _,
                max_zkid_signatures_per_txn: _,
                max_exp_horizon_secs: _,
                training_wheels_pubkey: _,
                nonce_commitment_num_bytes: _,
                max_commited_epk_bytes: _,
                max_iss_field_bytes: _,
                max_extra_field_bytes: _,
                max_jwt_header_b64_bytes: _,
            } = move_from<Configuration>(signer::address_of(fx));
        };

        move_to(fx, config);
    }

    // Convenience method to set the zkID training wheels, only callable via governance proposal.
    // WARNING: If a malicious key is set, this would lead to stolen funds.
    public fun update_training_wheels(fx: &signer, pk: Option<vector<u8>>) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);
        if (option::is_some(&pk)) {
            assert!(vector::length(option::borrow(&pk)) == 32, E_TRAINING_WHEELS_PK_WRONG_SIZE)
        };

        let config = borrow_global_mut<Configuration>(signer::address_of(fx));
        config.training_wheels_pubkey = pk;
    }
}
