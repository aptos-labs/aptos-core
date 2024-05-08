// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod module_view;

use crate::module_view::ModuleView;
use aptos_types::state_store::StateView;
use move_binary_format::CompiledModule;
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{MoveTypeLayout, MoveValue},
};
use move_resource_viewer::MoveValueAnnotator;
pub use move_resource_viewer::{AnnotatedMoveStruct, AnnotatedMoveValue};
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

    pub fn view_module(&self, module_id: &ModuleId) -> anyhow::Result<Arc<CompiledModule>> {
        self.0.view_module(module_id)
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
    ) -> anyhow::Result<Vec<(Identifier, MoveValue)>> {
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

    pub fn view_fully_decorated_ty_layout(
        &self,
        type_tag: &TypeTag,
    ) -> anyhow::Result<MoveTypeLayout> {
        self.0.get_type_layout_with_types(type_tag)
    }
}
