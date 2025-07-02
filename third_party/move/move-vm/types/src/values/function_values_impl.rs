// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    values::{
        Container, DeserializationSeed, SerializationReadyValue, VMValueCast, Value, ValueImpl,
    },
    views::ValueVisitor,
};
use better_any::Tid;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    function::{ClosureMask, FUNCTION_DATA_SERIALIZATION_FORMAT_V1},
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use serde::{
    de::Error as DeError,
    ser::{Error, SerializeSeq},
    Deserialize, Serialize,
};
use std::{
    cell::RefCell,
    cmp::Ordering,
    fmt,
    fmt::{Debug, Display, Formatter},
    rc::Rc,
};

/// A trait describing a function which can be executed. If this is a generic
/// function, the type instantiation is part of this.
/// The value system is agnostic about how this is implemented in the runtime.
/// The `FunctionValueExtension` trait describes how to construct and
/// deconstruct instances for serialization.
pub trait AbstractFunction: for<'a> Tid<'a> {
    fn closure_mask(&self) -> ClosureMask;
    fn cmp_dyn(&self, other: &dyn AbstractFunction) -> PartialVMResult<Ordering>;
    fn clone_dyn(&self) -> PartialVMResult<Box<dyn AbstractFunction>>;
    fn to_canonical_string(&self) -> String;
}

/// A closure, consisting of an abstract function descriptor and the captured arguments.
pub struct Closure {
    fun: Box<dyn AbstractFunction>,
    // Note: captured arguments are never shared, but we store them behind a shared pointer so they
    // can be traversed in the same manner as containers.
    captured_arguments: Rc<RefCell<Vec<ValueImpl>>>,
}

/// The representation of a function in storage.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SerializedFunctionData {
    pub format_version: u16,
    pub module_id: ModuleId,
    pub fun_id: Identifier,
    pub ty_args: Vec<TypeTag>,
    pub mask: ClosureMask,
    /// The layouts used for deserialization of the captured arguments
    /// are stored so one can verify type consistency at
    /// resolution time. It also allows to serialize an unresolved
    /// closure, making unused closure data cheap in round trips.
    pub captured_layouts: Vec<MoveTypeLayout>,
}

impl Closure {
    pub fn pack(fun: Box<dyn AbstractFunction>, captured: impl IntoIterator<Item = Value>) -> Self {
        let captured_arguments = captured.into_iter().map(|v| v.0).collect();
        Self {
            fun,
            captured_arguments: Rc::new(RefCell::new(captured_arguments)),
        }
    }

    pub(crate) fn copy_value(&self) -> PartialVMResult<Self> {
        let captured_arguments = self
            .captured_arguments
            .borrow()
            .iter()
            .map(|v| v.copy_value())
            .collect::<PartialVMResult<_>>()?;
        Ok(Closure {
            fun: self.fun.clone_dyn()?,
            captured_arguments: Rc::new(RefCell::new(captured_arguments)),
        })
    }

    pub(crate) fn equals(&self, other: &Self) -> PartialVMResult<bool> {
        let captured_arguments = self.captured_arguments.borrow();
        let other_captured_arguments = other.captured_arguments.borrow();

        let equal = if self.fun.cmp_dyn(other.fun.as_ref())? == Ordering::Equal
            && captured_arguments.len() == other_captured_arguments.len()
        {
            for (v1, v2) in captured_arguments
                .iter()
                .zip(other_captured_arguments.iter())
            {
                if !v1.equals(v2)? {
                    return Ok(false);
                }
            }
            true
        } else {
            false
        };
        Ok(equal)
    }

    pub(crate) fn compare(&self, other: &Self) -> PartialVMResult<Ordering> {
        let captured_arguments = self.captured_arguments.borrow();
        let other_captured_arguments = other.captured_arguments.borrow();

        let o = self.fun.cmp_dyn(other.fun.as_ref())?;
        Ok(if o == Ordering::Equal {
            for (v1, v2) in captured_arguments
                .iter()
                .zip(other_captured_arguments.iter())
            {
                let o = v1.compare(v2)?;
                if o != Ordering::Equal {
                    return Ok(o);
                }
            }
            captured_arguments
                .iter()
                .len()
                .cmp(&other_captured_arguments.len())
        } else {
            o
        })
    }

