// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Code to generate compiled Move script from a series of individual Move calls.
//!
//! Defined a struct called `TransactionComposer`. With `TransactionComposer`, you will be able to add new Move calls
//! to it via the `add_batched_call` api. The code will perform a light weight type check to make sure the ability and
//! type safety of the Move calls will be held. After adding the calls, you will be able to consume the `TransactionComposer`
//! and get the corresponding `CompiledScript` via the `generate_batched_calls` api.

#[cfg(test)]
use crate::MoveFunctionCall;
use crate::{
    helpers::{import_signature, import_type_tag, Script},
    ArgumentOperation, CallArgument, ComposerVersion, PreviousResult, APTOS_SCRIPT_COMPOSER_KEY,
};
use anyhow::{anyhow, bail, Result};
use move_binary_format::{
    access::ScriptAccess,
    binary_views::BinaryIndexedView,
    builders::CompiledScriptBuilder,
    errors::PartialVMResult,
    file_format::{
        empty_script, Bytecode, FunctionHandleIndex, FunctionInstantiation,
        FunctionInstantiationIndex, Signature, SignatureToken, TableIndex,
    },
    CompiledModule,
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    transaction_argument::TransactionArgument,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    str::FromStr,
};
use wasm_bindgen::prelude::*;

thread_local! {
    static LOADED_MODULES: RefCell<BTreeMap<ModuleId, CompiledModule>> = const { RefCell::new(BTreeMap::new()) };
}

#[wasm_bindgen]
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct AllocatedLocal {
    op_type: ArgumentOperation,
    is_parameter: bool,
    local_idx: u16,
}

#[derive(Clone, Debug)]
#[wasm_bindgen]
struct BuilderCall {
    type_args: Vec<SignatureToken>,
    call_idx: FunctionHandleIndex,
    arguments: Vec<AllocatedLocal>,
    // Assigned index of returned values in the local pool.
    returns: Vec<u16>,

    #[cfg(test)]
    type_tags: Vec<TypeTag>,
}

#[derive(Clone, Debug)]
#[wasm_bindgen]
pub struct TransactionComposer {
    builder: CompiledScriptBuilder,
    calls: Vec<BuilderCall>,
    parameters: Vec<Vec<u8>>,
    locals_availability: Vec<bool>,

    locals_ty: Vec<SignatureToken>,
    parameters_ty: Vec<SignatureToken>,

    #[cfg(test)]
    signer_count: u16,
}

#[wasm_bindgen]
impl TransactionComposer {
    /// Create a builder with one distinct signer available. This should be the default configuration.
    pub fn single_signer() -> Self {
        let mut script = empty_script();
        script.code.code = vec![];

        let builder = CompiledScriptBuilder::new(script);
        Self {
            builder,
            calls: vec![],
            parameters: vec![],
            locals_availability: vec![],
            locals_ty: vec![],
            parameters_ty: vec![SignatureToken::Reference(Box::new(SignatureToken::Signer))],

            #[cfg(test)]
            signer_count: 1,
        }
    }

    /// Create a builder with one signer needed for script. This would be needed for multi-agent
    /// transaction where multiple signers are present.
    pub fn multi_signer(signer_count: u16) -> Self {
        let mut script = empty_script();
        script.code.code = vec![];

        let builder = CompiledScriptBuilder::new(script);
        Self {
            builder,
            calls: vec![],
            parameters: vec![],
            locals_availability: vec![],
            locals_ty: vec![],
            parameters_ty: std::iter::repeat(SignatureToken::Reference(Box::new(
                SignatureToken::Signer,
            )))
            .take(signer_count.into())
            .collect(),

            #[cfg(test)]
            signer_count,
        }
    }

    /// Consume the builder and generate a serialized script with calls in the builder.
    pub fn generate_batched_calls(self, with_metadata: bool) -> Result<Vec<u8>, String> {
        self.generate_batched_calls_impl(with_metadata)
            .map_err(|e| e.to_string())
    }

    /// Load up a module from a remote endpoint. Will need to invoke this function prior to the
    /// call.
    pub async fn load_module(
        &mut self,
        api_url: String,
        api_key: String,
        module_name: String,
    ) -> Result<(), JsValue> {
        let module_id = ModuleId::from_str(&module_name)
            .map_err(|err| JsValue::from(format!("Invalid module name: {:?}", err)))?;
        self.load_module_impl(api_url.as_str(), api_key.as_str(), module_id)
            .await
            .map_err(|err| JsValue::from(format!("{:?}", err)))
    }

