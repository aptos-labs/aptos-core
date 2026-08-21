// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    value_serde::ValueSerDeContext,
    values::{DepthDisplay, DeserializationSeed, SerializationReadyValue, VMValueCast, Value},
};
use better_any::Tid;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    function::{
        ClosureMask, FUNCTION_DATA_SERIALIZATION_FORMAT_V1, FUNCTION_DATA_SERIALIZATION_FORMAT_V2,
    },
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::{check_layout_within_bounds, MoveTypeLayout},
    vm_status::StatusCode,
};
use serde::{
    de::Error as DeError,
    ser::{Error, SerializeSeq, SerializeTuple},
    Deserialize, Serialize,
};
use std::{
    cell::RefCell,
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
    fn to_canonical_string(&self) -> String;

    /// Stored nesting depth of this closure's captured arguments. Returns
    ///  - zero if nothing is captured;
    ///  - `1 + max over captured args a of depth(a)`, where a serialized
    ///    captured closure contributes its own stored depth.
    fn captured_depth(&self) -> u16;
}

/// A closure, consisting of an abstract function descriptor and the captured arguments.
pub struct Closure(
    pub(crate) Box<dyn AbstractFunction>,
    pub(crate) Box<RefCell<ClosureCapturedArgs>>,
);

/// Captured arguments of a closure.
#[derive(Debug)]
pub enum ClosureCapturedArgs {
    /// Eagerly deserialized captured arguments.
    Deserialized(Vec<Value>),
    /// Not yet deserialized captured arguments, still stored as an opaque
    /// blob. Deserializing requires loading a function to obtain its type
    /// signature and layouts of values captured.
    Serialized(Vec<u8>),
}

/// Materializes closure's captured arguments. Semantics as follows:
///
///  - If closure carries deserialized captured arguments - a no-op.
///  - If closure carries serialized captured arguments:
///      a. Resolves the function by loading it, if not yet done.
///      b. Obtains the layouts for the captured arguments.
///      c. Uses the layouts to deserialize the captured blob.
pub trait ClosureCapturedArgsMaterializer {
    fn materialize_captured_args(&mut self, closure: &Closure) -> PartialVMResult<()>;
}

/// The representation of a function in storage.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SerializedFunctionData {
    pub module_id: ModuleId,
    pub fun_id: Identifier,
    pub ty_args: Vec<TypeTag>,
    pub mask: ClosureMask,
    /// Layouts of the captured arguments, when available: computed at pack time for
    /// storable closures, or read from storage for format V1. `None` for closures in
    /// storage format V2, whose layouts are derived from the function signature at
    /// materialization time.
    pub captured_layouts: Option<Vec<MoveTypeLayout>>,
    /// Nesting depth of the captured arguments: `0` if nothing is captured, else
    /// `1 + max over captured args of their value depth`, where a serialized
    /// captured closure contributes its own stored depth. Computed at pack time,
    /// read from the wire for format V2, or computed from the decoded values when
    /// converting format V1 to V2 on read. Lets depth checks see through an opaque
    /// captured blob without decoding it.
    pub captured_depth: u16,
}

impl Closure {
    pub fn pack(fun: Box<dyn AbstractFunction>, captured: impl IntoIterator<Item = Value>) -> Self {
        let captured = ClosureCapturedArgs::Deserialized(captured.into_iter().collect());
        Self(fun, Box::new(RefCell::new(captured)))
    }

    pub fn pack_serialized(fun: Box<dyn AbstractFunction>, blob: Vec<u8>) -> Self {
        Self(
            fun,
            Box::new(RefCell::new(ClosureCapturedArgs::Serialized(blob))),
        )
    }

    /// Consumes the closure, returning the function and the decoded captured
    /// arguments. The caller must materialize the closure first.
    pub fn unpack_decoded(self) -> PartialVMResult<(Box<dyn AbstractFunction>, Vec<Value>)> {
        let Self(fun, captured) = self;
        match RefCell::into_inner(*captured) {
            ClosureCapturedArgs::Deserialized(values) => Ok((fun, values)),
            ClosureCapturedArgs::Serialized(_) => Err(PartialVMError::new_invariant_violation(
                "cannot unpack closure with serialized captured arguments",
            )),
        }
    }

    pub fn function(&self) -> &dyn AbstractFunction {
        self.0.as_ref()
    }

    pub fn has_deserialized_captured_args(&self) -> bool {
        matches!(&*self.1.borrow(), ClosureCapturedArgs::Deserialized(_))
    }

    /// If the captured arguments are serialized, decodes them with `decode` and stores
    /// the result. No-op if already decoded.
    pub fn materialize_with(
        &self,
        decode: impl FnOnce(&[u8]) -> PartialVMResult<Vec<Value>>,
    ) -> PartialVMResult<()> {
        let decoded = match &*self.1.borrow() {
            ClosureCapturedArgs::Deserialized(_) => return Ok(()),
            ClosureCapturedArgs::Serialized(blob) => decode(blob)?,
        };
        *self.1.borrow_mut() = ClosureCapturedArgs::Deserialized(decoded);
        Ok(())
    }
}

