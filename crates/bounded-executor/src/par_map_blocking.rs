// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Parallel map via [`tokio::task::spawn_blocking`], as a drop-in for rayon's
//! `par_iter().map()` pattern.
//!
//! Items are divided into roughly equal chunks, each chunk is processed
//! sequentially on its own blocking thread, and results are collected in
//! order.  This gives fork-join parallelism without rayon's permanent
//! thread pools and work-stealing overhead.

use std::sync::Arc;

/// Apply `func` to every item in `items`, in parallel, preserving order.
///
/// The work is spread across at most `parallelism` blocking tokio tasks.
/// Each task processes a contiguous chunk of items sequentially, so per-item
/// overhead is near zero.
///
/// Must be called from within a tokio runtime context.
pub async fn par_map_blocking<T, F, R>(items: Vec<T>, parallelism: usize, func: F) -> Vec<R>
where
    T: Send + 'static,
    F: Fn(T) -> R + Send + Sync + 'static,
    R: Send + 'static,
{
    if items.is_empty() {
        return Vec::new();
    }

    let parallelism = parallelism.clamp(1, items.len());
    let chunk_size = items.len().div_ceil(parallelism);
    let func = Arc::new(func);

    // Partition into owned chunks.
    let mut chunks: Vec<Vec<T>> = Vec::with_capacity(parallelism);
    let mut iter = items.into_iter();
    for _ in 0..parallelism {
        let chunk: Vec<T> = iter.by_ref().take(chunk_size).collect();
        if chunk.is_empty() {
            break;
        }
        chunks.push(chunk);
    }

    let handles: Vec<_> = chunks
        .into_iter()
        .map(|chunk| {
            let func = Arc::clone(&func);
            tokio::task::spawn_blocking(move || {
                chunk.into_iter().map(|item| func(item)).collect::<Vec<R>>()
            })
        })
        .collect();

    futures::future::join_all(handles)
        .await
        .into_iter()
        .flat_map(|r| r.expect("par_map_blocking: spawn_blocking task panicked"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn preserves_order() {
        let items: Vec<u32> = (0..100).collect();
        let result = par_map_blocking(items, 8, |x| x * 2).await;
        let expected: Vec<u32> = (0..100).map(|x| x * 2).collect();
        assert_eq!(result, expected);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn respects_parallelism() {
        static MAX_CONCURRENT: AtomicUsize = AtomicUsize::new(0);
        static CURRENT: AtomicUsize = AtomicUsize::new(0);

        MAX_CONCURRENT.store(0, Ordering::SeqCst);
        CURRENT.store(0, Ordering::SeqCst);

        let items: Vec<u32> = (0..100).collect();
        par_map_blocking(items, 4, |x| {
            let prev = CURRENT.fetch_add(1, Ordering::SeqCst);
            MAX_CONCURRENT.fetch_max(prev + 1, Ordering::SeqCst);
            // Simulate some work.
            std::thread::sleep(std::time::Duration::from_millis(1));
            CURRENT.fetch_sub(1, Ordering::SeqCst);
            x
        })
        .await;

        assert!(MAX_CONCURRENT.load(Ordering::SeqCst) <= 4);
    }

    #[tokio::test]
    async fn empty_input() {
        let result = par_map_blocking(Vec::<u32>::new(), 8, |x| x).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn single_item() {
        let result = par_map_blocking(vec![42], 8, |x| x + 1).await;
        assert_eq!(result, vec![43]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fewer_items_than_parallelism() {
        let result = par_map_blocking(vec![1, 2, 3], 100, |x| x * 10).await;
        assert_eq!(result, vec![10, 20, 30]);
    }
}