    /// Load up the dependency modules of a TypeTag from a remote endpoint.
    pub async fn load_type_tag(
        &mut self,
        api_url: String,
        api_key: String,
        type_tag: String,
    ) -> Result<(), JsValue> {
        let type_tag = TypeTag::from_str(&type_tag)
            .map_err(|err| JsValue::from(format!("Invalid type name: {:?}", err)))?;
        self.load_type_tag_impl(api_url.as_str(), api_key.as_str() , &type_tag)
            .await
            .map_err(|err| JsValue::from(format!("{:?}", err)))
    }

    /// This would be the core api for the `TransactionComposer`. The function would:
    /// - add the function call to the builder
    /// - allocate the locals and parameters needed for this function call
    /// - return the arguments back to the caller which could be passed into subsequent calls
    ///   into `add_batched_call`.
    ///
    /// This function would also check for the ability and type safety when passing values
    /// into the function call, and will abort if there's a violation.
    #[wasm_bindgen(js_name = add_batched_call)]
    pub fn add_batched_call_wasm(
        &mut self,
        module: String,
        function: String,
        ty_args: Vec<String>,
        args: Vec<CallArgumentWasm>,
    ) -> Result<Vec<CallArgumentWasm>, JsValue> {
        self.add_batched_call(
            module,
            function,
            ty_args,
            args.into_iter().map(|a| a.into()).collect(),
        )
        .map_err(|err| JsValue::from(format!("{:?}", err)))
        .map(|results| results.into_iter().map(|a| a.into()).collect())
    }
}

impl TransactionComposer {
    pub fn insert_module(&mut self, module: CompiledModule) {
        LOADED_MODULES.with(|modules| modules.borrow_mut().insert(module.self_id(), module));
    }

    fn check_argument_compatibility(
        &mut self,
        argument: &AllocatedLocal,
        expected_ty: &SignatureToken,
    ) -> anyhow::Result<()> {
        let local_ty = if argument.is_parameter {
            self.parameters_ty[argument.local_idx as usize].clone()
        } else {
            self.locals_ty[argument.local_idx as usize].clone()
        };

        let ty = match argument.op_type {
            ArgumentOperation::Borrow => SignatureToken::Reference(Box::new(local_ty)),
            ArgumentOperation::BorrowMut => SignatureToken::MutableReference(Box::new(local_ty)),
            ArgumentOperation::Copy => {
                let ability = BinaryIndexedView::Script(self.builder.as_script())
                    .abilities(&local_ty, &[])
                    .map_err(|_| anyhow!("Failed to calculate ability for type"))?;
                if !ability.has_copy() {
                    bail!("Trying to copy move values that does NOT have copy ability");
                }
                local_ty
            },
            ArgumentOperation::Move => {
                if !argument.is_parameter {
                    if self.locals_availability[argument.local_idx as usize] {
                        self.locals_availability[argument.local_idx as usize] = false;
                    } else {
                        bail!("Trying to use a Move value that has already been moved");
                    }
                }
                local_ty
            },
        };

        if &ty != expected_ty {
            bail!("Type mismatch when passing arugments around");
        }
        Ok(())
    }