impl Debug for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self(fun, captured_args) = self;
        let mut s = f.debug_struct("Closure");
        s.field("function", &fun.to_canonical_string())
            .field("closure_mask", &fun.closure_mask());
        match &*captured_args.borrow() {
            ClosureCapturedArgs::Serialized(blob) => {
                s.field("captured_blob_num_bytes", &blob.len())
            },
            ClosureCapturedArgs::Deserialized(captured) => s
                .field("captured_count", &captured.len())
                .field("captured_values", captured),
        }
        .finish()
    }
}

pub(crate) fn fmt_closure(c: &Closure, f: &mut Formatter<'_>, depth: usize) -> fmt::Result {
    let Closure(fun, captured_args) = c;
    let mask = fun.closure_mask();
    match &*captured_args.borrow() {
        ClosureCapturedArgs::Serialized(blob) => {
            // Never decode in Display: no gas can be charged here.
            write!(
                f,
                "{}(<{} captured args: serialized, {} bytes>)",
                fun.to_canonical_string(),
                mask.captured_count(),
                blob.len()
            )
        },
        ClosureCapturedArgs::Deserialized(captured) => {
            let captured = mask.format_arguments(
                captured
                    .iter()
                    .map(|v| DepthDisplay(v, depth + 1).to_string())
                    .collect(),
            );
            write!(f, "{}({})", fun.to_canonical_string(), captured.join(", "))
        },
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt_closure(self, f, 0)
    }
}

impl VMValueCast<Closure> for Value {
    fn cast(self) -> PartialVMResult<Closure> {
        match self {
            Value::ClosureValue(c) => Ok(c),
            v => Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("cannot cast {:?} to closure", v))),
        }
    }
}

/// Serializes captured values as a BCS tuple, i.e., the concatenation of the BCS
/// encodings of the values.
pub(crate) struct CapturedValuesForBlob<'c, 'b> {
    pub(crate) ctx: &'c ValueSerDeContext<'c>,
    pub(crate) layouts: &'b [MoveTypeLayout],
    pub(crate) values: &'b [Value],
    pub(crate) depth: u64,
}

impl serde::Serialize for CapturedValuesForBlob<'_, '_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut t = serializer.serialize_tuple(self.values.len())?;
        for (layout, value) in self.layouts.iter().zip(self.values.iter()) {
            t.serialize_element(&SerializationReadyValue {
                ctx: self.ctx,
                layout,
                value,
                depth: self.depth,
            })?;
        }
        t.end()
    }
}

/// Computes the stored nesting depth of a closure from its captured arguments:
/// `0` if nothing is captured, else `1 + max over captured args of their value
/// depth`, where a serialized captured closure contributes its own stored depth
/// (via `Closure::visit_impl`). `per_captured_limit` bounds each captured value's
/// depth; pass `u64::MAX` to compute without enforcing a limit, or the enforced
/// bound to reject a too-deep captured value with `VM_MAX_VALUE_DEPTH_REACHED`.
/// The result saturates to `u16`.
pub fn closure_captured_depth(captured: &[Value], per_captured_limit: u64) -> PartialVMResult<u16> {
    let mut max_child = 0u64;
    for value in captured {
        max_child = max_child.max(value.check_depth_of_value(per_captured_limit)?);
    }
    let depth = if captured.is_empty() {
        0
    } else {
        max_child.saturating_add(1)
    };
    Ok(depth.min(u16::MAX as u64) as u16)
}

fn serialize_v2<S: serde::Serializer>(
    serializer: S,
    data: &SerializedFunctionData,
    blob: &[u8],
) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(7))?;
    seq.serialize_element(&FUNCTION_DATA_SERIALIZATION_FORMAT_V2)?;
    seq.serialize_element(&data.module_id)?;
    seq.serialize_element(&data.fun_id)?;
    seq.serialize_element(&data.ty_args)?;
    seq.serialize_element(&data.mask)?;
    seq.serialize_element(&data.captured_depth)?;
    seq.serialize_element(blob)?;
    seq.end()
}