    pub fn unpack(self) -> (Box<dyn AbstractFunction>, impl Iterator<Item = Value>) {
        let Self {
            fun,
            captured_arguments,
        } = self;
        let captured_arguments = Rc::into_inner(captured_arguments)
            .expect("Captured arguments are never shared")
            .into_inner();
        (fun, captured_arguments.into_iter().map(Value))
    }

    pub fn into_call_data(
        self,
        args: Vec<Value>,
    ) -> PartialVMResult<(Box<dyn AbstractFunction>, Vec<Value>)> {
        let (fun, captured) = self.unpack();
        if let Some(all_args) = fun.closure_mask().compose(captured, args) {
            Ok((fun, all_args))
        } else {
            Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("invalid closure mask".to_string()),
            )
        }
    }

    pub(crate) fn visit_impl(&self, visitor: &mut impl ValueVisitor, depth: usize) {
        let captured_arguments = self.captured_arguments.borrow();
        if visitor.visit_closure(depth, captured_arguments.len()) {
            for val in captured_arguments.iter() {
                val.visit_impl(visitor, depth + 1);
            }
        }
    }
}

impl Debug for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Closure({}, {:?})",
            self.fun.to_canonical_string(),
            self.captured_arguments.borrow(),
        )
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let captured = self.fun.closure_mask().format_arguments(
            self.captured_arguments
                .borrow()
                .iter()
                .map(|v| v.to_string())
                .collect(),
        );
        write!(
            f,
            "{}({})",
            self.fun.to_canonical_string(),
            captured.join(", ")
        )
    }
}

impl VMValueCast<Closure> for Value {
    fn cast(self) -> PartialVMResult<Closure> {
        match self.0 {
            ValueImpl::ClosureValue(c) => Ok(c),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to closure", v))),
        }
    }
}

impl serde::Serialize for SerializationReadyValue<'_, '_, '_, (), Closure> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Closure {
            fun,
            captured_arguments,
        } = self.value;
        let captured_arguments = captured_arguments.borrow();

        let fun_ext = self
            .ctx
            .required_function_extension()
            .map_err(S::Error::custom)?;
        let data = fun_ext
            .get_serialization_data(fun.as_ref())
            .map_err(S::Error::custom)?;
        let mut seq = serializer.serialize_seq(Some(5 + captured_arguments.len() * 2))?;
        seq.serialize_element(&data.format_version)?;
        seq.serialize_element(&data.module_id)?;
        seq.serialize_element(&data.fun_id)?;
        seq.serialize_element(&data.ty_args)?;
        seq.serialize_element(&data.mask)?;
        for (layout, value) in data
            .captured_layouts
            .into_iter()
            .zip(captured_arguments.iter())
        {
            seq.serialize_element(&layout)?;
            seq.serialize_element(&SerializationReadyValue {
                ctx: self.ctx,
                layout: &layout,
                value,
            })?
        }
        seq.end()
    }
}

pub(crate) struct ClosureVisitor<'c>(pub(crate) DeserializationSeed<'c, ()>);

impl<'d, 'c> serde::de::Visitor<'d> for ClosureVisitor<'c> {
    type Value = Closure;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Closure")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let fun_ext = self
            .0
            .ctx
            .required_function_extension()
            .map_err(A::Error::custom)?;
        let format_version = read_required_value::<_, u16>(&mut seq)?;
        if format_version != FUNCTION_DATA_SERIALIZATION_FORMAT_V1 {
            return Err(A::Error::custom(format!(
                "invalid function data version {}",
                format_version
            )));
        }
        let module_id = read_required_value::<_, ModuleId>(&mut seq)?;
        let fun_id = read_required_value::<_, Identifier>(&mut seq)?;
        let ty_args = read_required_value::<_, Vec<TypeTag>>(&mut seq)?;
        let mask = read_required_value::<_, ClosureMask>(&mut seq)?;

