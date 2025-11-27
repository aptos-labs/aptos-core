use aptos_batch_encryption::{
    schemes::fptx::*, shared::algebra::shamir::ThresholdConfig,
    traits::BatchThresholdEncryption as _,
};
use ark_std::rand::{thread_rng, Rng as _};
use std::{fs::File, io::Write as _};

fn main() {
    println!("hi");

    let mut rng = thread_rng();
    let tc_happy = ThresholdConfig::new(8, 5);
    let tc_slow = ThresholdConfig::new(8, 3);

    let (_ek, dk, _vks_happy, _msk_shares_happy, _vks_slow, _msk_shares_slow) =
        FPTX::setup_for_testing(rng.gen(), 128, 108000, &tc_happy, &tc_slow).unwrap();

    let mut file = File::create("test_trusted_setup.bin").unwrap();
    file.write_all(&bcs::to_bytes(&dk).unwrap()).unwrap();
}
