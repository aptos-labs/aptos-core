// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::response::InternalError;
use aptos_api_types::{AptosErrorCode, GasEstimation, LedgerInfo};
use aptos_config::config::{
    FromLocalHistoryGasEstimationMode, FromOnChainGasEstimationMode, GasEstimationConfig,
    GasEstimationMode,
};
use aptos_logger::error;
use aptos_mempool::{
    GetLatencySummaryRequest, IndividualLatencyStat, MempoolClientRequest, MempoolClientSender,
    MempoolLatencySummary,
};
use aptos_storage_interface::DbReader;
use aptos_types::{
    on_chain_config::OnChainExecutionConfig,
    transaction::{
        use_case::{UseCaseAwareTransaction, UseCaseKey},
        BlockEndInfo, Transaction, Version,
    },
};
use futures::{channel::oneshot, SinkExt};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Bound::Included, Deref},
    sync::{Arc, RwLock, RwLockWriteGuard},
    time::Instant,
};

pub struct GasEstimationCache {
    last_updated_epoch: Option<u64>,
    last_updated_time: Option<Instant>,
    estimation: Option<GasEstimation>,
    /// (epoch, lookup_version) -> min_inclusion_price
    min_inclusion_prices: BTreeMap<(u64, u64), u64>,
}

struct GasBuckets {
    gas_buckets: Vec<u64>,
}

impl GasBuckets {
    fn new(gas_buckets: Vec<u64>) -> Self {
        Self { gas_buckets }
    }

    fn next_bucket(&self, gas_unit_price: u64) -> u64 {
        match self
            .gas_buckets
            .iter()
            .find(|bucket| **bucket > gas_unit_price)
        {
            None => gas_unit_price,
            Some(bucket) => *bucket,
        }
    }
}

pub struct GasEstimator {
    config: GasEstimationConfig,
    gas_buckets: GasBuckets,
    gas_estimation_cache: Arc<RwLock<GasEstimationCache>>,

    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
}

impl GasEstimator {
    pub(crate) fn new(
        config: GasEstimationConfig,
        gas_buckets: Vec<u64>,
        db: Arc<dyn DbReader>,
        mp_sender: MempoolClientSender,
    ) -> Self {
        Self {
            config,
            gas_buckets: GasBuckets::new(gas_buckets),
            gas_estimation_cache: Arc::new(RwLock::new(GasEstimationCache {
                last_updated_epoch: None,
                last_updated_time: None,
                estimation: None,
                min_inclusion_prices: BTreeMap::new(),
            })),
            db,
            mp_sender,
        }
    }

    fn default_gas_estimation(&self, min_gas_unit_price: u64) -> GasEstimation {
        GasEstimation {
            deprioritized_gas_estimate: Some(min_gas_unit_price),
            gas_estimate: min_gas_unit_price,
            prioritized_gas_estimate: Some(self.gas_buckets.next_bucket(min_gas_unit_price)),
        }
    }

    fn cached_gas_estimation<T>(&self, cache: &T, current_epoch: u64) -> Option<GasEstimation>
    where
        T: Deref<Target = GasEstimationCache>,
    {
        if let Some(epoch) = cache.last_updated_epoch {
            if let Some(time) = cache.last_updated_time {
                if let Some(estimation) = cache.estimation {
                    if epoch == current_epoch
                        && (time.elapsed().as_millis() as u64) < self.config.cache_expiration_ms
                    {
                        return Some(estimation);
                    }
                }
            }
        }
        None
    }

    fn update_cached_gas_estimation(
        cache: &mut RwLockWriteGuard<GasEstimationCache>,
        epoch: u64,
        estimation: GasEstimation,
    ) {
        cache.last_updated_epoch = Some(epoch);
        cache.estimation = Some(estimation);
        cache.last_updated_time = Some(Instant::now());
    }

    pub fn last_updated_gas_estimation_cache_size(&self) -> usize {
        self.gas_estimation_cache
            .read()
            .unwrap()
            .min_inclusion_prices
            .len()
    }

