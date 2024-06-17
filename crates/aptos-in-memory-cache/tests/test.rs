mod common;

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_in_memory_cache::caches::sync_mutex::SyncMutexCache;

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_insert_out_of_order() {
        let cache = SyncMutexCache::with_capacity(10);
        common::test_insert_out_of_order_impl(cache).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_array_wrap_around() {
        let cache = SyncMutexCache::with_capacity(10);
        common::test_array_wrap_around_impl(cache);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_on_size_limit() {
        let cache = SyncMutexCache::with_capacity(10);
        common::test_eviction_on_size_limit_impl(cache).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_out_of_order_inserts() {
        let cache = SyncMutexCache::with_capacity(20);
        common::test_eviction_out_of_order_inserts_impl(cache).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_with_array_wrap_around() {
        let cache = SyncMutexCache::with_capacity(10);
        common::test_eviction_with_array_wrap_around_impl(cache).await;
    }
}
