// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::types::ChunkyTranscriptWithHash;
use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress;
use std::collections::{HashMap, VecDeque};

type CacheKey = (AccountAddress, HashValue);

/// Extra, verified transcript versions fetched during certification. The manager owns
/// this cache for one epoch; its original aggregation transcripts are stored separately.
/// Bound both canonical transcript bytes and entries, including variants per dealer,
/// so an equivocating dealer cannot fill the cache with arbitrarily many versions.
pub(crate) struct TranscriptCache {
    entries: HashMap<CacheKey, (ChunkyTranscriptWithHash, usize)>,
    insertion_order: VecDeque<CacheKey>,
    bytes: usize,
    max_bytes: usize,
    max_entries: usize,
}

impl Default for TranscriptCache {
    fn default() -> Self {
        Self::new(128 * 1024 * 1024, 256)
    }
}

impl TranscriptCache {
    const MAX_VARIANTS_PER_DEALER: usize = 2;

    fn new(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            insertion_order: VecDeque::new(),
            bytes: 0,
            max_bytes,
            max_entries,
        }
    }

    pub(crate) fn get(
        &self,
        dealer: AccountAddress,
        hash: HashValue,
    ) -> Option<&ChunkyTranscriptWithHash> {
        self.entries.get(&(dealer, hash)).map(|(value, _)| value)
    }

    /// The caller must verify the transcript and its expected hash before insertion.
    pub(crate) fn insert(
        &mut self,
        dealer: AccountAddress,
        transcript: ChunkyTranscriptWithHash,
        serialized_bytes: usize,
    ) {
        let key = (dealer, transcript.hash());
        if serialized_bytes > self.max_bytes
            || self.max_entries == 0
            || self.entries.contains_key(&key)
        {
            return;
        }
        if self
            .entries
            .keys()
            .filter(|(addr, _)| *addr == dealer)
            .count()
            >= Self::MAX_VARIANTS_PER_DEALER
        {
            let index = self
                .insertion_order
                .iter()
                .position(|(addr, _)| *addr == dealer)
                .expect("dealer has cached entries");
            let oldest = self.insertion_order.remove(index).unwrap();
            self.remove(oldest);
        }
        while self.entries.len() >= self.max_entries
            || serialized_bytes > self.max_bytes - self.bytes
        {
            let oldest = self.insertion_order.pop_front().expect("cache is nonempty");
            self.remove(oldest);
        }
        self.bytes += serialized_bytes;
        self.entries.insert(key, (transcript, serialized_bytes));
        self.insertion_order.push_back(key);
    }

    fn remove(&mut self, key: CacheKey) {
        let (_, bytes) = self.entries.remove(&key).expect("cache entry exists");
        self.bytes -= bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunky::test_utils::ChunkyTestSetup;

    #[test]
    fn bounds_variants_entries_and_bytes() {
        let setup = ChunkyTestSetup::new_uniform(4);
        let (_, transcript) = setup.deal_transcript(0);
        let value = |n| ChunkyTranscriptWithHash::new(transcript.clone(), HashValue::from_u64(n));
        let a = setup.addrs[0];
        let b = setup.addrs[1];
        let c = setup.addrs[2];
        let mut cache = TranscriptCache::new(30, 3);
        cache.insert(a, value(1), 10);
        cache.insert(b, value(1), 10);
        cache.insert(a, value(2), 10);
        cache.insert(a, value(3), 10);
        assert!(cache.get(a, HashValue::from_u64(1)).is_none());
        assert!(cache.get(b, HashValue::from_u64(1)).is_some());
        assert_eq!(cache.bytes, 30);
        cache.insert(c, value(1), 20);
        assert_eq!(cache.bytes, 30);
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.get(a, HashValue::from_u64(3)).is_some());
        cache.insert(c, value(2), 31);
        assert_eq!(cache.bytes, 30);
        assert!(cache.get(c, HashValue::from_u64(2)).is_none());

        let mut cache = TranscriptCache::new(100, 1);
        cache.insert(a, value(1), 10);
        cache.insert(b, value(1), 10);
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.insertion_order.len(), 1);
        assert_eq!(cache.bytes, 10);
        assert!(cache.get(a, HashValue::from_u64(1)).is_none());
        cache.insert(b, value(1), 10);
        assert_eq!(cache.bytes, 10);
        assert_eq!(cache.insertion_order.len(), 1);
    }
}