    fn get_gas_prices_and_used(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
        count_majority_usecase: bool,
    ) -> anyhow::Result<(Vec<(u64, u64)>, Vec<BlockEndInfo>, Option<f32>)> {
        if start_version > ledger_version || limit == 0 {
            return Ok((vec![], vec![], None));
        }

        // This is just an estimation, so we can just skip over errors
        let limit = std::cmp::min(limit, ledger_version - start_version + 1);
        let txns = self.db.get_transaction_iterator(start_version, limit)?;
        let infos = self
            .db
            .get_transaction_info_iterator(start_version, limit)?;

        let mut gas_prices = Vec::new();
        let mut block_end_infos = Vec::new();
        let mut count_by_usecase = HashMap::new();
        for (txn, info) in txns.zip(infos) {
            match txn.as_ref() {
                Ok(Transaction::UserTransaction(txn)) => {
                    if let Ok(info) = info.as_ref() {
                        gas_prices.push((txn.gas_unit_price(), info.gas_used()));
                        if count_majority_usecase {
                            let use_case_key = txn.parse_use_case();
                            *count_by_usecase.entry(use_case_key).or_insert(0) += 1;
                        }
                    }
                },
                Ok(Transaction::BlockEpilogue(txn)) => {
                    if let Some(block_end_info) = txn.try_as_block_end_info() {
                        block_end_infos.push(block_end_info.clone());
                    }
                },
                _ => {},
            }
        }

        let majority_usecase_fraction = if count_majority_usecase {
            count_by_usecase
                .iter()
                .max_by_key(|(_, v)| *v)
                .and_then(|(max_usecase, max_value)| {
                    if let UseCaseKey::ContractAddress(_) = max_usecase {
                        Some(*max_value as f32 / count_by_usecase.values().sum::<u64>() as f32)
                    } else {
                        None
                    }
                })
        } else {
            None
        };
        Ok((gas_prices, block_end_infos, majority_usecase_fraction))
    }

    fn block_min_inclusion_price(
        &self,
        ledger_info: &LedgerInfo,
        first: Version,
        last: Version,
        gas_estimation_config: &FromOnChainGasEstimationMode,
        execution_config: &OnChainExecutionConfig,
    ) -> Option<u64> {
        let user_use_case_spread_factor = execution_config
            .transaction_shuffler_type()
            .user_use_case_spread_factor();

        match self.get_gas_prices_and_used(
            first,
            last - first,
            ledger_info.ledger_version.0,
            user_use_case_spread_factor.is_some(),
        ) {
            Ok((prices_and_used, block_end_infos, majority_usecase_fraction)) => {
                let is_full_block =
                    if majority_usecase_fraction.map_or(false, |fraction| fraction > 0.5) {
                        // If majority usecase is above half of transactions, UseCaseAware block reordering
                        // will allow other transactions to get in the block (AIP-68)
                        false
                    } else if prices_and_used.len() >= gas_estimation_config.full_block_txns {
                        true
                    } else if !block_end_infos.is_empty() {
                        assert_eq!(1, block_end_infos.len());
                        block_end_infos.first().unwrap().limit_reached()
                    } else if let Some(block_gas_limit) =
                        execution_config.block_gas_limit_type().block_gas_limit()
                    {
                        let gas_used = prices_and_used.iter().map(|(_, used)| *used).sum::<u64>();
                        gas_used >= block_gas_limit
                    } else {
                        false
                    };

                if is_full_block {
                    Some(
                        self.gas_buckets.next_bucket(
                            prices_and_used
                                .iter()
                                .map(|(price, _)| *price)
                                .min()
                                .unwrap(),
                        ),
                    )
                } else {
                    None
                }
            },
            Err(_) => None,
        }
    }

