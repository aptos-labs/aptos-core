// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    schemes::fptx_succinct::FPTXSuccinct, tests::smoke::SmokeTest, traits::BatchThresholdEncryption,
};
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use ark_std::rand::{thread_rng, Rng as _};

#[test]
fn smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc = ShamirThresholdConfig::new(5, 8);

    let (ek, dk, vks, msk_shares) =
        FPTXSuccinct::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

    let smoke_test = SmokeTest::<FPTXSuccinct>::new(tc, ek, dk, vks, msk_shares);
    smoke_test.run_with_one_ct(0).test_decryption_verification();
    smoke_test
        .run_with_max_cts(0)
        .test_decryption_verification();
}
