// Copyright © Aptos Foundation

use crate::{assert_abort, assert_success, setup_staking, MoveHarness};
use aptos_crypto::HashValue;
use aptos_types::{
    account_config::BlockResource,
    dkg::{DKGSessionState, DKGState},
    on_chain_config::{
        ConfigurationResource, CurrentTimeMicroseconds, FeatureFlag, Features, OnChainConfig,
        ValidatorSet,
    },
};
use move_core_types::{account_address::AccountAddress, move_resource::MoveStructType};
use std::time::Duration;

struct HarnessWrapper {
    cur_epoch: u64,
    cur_round: u64,
    cur_microseconds: u64,
    epoch_interval: Duration,
    validator_addrs: Vec<AccountAddress>,
    pub inner: MoveHarness,
}

impl HarnessWrapper {
    pub fn new() -> Self {
        let inner = MoveHarness::new();
        let mut ret = Self {
            cur_epoch: 0,
            cur_round: 0,
            cur_microseconds: 0,
            epoch_interval: Duration::ZERO,
            validator_addrs: vec![],
            inner,
        };
        ret.reload_metadata();
        ret
    }

    fn reload_metadata(&mut self) {
        let timestamp_resource = self.read::<CurrentTimeMicroseconds>();
        let validator_set = self.read::<ValidatorSet>();
        let configuration_resource = self.read::<ConfigurationResource>();
        let epoch_interval = Duration::from_micros(
            self.inner
                .read_resource::<BlockResource>(&AccountAddress::ONE, BlockResource::struct_tag())
                .unwrap()
                .epoch_interval(),
        );
        let validator_addrs = validator_set
            .active_validators
            .iter()
            .chain(validator_set.pending_inactive.iter())
            .map(|vi| vi.account_address)
            .collect();
        self.cur_epoch = configuration_resource.epoch();
        self.validator_addrs = validator_addrs;
        self.cur_round = 0;
        self.cur_microseconds = timestamp_resource.microseconds;
        self.epoch_interval = epoch_interval;
    }

    pub fn fast_forward_a_bit(&mut self) {
        self.cur_microseconds += 1000;
    }

    pub fn fast_forward_to_epoch_expiry(&mut self) {
        self.cur_microseconds += self.epoch_interval.as_micros() as u64
    }

    pub fn fast_forward_then_default_block_prologue_ext(&mut self, to_epoch_expiry: bool) {
        if to_epoch_expiry {
            self.fast_forward_to_epoch_expiry();
        } else {
            self.fast_forward_a_bit();
        }
        self.cur_round += 1;
        self.inner.run_block_prologue_ext(
            HashValue::zero(),
            self.cur_epoch,
            self.cur_round,
            self.validator_addrs[0],
            vec![],
            vec![],
            self.cur_microseconds,
            Some(vec![0; 32]),
        );
    }

    pub fn read<T: OnChainConfig>(&mut self) -> T {
        self.inner
            .read_resource::<T>(&AccountAddress::ONE, T::struct_tag())
            .unwrap()
    }

    #[allow(dead_code)]
    pub fn finish_reconfig_with_dkg(&mut self) {
        self.inner.finish_reconfig_with_dkg_result(vec![0xFF; 48]);
        self.reload_metadata();
    }
}

#[test]
fn reconfig_with_dkg_end_to_end() {
    let mut harness_wrapper = HarnessWrapper::new();

    harness_wrapper.fast_forward_then_default_block_prologue_ext(false);

    // Alice asks to join.
    let account_alice = harness_wrapper
        .inner
        .new_account_at(AccountAddress::from_hex_literal("0x1234").unwrap());
    assert_success!(setup_staking(
        &mut harness_wrapper.inner,
        &account_alice,
        1_000_000
    ));

    // let account_bob = harness_wrapper.inner.new_account_at(AccountAddress::from_hex_literal("0x5678").unwrap());
    // let account_carl = harness_wrapper.inner.new_account_at(AccountAddress::from_hex_literal("0x90ab").unwrap());

    harness_wrapper.fast_forward_then_default_block_prologue_ext(false);

    // Send a txn to disable a feature. It should not take effect immediately.
    assert!(harness_wrapper
        .read::<Features>()
        .is_enabled(FeatureFlag::BLS12_381_STRUCTURES));
    harness_wrapper
        .inner
        .change_features_for_next_epoch(vec![], vec![FeatureFlag::BLS12_381_STRUCTURES]);
    assert!(harness_wrapper
        .read::<Features>()
        .is_enabled(FeatureFlag::BLS12_381_STRUCTURES));

    // This block triggers reconfiguration.
    harness_wrapper.fast_forward_then_default_block_prologue_ext(true);

    let DKGState {
        last_complete,
        in_progress,
    } = harness_wrapper.read::<DKGState>();
    assert!(last_complete.is_none());
    let DKGSessionState { metadata, .. } = in_progress.unwrap();
    assert_eq!(2, metadata.target_validator_set.len());
    assert_eq!(1, metadata.dealer_validator_set.len());
    assert!(harness_wrapper
        .read::<Features>()
        .is_enabled(FeatureFlag::BLS12_381_STRUCTURES));

    // DKG may last multiple rounds.
    harness_wrapper.fast_forward_then_default_block_prologue_ext(false);

    // Join request in the middle of a reconfiguration should be rejected.
    let account_bob = harness_wrapper
        .inner
        .new_account_at(AccountAddress::from_hex_literal("0x5678").unwrap());
    let join_status = setup_staking(&mut harness_wrapper.inner, &account_bob, 1_000_000);
    assert_abort!(join_status, _);

    // It is still possible to make changes to on-chain config buffer.
    assert!(harness_wrapper
        .read::<Features>()
        .is_enabled(FeatureFlag::BN254_STRUCTURES));
    harness_wrapper
        .inner
        .change_features_for_next_epoch(vec![], vec![FeatureFlag::BN254_STRUCTURES]);
    assert!(harness_wrapper
        .read::<Features>()
        .is_enabled(FeatureFlag::BN254_STRUCTURES));

    // TODO: fix the aggregator issue and complete it.
    // harness_wrapper.finish_reconfig_with_dkg();
}
