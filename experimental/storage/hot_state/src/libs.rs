

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

struct DynamicArray<T> {
    chunks: Vec<Vec<T>>,
    chunk_size: usize,
}

enum ValueInfo {
    InMem(StateValue),
    OnDisk,
}

struct HotStateItemInfo {
    updated_ksecs: u32,
    value: ValueInfo,
}


type ItemId = u32;

struct ListEntry {
    prev: ItemId,
    next: ItemId,
}

struct Config {
    max_size: usize,
    target_size: usize,
    target_evicts_per_second: usize,
}

struct HotStateStatus {
    capacity: usize,
    size: usize,
    num_in_mem_values: usize,
    total_hot_bytes: usize,

    hottest_item: ItemId,
    coldest_item: ItemId,
    coldest_in_mem_value: ItemId,
    first_empty_slot: ItemId,
}

struct HotStateBase {
    config: Config,
    idx_by_key: DashMap<HashValue, ItemId>,
    items: DynamicArray<Option<HotStateItemInfo>>,
    internal_hashes: DynamicArray<HashValue>,
    list_entries: DynamicArray<ListEntry>,
    status: HotStateStatus,
}

struct HotStateView<'a> {
    base: &' HotStateBase,
    idx_by_key: MapLayer<HashValue, Option<ItemId>>,
    items: MapLayer<usize, Option<HotStateItemInfo>>,
    internal_hashes: MapLayer<usize, HashValue>,
    list_entries: MapLayer<usize, ListEntry>,
    status: HotStateStatus,
}
