// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use std::sync::Arc;
use criterion::{BenchmarkId, Criterion};
use rand::{Rng, thread_rng};
use aptos_crypto::{bls12381, Uniform};
use aptos_crypto::bls12381::{PrivateKey, PublicKey};
use aptos_dkg_runtime::dkg_manager::setup_deal_main;
use aptos_dkg_runtime::{dummy_dkg_init, dummy_dkg_init_deal, dummy_dkg_init_deal_verify};
use aptos_dkg_runtime::transcript_aggregation::verify_main;
use aptos_types::dkg::{DKGSessionMetadata, DKGTrait, DKGTranscript};
use aptos_types::dkg::real_dkg::{RealDKG, RealDKGPublicParams, Transcripts};
use aptos_types::dkg::real_dkg::rounding::MAINNET_STAKES;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct};
use move_core_types::account_address::AccountAddress;

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("foo");
    group.bench_function("v1_setup_deal", move |b| {
        b.iter_with_setup(
            || {
                dummy_dkg_init(OnChainRandomnessConfig::default_v1())
            },
            |(my_addr, my_index, my_sk, session_metadata)| {
                let _ = setup_deal_main::<RealDKG>(my_addr, my_index, my_sk, &session_metadata);
            }
        )
    });

    group.bench_function("v1_verify", move |b| {
        b.iter_with_setup(
            || {
                dummy_dkg_init_deal(OnChainRandomnessConfig::default_v1())
            },
            |(pub_params, transcript)| {
                let _ = verify_main::<RealDKG>(&pub_params, transcript.transcript_bytes);
            }
        )
    });

    group.bench_function("v1_agg", move |b| {
        b.iter_with_setup(
            || {
                dummy_dkg_init_deal_verify(OnChainRandomnessConfig::default_v1())
            },
            |(pub_params, mut trx_0, trx_1)| {
                <RealDKG as DKGTrait>::aggregate_transcripts(&pub_params, &mut trx_0, trx_1);
            }
        )
    });

    group.bench_function("v2_setup_deal", move |b| {
        b.iter_with_setup(
            || {
                dummy_dkg_init(OnChainRandomnessConfig::default_enabled())
            },
            |(my_addr, my_index, my_sk, session_metadata)| {
                let _ = setup_deal_main::<RealDKG>(my_addr, my_index, my_sk, &session_metadata);
            }
        )
    });

    group.bench_function("v2_verify", move |b| {
        b.iter_with_setup(
            || {
                dummy_dkg_init_deal(OnChainRandomnessConfig::default_enabled())
            },
            |(pub_params, transcript)| {
                let _ = verify_main::<RealDKG>(&pub_params, transcript.transcript_bytes);
            }
        )
    });

    group.bench_function("v2_agg", move |b| {
        b.iter_with_setup(
            || {
                dummy_dkg_init_deal_verify(OnChainRandomnessConfig::default_enabled())
            },
            |(pub_params, mut trx_0, trx_1)| {
                <RealDKG as DKGTrait>::aggregate_transcripts(&pub_params, &mut trx_0, trx_1);
            }
        )
    });

    group.finish();
}

criterion_group!(
    name = foo_benches;
    config = Criterion::default();
    targets = bench_group);
criterion_main!(foo_benches);
