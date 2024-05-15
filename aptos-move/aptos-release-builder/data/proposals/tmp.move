// Initialize AIP-28 parital governance voting.
// This script MUST be run before enabling the feature flag, otherwise no new proposal can be passed anymore.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::keyless_account;

    fun main(core_resources: &signer) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let alpha_g1 = vector[226, 242, 109, 190, 162, 153, 245, 34, 59, 100, 108, 177, 251, 51, 234, 219, 5, 157, 148, 7, 85, 157, 116, 65, 223, 217, 2, 227, 167, 154, 77, 45];
        let beta_g2 = vector[171, 183, 61, 193, 127, 188, 19, 2, 30, 36, 113, 224, 192, 139, 214, 125, 132, 1, 245, 43, 115, 214, 208, 116, 131, 121, 76, 173, 71, 120, 24, 14, 12, 6, 243, 59, 188, 76, 121, 169, 202, 222, 242, 83, 166, 128, 132, 211, 130, 241, 119, 136, 248, 133, 201, 175, 209, 118, 247, 203, 47, 3, 103, 137];
        let gamma_g2 = vector[237, 246, 146, 217, 92, 189, 222, 70, 221, 218, 94, 247, 212, 34, 67, 103, 121, 68, 92, 94, 102, 0, 106, 66, 118, 30, 31, 18, 239, 222, 0, 24, 194, 18, 243, 174, 183, 133, 228, 151, 18, 231, 169, 53, 51, 73, 170, 241, 37, 93, 251, 49, 183, 191, 96, 114, 58, 72, 13, 146, 147, 147, 142, 25];
        let delta_g2 = vector[205, 217, 151, 196, 248, 84, 45, 238, 193, 26, 1, 41, 17, 128, 191, 96, 250, 206, 79, 194, 34, 157, 155, 176, 236, 252, 208, 38, 247, 163, 103, 24, 31, 177, 139, 38, 20, 60, 191, 221, 154, 182, 94, 212, 42, 153, 27, 72, 145, 111, 161, 5, 197, 55, 200, 231, 217, 205, 108, 74, 132, 76, 255, 147];
        let gamma_abc_g1 = vector[
            vector[101, 97, 162, 155, 53, 67, 187, 159, 151, 107, 214, 235, 197, 112, 69, 7, 57, 28, 60, 2, 145, 182, 96, 181, 172, 71, 214, 221, 250, 75, 38, 44],
            vector[70, 159, 47, 207, 97, 141, 214, 199, 116, 40, 62, 236, 105, 233, 207, 163, 190, 217, 85, 185, 114, 155, 120, 236, 191, 104, 97, 98, 2, 233, 150, 161],
        ];
        let vk = keyless_account::new_groth16_verification_key(alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1);
        keyless_account::set_groth16_verification_key_for_next_epoch(&framework, vk);
        aptos_governance::reconfigure(&framework);
    }
}
