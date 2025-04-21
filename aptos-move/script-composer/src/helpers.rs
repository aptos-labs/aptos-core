// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    builders::CompiledScriptBuilder,
    errors::{PartialVMError, PartialVMResult},
    file_format::*,
};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    transaction_argument::TransactionArgument,
    vm_status::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub(crate) struct Script {
    #[serde(with = "serde_bytes")]
    pub code: Vec<u8>,
    pub ty_args: Vec<TypeTag>,
    pub args: Vec<TransactionArgument>,
}

pub(crate) fn import_type_tag(
    script_builder: &mut CompiledScriptBuilder,
    type_tag: &TypeTag,
    module_resolver: &BTreeMap<ModuleId, CompiledModule>,
) -> PartialVMResult<SignatureToken> {
    let to_list = |script_builder: &mut CompiledScriptBuilder, ts: &[TypeTag]| {
        ts.iter()
            .map(|t| import_type_tag(script_builder, t, module_resolver))
            .collect::<PartialVMResult<Vec<_>>>()
    };
    Ok(match type_tag {
        TypeTag::Address => SignatureToken::Address,
        TypeTag::U8 => SignatureToken::U8,
        TypeTag::U16 => SignatureToken::U16,
        TypeTag::U32 => SignatureToken::U32,
        TypeTag::U64 => SignatureToken::U64,
        TypeTag::U128 => SignatureToken::U128,
        TypeTag::U256 => SignatureToken::U256,
        TypeTag::Bool => SignatureToken::Bool,
        TypeTag::Signer => SignatureToken::Signer,
        TypeTag::Vector(t) => SignatureToken::Vector(Box::new(import_type_tag(
            script_builder,
            t,
            module_resolver,
        )?)),
        TypeTag::Struct(s) => {
            let (module, handle_idx) =
                find_struct(module_resolver, &s.module_id(), s.name.as_ident_str())?;
            let struct_idx = script_builder.import_struct(module, handle_idx)?;
            if s.type_args.is_empty() {
                SignatureToken::Struct(struct_idx)
            } else {
                SignatureToken::StructInstantiation(
                    struct_idx,
                    to_list(script_builder, &s.type_args)?,
                )
            }
        },
        TypeTag::Function(f) => SignatureToken::Function(
            to_list(script_builder, &f.args)?,
            to_list(script_builder, &f.results)?,
            f.abilities,
        ),
    })
}

/// Given a module, return the handle idx of the named struct
pub(crate) fn find_struct<'a>(
    map: &'a BTreeMap<ModuleId, CompiledModule>,
    module_id: &ModuleId,
    struct_name: &IdentStr,
) -> PartialVMResult<(&'a CompiledModule, StructHandleIndex)> {
    if let Some(module) = map.get(module_id) {
        for (idx, handle) in module.struct_handles().iter().enumerate() {
            if module.identifier_at(handle.name) == struct_name {
                return Ok((module, StructHandleIndex::new(idx as TableIndex)));
            }
        }
        return Err(
            PartialVMError::new(StatusCode::LOOKUP_FAILED).with_message(format!(
                "Struct {}::{} doesn't yet exist in the cache",
                module_id, struct_name
            )),
        );
    }
    Err(
        PartialVMError::new(StatusCode::LOOKUP_FAILED).with_message(format!(
            "Module {} doesn't yet exist in the cache",
            module_id
        )),
    )
}

/// Given a compiled script, add a signature into its pool if it's not present already.
pub(crate) fn import_signature(
    script: &mut CompiledScript,
    sig: Signature,
) -> PartialVMResult<SignatureIndex> {
    Ok(SignatureIndex(
        match script.signatures().iter().position(|item| item == &sig) {
            Some(idx) => idx,
            None => {
                let idx = script.signatures().len();
                if idx >= TableIndex::MAX as usize {
                    return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                }
                script.signatures.push(sig);
                idx
            },
        } as u16,
    ))
}
