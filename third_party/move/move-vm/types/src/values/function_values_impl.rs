// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::values::{DeserializationSeed, SerializationReadyValue, VMValueCast, Value, ValueImpl};
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
    cmp::Ordering,
    fmt,
    fmt::{Debug, Display, Formatter},
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
    fn to_stable_string(&self) -> String;
}

/// A closure, consisting of an abstract function descriptor and the captured arguments.
pub struct Closure(
    pub(crate) Box<dyn AbstractFunction>,
    pub(crate) Vec<ValueImpl>,
);

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
        Self(fun, captured.into_iter().map(|v| v.0).collect())
    }

    pub fn unpack(self) -> (Box<dyn AbstractFunction>, impl Iterator<Item = Value>) {
        let Self(fun, captured) = self;
        (fun, captured.into_iter().map(Value))
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
}

impl Debug for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self(fun, captured) = self;
        write!(f, "Closure({}, {:?})", fun.to_stable_string(), captured)
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self(fun, captured) = self;
        let captured = fun
            .closure_mask()
            .format_arguments(captured.iter().map(|v| v.to_string()).collect());
        write!(f, "{}({})", fun.to_stable_string(), captured.join(", "))
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
        let Closure(fun, captured) = self.value;
        let fun_ext = self
            .ctx
            .required_function_extension()
            .map_err(S::Error::custom)?;
        let data = fun_ext
            .get_serialization_data(fun.as_ref())
            .map_err(S::Error::custom)?;
        let mut seq = serializer.serialize_seq(Some(5 + captured.len() * 2))?;
        seq.serialize_element(&data.format_version)?;
        seq.serialize_element(&data.module_id)?;
        seq.serialize_element(&data.fun_id)?;
        seq.serialize_element(&data.ty_args)?;
        seq.serialize_element(&data.mask)?;
        for (layout, value) in data.captured_layouts.into_iter().zip(captured) {
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
        let mut captured = Vec::with_capacity(num_captured_values);
        for _ in 0..num_captured_values {
            let layout = read_required_value::<_, MoveTypeLayout>(&mut seq)?;
            match seq.next_element_seed(DeserializationSeed {
                ctx: self.0.ctx,
                layout: &layout,
            })? {
                Some(v) => {
                    captured_layouts.push(layout);
                    captured.push(v.0)
                },
                None => return Err(A::Error::invalid_length(captured.len(), &self)),
            }
        }
        // If the sequence length is known, check whether there are no extra values
        if matches!(seq.size_hint(), Some(remaining) if remaining != 0) {
            return Err(A::Error::invalid_length(captured.len(), &self));
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
        Ok(Closure(fun, captured))
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
