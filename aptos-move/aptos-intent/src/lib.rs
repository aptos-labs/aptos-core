// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Result};
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    file_format::{Ability, AbilitySet, FunctionHandle, SignatureToken},
    CompiledModule,
};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, str::FromStr};
use wasm_bindgen::prelude::*;

mod codegen;
#[cfg(test)]
pub mod tests;

#[wasm_bindgen]
/// Arguments for each function.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum BatchArgumentType {
    Raw,
    Signer,
    PreviousResult,
}

#[wasm_bindgen]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviousResult {
    call_idx: u16,
    return_idx: u16,
    operation_type: ArgumentOperation,
    ability: AbilitySet,
    ty: TypeTag,
}

#[wasm_bindgen]
/// Arguments for each function. Wasm bindgen only support C-style enum so use option to work around.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BatchArgument {
    ty: BatchArgumentType,
    raw: Option<Vec<u8>>,
    signer: Option<u16>,
    previous_result: Option<PreviousResult>,
}

#[wasm_bindgen]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArgumentOperation {
    Move,
    Copy,
    Borrow,
    BorrowMut,
}

#[wasm_bindgen]
/// Call a Move entry function.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BatchedFunctionCall {
    module: ModuleId,
    function: Identifier,
    ty_args: Vec<TypeTag>,
    args: Vec<BatchArgument>,
}

#[wasm_bindgen]
pub struct BatchedFunctionCallBuilder {
    calls: Vec<BatchedFunctionCall>,
    num_signers: u16,
    modules: BTreeMap<ModuleId, CompiledModule>,
}

#[wasm_bindgen]
impl BatchedFunctionCallBuilder {
    pub fn single_signer() -> Self {
        Self {
            calls: vec![],
            num_signers: 1,
            modules: BTreeMap::new(),
        }
    }

    pub fn multi_signer(signer_count: u16) -> Self {
        Self {
            calls: vec![],
            num_signers: signer_count,
            modules: BTreeMap::new(),
        }
    }

    pub fn add_batched_call(
        &mut self,
        module: String,
        function: String,
        ty_args: Vec<String>,
        args: Vec<BatchArgument>,
    ) -> Result<Vec<BatchArgument>, String> {
        self.add_batched_call_impl(module, function, ty_args, args)
            .map_err(|err| format!("{:?}", err))
    }

    pub fn generate_batched_calls(self) -> Result<Vec<u8>, String> {
        crate::codegen::generate_script_from_batched_calls(
            &self.calls,
            self.num_signers,
            &self.modules,
        )
        .map_err(|err| format!("{:?}", err))
    }
}

impl BatchedFunctionCallBuilder {
    fn find_function(
        &self,
        module_id: &ModuleId,
        func_name: &IdentStr,
    ) -> anyhow::Result<(&CompiledModule, &FunctionHandle)> {
        if let Some(module) = self.modules.get(module_id) {
            for def in module.function_defs() {
                let handle = module.function_handle_at(def.function);
                if module.identifier_at(handle.name) == func_name {
                    return Ok((module, handle));
                }
            }
        }
        anyhow::bail!("{}::{} doesn't exist on chain", module_id, func_name)
    }

