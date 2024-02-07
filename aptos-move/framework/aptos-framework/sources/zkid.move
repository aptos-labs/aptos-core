module aptos_framework::zkid {
    use std::option;
    use std::option::Option;
    use std::signer;
    use aptos_framework::system_addresses;

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
        /// `\forall i \in {0, \ell}, 64-byte serialization of gamma^{-1} * (beta * a_i + alpha * b_i + c_i) * H`, where
        /// `H` is the generator of `G1` and `\ell` is 1 for the zkID relation.
        gamma_abc_g1: vector<vector<u8>>,
    }

    #[resource_group_member(group = aptos_framework::zkid::Group)]
    struct Configuration has key, store {
        // No transaction can have more than this many zkID signatures.
        max_zkid_signatures_per_txn: u16,
        // How far in the future from the JWT issued at time the EPK expiry can be set.
        max_exp_horizon_secs: u64,
        // The training wheels PK, if training wheels are on
        training_wheels_pubkey: Option<vector<u8>>,
        // The size of the "nonce commitment (to the EPK and expiration date)" stored in the JWT's `nonce`
        // field.
        nonce_commitment_num_bytes: u16,
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

    /// Returns the Groth16 VK for our devnet deployment.
    public fun devnet_groth16_vk(): Groth16VerificationKey {
        Groth16VerificationKey {
            alpha_g1: x"6d1c152d2705e35fe7a07a66eb8a10a7f42f1e38c412fbbc3ac7f9affc25dc24",
            beta_g2: x"e20a834c55ae6e2fcbd66636e09322727f317aff8957dd342afa11f936ef7c02cfdc8c9862849a0442bcaa4e03f45343e8bf261ef4ab58cead2efc17100a3b16",
            gamma_g2: x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            delta_g2: x"98c9283068e4bfc51266dcbabffb56bebeb65ece8d9104609026d0d89187961d0c69a4688b23f8a813ee74349785d116aedfcf3f3de15d7c9123b32eba326f23",
            gamma_abc_g1: vector[
                x"29f65be8be6b13c84c1b29d219f35b998db14be4f7506fff4a475512ef0d959f",
                x"1ddc291dfd35684b634f03cda96ae18139db1653471921c555b2750cbf49908c",
            ],
        }
    }

    public fun default_devnet_configuration(): Configuration {
        // TODO(zkid): Put reasonable defaults here.
        Configuration {
            max_zkid_signatures_per_txn: 3,
            max_exp_horizon_secs: 100_255_944, // ~1160 days
            training_wheels_pubkey: option::some(x"aa"),
            // The commitment is using the Poseidon-BN254 hash function, hence the 254-bit (32 byte) size.
            nonce_commitment_num_bytes: 32,
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
    public fun update_configuration(fx: &signer,
                                    max_zkid_signatures_per_txn: u16,
                                    max_exp_horizon_secs: u64,
                                    training_wheels_pubkey: Option<vector<u8>>,
                                    nonce_commitment_num_bytes: u16,
    ) acquires Configuration {
        system_addresses::assert_aptos_framework(fx);

        if (exists<Configuration>(signer::address_of(fx))) {
            let Configuration {
                max_zkid_signatures_per_txn: _,
                max_exp_horizon_secs: _,
                training_wheels_pubkey: _,
                nonce_commitment_num_bytes: _,
            } = move_from<Configuration>(signer::address_of(fx));
        };

        let configs = Configuration {
            max_zkid_signatures_per_txn,
            max_exp_horizon_secs,
            training_wheels_pubkey,
            nonce_commitment_num_bytes,
        };

        move_to(fx, configs);
    }
}