    pub(crate) fn estimate_gas_price_based_on_onchain(
        &self,
        min_gas_unit_price: u64,
        ledger_info: &LedgerInfo,
        config: &FromOnChainGasEstimationMode,
        execution_config: OnChainExecutionConfig,
        cache: &mut GasEstimationCache,
    ) -> GasEstimation {
        let epoch = ledger_info.epoch.0;
        // Clear the cache if the epoch has changed
        if let Some(cached_epoch) = cache.last_updated_epoch {
            if cached_epoch != epoch {
                cache.min_inclusion_prices.clear();
            }
        }

        let max_block_history = config.aggressive_block_history;
        // 1. Get the block metadata txns
        let mut lookup_version = ledger_info.ledger_version.0;
        let mut blocks = vec![];
        // Skip the first block, which may be partial
        if let Ok((first, _, block)) = self.db.get_block_info_by_version(lookup_version) {
            if block.epoch() == epoch {
                lookup_version = first.saturating_sub(1);
            }
        }
        let mut cached_blocks_hit = false;
        for _i in 0..max_block_history {
            if cache
                .min_inclusion_prices
                .contains_key(&(epoch, lookup_version))
            {
                cached_blocks_hit = true;
                break;
            }
            match self.db.get_block_info_by_version(lookup_version) {
                Ok((first, last, block)) => {
                    if block.epoch() != epoch {
                        break;
                    }
                    lookup_version = first.saturating_sub(1);
                    blocks.push((first, last));
                    if lookup_version == 0 {
                        break;
                    }
                },
                Err(_) => {
                    break;
                },
            }
        }
        if blocks.is_empty() && !cached_blocks_hit {
            let estimation = self.default_gas_estimation(min_gas_unit_price);
            return estimation;
        }
        let blocks_len = blocks.len();
        let remaining = max_block_history - blocks_len;

        // 2. Get gas prices per block
        let mut min_inclusion_prices = vec![];
        // TODO: if multiple calls to db is a perf issue, combine into a single call and then split
        for (first, last) in blocks {
            let min_inclusion_price = self
                .block_min_inclusion_price(ledger_info, first, last, config, &execution_config)
                .unwrap_or(min_gas_unit_price);
            min_inclusion_prices.push(min_inclusion_price);
            cache
                .min_inclusion_prices
                .insert((epoch, last), min_inclusion_price);
        }
        if cached_blocks_hit {
            for (_, v) in cache
                .min_inclusion_prices
                .range((Included(&(epoch, 0)), Included(&(epoch, lookup_version))))
                .rev()
                .take(remaining)
            {
                min_inclusion_prices.push(*v);
            }
        }

        // 3. Get values
        // (1) low
        let low_price = match min_inclusion_prices
            .iter()
            .take(config.low_block_history)
            .min()
        {
            Some(price) => *price,
            None => min_gas_unit_price,
        };

        // (2) market
        let mut latest_prices: Vec<_> = min_inclusion_prices
            .iter()
            .take(config.market_block_history)
            .cloned()
            .collect();
        latest_prices.sort();
        let market_price = match latest_prices.get(latest_prices.len() / 2) {
            None => {
                error!(
                    "prices empty, blocks.len={}, cached_blocks_hit={}, epoch={}, version={}",
                    blocks_len,
                    cached_blocks_hit,
                    ledger_info.epoch.0,
                    ledger_info.ledger_version.0
                );
                return self.default_gas_estimation(min_gas_unit_price);
            },
            Some(price) => low_price.max(*price),
        };

        // (3) aggressive
        min_inclusion_prices.sort();
        let p90_price = match min_inclusion_prices.get(min_inclusion_prices.len() * 9 / 10) {
            None => {
                error!(
                    "prices empty, blocks.len={}, cached_blocks_hit={}, epoch={}, version={}",
                    blocks_len,
                    cached_blocks_hit,
                    ledger_info.epoch.0,
                    ledger_info.ledger_version.0
                );
                return self.default_gas_estimation(min_gas_unit_price);
            },
            Some(price) => market_price.max(*price),
        };
        // round up to next bucket
        let aggressive_price = self.gas_buckets.next_bucket(p90_price);

        let estimation = GasEstimation {
            deprioritized_gas_estimate: Some(low_price),
            gas_estimate: market_price,
            prioritized_gas_estimate: Some(aggressive_price),
        };
        // 4. Update cache
        // GC old entries
        if cache.min_inclusion_prices.len() > max_block_history {
            for _i in max_block_history..cache.min_inclusion_prices.len() {
                cache.min_inclusion_prices.pop_first();
            }
        }

        estimation
    }

    fn get_mempool_latency_summary<E: InternalError>(
        &self,
        target_samples: usize,
    ) -> Result<MempoolLatencySummary, E> {
        let (req_sender, callback) = oneshot::channel();

        futures::executor::block_on(self.mp_sender.clone().send(
            MempoolClientRequest::GetLatencySummary(
                GetLatencySummaryRequest { target_samples },
                req_sender,
            ),
        ))
        .map_err(|e| E::internal_with_code_no_info(e, AptosErrorCode::InternalError))?;

        futures::executor::block_on(callback)
            .map_err(|e| E::internal_with_code_no_info(e, AptosErrorCode::InternalError))
    }

    fn compute_gas_estimate_from_bottom(
        gas_buckets: &GasBuckets,
        sorted: &[(u64, IndividualLatencyStat)],
        target_inclusion_latency_s: f64,
    ) -> Option<u64> {
        let mut last_over = None;
        for (gas, stat) in sorted.iter() {
            if stat.avg_s() < target_inclusion_latency_s {
                return last_over.map(|gas| gas_buckets.next_bucket(gas));
            }
            last_over = Some(*gas);
        }
        Some(gas_buckets.next_bucket(sorted.last().unwrap().0))
    }

    fn compute_gas_estimate_from_top(
        gas_buckets: &GasBuckets,
        sorted: &[(u64, IndividualLatencyStat)],
        target_inclusion_latency_s: f64,
    ) -> Option<u64> {
        for (gas, stat) in sorted.iter().rev() {
            if stat.avg_s() > target_inclusion_latency_s {
                return Some(gas_buckets.next_bucket(*gas));
            }
        }

        None
    }