        let num_captured_values = mask.captured_count() as usize;
        let mut captured_layouts = Vec::with_capacity(num_captured_values);
        let mut captured_arguments = Vec::with_capacity(num_captured_values);
        for _ in 0..num_captured_values {
            let layout = read_required_value::<_, MoveTypeLayout>(&mut seq)?;
            match seq.next_element_seed(DeserializationSeed {
                ctx: self.0.ctx,
                layout: &layout,
            })? {
                Some(v) => {
                    captured_layouts.push(layout);
                    captured_arguments.push(v.0)
                },
                None => return Err(A::Error::invalid_length(captured_arguments.len(), &self)),
            }
        }
        // If the sequence length is known, check whether there are no extra values
        if matches!(seq.size_hint(), Some(remaining) if remaining != 0) {
            return Err(A::Error::invalid_length(captured_arguments.len(), &self));
        }
        let fun = fun_ext
            .create_from_serialization_data(SerializedFunctionData {
                format_version: FUNCTION_DATA_SERIALIZATION_FORMAT_V1,
                module_id,
                fun_id,
                ty_args,
                mask,
                captured_layouts,
            })
            .map_err(A::Error::custom)?;
        Ok(Closure {
            fun,
            captured_arguments: Rc::new(RefCell::new(captured_arguments)),
        })
    }
}

fn read_required_value<'a, A, T>(seq: &mut A) -> Result<T, A::Error>
where
    A: serde::de::SeqAccess<'a>,
    T: serde::de::Deserialize<'a>,
{
    match seq.next_element::<T>()? {
        Some(x) => Ok(x),
        None => Err(A::Error::custom("expected more elements")),
    }
}

/// A pre-order traversal over the value tree. For each node, its depth is tracked. In addition,
/// callers can provide a callback to perform actions over each node.
///
/// INVARIANT: the traversal does not traverse reference fields. When encountering a reference, it
/// is returned to the caller to handle via callback.
pub(crate) fn walk_preorder<F, E>(value: &ValueImpl, mut visit: F) -> Result<(), E>
where
    F: FnMut(&ValueImpl, u64) -> Result<(), E>,
{
    /// Work items kept on the explicit stack.
    enum Item<'v> {
        /// A node we already own by shared reference.
        Direct { value: &'v ValueImpl, depth: u64 },
        /// An element that lives inside a vector behind a shared pointer.
        Element {
            rc: Rc<RefCell<Vec<ValueImpl>>>,
            idx: usize,
            depth: u64,
        },
    }

    fn push(value: &ValueImpl, depth: u64, stack: &mut Vec<Item>) {
        use ValueImpl as V;
        match value {
            // Leaf nodes:
            V::Invalid
            | V::U8(_)
            | V::U16(_)
            | V::U32(_)
            | V::U64(_)
            | V::U128(_)
            | V::U256(_)
            | V::Bool(_)
            | V::Address(_)
            | V::DelayedFieldID { .. } => (),

            // References are consider to be leaves as well. No need to push anything.
            V::ContainerRef(_) | V::IndexedRef(_) => (),

            ValueImpl::ClosureValue(closure) => {
                let len = closure.captured_arguments.borrow().len();
                for idx in (0..len).rev() {
                    stack.push(Item::Element {
                        rc: Rc::clone(&closure.captured_arguments),
                        idx,
                        depth: depth + 1,
                    });
                }
            },

            ValueImpl::Container(container) => {
                use Container::*;
                match container {
                    Locals(rc) | Vec(rc) | Struct(rc) => {
                        let len = rc.borrow().len();
                        for idx in (0..len).rev() {
                            stack.push(Item::Element {
                                rc: Rc::clone(rc),
                                idx,
                                depth: depth + 1,
                            });
                        }
                    },
                    // These are leaves. No need to push.
                    VecU8(_) | VecU64(_) | VecU128(_) | VecBool(_) | VecAddress(_) | VecU16(_)
                    | VecU32(_) | VecU256(_) => (),
                }
            },
        }
    }

    let mut stack = vec![Item::Direct { value, depth: 1 }];

    while let Some(item) = stack.pop() {
        match item {
            Item::Direct { value, depth } => {
                visit(value, depth)?;
                push(value, depth, &mut stack);
            },

            Item::Element { rc, idx, depth } => {
                let guard = rc.borrow();
                let value = &guard[idx];
                visit(value, depth)?;
                push(value, depth, &mut stack);
            },
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // cuse super::*;

    #[test]
    fn test() {}
}
