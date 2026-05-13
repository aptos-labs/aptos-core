// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::proto;
use dap::types::Variable;
use move_vm_runtime::debug::dap::VmStoppedState;
use move_vm_types::values::debug::DebugValue;

const FRAME_LOCALS_OFFSET: i64 = 1000;
const EXPANDABLE_VARS_OFFSET: i64 = 100_000;

pub fn frame_locals_ref_id(frame_id: i64) -> i64 {
    FRAME_LOCALS_OFFSET + frame_id
}

pub(crate) struct StoredVariables {
    // Arena for expandable DAP variables (structs, vectors, refs). Each gets
    // an integer ID (EXPANDABLE_VARS_OFFSET + index) that VS Code sends back
    // when the user expands a tree node. Cleared on every VM stop.
    //
    // See `StoredVariables::get_variable()`.
    expandable_vars: Vec<DebugValue>,
}

impl StoredVariables {
    pub(crate) fn new() -> Self {
        Self {
            expandable_vars: vec![],
        }
    }

    pub(crate) fn store_expandable(&mut self, sv: DebugValue) -> i64 {
        let ref_id = EXPANDABLE_VARS_OFFSET + self.expandable_vars.len() as i64;
        self.expandable_vars.push(sv);
        ref_id
    }

    pub(crate) fn clear(&mut self) {
        self.expandable_vars.clear();
    }

    pub(crate) fn get_variables(
        &mut self,
        vm_stopped_state: Option<&VmStoppedState>,
        variable_ref_id: i64,
    ) -> Vec<Variable> {
        match variable_ref_id {
            FRAME_LOCALS_OFFSET..EXPANDABLE_VARS_OFFSET => {
                let frame_id = (variable_ref_id - FRAME_LOCALS_OFFSET) as usize;
                match vm_stopped_state {
                    Some(vm_state) => self.locals_for_frame(frame_id, vm_state),
                    None => vec![],
                }
            },
            EXPANDABLE_VARS_OFFSET.. => {
                let var_ref_idx = (variable_ref_id - EXPANDABLE_VARS_OFFSET) as usize;
                self.expanded_children(var_ref_idx)
            },
            _ => vec![],
        }
    }

    fn debug_value_to_variable(&mut self, name: String, sv: &DebugValue) -> Variable {
        let display = sv.to_string();
        match sv {
            DebugValue::Primitive(_)
            | DebugValue::Address(_)
            | DebugValue::Signer(_)
            | DebugValue::Error(_)
            | DebugValue::Invalid
            | DebugValue::Closure(_)
            | DebugValue::Delayed => proto::var(name, display),
            DebugValue::EnumVariant(_, fields) if fields.is_empty() => proto::var(name, display),
            DebugValue::Struct(_)
            | DebugValue::EnumVariant(_, _)
            | DebugValue::Vector(_)
            | DebugValue::ContainerRef(_)
            | DebugValue::IndexedRef(_) => {
                let ref_id = self.store_expandable(sv.clone());
                Variable {
                    variables_reference: ref_id,
                    ..proto::var(name, display)
                }
            },
        }
    }

    fn expanded_children(&mut self, container_idx: usize) -> Vec<Variable> {
        let Some(container_value) = self.expandable_vars.get(container_idx).cloned() else {
            return vec![];
        };
        match &container_value {
            DebugValue::Struct(fields) | DebugValue::EnumVariant(_, fields) => fields
                .iter()
                .map(|(name, child)| self.debug_value_to_variable(name.clone(), child))
                .collect(),
            DebugValue::Vector(items) => items
                .iter()
                .enumerate()
                .map(|(i, child)| self.debug_value_to_variable(format!("[{}]", i), child))
                .collect(),
            DebugValue::ContainerRef(inner) => {
                vec![self.debug_value_to_variable("*ref".to_string(), inner)]
            },
            DebugValue::IndexedRef(inner) => {
                vec![self.debug_value_to_variable("*ref".to_string(), inner)]
            },
            DebugValue::Primitive(_)
            | DebugValue::Address(_)
            | DebugValue::Signer(_)
            | DebugValue::Error(_)
            | DebugValue::Invalid
            | DebugValue::Closure(_)
            | DebugValue::Delayed => vec![],
        }
    }

    fn locals_for_frame(&mut self, frame_id: usize, vm_state: &VmStoppedState) -> Vec<Variable> {
        let locals = if frame_id == 0 {
            &vm_state.dap_locals
        } else {
            match vm_state.dap_stack_trace.get(frame_id - 1) {
                Some(frame) => &frame.locals,
                None => return vec![],
            }
        };

        let visible: Vec<_> = locals
            .iter()
            .filter(|l| !matches!(l.value, DebugValue::Invalid))
            .map(|l| (l.name.clone(), l.value.clone()))
            .collect();
        visible
            .into_iter()
            .map(|(name, sv)| self.debug_value_to_variable(name, &sv))
            .collect()
    }
}
