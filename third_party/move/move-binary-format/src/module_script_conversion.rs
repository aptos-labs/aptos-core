// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_bounds::BoundsChecker,
    file_format::{
        AddressIdentifierIndex, CompiledScript, FunctionDefinition, FunctionHandle,
        FunctionHandleIndex, IdentifierIndex, ModuleHandle, ModuleHandleIndex, Signature,
        SignatureIndex, Visibility,
    },
    CompiledModule,
};
use anyhow::bail;
use move_core_types::{identifier::Identifier, language_storage::pseudo_script_module_id};

/// Converts a compiled module into a script. The module must define exactly one function
/// and no types. The `main_handle` is the handle info of the one function in the module
/// which must not be contained in function_handles table.
pub fn module_into_script(
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

/// Converts a compiled script into a module. The function in the script gets
/// `name` assigned in the produced module.
pub fn script_into_module(compiled_script: CompiledScript, name: &str) -> CompiledModule {
    let mut script = compiled_script;

    // Add the "<SELF>" identifier if it isn't present.
    let self_ident_idx = match script
        .identifiers
        .iter()
        .position(|ident| ident.as_ident_str().as_str() == name)
    {
        Some(idx) => IdentifierIndex::new(idx as u16),
        None => {
            let idx = IdentifierIndex::new(script.identifiers.len() as u16);
            script
                .identifiers
                .push(Identifier::new(name.to_string()).unwrap());
            idx
        },
    };

    // Add a dummy address if none exists.
    let dummy_addr = pseudo_script_module_id().address;
    let dummy_addr_idx = match script
        .address_identifiers
        .iter()
        .position(|addr| addr == &dummy_addr)
    {
        Some(idx) => AddressIdentifierIndex::new(idx as u16),
        None => {
            let idx = AddressIdentifierIndex::new(script.address_identifiers.len() as u16);
            script.address_identifiers.push(dummy_addr);
            idx
        },
    };

    // Add a self module handle.
    let self_module_handle_idx = match script
        .module_handles
        .iter()
        .position(|handle| handle.address == dummy_addr_idx && handle.name == self_ident_idx)
    {
        Some(idx) => ModuleHandleIndex::new(idx as u16),
        None => {
            let idx = ModuleHandleIndex::new(script.module_handles.len() as u16);
            script.module_handles.push(ModuleHandle {
                address: dummy_addr_idx,
                name: self_ident_idx,
            });
            idx
        },
    };

    // Find the index to the empty signature [].
    // Create one if it doesn't exist.
    let return_sig_idx = match script.signatures.iter().position(|sig| sig.0.is_empty()) {
        Some(idx) => SignatureIndex::new(idx as u16),
        None => {
            let idx = SignatureIndex::new(script.signatures.len() as u16);
            script.signatures.push(Signature(vec![]));
            idx
        },
    };

    // Create a function handle for the main function.
    let main_handle_idx = FunctionHandleIndex::new(script.function_handles.len() as u16);
    script.function_handles.push(FunctionHandle {
        module: self_module_handle_idx,
        name: self_ident_idx,
        parameters: script.parameters,
        return_: return_sig_idx,
        type_parameters: script.type_parameters,
        access_specifiers: None, // TODO: access specifiers for script functions
        attributes: vec![],
    });

    // Create a function definition for the main function.
    let main_def = FunctionDefinition {
        function: main_handle_idx,
        visibility: Visibility::Public,
        is_entry: true,
        acquires_global_resources: vec![],
        code: Some(script.code),
    };

    let module = CompiledModule {
        version: script.version,
        module_handles: script.module_handles,
        self_module_handle_idx,
        struct_handles: script.struct_handles,
        function_handles: script.function_handles,
        field_handles: vec![],
        friend_decls: vec![],

        struct_def_instantiations: vec![],
        function_instantiations: script.function_instantiations,
        field_instantiations: vec![],

        signatures: script.signatures,

        identifiers: script.identifiers,
        address_identifiers: script.address_identifiers,
        constant_pool: script.constant_pool,
        metadata: script.metadata,

        struct_defs: vec![],
        function_defs: vec![main_def],

        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };
    BoundsChecker::verify_module(&module).expect("invalid bounds in module");
    module
}
