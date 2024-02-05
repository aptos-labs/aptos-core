module aptos_framework::zkid {
    use std::signer;
    use aptos_framework::system_addresses;

    #[resource_group(scope = global)]
    struct ConfigGroup {}

    #[resource_group_member(group = aptos_framework::zkid::ConfigGroup)]
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
        /// 64-byte serialization of `\forall i \in {0, 1}, gamma^{-1} * (beta * a_i + alpha * b_i + c_i) * H`, where `H` is the generator of `G1`.
        gamma_abc_g1: vector<vector<u8>>,
    }

    #[resource_group_member(group = aptos_framework::zkid::ConfigGroup)]
    struct Configs has key, store {
        // No transaction can have more than this many zkID signatures.
        max_zkid_signatures_per_txn: u16,
        // How far in the future from the JWT issued at time the EPK expiry can be set.
        max_exp_horizon: u64,
    }

    // genesis.move needs to initialize the devnet VK
    friend aptos_framework::genesis;

    public(friend) fun initialize(fx: &signer, vk: Groth16VerificationKey, constants: Configs) {
        system_addresses::assert_aptos_framework(fx);

        move_to(fx, vk);
        move_to(fx, constants);
    }

    #[test_only]
    public fun initialize_for_test(fx: &signer, vk: Groth16VerificationKey, constants: Configs) {
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
            delta_g2: x"46b1bdc6b810f95c114bee5301568e9c7476cef69c2ec821910edf76df53c62be50f9f0319f5803b18bfc30129f6f9f81f283c78dcd3acdd24c769826ecf0d90",
            gamma_abc_g1: vector[
                x"7b4d27193362b6bffdd509f8f873b1ed4041c7bed52bc3724ca5971aab97a323",
                x"883684186ab474fe93e99e28250814c980e3cb2cd8fe6c2b26869920f4aa2c1c",
            ],
        }
    }

    public fun devnet_constants(): Configs {
        // TODO(zkid): Put reasonable defaults here.
        Configs {
            max_zkid_signatures_per_txn: 3,
            max_exp_horizon: 100_255_944, // 1159.55 days
        }
    }

    // Sets the zkID Groth16 verification key, only callable via governance proposal.
    // WARNING: If a malicious key is set, this would lead to stolen funds.
    public entry fun set_groth16_verification_key(fx: &signer, alpha_g1: vector<u8>, beta_g2: vector<u8>, gamma_g2: vector<u8>, delta_g2: vector<u8>, gamma_abc_g1: vector<vector<u8>>) acquires Groth16VerificationKey {
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
}
