// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! FIXME(aldenhu): doc

use aptos_crypto::HashValue;
use bytes::Bytes;

const MAX_ITEMS: usize = 64 * (10 << 20); // 64M items
const ITEM_SIZE: usize = 48;
const MAX_BYTES: usize = 32 * (10 << 30); // 32 GB
const MAX_BYTES_PER_ITEM: usize = 1024;

struct StateKeyHash(HashValue);

type ItemId = u32;

struct Item {
    id_num: ItemId,
    prev: ItemId,
    next: ItemId,
    value: Value,
}

enum Value {
    InMemory { bytes: Bytes },
    OnDisk { size: u16 },
}

struct BaseTree {
    items: HashSet<StateKeyHash, Item>,
    internal_nodes: [HashValue; MAX_ITEMS],
    latest_item: ItemId,
    oldest_item_with_in_mem_value: ItemId,
    oldest_item: ItemId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds() {
        assert!(MAX_ITEMS < ItemId::MAX as usize);
    }

    #[test]
    fn test_item_size() {
        assert_eq!(std::mem::size_of::<Item>(), ITEM_SIZE);
    }
}
