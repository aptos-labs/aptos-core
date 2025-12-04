// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, ty::ReferenceKind};
use move_stackless_bytecode::function_target::FunctionTarget;
use std::{collections::BTreeMap, rc::Rc};

pub mod reference_safety_processor_v2;
pub mod reference_safety_processor_v3;

/// Annotation produced by implementations
#[derive(Clone)]
pub struct LifetimeAnnotation(pub BTreeMap<CodeOffset, LifetimeInfoAtCodeOffset>);

impl LifetimeAnnotation {
    /// Returns information for code offset.
    pub fn get_info_at(&self, code_offset: CodeOffset) -> &LifetimeInfoAtCodeOffset {
        self.0.get(&code_offset).expect("lifetime info")
    }
}

/// Information present at each code offset
#[derive(Clone)]
pub struct LifetimeInfoAtCodeOffset {
    /// Information hidden via a trait, specific to the processor. This is stored
    /// in an Rc so we can keep this clonable, even if the underlying data is a trait.
    before: Rc<dyn LifetimeInfo>,
    after: Rc<dyn LifetimeInfo>,
}

impl Default for LifetimeInfoAtCodeOffset {
    fn default() -> Self {
        Self {
            before: Rc::new(NoLifetimeInfo()),
            after: Rc::new(NoLifetimeInfo()),
        }
    }
}

impl LifetimeInfoAtCodeOffset {
    pub fn new(before: Rc<dyn LifetimeInfo>, after: Rc<dyn LifetimeInfo>) -> Self {
        Self { before, after }
    }

    /// Returns information about the borrow state of the given temporary
    /// before the program point. If there are any references active,
    /// the returned reference kind will determine whether there are only
    /// immutable or at least one mutable reference.
    pub fn borrow_kind_before(&self, temp: TempIndex) -> Option<ReferenceKind> {
        self.before.borrow_kind(temp)
    }

    /// Same as `borrow_kind_before` but after the given program point.
    pub fn borrow_kind_after(&self, temp: TempIndex) -> Option<ReferenceKind> {
        self.after.borrow_kind(temp)
    }

    /// Returns true if the given temporary is borrowed before or after the program point.
    pub fn is_borrowed(&self, temp: TempIndex) -> bool {
        self.borrow_kind_before(temp).is_some() || self.borrow_kind_after(temp).is_some()
    }
}

/// A trait to be implemented by reference safety processors
pub trait LifetimeInfo {
    fn borrow_kind(&self, temp: TempIndex) -> Option<ReferenceKind>;

    // For debugging
    fn display(&self, target: &FunctionTarget) -> Option<String>;
}

/// Needed to implement Default
struct NoLifetimeInfo();

impl LifetimeInfo for NoLifetimeInfo {
    fn borrow_kind(&self, _temp: TempIndex) -> Option<ReferenceKind> {
        None
    }

    fn display(&self, _target: &FunctionTarget) -> Option<String> {
        None
    }
}

/// Registers annotation formatter at the given function target. This is for debugging and
/// testing.
pub fn register_formatters(target: &FunctionTarget) {
    target.register_annotation_formatter(Box::new(format_lifetime_annotation))
}

fn format_lifetime_annotation(
    target: &FunctionTarget<'_>,
    code_offset: CodeOffset,
) -> Option<String> {
    if let Some(LifetimeAnnotation(map)) = target.get_annotations().get::<LifetimeAnnotation>() {
        map.get(&code_offset)
            .and_then(|info| info.before.display(target))
    } else {
        None
    }
}
