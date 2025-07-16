// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    file_format::{CompiledScript, FunctionDefinition, FunctionHandle},
    CompiledModule,
};
use anyhow::bail;

/// Converts a compiled module into a script. The module must define exactly one function
/// and no types. The `main_handle` is the handle info of the one function in the module
/// which must not be contained in function_handles table.
pub fn convert_module_to_script(
    module: CompiledModule,
    main_handle: FunctionHandle,
) -> anyhow::Result<CompiledScript> {
    let CompiledModule {
        version,
        self_module_handle_idx: _,
        module_handles,
        struct_handles,
        function_handles,
        field_handles: _,
        friend_decls: _,
        struct_def_instantiations: _,
        field_instantiations: _,
        struct_defs,
        mut function_defs,
        function_instantiations,
        signatures,
        identifiers,
        address_identifiers,
        constant_pool,
        metadata,
        struct_variant_handles: _,
        struct_variant_instantiations: _,
        variant_field_handles: _,
        variant_field_instantiations: _,
    } = module;
    if function_defs.len() != 1 {
        bail!("scripts can only contain one function")
    }
    if !struct_defs.is_empty() {
        bail!("scripts cannot have struct or enum declarations")
    }
    let FunctionDefinition {
        function: _,
        visibility: _,
        is_entry: _,
        acquires_global_resources: _,
        code,
    } = function_defs.pop().unwrap();
    let Some(code) = code else {
        bail!("script functions must have a body")
    };
    let FunctionHandle {
        module: _,
        name: _,
        parameters,
        return_,
        type_parameters,
        access_specifiers,
        attributes: _,
    } = main_handle;
    if signatures
        .get(return_.0 as usize)
        .map_or(true, |s| !s.is_empty())
    {
        bail!("main function must not return values")
    }
    Ok(CompiledScript {
        version,
        module_handles,
        struct_handles,
        function_handles,
        function_instantiations,
        signatures,
        identifiers,
        address_identifiers,
        constant_pool,
        metadata,
        code,
        type_parameters,
        parameters,
        access_specifiers,
    })
}
