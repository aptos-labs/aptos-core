// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Dedicated jemalloc-backed test binary for real-allocation annotation metering.
//! Only meaningful with `--features metered-allocations`; otherwise the reader is a
//! no-op and the module compiles to nothing (an empty, passing binary).

#[cfg(all(unix, feature = "metered-allocations"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(all(unix, feature = "metered-allocations"))]
mod metered {
    use move_binary_format::CompiledModule;
    use move_core_types::{
        language_storage::{ModuleId, TypeTag},
        value::MoveValue,
    };
    use move_resource_viewer::{CompiledModuleView, MoveValueAnnotator};
    use std::sync::Arc;

    // No modules needed: primitive vectors resolve without module lookup.
    struct EmptyModuleView;
    impl CompiledModuleView for EmptyModuleView {
        type Item = Arc<CompiledModule>;

        fn view_compiled_module(&self, _id: &ModuleId) -> anyhow::Result<Option<Self::Item>> {
            Ok(None)
        }
    }

    fn annotate(blob: &[u8], ty: &TypeTag, budget: usize) -> anyhow::Result<()> {
        // The annotator's configured budget seeds the fresh Meter inside view_value,
        // and the real jemalloc reader makes it measure live bytes.
        let annotator = MoveValueAnnotator::new_with_meter_config(
            EmptyModuleView,
            budget,
            aptos_jemalloc::current_live_bytes,
        );
        annotator.view_value(ty, blob)?;
        Ok(())
    }

    fn vec_u64_blob(n: usize) -> Vec<u8> {
        bcs::to_bytes(&MoveValue::Vector(vec![MoveValue::U64(0); n])).unwrap()
    }

    #[test]
    fn node_dense_but_cheap_payload_is_allowed() {
        // ~80k u64 elements: real footprint < 10 MB. The OLD model charged
        // ~1024/node on top of the floor (~118 MB) and would have rejected this.
        let blob = vec_u64_blob(80_000);
        let ty = TypeTag::Vector(Box::new(TypeTag::U64));
        assert!(
            annotate(&blob, &ty, 100_000_000).is_ok(),
            "node-dense but cheap payload must annotate under a 100MB live-byte budget"
        );
    }

    #[test]
    fn amplifying_input_aborts() {
        // vector<vector<vector<u64>>> = 64^3 = 262_144 leaves; tiny budget.
        let inner = MoveValue::Vector(vec![MoveValue::U64(0); 64]);
        let mid = MoveValue::Vector(vec![inner; 64]);
        let outer = MoveValue::Vector(vec![mid; 64]);
        let blob = bcs::to_bytes(&outer).unwrap();
        let ty = TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::Vector(
            Box::new(TypeTag::U64),
        )))));
        assert!(
            annotate(&blob, &ty, 1_000_000).is_err(),
            "amplifying input must be rejected, not OOM"
        );
    }
}