    pub fn add_batched_call(
        &mut self,
        module: String,
        function: String,
        ty_args: Vec<String>,
        args: Vec<CallArgument>,
    ) -> anyhow::Result<Vec<CallArgument>> {
        let ty_args = ty_args
            .iter()
            .map(|s| TypeTag::from_str(s))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let module = ModuleId::from_str(&module)?;
        let function = Identifier::new(function)?;
        let call_idx = LOADED_MODULES.with(|modules| match modules.borrow().get(&module) {
            Some(module_ref) => self
                .builder
                .import_call_by_name(function.as_ident_str(), module_ref)
                .map_err(|err| anyhow!("Cannot import module {}: {:?}", module, err)),
            None => Err(anyhow!("Module {} is not yet loaded", module)),
        })?;

        let type_arguments = LOADED_MODULES.with(|modules| {
            ty_args
                .iter()
                .map(|ty| import_type_tag(&mut self.builder, ty, &modules.borrow()))
                .collect::<PartialVMResult<Vec<_>>>()
        })?;

        let mut arguments = vec![];
        let expected_args_ty = {
            let script = self.builder.as_script();
            let func = script.function_handle_at(call_idx);
            if script.signature_at(func.parameters).0.len() != args.len() {
                bail!(
                    "Function {}::{} argument call size mismatch",
                    module,
                    function
                );
            }
            script
                .signature_at(func.parameters)
                .0
                .iter()
                .map(|ty| ty.instantiate(&type_arguments))
                .collect::<Vec<_>>()
        };

        for (arg, ty) in args.into_iter().zip(expected_args_ty) {
            match arg {
                CallArgument::PreviousResult(r) => {
                    let argument = AllocatedLocal {
                        op_type: r.operation_type,
                        is_parameter: false,
                        local_idx: self.calls[r.call_idx as usize].returns[r.return_idx as usize],
                    };
                    self.check_argument_compatibility(&argument, &ty)?;
                    arguments.push(argument)
                },
                CallArgument::Signer(i) => arguments.push(AllocatedLocal {
                    op_type: ArgumentOperation::Copy,
                    is_parameter: true,
                    local_idx: i,
                }),
                CallArgument::Raw(bytes) => {
                    let new_local_idx = self.parameters_ty.len() as u16;
                    self.parameters_ty.push(ty);
                    self.parameters.push(bytes);
                    arguments.push(AllocatedLocal {
                        op_type: ArgumentOperation::Move,
                        is_parameter: true,
                        local_idx: new_local_idx,
                    })
                },
            }
        }

        let mut returns = vec![];

        let expected_returns_ty = {
            let func = self.builder.as_script().function_handle_at(call_idx);
            self.builder
                .as_script()
                .signature_at(func.return_)
                .0
                .iter()
                .map(|ty| ty.instantiate(&type_arguments))
                .collect::<Vec<_>>()
        };

        let mut returned_arguments = vec![];
        let num_of_calls = self.calls.len() as u16;

        for (idx, return_ty) in expected_returns_ty.into_iter().enumerate() {
            let ability = BinaryIndexedView::Script(self.builder.as_script())
                .abilities(&return_ty, &[])
                .map_err(|_| anyhow!("Failed to calculate ability for type"))?;

            let local_idx = self.locals_ty.len() as u16;
            self.locals_ty.push(return_ty);
            self.locals_availability.push(true);
            returns.push(local_idx);

            // For values that has drop and copy ability, use copy by default to avoid calling copy manually
            // on the client side.
            returned_arguments.push(CallArgument::PreviousResult(PreviousResult {
                operation_type: if ability.has_drop() && ability.has_copy() {
                    ArgumentOperation::Copy
                } else {
                    ArgumentOperation::Move
                },
                call_idx: num_of_calls,
                return_idx: idx as u16,
            }));
        }

        if self.parameters.len() + self.locals_ty.len() > u8::MAX as usize {
            bail!("Too many locals being allocated, please truncate the transaction");
        }

        self.calls.push(BuilderCall {
            type_args: type_arguments,
            call_idx,
            arguments,
            returns: returns.clone(),
            #[cfg(test)]
            type_tags: ty_args,
        });

        Ok(returned_arguments)
    }

    fn check_drop_at_end(&self) -> anyhow::Result<()> {
        let view = BinaryIndexedView::Script(self.builder.as_script());
        for (available, ty) in self.locals_availability.iter().zip(self.locals_ty.iter()) {
            if *available {
                let ability = view
                    .abilities(ty, &[])
                    .map_err(|_| anyhow!("Failed to calculate ability for type"))?;
                if !ability.has_drop() {
                    bail!("Unused non-droppable Move value");
                }
            }
        }
        Ok(())
    }

