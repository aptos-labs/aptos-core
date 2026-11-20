// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_dkg::weighted_vuf::traits::WeightedVUF;
use aptos_types::{
    dkg::DKGSessionMetadata,
    randomness::{PKShare, SKShare, WvufPP, APK, ASK, WVUF},
};
use rand::{rngs::StdRng, SeedableRng};

fn augmentation_seed(
    sk: &SKShare,
    epoch: u64,
    metadata: &DKGSessionMetadata,
) -> Result<[u8; 32], bcs::Error> {
    const AUG_KEY_SEED_DOMAIN_SEPARATOR: &str = "APTOS_RANDOMNESS_AUG_KEY_SEED_V1";
    let seed_bytes = bcs::to_bytes(&(AUG_KEY_SEED_DOMAIN_SEPARATOR, sk, epoch, metadata))?;
    Ok(*HashValue::sha3_256_of(&seed_bytes).as_ref())
}

// Reproduce the same augmented key after losing RandDB within an epoch.
pub(super) fn derive_augmented_key_pair(
    pp: &WvufPP,
    sk: SKShare,
    pk: PKShare,
    epoch: u64,
    metadata: &DKGSessionMetadata,
) -> Result<(ASK, APK), bcs::Error> {
    let seed = augmentation_seed(&sk, epoch, metadata)?;
    let mut rng = StdRng::from_seed(seed);
    Ok(WVUF::augment_key_pair(pp, sk, pk, &mut rng))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_dkg::pvss::{das::PublicParameters, traits::HasEncryptionPublicParams};
    use aptos_types::{
        account_address::AccountAddress, on_chain_config::OnChainRandomnessConfig,
        validator_verifier::ValidatorConsensusInfoMoveStruct,
    };
    use blstrs::{G1Projective, Scalar};
    use group::Group;

    #[test]
    fn augmentation_derivation_golden() {
        const EXPECTED_SEED: &str =
            "2042a98ca46a2ad9dc35e99e377f2d944e0a072236b32efbb64973b187e9d2ee";
        const EXPECTED_ASK_BCS_SHA3_256: &str =
            "cb666954ebc144b4295984f814fc0bccc641ad1d4df3d4976162a7645e199878";
        const EXPECTED_APK_BCS_SHA3_256: &str =
            "56e7556690738486f97872d026244f22960e016320342f2f632d5431f5e9df40";

        let pp = PublicParameters::new_from_seed(b"randomness-augmentation-golden-v1");
        let scalars = [Scalar::from(7), Scalar::from(11)];
        let sk: SKShare = scalars
            .iter()
            .map(|s| {
                (pp.get_encryption_public_params().message_base() * s)
                    .to_compressed()
                    .as_slice()
                    .try_into()
                    .unwrap()
            })
            .collect();
        let pk: PKShare = scalars
            .iter()
            .map(|s| {
                (pp.get_commitment_base() * s)
                    .to_compressed()
                    .as_slice()
                    .try_into()
                    .unwrap()
            })
            .collect();
        let validators = vec![ValidatorConsensusInfoMoveStruct {
            addr: AccountAddress::ONE,
            pk_bytes: (G1Projective::generator() * Scalar::from(3))
                .to_compressed()
                .to_vec(),
            voting_power: 100,
        }];
        let metadata = DKGSessionMetadata {
            dealer_epoch: 41,
            randomness_config: OnChainRandomnessConfig::new_v1(50, 67).into(),
            dealer_validator_set: validators.clone(),
            target_validator_set: validators,
        };
        let seed = augmentation_seed(&sk, 42, &metadata).unwrap();
        let (ask, apk) =
            derive_augmented_key_pair(&WvufPP::from(&pp), sk, pk, 42, &metadata).unwrap();

        assert_eq!(hex::encode(seed), EXPECTED_SEED);
        assert_eq!(
            HashValue::sha3_256_of(&bcs::to_bytes(&ask).unwrap()).to_hex(),
            EXPECTED_ASK_BCS_SHA3_256
        );
        assert_eq!(
            HashValue::sha3_256_of(&bcs::to_bytes(&apk).unwrap()).to_hex(),
            EXPECTED_APK_BCS_SHA3_256
        );
    }
}
