// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Production native registry for the Block-STM integration.

use mono_move_core::{native::NativeName, Interner};
use mono_move_global_context::ExecutionGuard;
use mono_move_natives::{make_all_production_natives, Dispatch};
use mono_move_runtime::{ProductionContextFamily, ProductionNativeRegistry};

/// Builds the registry of production natives, keyed by interned name. Unlike
/// the testsuite's `build_natives`, no synthetic test natives are included.
pub fn build_production_natives(guard: &ExecutionGuard<'_>) -> ProductionNativeRegistry {
    let mut natives = ProductionNativeRegistry::new();
    natives
        .register_all(
            make_all_production_natives::<ProductionContextFamily>()
                .into_iter()
                .map(|(addr, module, function, dispatch, func)| {
                    let module = guard.module_id_of(&addr, &module);
                    let function = guard.identifier_of(&function);
                    let name = match dispatch {
                        Dispatch::Polymorphic => NativeName::Polymorphic { module, function },
                        Dispatch::Monomorphic(ty_args) => NativeName::Monomorphic {
                            module,
                            function,
                            ty_args: guard.type_list_of(ty_args),
                        },
                    };
                    (name, func)
                }),
        )
        .expect("natives have unique qualified names");
    natives
}
