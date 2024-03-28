

// Copyright © Aptos Foundation

#[cfg(test)]
mod test {
    use std::{alloc::{GlobalAlloc, Layout, System}, collections::BTreeMap, sync::atomic::{AtomicUsize, Ordering}};

    use aptos_types::block_executor::partitioner::TxnIndex;
    use crossbeam::utils::CachePadded;
    use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;

    use crate::versioned_delayed_fields::{VersionEntry, VersionedDelayedFields, VersionedValue};

    /// TrackingAllocator records the sum of how many bytes are allocated
    /// and deallocated for later analysis.
    struct TrackingAllocator;

    static ALLOC: AtomicUsize = AtomicUsize::new(0);
    static DEALLOC: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let p = System.alloc(layout);
            record_alloc(layout);
            p
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            record_dealloc(layout);
            System.dealloc(ptr, layout);
        }
    }

    pub fn record_alloc(layout: Layout) {
        ALLOC.fetch_add(layout.size(), Ordering::SeqCst);
    }

    pub fn record_dealloc(layout: Layout) {
        DEALLOC.fetch_add(layout.size(), Ordering::SeqCst);
    }


    #[global_allocator]
    static ALLOCATOR: TrackingAllocator = TrackingAllocator;

    pub struct Stats {
        pub alloc: usize,
        pub dealloc: usize,
        pub diff: isize,
    }

    pub fn reset() {
        ALLOC.store(0, Ordering::SeqCst);
        DEALLOC.store(0, Ordering::SeqCst);
    }


    pub fn stats() -> Stats {
        let alloc: usize = ALLOC.load(Ordering::SeqCst);
        let dealloc: usize = DEALLOC.load(Ordering::SeqCst);
        let diff = (alloc as isize) - (dealloc as isize);

        Stats {
            alloc,
            dealloc,
            diff,
        }
    }

    pub fn run_and_track<T>(name: &str, size: usize, f: impl FnOnce() -> T) {
        reset();

        let t = f();

        let Stats {
            alloc,
            dealloc,
            diff,
        } = stats();
        println!("{name},{size},{alloc},{dealloc},{diff}");

        drop(t);
    }

    #[test]
    fn test_alloc() {
        println!("{:?}", std::mem::size_of::<VersionEntry<DelayedFieldID>>());
        println!("{:?}", std::mem::size_of::<CachePadded<VersionEntry<DelayedFieldID>>>());
        println!("{:?}", std::mem::size_of::<VersionedValue<DelayedFieldID>>());
        println!("{:?}", std::mem::size_of::<BTreeMap<TxnIndex, CachePadded<VersionEntry<DelayedFieldID>>>>());
        println!("{:?}", std::mem::size_of::<Option<BTreeMap<TxnIndex, CachePadded<VersionEntry<DelayedFieldID>>>>>());
        println!("{:?}", std::mem::size_of::<Option<Box<BTreeMap<TxnIndex, CachePadded<VersionEntry<DelayedFieldID>>>>>>());

        run_and_track("empty btreemap", 0, || {
            BTreeMap::<TxnIndex, CachePadded<VersionEntry<DelayedFieldID>>>::new()
        });

        run_and_track("empty option of BTreeMap", 0, || {
            Option::<BTreeMap::<TxnIndex, CachePadded<VersionEntry<DelayedFieldID>>>>::None
        });
        run_and_track("empty option of box of BTreeMap", 0, || {
            Option::<Box<BTreeMap::<TxnIndex, CachePadded<VersionEntry<DelayedFieldID>>>>>::None
        });


        for size in [1, 10, 100, 1000, 10000, 100000, 1000000] {
            run_and_track("VersionedDelayedFields base", size, || {
                let data = VersionedDelayedFields::<DelayedFieldID>::new();

                for i in 0..size {
                    data.set_base_value(DelayedFieldID::new_for_test_for_u64(i as u32), aptos_aggregator::types::DelayedFieldValue::Snapshot(i as u128));
                }

                data
            });
        }


        for size in [1, 10, 100, 1000, 10000, 100000, 1000000] {
            run_and_track("VersionedDelayedFields data", size, || {
                let data = VersionedDelayedFields::<DelayedFieldID>::new();

                for i in 0..size {
                    data.initialize_delayed_field(DelayedFieldID::new_for_test_for_u64(i as u32), 1, aptos_aggregator::types::DelayedFieldValue::Snapshot(i as u128)).unwrap();
                }

                data
            });
        }
    }
}