impl serde::Serialize for SerializationReadyValue<'_, '_, '_, (), Closure> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.ctx.closure_serialization_disabled {
            return Err(S::Error::custom(
                "serialization of function values is disabled",
            ));
        }
        let Closure(fun, captured_args) = self.value;
        let fun_ext = self
            .ctx
            .required_function_extension()
            .map_err(S::Error::custom)?;
        let data = fun_ext
            .get_serialization_data(fun.as_ref())
            .map_err(S::Error::custom)?;
        match &*captured_args.borrow() {
            ClosureCapturedArgs::Serialized(blob) => {
                // The captured arguments were never decoded, so the bytes are still a
                // valid encoding: write them verbatim, always in V2.
                serialize_v2(serializer, &data, blob)
            },
            ClosureCapturedArgs::Deserialized(captured) => {
                let layouts = data.captured_layouts.as_ref().ok_or_else(|| {
                    S::Error::custom("captured layouts must be available for storable closures")
                })?;
                if layouts.len() != captured.len() {
                    return Err(S::Error::custom(
                        "captured layouts and values count mismatch",
                    ));
                }
                if fun_ext.is_function_data_format_v2_enabled() {
                    let blob = bcs::to_bytes(&CapturedValuesForBlob {
                        ctx: self.ctx,
                        layouts,
                        values: captured,
                        depth: self.depth + 1,
                    })
                    .map_err(S::Error::custom)?;
                    serialize_v2(serializer, &data, &blob)
                } else {
                    let mut seq = serializer.serialize_seq(Some(5 + captured.len() * 2))?;
                    seq.serialize_element(&FUNCTION_DATA_SERIALIZATION_FORMAT_V1)?;
                    seq.serialize_element(&data.module_id)?;
                    seq.serialize_element(&data.fun_id)?;
                    seq.serialize_element(&data.ty_args)?;
                    seq.serialize_element(&data.mask)?;
                    for (layout, value) in layouts.iter().zip(captured.iter()) {
                        // Reject captured layouts whose DAG would unfold past the
                        // node-count cap.
                        check_layout_within_bounds::<S::Error>(layout)?;
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
            },
        }
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
        let module_id = read_required_value::<_, ModuleId>(&mut seq)?;
        let fun_id = read_required_value::<_, Identifier>(&mut seq)?;
        let ty_args = read_required_value::<_, Vec<TypeTag>>(&mut seq)?;
        let mask = read_required_value::<_, ClosureMask>(&mut seq)?;
        let num_captured = mask.captured_count() as usize;

        let (depth, captured_layouts, captured_args) = match format_version {
            FUNCTION_DATA_SERIALIZATION_FORMAT_V1 => {
                let mut captured_layouts = Vec::with_capacity(num_captured);
                let mut captured = Vec::with_capacity(num_captured);
                for _ in 0..num_captured {
                    let layout = read_required_value::<_, MoveTypeLayout>(&mut seq)?;
                    match seq.next_element_seed(DeserializationSeed {
                        ctx: self.0.ctx,
                        layout: &layout,
                    })? {
                        Some(v) => {
                            captured_layouts.push(layout);
                            captured.push(v)
                        },
                        None => return Err(A::Error::invalid_length(captured.len(), &self)),
                    }
                }
                // Format V1 does not store depth; compute it from the decoded values.
                // No limit is enforced here (reads must not fail on already-stored
                // data); the depth is only recorded for later checks.
                let depth =
                    closure_captured_depth(&captured, u64::MAX).map_err(A::Error::custom)?;
                if fun_ext.is_function_data_format_v2_enabled() {
                    // Convert to the V2 in-memory form: re-encode the captured values
                    // into a blob and drop the layouts. V1 data then behaves exactly
                    // like V2 data, and re-serializes as V2.
                    let blob = bcs::to_bytes(&CapturedValuesForBlob {
                        ctx: self.0.ctx,
                        layouts: &captured_layouts,
                        values: &captured,
                        depth: 1,
                    })
                    .map_err(A::Error::custom)?;
                    (depth, None, ClosureCapturedArgs::Serialized(blob))
                } else {
                    (
                        depth,
                        Some(captured_layouts),
                        ClosureCapturedArgs::Deserialized(captured),
                    )
                }
            },
            FUNCTION_DATA_SERIALIZATION_FORMAT_V2 => {
                if !self.0.ctx.allow_function_values_v2_reads {
                    return Err(A::Error::custom("function data format V2 is not enabled"));
                }
                let depth = read_required_value::<_, u16>(&mut seq)?;
                let blob = read_required_value::<_, Vec<u8>>(&mut seq)?;
                // Each captured value takes at least one byte.
                if blob.len() < num_captured {
                    return Err(A::Error::custom("captured blob is too short"));
                }
                (depth, None, ClosureCapturedArgs::Serialized(blob))
            },
            v => {
                return Err(A::Error::custom(format!(
                    "invalid function data version {}",
                    v
                )))
            },
        };

        // If the sequence length is known, check whether there are no extra values
        if matches!(seq.size_hint(), Some(remaining) if remaining != 0) {
            return Err(A::Error::invalid_length(num_captured, &self));
        }
        let fun = fun_ext
            .create_from_serialization_data(SerializedFunctionData {
                module_id,
                fun_id,
                ty_args,
                mask,
                captured_depth: depth,
                captured_layouts,
            })
            .map_err(A::Error::custom)?;
        Ok(Closure(fun, Box::new(RefCell::new(captured_args))))
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
        function::ClosureMask,
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
                    module_id: ModuleId::new(AccountAddress::TWO, Identifier::new("m").unwrap()),
                    fun_id: Identifier::new(fun_name).unwrap(),
                    ty_args,
                    mask,
                    captured_depth: 0,
                    captured_layouts: Some(captured_layouts),
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

        fn clone_dyn(&self) -> PartialVMResult<Box<dyn AbstractFunction>> {
            Ok(Box::new(self.clone()))
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

        fn captured_depth(&self) -> u16 {
            self.data.captured_depth
        }
    }
}
