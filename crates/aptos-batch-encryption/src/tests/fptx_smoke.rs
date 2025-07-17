use ark_std::rand::{seq::{IteratorRandom, SliceRandom}, thread_rng};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{schemes::fptx::FPTX, shared::{algebra::shamir::ThresholdConfig, key_derivation::BIBEDecryptionKeyShare}, traits::BatchThresholdEncryption};
use anyhow::Result;



#[test] 
fn smoke() {
    let mut rng = thread_rng();
    let tc = ThresholdConfig::new(8, 5);
    let tp = ThreadPoolBuilder::new().build().unwrap();

    let (ek, dk, vks, msk_shares) = FPTX::setup(&mut rng, 8, 1, &tc).unwrap();

    let plaintext : String = String::from("hi");

    let ct = FPTX::encrypt(&ek, &mut rng, &plaintext).unwrap();
    FPTX::verify_ct(&ct).unwrap();

    let (d, mut pfs) = FPTX::digest(&dk, &vec![ct.clone()], 0, &tp).unwrap();
    FPTX::eval_proofs_compute_all(&mut pfs, &tp);

    let dk_shares : Vec<<FPTX as BatchThresholdEncryption>::DecryptionKeyShare> = msk_shares.into_iter()
        .map(|msk_share| msk_share.derive_decryption_key_share(&d))
        .collect();

    dk_shares.iter()
        .zip(vks)
        .map(|(dk_share, vk)| FPTX::verify_decryption_key_share(&vk, &d, &dk_share))
        .collect::<Result<Vec<()>>>().unwrap();

    let dk = FPTX::reconstruct_decryption_key(
        &dk_shares
        .choose_multiple(&mut rng, 5)
        .cloned()
        .collect::<Vec<BIBEDecryptionKeyShare>>(),
        &tc, &tp).unwrap();


    let decrypted_plaintexts : Vec<String> = 
        FPTX::decrypt(&dk, &vec![ct], &pfs, &tp).unwrap();
    
    assert_eq!(decrypted_plaintexts[0], plaintext);
}
