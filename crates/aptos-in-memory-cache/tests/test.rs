use allocative::{size_of_unique, Allocative};
use aptos_in_memory_cache::{caches::sync_mutex::SyncMutexCache, Cache, SizedCache};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Allocative, Debug, PartialEq)]
struct NotATransaction {
    transaction_version: i64,
}

impl NotATransaction {
    fn new(transaction_version: i64) -> Self {
        Self {
            transaction_version,
        }
    }
}

/// As discussed with Max previously, this cache uses a primitive Rust array as a circular buffer to store cache entries.
/// The metadata contains three pointers: tail, head, and watermark.
/// The actual contents of the cache are between the head and tail pointers.
/// We guarantee that all values between the head and tail pointers exist and are valid cache entries.
/// The watermark represents the highest key that has been inserted into the cache.
/// It exists to enable out of order insertions because we guarantee that new values between the tail and the watermark are not evicted.
/// As new values are inserted, the tail pointer is updated accordingly to ensure that the contents of the cache (between the tail and head pointers) are contiguous.
#[derive(Debug, Clone)]
struct TestCacheMetadataPointers {
    tail: Option<usize>,
    head: Option<usize>,
    watermark: Option<usize>,
}

#[derive(Debug, Clone)]
struct TestCacheMetadata {
    eviction_trigger_size_in_bytes: usize,
    target_size_in_bytes: usize,
    capacity: usize,
    pointers: Arc<Mutex<TestCacheMetadataPointers>>,
}

struct TestCache {
    metadata: Arc<TestCacheMetadata>,
    cache: Arc<SyncMutexCache<NotATransaction>>,
    insert_notify: Arc<Notify>,
    _cancellation_token_drop_guard: tokio_util::sync::DropGuard,
}

impl TestCache {
    fn with_capacity(
        capacity: usize,
        eviction_trigger_size_in_bytes: usize,
        target_size_in_bytes: usize,
    ) -> Self {
        let cancellation_token: CancellationToken = CancellationToken::new();

        let cache = Self {
            metadata: Arc::new(TestCacheMetadata {
                eviction_trigger_size_in_bytes,
                target_size_in_bytes,
                capacity,
                pointers: Arc::new(Mutex::new(TestCacheMetadataPointers {
                    tail: None,
                    head: None,
                    watermark: None,
                })),
            }),
            cache: Arc::new(SyncMutexCache::with_capacity(capacity)),
            insert_notify: Arc::new(Notify::new()),
            _cancellation_token_drop_guard: cancellation_token.clone().drop_guard(),
        };

        cache.spawn_eviction_task(cancellation_token.clone());

        cache
    }

    /// Perform cache eviction on a separate task.
    fn spawn_eviction_task(&self, cancellation_token: CancellationToken) {
        let insert_notify = self.insert_notify.clone();
        let metadata_arc = self.metadata.clone();
        let metadata_pointers_arc = metadata_arc.pointers.clone();
        let cache_arc = self.cache.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = insert_notify.notified() => {
                        if cache_arc.total_size() < { metadata_arc.eviction_trigger_size_in_bytes } {
                            continue;
                            }

                        let mut metadata_pointers = metadata_pointers_arc.lock();

                        // Evict entries until the cache size is below the target size
                        while cache_arc.total_size() > metadata_arc.target_size_in_bytes {
                            cache_arc.evict(&metadata_pointers.head.unwrap());
                            // Increment head pointer
                            metadata_pointers.head = Some((metadata_pointers.head.unwrap() + 1) % metadata_arc.capacity);
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

impl Cache<usize, NotATransaction> for TestCache {
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
        let mut metadata_pointers = self.metadata.pointers.lock();

        // Return early if key to insert is less than the head
        if let Some(head) = metadata_pointers.head {
            if let Some(entry) = self.cache.get(&head) {
                if entry.key > key {
                    return;
                }
            }
        }

        // Insert
        let key_index = self.cache.insert_with_size(key, value, size_in_bytes);

        // Fill in metadata if cache is empty
        if metadata_pointers.tail.is_none() {
            metadata_pointers.tail = Some(key_index);
        }

        if metadata_pointers.head.is_none() {
            metadata_pointers.head = Some(key_index);
        }

        if metadata_pointers.watermark.is_none() {
            metadata_pointers.watermark = Some(key_index);
        }

        // Set watermark to the highest key
        // metadata.watermark should always exist here because we just populated it if it was None
        if let Some(entry) = self.cache.get(&metadata_pointers.watermark.unwrap()) {
            if entry.key < key {
                metadata_pointers.watermark = Some(key_index);
            }
        }

        // Update tail if necessary
        let mut tail = self.cache.get(&metadata_pointers.tail.unwrap()).unwrap();
        while metadata_pointers.tail != metadata_pointers.watermark {
            let next_tail_key = (metadata_pointers.tail.unwrap() + 1) % self.metadata.capacity;
            let next_tail = self.cache.get(&next_tail_key);
            if Some(tail.key + 1) == next_tail.as_ref().and_then(|entry| Some(entry.key)) {
                metadata_pointers.tail = Some(next_tail_key);
                tail = next_tail.unwrap();
                continue;
            }
            break;
        }

        self.insert_notify.notify_waiters();
    }

    fn total_size(&self) -> u64 {
        self.cache.total_size() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_insert_out_of_order() {
        let cache = TestCache::with_capacity(10, 150, 100);
        let key = 100;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(0));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(0));

        let key = 101;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(1));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(1));

        let key = 105;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(1));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(5));

        let key = 103;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(1));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(5));

        let key = 102;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(3));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(5));

        let key = 104;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(5));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(5));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_array_wrap_around() {
        let cache = TestCache::with_capacity(10, 150, 100);
        let key = 7;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(7));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(7));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(7));

        let key = 8;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(7));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(8));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(8));

        let key = 12;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(7));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(8));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(2));

        let key = 10;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(7));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(8));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(2));

        let key = 9;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(7));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(0));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(2));

        let key = 11;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key), Some(value));
        assert_eq!(cache.metadata.pointers.lock().head, Some(7));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(2));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(2));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_on_size_limit() {
        let cache = TestCache::with_capacity(10, 56, 48);

        // Insert initial items
        for i in 0..6 {
            let value = NotATransaction::new(i as i64);
            cache.insert(i, value);
        }

        assert_eq!(
            cache.total_size(),
            6 * size_of_unique(&NotATransaction::new(0)) as u64
        );
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(5));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(5));

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(1));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(6));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(6));

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(4));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(9));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(9));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_out_of_order_inserts() {
        let cache = TestCache::with_capacity(20, 88, 80);

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(9));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(9));

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(1));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(10));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(10));

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(2));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(11));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(11));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_with_array_wrap_around() {
        let cache = TestCache::with_capacity(10, 48, 40);

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(5));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(9));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(9));

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(7));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(1));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(1));

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
        assert_eq!(cache.metadata.pointers.lock().head, Some(0));
        assert_eq!(cache.metadata.pointers.lock().tail, Some(4));
        assert_eq!(cache.metadata.pointers.lock().watermark, Some(4));
    }
}