    fn generate_batched_calls_impl(self, with_metadata: bool) -> anyhow::Result<Vec<u8>> {
        self.check_drop_at_end()?;
        let parameters_count = self.parameters_ty.len() as u16;
        let mut script = self.builder.into_script();
        let mut instantiations = HashMap::new();
        for call in self.calls {
            for arg in call.arguments {
                script.code.code.push(arg.to_instruction(parameters_count)?);
            }

            // Calls
            if call.type_args.is_empty() {
                script.code.code.push(Bytecode::Call(call.call_idx));
            } else {
                if script.function_instantiations.len() >= TableIndex::MAX as usize {
                    bail!("Too many function instantiations");
                }
                let type_parameters = import_signature(&mut script, Signature(call.type_args))?;

                let inst = FunctionInstantiation {
                    handle: call.call_idx,
                    type_parameters,
                };
                if let Some(idx) = instantiations.get(&inst) {
                    script.code.code.push(Bytecode::CallGeneric(*idx));
                } else {
                    let fi_idx =
                        FunctionInstantiationIndex(script.function_instantiations.len() as u16);
                    script.function_instantiations.push(inst.clone());
                    instantiations.insert(inst, fi_idx);
                    script.code.code.push(Bytecode::CallGeneric(fi_idx));
                }
            }

            // Storing return values
            for arg in call.returns.iter().rev() {
                script
                    .code
                    .code
                    .push(Bytecode::StLoc((*arg + parameters_count) as u8));
            }
        }
        script.code.code.push(Bytecode::Ret);
        script.parameters = import_signature(&mut script, Signature(self.parameters_ty))?;
        script.code.locals = import_signature(&mut script, Signature(self.locals_ty))?;

        move_bytecode_verifier::verify_script(&script).map_err(|err| anyhow!("{:?}", err))?;
        if with_metadata {
            script.metadata.push(Metadata {
                key: APTOS_SCRIPT_COMPOSER_KEY.to_owned(),
                value: bcs::to_bytes(&ComposerVersion::V1).unwrap(),
            });
        }
        let mut bytes = vec![];
        script
            .serialize(&mut bytes)
            .map_err(|err| anyhow!("Failed to serialize script: {:?}", err))?;

        Ok(bcs::to_bytes(&Script {
            code: bytes,
            ty_args: vec![],
            args: self
                .parameters
                .into_iter()
                .map(TransactionArgument::Serialized)
                .collect(),
        })
        .unwrap())
    }

    async fn load_module_impl(&mut self, api_url: &str, api_key: &str, module_id: ModuleId) -> anyhow::Result<()> {
        // check if stored in local cache
        if LOADED_MODULES.with(|modules| modules.borrow().get(&module_id).is_some()) {
            return Ok(());
        };

        let url = format!(
            "{}/{}/{}/{}/{}",
            api_url, "accounts", &module_id.address, "module", &module_id.name
        );
       
       let response = if api_key.is_empty() {
            reqwest::get(url).await?
        }else{
            let client = reqwest::Client::new();
            client
                .get(url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?
        };

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
                self.insert_module(CompiledModule::deserialize(
                    hex::decode(bytes_hex.replace("0x", "").replace('\"', ""))?.as_slice(),
                )?);
                Ok(())
            },
            Err(_message) => Ok(()),
        }
    }

    async fn load_type_tag_impl(
        &mut self,
        api_url: &str,
        api_key: &str,
        type_tag: &TypeTag,
    ) -> anyhow::Result<()> {
        match type_tag {
            TypeTag::Struct(s) => {
                self.load_module_impl(api_url, api_key, s.module_id()).await?;
                for ty in s.type_args.iter() {
                    Box::pin(self.load_type_tag_impl(api_url, api_key,ty)).await?;
                }
                Ok(())
            },
            TypeTag::Vector(v) => Box::pin(self.load_type_tag_impl(api_url, api_key,v)).await,
            _ => Ok(()),
        }
    }

    #[cfg(test)]
    pub(crate) fn assert_decompilation_eq(&self, calls: &[MoveFunctionCall]) {
        let script = self.builder.as_script();

        assert_eq!(self.calls.len(), calls.len());
        for (lhs_call, rhs_call) in self.calls.iter().zip(calls.iter()) {
            let fh = script.function_handle_at(lhs_call.call_idx);
            assert_eq!(
                script.identifier_at(fh.name),
                rhs_call.function.as_ident_str()
            );
            assert_eq!(
                script.module_id_for_handle(script.module_handle_at(fh.module)),
                rhs_call.module
            );
            assert_eq!(lhs_call.type_tags, rhs_call.ty_args);
            assert_eq!(lhs_call.arguments.len(), rhs_call.args.len());
            for (lhs_arg, rhs_arg) in lhs_call.arguments.iter().zip(rhs_call.args.iter()) {
                self.check_argument_eq(lhs_arg, rhs_arg);
            }
        }
    }

    #[cfg(test)]
    fn check_argument_eq(&self, lhs: &AllocatedLocal, rhs: &CallArgument) {
        use crate::PreviousResult;

        match rhs {
            CallArgument::PreviousResult(PreviousResult {
                call_idx,
                return_idx,
                operation_type,
            }) => {
                let local_idx = self.calls[*call_idx as usize].returns[*return_idx as usize];
                assert_eq!(lhs, &AllocatedLocal {
                    local_idx,
                    op_type: operation_type.clone(),
                    is_parameter: false,
                });
            },
            CallArgument::Raw(input) => {
                assert!(lhs.is_parameter);
                assert_eq!(lhs.op_type, ArgumentOperation::Move);
                assert_eq!(
                    &self.parameters[(lhs.local_idx - self.signer_count) as usize],
                    input
                );
            },
            CallArgument::Signer(idx) => {
                assert_eq!(lhs, &AllocatedLocal {
                    op_type: ArgumentOperation::Copy,
                    is_parameter: true,
                    local_idx: *idx
                })
            },
        }
    }
}

