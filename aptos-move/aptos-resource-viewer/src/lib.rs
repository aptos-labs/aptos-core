// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Allows to view detailed on-chain information from modules and resources.
//! The library is not supposed to be used for runtime (e.g., in the VM), but
//! rather in "static" contexts, such as indexer, DB, etc.

pub mod module_view;

use crate::module_view::ModuleView;
use aptos_types::state_store::StateView;
use aptos_vm::data_cache::get_resource_group_member_from_metadata;
use move_binary_format::CompiledModule;
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    transaction_argument::TransactionArgument,
    value::{MoveTypeLayout, MoveValue},
};
use move_resource_viewer::MoveValueAnnotator;
pub use move_resource_viewer::{
    AnnotatedMoveClosure, AnnotatedMoveStruct, AnnotatedMoveValue, RawMoveStruct,
};
use std::sync::Arc;

pub struct AptosValueAnnotator<'a, S>(MoveValueAnnotator<ModuleView<'a, S>>);

impl<'a, S: StateView> AptosValueAnnotator<'a, S> {
    pub fn new(state_view: &'a S) -> Self {
        let view = ModuleView::new(state_view);
        Self(MoveValueAnnotator::new(view))
    }

    pub fn view_value(&self, ty_tag: &TypeTag, blob: &[u8]) -> anyhow::Result<AnnotatedMoveValue> {
        self.0.view_value(ty_tag, blob)
    }

    pub fn view_module(&self, module_id: &ModuleId) -> anyhow::Result<Option<Arc<CompiledModule>>> {
        self.0.view_module(module_id)
    }

    pub fn view_existing_module(
        &self,
        module_id: &ModuleId,
    ) -> anyhow::Result<Arc<CompiledModule>> {
        self.0.view_existing_module(module_id)
    }

    pub fn view_resource_group_member(&self, tag: &StructTag) -> Option<StructTag> {
        match self.view_module(&tag.module_id()) {
            Ok(Some(module)) => get_resource_group_member_from_metadata(tag, &module.metadata),
            // Even if module does not exist, we do not return an error but instead
            // say that the group tag does not exist.
            _ => None,
        }
    }

    pub fn view_resource(
        &self,
        tag: &StructTag,
        blob: &[u8],
    ) -> anyhow::Result<AnnotatedMoveStruct> {
        self.0.view_resource(tag, blob)
    }

    pub fn view_struct_fields(
        &self,
        tag: &StructTag,
        blob: &[u8],
    ) -> anyhow::Result<(Option<Identifier>, Vec<(Identifier, MoveValue)>)> {
        self.0.move_struct_fields(tag, blob)
    }

    pub fn view_function_arguments(
        &self,
        module: &ModuleId,
        function: &IdentStr,
        ty_args: &[TypeTag],
        args: &[Vec<u8>],
    ) -> anyhow::Result<Vec<AnnotatedMoveValue>> {
        self.0
            .view_function_arguments(module, function, ty_args, args)
    }

    pub fn view_function_returns(
        &self,
        module: &ModuleId,
        function: &IdentStr,
        ty_args: &[TypeTag],
        returns: &[Vec<u8>],
    ) -> anyhow::Result<Vec<AnnotatedMoveValue>> {
        self.0
            .view_function_returns(module, function, ty_args, returns)
    }

    pub fn view_script_arguments(
        &self,
        script_bytes: &[u8],
        args: &[TransactionArgument],
        ty_args: &[TypeTag],
    ) -> anyhow::Result<Vec<AnnotatedMoveValue>> {
        self.0.view_script_arguments(script_bytes, args, ty_args)
    }

    pub fn view_fully_decorated_ty_layout(
        &self,
        type_tag: &TypeTag,
    ) -> anyhow::Result<MoveTypeLayout> {
        self.0.get_type_layout_with_types(type_tag)
    }
}
