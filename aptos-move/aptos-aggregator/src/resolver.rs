// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::table::TableHandle;

/// A resolver trait to get aggregator values from storage. In the future,
/// we want to make aggregators native so this trait will be removed. It is
/// still better to use a stand-alone trait instead of `TableResolver` because
/// this way we can return non-serialized types.
pub trait AggregatorResolver {
    fn resolve_aggregator_value(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> anyhow::Result<Option<Vec<u8>>>;
}