    fn return_values(
        &self,
        module_id: &ModuleId,
        func_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> anyhow::Result<Vec<BatchArgument>> {
        let (module, handle) = self.find_function(module_id, func_name)?;
        let mut returns = vec![];
        for (idx, sig) in module.signature_at(handle.return_).0.iter().enumerate() {
            returns.push(BatchArgument {
                ty: BatchArgumentType::PreviousResult,
                raw: None,
                signer: None,
                previous_result: Some(PreviousResult {
                    call_idx: (self.calls.len() - 1) as u16,
                    return_idx: idx as u16,
                    operation_type: ArgumentOperation::Move,
                    ty: self.type_tag_from_signature(module, sig, ty_args)?,
                    ability: BinaryIndexedView::Module(module)
                        .abilities(sig, &handle.type_parameters)
                        .unwrap(),
                }),
            });
        }
        return Ok(returns);
    }

    fn check_parameters(
        &self,
        module_id: &ModuleId,
        func_name: &IdentStr,
        params: &[BatchArgument],
        ty_args: &[TypeTag],
    ) -> anyhow::Result<()> {
        let (module, handle) = self.find_function(module_id, func_name)?;
        let param_tys = module.signature_at(handle.parameters);
        if param_tys.0.len() != params.len() {
            bail!(
                "{}::{} function argument length mismatch, expected: {:?}, got: {:?}",
                module_id,
                func_name,
                param_tys.len(),
                params.len()
            );
        }
        for (tok, param) in param_tys.0.iter().zip(params.iter()) {
            match param.ty {
                BatchArgumentType::PreviousResult => {
                    let PreviousResult {
                        call_idx: _,
                        return_idx: _,
                        operation_type,
                        ability,
                        ty,
                    } = param.previous_result.as_ref().unwrap();
                    match operation_type {
                        ArgumentOperation::Copy => {
                            if !ability.has_ability(Ability::Copy) {
                                bail!("Copying value of non-copyable type: {}", ty);
                            }
                            let expected_ty = self.type_tag_from_signature(module, tok, ty_args)?;
                            if &expected_ty != ty {
                                bail!(
                                    "Type mismatch in arguments. Expected: {}, got: {}",
                                    expected_ty,
                                    ty
                                );
                            }
                        },
                        ArgumentOperation::Move => {
                            let expected_ty = self.type_tag_from_signature(module, tok, ty_args)?;
                            if &expected_ty != ty {
                                bail!(
                                    "Type mismatch in arguments. Expected: {}, got: {}",
                                    expected_ty,
                                    ty
                                );
                            }
                        },
                        ArgumentOperation::Borrow => {
                            let expected_ty = match tok {
                                SignatureToken::Reference(inner_ty) => {
                                    self.type_tag_from_signature(module, &inner_ty, ty_args)?
                                },
                                _ => bail!("Type mismatch in arguments. Got reference of {}", ty),
                            };

                            if &expected_ty != ty {
                                bail!(
                                    "Type mismatch in arguments. Expected: {}, got: {}",
                                    expected_ty,
                                    ty
                                );
                            }
                        },
                        ArgumentOperation::BorrowMut => {
                            let expected_ty = match tok {
                                SignatureToken::MutableReference(inner_ty) => {
                                    self.type_tag_from_signature(module, &inner_ty, ty_args)?
                                },
                                _ => bail!(
                                    "Type mismatch in arguments. Got mutable reference of {}",
                                    ty
                                ),
                            };
                            if &expected_ty != ty {
                                bail!(
                                    "Type mismatch in arguments. Expected: {}, got: {}",
                                    expected_ty,
                                    ty
                                );
                            }
                        },
                    }
                },
                _ => (),
            }
        }
        Ok(())
    }

    fn type_tag_from_signature(
        &self,
        module: &CompiledModule,
        tok: &SignatureToken,
        ty_args: &[TypeTag],
    ) -> anyhow::Result<TypeTag> {
        Ok(match tok {
            SignatureToken::Address => TypeTag::Address,
            SignatureToken::Bool => TypeTag::Bool,
            SignatureToken::U8 => TypeTag::U8,
            SignatureToken::U16 => TypeTag::U16,
            SignatureToken::U32 => TypeTag::U32,
            SignatureToken::U64 => TypeTag::U64,
            SignatureToken::U128 => TypeTag::U128,
            SignatureToken::U256 => TypeTag::U256,
            SignatureToken::Signer => TypeTag::Signer,
            SignatureToken::MutableReference(_) | SignatureToken::Reference(_) => {
                bail!("Unexpected reference type in the return value")
            },
            SignatureToken::Struct(idx) => {
                let st_handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(st_handle.module);
                TypeTag::Struct(Box::new(StructTag {
                    address: *module.address_identifier_at(module_handle.address),
                    module: module.identifier_at(module_handle.name).to_owned(),
                    name: module.identifier_at(st_handle.name).to_owned(),
                    type_args: vec![],
                }))
            },
            SignatureToken::Vector(tok) => TypeTag::Vector(Box::new(
                self.type_tag_from_signature(module, tok, ty_args)?,
            )),
            SignatureToken::StructInstantiation(idx, toks) => {
                let st_handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(st_handle.module);
                TypeTag::Struct(Box::new(StructTag {
                    address: *module.address_identifier_at(module_handle.address),
                    module: module.identifier_at(module_handle.name).to_owned(),
                    name: module.identifier_at(st_handle.name).to_owned(),
                    type_args: toks
                        .iter()
                        .map(|tok| self.type_tag_from_signature(module, tok, ty_args))
                        .collect::<anyhow::Result<_>>()?,
                }))
            },
            SignatureToken::TypeParameter(idx) => ty_args
                .get(*idx as usize)
                .ok_or_else(|| anyhow!("Type parameter access out of bound"))?
                .clone(),
        })
    }

    fn add_batched_call_impl(
        &mut self,
        module: String,
        function: String,
        ty_args: Vec<String>,
        args: Vec<BatchArgument>,
    ) -> anyhow::Result<Vec<BatchArgument>> {
        let ty_args = ty_args
            .iter()
            .map(|s| TypeTag::from_str(s))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let module = ModuleId::from_str(&module)?;
        let function = Identifier::new(function)?;
        self.check_parameters(&module, &function, &args, &ty_args)?;
        self.calls.push(BatchedFunctionCall {
            module: module.clone(),
            function: function.clone(),
            ty_args: ty_args.clone(),
            args,
        });
        self.return_values(&module, &function, &ty_args)
    }

    #[cfg(test)]
    pub(crate) fn insert_module(&mut self, module: CompiledModule) {
        self.modules.insert(module.self_id(), module);
    }

    async fn load_module_impl(
        &mut self,
        api_url: String,
        module_name: String,
    ) -> anyhow::Result<()> {
        let module_id = ModuleId::from_str(&module_name).unwrap();
        let url = format!(
            "{}/{}/{}/{}/{}",
            api_url, "accounts", &module_id.address, "module", &module_id.name
        );
        let response = reqwest::get(url).await?;
        let result = if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(response.text().await?)
        };

        let bytes_result = match result {
            Ok(json_string) => {
                let v: Value = serde_json::from_str(&json_string)?;
                Ok(v["bytecode"].to_string())
            },
            Err(json_string) => {
                let v: Value = serde_json::from_str(&json_string)?;
                Err(v["message"].to_string())
            },
        };

        match bytes_result {
            Ok(bytes_hex) => {
                self.modules.insert(
                    module_id,
                    CompiledModule::deserialize(
                        hex::decode(bytes_hex.replace("0x", "").replace("\"", ""))?.as_slice(),
                    )?,
                );
                Ok(())
            },
            Err(_message) => Ok(()),
        }
    }

