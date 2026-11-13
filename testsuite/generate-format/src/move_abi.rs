// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::transaction;
use move_core_types::language_storage;
use serde_reflection::{Registry, Result, Samples, Tracer, TracerConfig};

/// Default output file.
pub fn output_file() -> Option<&'static str> {
    Some("tests/staged/move_abi.yaml")
}

pub fn get_registry() -> Result<Registry> {
    let mut tracer = Tracer::new(
        TracerConfig::default()
            .record_samples_for_structs(true)
            .is_human_readable(bcs::is_human_readable()),
    );
    let mut samples = Samples::new();
    // 1. Record samples for types with custom deserializers.
    crate::trace_type_tags(&mut tracer, &mut samples)?;

    // 2. Trace the main entry point(s) + every enum separately.
    tracer.trace_type::<transaction::EntryABI>(&samples)?;
    tracer.trace_type::<language_storage::FunctionParamOrReturnTag>(&samples)?;
    tracer.trace_type::<language_storage::TypeTag>(&samples)?;

    tracer.registry()
}
