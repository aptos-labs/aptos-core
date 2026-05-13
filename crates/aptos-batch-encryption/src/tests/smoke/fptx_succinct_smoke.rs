// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    schemes::fptx_succinct::FPTXSuccinct, tests::smoke::run_smoke, traits::BatchThresholdEncryption,
};
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use ark_std::rand::{thread_rng, Rng as _};

#[test]
fn smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc = ShamirThresholdConfig::new(5, 8);

    let (ek, dk, vks, msk_shares) =
        FPTXSuccinct::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

    run_smoke::<FPTXSuccinct>(tc, ek, dk, vks, msk_shares);
}