impl AllocatedLocal {
    fn to_instruction(&self, parameter_size: u16) -> anyhow::Result<Bytecode> {
        let local_idx = if self.is_parameter {
            self.local_idx
        } else {
            parameter_size
                .checked_add(self.local_idx)
                .ok_or_else(|| anyhow!("Too many locals"))?
        };
        if local_idx >= u8::MAX as u16 {
            bail!("Too many locals");
        };
        Ok(match self.op_type {
            ArgumentOperation::Borrow => Bytecode::ImmBorrowLoc(local_idx as u8),
            ArgumentOperation::BorrowMut => Bytecode::MutBorrowLoc(local_idx as u8),
            ArgumentOperation::Move => Bytecode::MoveLoc(local_idx as u8),
            ArgumentOperation::Copy => Bytecode::CopyLoc(local_idx as u8),
        })
    }
}

#[derive(Clone, Debug)]
pub enum ArgumentType {
    Signer,
    Raw,
    PreviousResult,
}

/// WASM Representation of CallArgument. This is because wasm_bindgen can only support c-style enum.
#[wasm_bindgen(js_name = "CallArgument")]
#[derive(Clone, Debug)]
pub struct CallArgumentWasm {
    ty: ArgumentType,
    signer: Option<u16>,
    raw: Option<Vec<u8>>,
    previous_result: Option<PreviousResult>,
}

impl From<CallArgument> for CallArgumentWasm {
    fn from(value: CallArgument) -> Self {
        match value {
            CallArgument::PreviousResult(r) => CallArgumentWasm {
                ty: ArgumentType::PreviousResult,
                signer: None,
                raw: None,
                previous_result: Some(r),
            },
            CallArgument::Raw(b) => CallArgumentWasm {
                ty: ArgumentType::Raw,
                signer: None,
                raw: Some(b),
                previous_result: None,
            },
            CallArgument::Signer(i) => CallArgumentWasm {
                ty: ArgumentType::Signer,
                signer: Some(i),
                raw: None,
                previous_result: None,
            },
        }
    }
}

impl From<CallArgumentWasm> for CallArgument {
    fn from(value: CallArgumentWasm) -> Self {
        match value.ty {
            ArgumentType::PreviousResult => {
                CallArgument::PreviousResult(value.previous_result.unwrap())
            },
            ArgumentType::Raw => CallArgument::Raw(value.raw.unwrap()),
            ArgumentType::Signer => CallArgument::Signer(value.signer.unwrap()),
        }
    }
}

#[wasm_bindgen(js_class = "CallArgument")]
impl CallArgumentWasm {
    #[wasm_bindgen(js_name = newBytes)]
    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        CallArgument::Raw(bytes).into()
    }

    #[wasm_bindgen(js_name = newSigner)]
    pub fn new_signer(signer_idx: u16) -> Self {
        CallArgument::Signer(signer_idx).into()
    }

    pub fn borrow(&self) -> Result<Self, String> {
        self.change_op_type(ArgumentOperation::Borrow)
    }

    #[wasm_bindgen(js_name = borrowMut)]
    pub fn borrow_mut(&self) -> Result<Self, String> {
        self.change_op_type(ArgumentOperation::BorrowMut)
    }

    pub fn copy(&self) -> Result<Self, String> {
        self.change_op_type(ArgumentOperation::Copy)
    }

    fn change_op_type(&self, operation_type: ArgumentOperation) -> Result<Self, String> {
        Ok(CallArgument::from(self.clone())
            .change_op_type(operation_type)?
            .into())
    }
}
