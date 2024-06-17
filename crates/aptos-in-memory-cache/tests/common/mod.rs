use allocative::{size_of_unique, Allocative};
use aptos_in_memory_cache::{Cache, SizedCache};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Allocative, Debug, PartialEq)]
pub struct NotATransaction {
    transaction_version: i64,
}

impl NotATransaction {
    pub fn new(transaction_version: i64) -> Self {
        Self {
            transaction_version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestCacheMetadata {
    pub eviction_trigger_size_in_bytes: usize,
    pub target_size_in_bytes: usize,
    pub capacity: usize,
}

pub struct TestCache<C: SizedCache<NotATransaction> + 'static> {
    pub metadata: Arc<TestCacheMetadata>,
    pub cache: Arc<C>,
    pub insert_watch: Arc<Sender<usize>>,
    pub _cancellation_token_drop_guard: tokio_util::sync::DropGuard,
}

impl<C: SizedCache<NotATransaction> + 'static> TestCache<C> {
    pub fn with_capacity(
        c: C,
        eviction_trigger_size_in_bytes: usize,
        target_size_in_bytes: usize,
    ) -> Self {
        let cancellation_token: CancellationToken = CancellationToken::new();
        let (tx, rx) = tokio::sync::watch::channel(0_usize);

        let cache = Self {
            metadata: Arc::new(TestCacheMetadata {
                eviction_trigger_size_in_bytes,
                target_size_in_bytes,
                capacity: c.capacity(),
            }),
            cache: Arc::new(c),
            insert_watch: Arc::new(tx),
            _cancellation_token_drop_guard: cancellation_token.clone().drop_guard(),
        };

        cache.spawn_eviction_task(rx, cancellation_token.clone());

        cache
    }

    /// Perform cache eviction on a separate task.
    fn spawn_eviction_task(
        &self,
        mut insert_watch_read: Receiver<usize>,
        cancellation_token: CancellationToken,
    ) {
        let metadata_arc = self.metadata.clone();
        let cache_arc = self.cache.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = insert_watch_read.changed() => {
                        let watermark_value = *insert_watch_read.borrow();
                        let mut eviction_index = (watermark_value + 1) % metadata_arc.capacity;

                        if cache_arc.total_size() < metadata_arc.eviction_trigger_size_in_bytes  {
                            continue;
                        }

                        // Evict entries until the cache size is below the target size
                        while cache_arc.total_size() > metadata_arc.target_size_in_bytes {
                            if let Some(entry) = cache_arc.get(&eviction_index) {
                                if entry.key < watermark_value {
                                    if let Some(value) = cache_arc.evict(&entry.key) {
                                        if value.key > watermark_value {
                                            cache_arc.insert_with_size(value.key, value.value, value.size_in_bytes);
                                        }
                                    }
                                }
                            }
                            eviction_index = (eviction_index + 1) % metadata_arc.capacity;
                        }
                    },
                    _ = cancellation_token.cancelled() => {
                        return;
                    }
                }
            }
        });
    }
}

impl<C: SizedCache<NotATransaction> + 'static> Cache<usize, NotATransaction> for TestCache<C> {
    fn get(&self, key: &usize) -> Option<NotATransaction> {
        self.cache.get(key).and_then(|entry| {
            if entry.key == *key {
                return Some(entry.value.clone());
            }
            None
        })
    }

    fn insert(&self, key: usize, value: NotATransaction) {
        let size_in_bytes = size_of_unique(&value);
        self.cache.insert_with_size(key, value, size_in_bytes);
        self.insert_watch.send(key).unwrap();
    }

    fn total_size(&self) -> u64 {
        self.cache.total_size() as u64
    }
}

pub async fn test_insert_out_of_order_impl<C: SizedCache<NotATransaction> + 'static>(c: C) {
    let cache = TestCache::with_capacity(c, 150, 100);
    let key = 100;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 101;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 105;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 103;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 102;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 104;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));
}

pub fn test_array_wrap_around_impl<C: SizedCache<NotATransaction> + 'static>(c: C) {
    let cache = TestCache::with_capacity(c, 150, 100);
    let key = 7;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 8;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 12;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 10;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 9;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));

    let key = 11;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key), Some(value));
}

pub async fn test_eviction_on_size_limit_impl<C: SizedCache<NotATransaction> + 'static>(c: C) {
    let cache = TestCache::with_capacity(c, 56, 48);

    // Insert initial items
    for i in 0..6 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    assert_eq!(
        cache.total_size(),
        6 * size_of_unique(&NotATransaction::new(0)) as u64
    );

    tokio::time::sleep(Duration::from_micros(1)).await;

    // This insert should trigger eviction
    let key = 6;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value);

    // Wait for eviction to occur
    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        6 * size_of_unique(&NotATransaction::new(0)) as u64
    );

    // Further inserts to ensure eviction continues correctly
    for i in 7..10 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    // Wait for eviction to occur
    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        6 * size_of_unique(&NotATransaction::new(0)) as u64
    );
}

pub async fn test_eviction_out_of_order_inserts_impl<C: SizedCache<NotATransaction> + 'static>(
    c: C,
) {
    let cache = TestCache::with_capacity(c, 88, 80);

    // Insert items out of order
    let keys = [0, 5, 1, 3, 7, 2, 6, 4, 9, 8];
    for &key in &keys {
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        10 * size_of_unique(&NotATransaction::new(0)) as u64
    );

    tokio::time::sleep(Duration::from_micros(1)).await;

    // This insert should trigger eviction
    let key = 10;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value);

    // Wait for eviction to occur
    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        10 * size_of_unique(&NotATransaction::new(0)) as u64
    );

    tokio::time::sleep(Duration::from_micros(1)).await;

    // Further inserts to ensure eviction continues correctly
    let key = 11;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value);

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        10 * size_of_unique(&NotATransaction::new(0)) as u64
    );
}

pub async fn test_eviction_with_array_wrap_around_impl<C: SizedCache<NotATransaction> + 'static>(
    c: C,
) {
    let cache = TestCache::with_capacity(c, 48, 40);

    // Insert items to fill the cache
    for i in 5..10 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        5 * size_of_unique(&NotATransaction::new(0)) as u64
    );

    tokio::time::sleep(Duration::from_micros(1)).await;

    // Insert more items to cause wrap-around
    for i in 10..12 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        5 * size_of_unique(&NotATransaction::new(0)) as u64
    );

    // Insert even more items to fully wrap-around
    for i in 12..15 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(
        cache.total_size(),
        5 * size_of_unique(&NotATransaction::new(0)) as u64
    );
}