    fn estimate_gas_price_based_on_local_history<E: InternalError>(
        &self,
        config: &FromLocalHistoryGasEstimationMode,
        min_gas_unit_price: u64,
    ) -> Result<GasEstimation, E> {
        let latency_summary = self.get_mempool_latency_summary(config.target_samples)?;

        if latency_summary.count < config.min_samples_needed {
            return Ok(self.default_gas_estimation(min_gas_unit_price));
        }

        Ok(Self::estimate_gas_price_based_on_local_history_impl(
            &self.gas_buckets,
            latency_summary,
            min_gas_unit_price,
            config,
        ))
    }

    fn estimate_gas_price_based_on_local_history_impl(
        gas_buckets: &GasBuckets,
        latency_summary: MempoolLatencySummary,
        min_gas_unit_price: u64,
        config: &FromLocalHistoryGasEstimationMode,
    ) -> GasEstimation {
        let sorted = latency_summary
            .inclusion_latency
            .into_iter()
            .sorted_by_key(|(gas, _stat)| *gas)
            .collect::<Vec<_>>();

        GasEstimation {
            deprioritized_gas_estimate: Some(min_gas_unit_price),
            gas_estimate: Self::compute_gas_estimate_from_bottom(
                gas_buckets,
                &sorted,
                config.target_inclusion_latency_s,
            )
            .unwrap_or(min_gas_unit_price),
            prioritized_gas_estimate: Some(
                Self::compute_gas_estimate_from_top(
                    gas_buckets,
                    &sorted,
                    config.prioritized_target_inclusion_latency_s,
                )
                .unwrap_or(min_gas_unit_price),
            ),
        }
    }

    pub(crate) fn estimate_gas_price<E: InternalError>(
        &self,
        min_gas_unit_price: u64,
        execution_config: OnChainExecutionConfig,
        ledger_info: &LedgerInfo,
    ) -> Result<GasEstimation, E> {
        if !self.config.enabled {
            return Ok(self.default_gas_estimation(min_gas_unit_price));
        }
        if let Some(static_override) = &self.config.static_override {
            return Ok(GasEstimation {
                deprioritized_gas_estimate: Some(static_override.low),
                gas_estimate: static_override.market,
                prioritized_gas_estimate: Some(static_override.aggressive),
            });
        }

        let epoch = ledger_info.epoch.0;

        // 0. (0) Return cached result if it exists
        let cache = self.gas_estimation_cache.read().unwrap();
        if let Some(cached_gas_estimation) = self.cached_gas_estimation(&cache, epoch) {
            return Ok(cached_gas_estimation);
        }
        drop(cache);

        // 0. (1) Write lock and prepare cache
        let mut cache = self.gas_estimation_cache.write().unwrap();
        // Retry cached result after acquiring write lock
        if let Some(cached_gas_estimation) = self.cached_gas_estimation(&cache, epoch) {
            return Ok(cached_gas_estimation);
        }

        let estimation = match &self.config.mode {
            GasEstimationMode::OnChainEstimation(from_onchain_config) => self
                .estimate_gas_price_based_on_onchain(
                    min_gas_unit_price,
                    ledger_info,
                    from_onchain_config,
                    execution_config,
                    &mut cache,
                ),
            GasEstimationMode::LocalHistory(local_history_config) => self
                .estimate_gas_price_based_on_local_history(
                    local_history_config,
                    min_gas_unit_price,
                )?,
        };

        Self::update_cached_gas_estimation(&mut cache, epoch, estimation);
        Ok(estimation)
    }
}

#[cfg(test)]
mod tests {
    use crate::gas_estimation::{GasBuckets, GasEstimator};
    use aptos_mempool::IndividualLatencyStat;
    use claims::assert_le;
    use proptest::prelude::*;

    prop_compose! {
        #[cfg(test)]
        fn arb_latencies(len: usize)
                        (bits in 1..(1<<len), latencies in prop::collection::vec(0.0..1.0, len))
                        -> Vec<(u64, IndividualLatencyStat)> {
            let mut result = Vec::new();
            for idx in 0..len {
                if bits & (1<<idx) != 0 {
                    result.push((((idx + 1) * 10) as u64, IndividualLatencyStat::new(1, latencies[result.len()])))
                }
            }
            result
        }
    }

    proptest! {
        #[test]
        fn test_bottom_estimate_below_top(sorted in arb_latencies(5)) {
            let target_inclusion_latency_s = 0.5;
            let gas_buckets = GasBuckets::new(vec![10, 20, 30, 40, 50, 60]);

            let bottom = GasEstimator::compute_gas_estimate_from_bottom(
                &gas_buckets,
                &sorted,
                target_inclusion_latency_s,
            );

            let top = GasEstimator::compute_gas_estimate_from_top(
                &gas_buckets,
                &sorted,
                target_inclusion_latency_s,
            );

            if bottom.is_some() {
                assert!(top.is_some());
                assert_le!(bottom.unwrap(), top.unwrap());
            }
        }
    }
}
