// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fixed-point path tracer over smallpt's Cornell box. Signed 128/256-bit
//! arithmetic, recursion, branchy control flow. Correctness is covered by the
//! package's own Move unit tests.

use move_binary_format::file_format::CompiledModule;

/// Canonical Move source, shared with the benchmark package. Two modules, in
/// dependency order, since the compiler takes one source file.
pub const SOURCE: &str = concat!(
    include_str!("../../../benchmarks/bench_pathtracer/sources/pathtracer_fixed.move"),
    "\n",
    include_str!("../../../benchmarks/bench_pathtracer/sources/pathtracer.move"),
);

/// Compile the canonical Move source, returning `(pathtracer, fixed)`. The
/// legacy VM needs the dependency published alongside the caller.
pub fn move_bytecode_pathtracer() -> (CompiledModule, CompiledModule) {
    let modules = super::compile_all(SOURCE);
    let find = |name: &str| {
        modules
            .iter()
            .find(|m| m.self_name().as_str() == name)
            .unwrap_or_else(|| panic!("module {name} missing from compiled output"))
            .clone()
    };
    (find("pathtracer"), find("pathtracer_fixed"))
}