    pub async fn load_module(
        &mut self,
        api_url: String,
        module_name: String,
    ) -> Result<(), JsValue> {
        self.load_module_impl(api_url, module_name)
            .await
            .map_err(|err| JsValue::from(format!("{:?}", err)))
    }
}

#[wasm_bindgen]
impl BatchArgument {
    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        Self {
            ty: BatchArgumentType::Raw,
            raw: Some(bytes),
            signer: None,
            previous_result: None,
        }
    }

    pub fn new_signer(signer_idx: u16) -> Self {
        Self {
            ty: BatchArgumentType::Signer,
            raw: None,
            signer: Some(signer_idx),
            previous_result: None,
        }
    }

    pub fn borrow(&self) -> Result<BatchArgument, String> {
        self.change_op_type(ArgumentOperation::Borrow)
    }

    pub fn borrow_mut(&self) -> Result<BatchArgument, String> {
        self.change_op_type(ArgumentOperation::BorrowMut)
    }

    pub fn copy(&self) -> Result<BatchArgument, String> {
        self.change_op_type(ArgumentOperation::Copy)
    }

    fn change_op_type(&self, operation_type: ArgumentOperation) -> Result<BatchArgument, String> {
        match (&self.ty, &self.previous_result) {
            (
                BatchArgumentType::PreviousResult,
                Some(PreviousResult {
                    call_idx,
                    return_idx,
                    operation_type: _,
                    ability,
                    ty,
                }),
            ) => Ok(BatchArgument {
                ty: BatchArgumentType::PreviousResult,
                raw: None,
                signer: None,
                previous_result: Some(PreviousResult {
                    call_idx: *call_idx,
                    return_idx: *return_idx,
                    operation_type,
                    ability: *ability,
                    ty: ty.clone(),
                }),
            }),
            _ => Err(
                "Unexpected argument type, can only borrow from previous function results"
                    .to_string(),
            ),
        }
    }
}
