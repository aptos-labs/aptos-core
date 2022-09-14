script {
    use aptos_framework::aptos_governance;
    use std::features;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0000000000000000000000000000000000000000000000000000000000000001,
            vector[10u8,80u8,100u8,161u8,216u8,60u8,74u8,107u8,22u8,225u8,197u8,132u8,207u8,218u8,245u8,146u8,230u8,39u8,197u8,140u8,31u8,52u8,219u8,219u8,32u8,219u8,80u8,250u8,182u8,195u8,169u8,66u8,],
        };
        let enabled_blob: vector<u64> = vector[
            1, 2,
        ];

        let disabled_blob: vector<u64> = vector[

        ];

        features::change_feature_flags(framework_signer, enabled_blob, disabled_blob);
    }
}
