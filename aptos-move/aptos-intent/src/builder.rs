// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ArgumentOperation, BatchArgument, BatchedFunctionCall, PreviousResult};
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

#[wasm_bindgen]
/// Arguments for each function.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum BatchArgumentType {
    Raw,
    Signer,
    PreviousResult,
}

#[wasm_bindgen(js_name = BatchArgument)]
/// Arguments for each function. Wasm bindgen only support C-style enum so use option to work around.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BatchArgumentWASM {
    ty: BatchArgumentType,
    raw: Option<Vec<u8>>,
    signer: Option<u16>,
    previous_result: Option<PreviousResult>,
}

#[wasm_bindgen]
pub struct BatchedFunctionCallBuilder {
    calls: Vec<BatchedFunctionCall>,
    num_signers: u16,
    modules: BTreeMap<ModuleId, CompiledModule>,
    type_of_returns: BTreeMap<(u16, u16), (TypeTag, AbilitySet)>,
}

#[wasm_bindgen]
impl BatchedFunctionCallBuilder {
    pub fn single_signer() -> Self {
        Self {
            calls: vec![],
            num_signers: 1,
            modules: BTreeMap::new(),
            type_of_returns: BTreeMap::new(),
        }
    }

    pub fn multi_signer(signer_count: u16) -> Self {
        Self {
            calls: vec![],
            num_signers: signer_count,
            modules: BTreeMap::new(),
            type_of_returns: BTreeMap::new(),
        }
    }

    pub fn add_batched_call(
        &mut self,
        module: String,
        function: String,
        ty_args: Vec<String>,
        args: Vec<BatchArgumentWASM>,
    ) -> Result<Vec<BatchArgumentWASM>, String> {
        Ok(self
            .add_batched_call_impl(
                module,
                function,
                ty_args,
                args.into_iter().map(BatchArgument::from).collect(),
            )
            .map_err(|err| format!("{:?}", err))?
            .into_iter()
            .map(|arg| arg.into())
            .collect())
    }

    pub fn generate_batched_calls(self) -> Result<Vec<u8>, String> {
        crate::codegen::generate_script_from_batched_calls(
            &self.calls,
            self.num_signers,
            &self.modules,
        )
        .map_err(|err| format!("{:?}", err))
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

fn find_function<'a>(
    map: &'a BTreeMap<ModuleId, CompiledModule>,
    module_id: &ModuleId,
    func_name: &IdentStr,
) -> anyhow::Result<(&'a CompiledModule, &'a FunctionHandle)> {
    if let Some(module) = map.get(module_id) {
        for def in module.function_defs() {
            let handle = module.function_handle_at(def.function);
            if module.identifier_at(handle.name) == func_name {
                return Ok((module, handle));
            }
        }
    }
    anyhow::bail!("{}::{} doesn't exist on chain", module_id, func_name)
}

impl BatchedFunctionCallBuilder {
    fn return_values(
        &mut self,
        module_id: &ModuleId,
        func_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> anyhow::Result<Vec<BatchArgument>> {
        if self.calls.is_empty() {
            bail!("No calls exists in the builder yet");
        }
        let (module, handle) = find_function(&self.modules, module_id, func_name)?;
        let mut returns = vec![];
        for (idx, sig) in module.signature_at(handle.return_).0.iter().enumerate() {
            let call_idx = (self.calls.len() - 1) as u16;
            let return_idx = idx as u16;
            let ty = Self::type_tag_from_signature(module, sig, ty_args)?;
            let ability = BinaryIndexedView::Module(module)
                .abilities(sig, &handle.type_parameters)
                .unwrap();

            self.type_of_returns
                .insert((call_idx, return_idx), (ty, ability));

            returns.push(BatchArgument::PreviousResult(PreviousResult {
                call_idx,
                return_idx,
                operation_type: ArgumentOperation::Move,
            }));
        }
        Ok(returns)
    }

    fn check_parameters(
        &self,
        module_id: &ModuleId,
        func_name: &IdentStr,
        params: &[BatchArgument],
        ty_args: &[TypeTag],
    ) -> anyhow::Result<()> {
        let (module, handle) = find_function(&self.modules, module_id, func_name)?;
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
            if let BatchArgument::PreviousResult(PreviousResult {
                call_idx,
                return_idx,
                operation_type,
            }) = param
            {
                let (ty, ability) = self
                    .type_of_returns
                    .get(&(*call_idx, *return_idx))
                    .ok_or_else(|| anyhow!("Type of return not found"))?;
                match operation_type {
                    ArgumentOperation::Copy => {
                        if !ability.has_ability(Ability::Copy) {
                            bail!("Copying value of non-copyable type: {}", ty);
                        }
                        let expected_ty = Self::type_tag_from_signature(module, tok, ty_args)?;
                        if &expected_ty != ty {
                            bail!(
                                "Type mismatch in arguments. Expected: {}, got: {}",
                                expected_ty,
                                ty
                            );
                        }
                    },
                    ArgumentOperation::Move => {
                        let expected_ty = Self::type_tag_from_signature(module, tok, ty_args)?;
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
                                Self::type_tag_from_signature(module, inner_ty, ty_args)?
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
                                Self::type_tag_from_signature(module, inner_ty, ty_args)?
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
            }
        }
        Ok(())
    }

    fn type_tag_from_signature(
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
                Self::type_tag_from_signature(module, tok, ty_args)?,
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
                        .map(|tok| Self::type_tag_from_signature(module, tok, ty_args))
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
                        hex::decode(bytes_hex.replace("0x", "").replace('\"', ""))?.as_slice(),
                    )?,
                );
                Ok(())
            },
            Err(_message) => Ok(()),
        }
    }

    #[cfg(test)]
    pub(crate) fn calls(&self) -> &[BatchedFunctionCall] {
        &self.calls
    }
}

