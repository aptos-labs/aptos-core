// Script hash: 12e7b871
// Modifying on-chain feature flags:
// Enabled Features: [CodeDependencyCheck, TreatFriendAsPrivate, Sha512AndRipeMd160Natives, VelorStdChainIdNatives, VMBinaryFormatV6, MultiEd25519PkValidateV2Natives, Blake2b256Native, ResourceGroups, MultisigAccounts, DelegationPools, Ed25519PubkeyValidateReturnFalseWrongLength]
// Disabled Features: []
//
script {
    use velor_framework::velor_governance;
    use std::features;

    fun main(proposal_id: u64) {
        let framework_signer = velor_governance::resolve_multi_step_proposal(
            proposal_id,
            @0000000000000000000000000000000000000000000000000000000000000001,
            vector[233u8,115u8,222u8,109u8,33u8,95u8,157u8,37u8,189u8,240u8,180u8,14u8,191u8,215u8,233u8,110u8,223u8,235u8,97u8,190u8,166u8,210u8,218u8,4u8,185u8,212u8,159u8,97u8,53u8,157u8,198u8,168u8,],
        );
        let enabled_blob: vector<u64> = vector[
            1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 14,
        ];

        let disabled_blob: vector<u64> = vector[

        ];

        features::change_feature_flags(&framework_signer, enabled_blob, disabled_blob);
    }
}
