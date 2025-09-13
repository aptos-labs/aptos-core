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
    sync::Arc,
};

/// A trait describing a function which can be executed. If this is a generic
/// function, the type instantiation is part of this.
/// The value system is agnostic about how this is implemented in the runtime.
/// The `FunctionValueExtension` trait describes how to construct and
/// deconstruct instances for serialization.
pub trait AbstractFunction: Send + Sync + for<'a> Tid<'a> {
    fn closure_mask(&self) -> ClosureMask;
    fn cmp_dyn(&self, other: &dyn AbstractFunction) -> PartialVMResult<Ordering>;
    fn clone_dyn(&self) -> PartialVMResult<Arc<dyn AbstractFunction>>;
    fn to_canonical_string(&self) -> String;
}

/// A closure, consisting of an abstract function descriptor and the captured arguments.
pub struct Closure(
    pub(crate) Arc<dyn AbstractFunction>,
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
    pub captured_layouts: Arc<Vec<MoveTypeLayout>>,
}

impl Closure {
    pub fn pack(fun: Arc<dyn AbstractFunction>, captured: impl IntoIterator<Item = Value>) -> Self {
        Self(fun, captured.into_iter().map(|v| v.0).collect())
    }

    pub fn unpack(self) -> (Arc<dyn AbstractFunction>, impl Iterator<Item = Value>) {
        let Self(fun, captured) = self;
        (fun, captured.into_iter().map(Value))
    }

    pub fn into_call_data(
        self,
        args: Vec<Value>,
    ) -> PartialVMResult<(Arc<dyn AbstractFunction>, Vec<Value>)> {
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
        let mask = fun.closure_mask();

        f.debug_struct("Closure")
            .field("function", &fun.to_canonical_string())
            .field("closure_mask", &mask)
            .field("captured_count", &captured.len())
            .field("captured_values", captured)
            .finish()
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self(fun, captured) = self;
        let captured = fun
            .closure_mask()
            .format_arguments(captured.iter().map(|v| v.to_string()).collect());
        write!(f, "{}({})", fun.to_canonical_string(), captured.join(", "))
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
        for (layout, value) in data.captured_layouts.iter().zip(captured) {
            seq.serialize_element(layout)?;
            seq.serialize_element(&SerializationReadyValue {
                ctx: self.ctx,
                layout,
                value,
                depth: self.depth + 1,
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
                captured_layouts: Arc::new(captured_layouts),
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

/// Mock AbstractFunction for testing
/// Value:closure(AbstractFunction, [Value]) requires an AbstractFunction, which is agnostic from runtime implementation.
/// This mock is used to test the function values system.
#[cfg(any(test, feature = "fuzzing", feature = "testing"))]
pub(crate) mod mock {
    use super::*;
    use better_any::{Tid, TidAble, TidExt};
    use move_binary_format::errors::PartialVMResult;
    use move_core_types::{
        account_address::AccountAddress,
        function::{ClosureMask, FUNCTION_DATA_SERIALIZATION_FORMAT_V1},
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
        value::MoveTypeLayout,
    };
    use std::cmp::Ordering;

    // Since Abstract functions are `Tid`, we cannot auto-mock them, so need to mock manually.
    #[derive(Clone, Tid)]
    pub(crate) struct MockAbstractFunction {
        pub(crate) data: SerializedFunctionData,
    }

    impl MockAbstractFunction {
        #[allow(dead_code)]
        pub(crate) fn new(
            fun_name: &str,
            ty_args: Vec<TypeTag>,
            mask: ClosureMask,
            captured_layouts: Vec<MoveTypeLayout>,
        ) -> MockAbstractFunction {
            Self {
                data: SerializedFunctionData {
                    format_version: FUNCTION_DATA_SERIALIZATION_FORMAT_V1,
                    module_id: ModuleId::new(AccountAddress::TWO, Identifier::new("m").unwrap()),
                    fun_id: Identifier::new(fun_name).unwrap(),
                    ty_args,
                    mask,
                    captured_layouts: Arc::new(captured_layouts),
                },
            }
        }

        #[allow(dead_code)]
        pub(crate) fn new_from_data(data: SerializedFunctionData) -> Self {
            Self { data }
        }
    }

    impl AbstractFunction for MockAbstractFunction {
        fn closure_mask(&self) -> ClosureMask {
            self.data.mask
        }

        fn cmp_dyn(&self, other: &dyn AbstractFunction) -> PartialVMResult<Ordering> {
            // We only need equality for tests
            let other_mock = other.downcast_ref::<MockAbstractFunction>().unwrap();
            Ok(if self.data == other_mock.data {
                Ordering::Equal
            } else {
                Ordering::Less
            })
        }

        fn clone_dyn(&self) -> PartialVMResult<Arc<dyn AbstractFunction>> {
            // Didn't need it in the test
            unimplemented!("clone_dyn is not implemented for MockAbstractFunction")
        }

        fn to_canonical_string(&self) -> String {
            // Needed for assertion failure printing
            let ty_args_str = if self.data.ty_args.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    self.data
                        .ty_args
                        .iter()
                        .map(|t| t.to_canonical_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            format!(
                "{}::{}::{}{}",
                self.data.module_id.address(),
                self.data.module_id.name(),
                self.data.fun_id,
                ty_args_str
            )
        }
    }
}