#[wasm_bindgen(js_class = BatchArgument)]
impl BatchArgumentWASM {
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

    pub fn borrow(&self) -> Result<BatchArgumentWASM, String> {
        self.change_op_type(ArgumentOperation::Borrow)
    }

    pub fn borrow_mut(&self) -> Result<BatchArgumentWASM, String> {
        self.change_op_type(ArgumentOperation::BorrowMut)
    }

    pub fn copy(&self) -> Result<BatchArgumentWASM, String> {
        self.change_op_type(ArgumentOperation::Copy)
    }

    fn change_op_type(
        &self,
        operation_type: ArgumentOperation,
    ) -> Result<BatchArgumentWASM, String> {
        match (&self.ty, &self.previous_result) {
            (
                BatchArgumentType::PreviousResult,
                Some(PreviousResult {
                    call_idx,
                    return_idx,
                    operation_type: _,
                }),
            ) => Ok(BatchArgumentWASM {
                ty: BatchArgumentType::PreviousResult,
                raw: None,
                signer: None,
                previous_result: Some(PreviousResult {
                    call_idx: *call_idx,
                    return_idx: *return_idx,
                    operation_type,
                }),
            }),
            _ => Err(
                "Unexpected argument type, can only borrow from previous function results"
                    .to_string(),
            ),
        }
    }
}

impl From<BatchArgumentWASM> for BatchArgument {
    fn from(value: BatchArgumentWASM) -> Self {
        match value.ty {
            BatchArgumentType::PreviousResult => {
                BatchArgument::PreviousResult(value.previous_result.unwrap())
            },
            BatchArgumentType::Raw => BatchArgument::Raw(value.raw.unwrap()),
            BatchArgumentType::Signer => BatchArgument::Signer(value.signer.unwrap()),
        }
    }
}

impl From<BatchArgument> for BatchArgumentWASM {
    fn from(value: BatchArgument) -> BatchArgumentWASM {
        match value {
            BatchArgument::PreviousResult(r) => BatchArgumentWASM {
                ty: BatchArgumentType::PreviousResult,
                raw: None,
                signer: None,
                previous_result: Some(r),
            },
            BatchArgument::Raw(r) => BatchArgumentWASM {
                ty: BatchArgumentType::Raw,
                raw: Some(r),
                signer: None,
                previous_result: None,
            },
            BatchArgument::Signer(r) => BatchArgumentWASM {
                ty: BatchArgumentType::Signer,
                raw: None,
                signer: Some(r),
                previous_result: None,
            },
        }
    }
}
