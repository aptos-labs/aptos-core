// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// `derive(Dearbitrary)` generates unused-variable and unused-assignment warnings for
// the named-field `MoveClosureCapturedArgs::Serialized` variant that can only be
// silenced at the module level (see the same allow in `value.rs`). The derive only
// exists under `test`/`fuzzing`, so gate the allow to those configs and keep the
// lints active for production builds of this module.
#![cfg_attr(
    any(test, feature = "fuzzing"),
    allow(unused_variables, unused_assignments)
)]

use crate::{
    ability::AbilitySet,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::{MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue},
};
use anyhow::bail;
use serde::{de::Error, ser::SerializeSeq, Deserialize, Serialize};
use std::fmt;

/// Version number for the serialization format of function data.
pub const FUNCTION_DATA_SERIALIZATION_FORMAT_V1: u16 = 1;

/// Version number for the V2 serialization format of function data. In V2, captured
/// arguments are stored as a single opaque blob without layouts.
pub const FUNCTION_DATA_SERIALIZATION_FORMAT_V2: u16 = 2;

//===========================================================================================

/// A `ClosureMask` is a value which determines how to distinguish those function arguments
/// which are captured and which are not when a closure is constructed. For instance,
/// with `_` representing an omitted argument, the mask for `f(a,_,b,_)` would have the argument
/// at index 0 and at index 2 captured. The mask can be used to transform lists of types.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary),
    derive(dearbitrary::Dearbitrary)
)]
pub struct ClosureMask(u64);

impl fmt::Display for ClosureMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl ClosureMask {
    /// The maximal number of arguments which can be handled by a closure mask.
    /// A captured argument's position in the argument list must be lower than
    /// this number. Notice that this property is implicit in the bytecode:
    /// a PACK_CLOSURE instruction will never pop more arguments from the
    /// stack than this number.
    pub const MAX_ARGS: usize = 64;

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn new(mask: u64) -> Self {
        Self(mask)
    }

    pub fn new_for_leading(n: usize) -> Result<Self, String> {
        let mut mask = Self::new(0);
        for i in 0..n {
            mask.set_captured(i)?;
        }
        Ok(mask)
    }

    pub fn bits(&self) -> u64 {
        self.0
    }

    /// Returns true if the i'th argument is captured. If `i` is out of range, false will
    /// be returned.
    #[inline(always)]
    pub fn is_captured(&self, i: usize) -> bool {
        i < Self::MAX_ARGS && self.0 & (1 << i) != 0
    }

    /// Sets the ith argument to be captured
    pub fn set_captured(&mut self, i: usize) -> Result<(), String> {
        if i >= Self::MAX_ARGS {
            return Err(format!(
                "Captured argument index {} exceeds maximum allowed captured arguments {}",
                i,
                Self::MAX_ARGS
            ));
        }
        self.0 |= 1 << i;
        Ok(())
    }

    /// Apply a closure mask to a list of elements, returning only those
    /// where position `i` is set in the mask (if `collect_captured` is true) or not
    /// set (otherwise).
    pub fn extract<'a, T>(
        &self,
        values: impl IntoIterator<Item = &'a T>,
        collect_captured: bool,
    ) -> Vec<&'a T> {
        let mut mask = self.0;
        values
            .into_iter()
            .filter(|_| {
                let set = mask & 0x1 != 0;
                mask >>= 1;
                set && collect_captured || !set && !collect_captured
            })
            .collect()
    }

    /// Compose two lists of elements into one based on the given mask such that the
    /// following holds:
    /// ```ignore
    ///   mask.compose(mask.extract(v, true), mask.extract(v, false)) == v
    /// ```
    /// This returns `None` if the provided lists are inconsistent w.r.t the mask
    /// and cannot be composed. This should not happen in verified code, but
    /// a caller should decide whether to crash or to error.
    pub fn compose<T>(
        &self,
        captured: impl IntoIterator<Item = T>,
        provided: impl IntoIterator<Item = T>,
    ) -> Option<Vec<T>> {
        let mut captured = captured.into_iter();
        let mut provided = provided.into_iter();
        let mut result = vec![];
        let mut mask = self.0;
        while mask != 0 {
            if mask & 0x1 != 0 {
                result.push(captured.next()?)
            } else {
                result.push(provided.next()?)
            }
            mask >>= 1;
        }
        if captured.next().is_some() {
            // Not all captured arguments consumed
            return None;
        }
        result.extend(provided);
        Some(result)
    }

    /// Return the max index of captured argument, or None if none is captured.
    pub fn max_captured(&self) -> Option<usize> {
        if self.0 == 0 {
            return None;
        }
        let mut i = 0;
        let mut mask = self.0;
        loop {
            mask >>= 1;
            if mask == 0 {
                return Some(i);
            }
            i += 1
        }
    }

    /// Return the # of captured arguments in the mask
    pub fn captured_count(&self) -> u16 {
        let mut i = 0;
        let mut mask = self.0;
        while mask != 0 {
            if mask & 0x1 != 0 {
                i += 1
            }
            mask >>= 1;
        }
        i
    }

    /// Given a vector of captured arguments (converted into strings), formats them as a vector of
    /// all arguments such that:
    ///   - if argument is captured, an entry from the vector is added to the final vector,
    ///   - if argument is not captured, "_" is added to the final vector.
    /// The last element of a vector is "..", indicating possibly mor non-captured arguments (it is
    /// not possible to deduce if there are any because the mask is simply 0).
    ///
    /// In case there is any error, a vector of a single dummy value is returned.
    pub fn format_arguments(&self, captured: Vec<String>) -> Vec<String> {
        // If the function returns None, this means not all arguments were captured. Should not
        // happen. Do not return an error because this is used to implement `Display`, which can
        // make `format!` panic.
        self.format_arguments_impl(captured)
            .unwrap_or_else(|| vec!["*invalid*".to_string()])
    }

    fn format_arguments_impl(&self, captured: Vec<String>) -> Option<Vec<String>> {
        let mut mask = self.0;
        let mut captured = captured.into_iter();

        let mut result = vec![];
        while mask != 0 {
            if mask & 0x1 != 0 {
                result.push(captured.next()?)
            } else {
                result.push("_".to_string())
            }
            mask >>= 1;
        }

        // We do not know arity information of the function, so the simplest option is to indicate
        // that there can be more arguments in the end.
        result.push("..".to_string());

        if captured.next().is_some() {
            return None;
        }

        Some(result)
    }
}

#[cfg(test)]
mod closure_mask_tests {
    use super::*;

    #[test]
    fn extract_compose_roundtrip_test() {
        // mask.compose(mask.extract(v, true), mask.extract(v, false)) == v
        let mask = ClosureMask::new(0b101011);
        let v = vec![1, 2, 3, 4, 5, 6];
        let captured = mask.extract(&v, true);
        assert_eq!(captured, vec![&1, &2, &4, &6]);
        let not_captured = mask.extract(&v, false);
        assert_eq!(not_captured, vec![&3, &5]);
        assert_eq!(
            v,
            mask.compose(captured, not_captured)
                .expect("composition must succeed")
                .into_iter()
                .copied()
                .collect::<Vec<_>>()
        );
    }
}

//===========================================================================================

/// Function type layout, with arguments and result types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct MoveFunctionLayout(
    pub Vec<MoveTypeLayout>,
    pub Vec<MoveTypeLayout>,
    pub AbilitySet,
);

/// Captured arguments of a closure, in one of the two serialization formats.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum MoveClosureCapturedArgs {
    /// Eagerly deserialized captured arguments and their layouts (used to
    /// guide the deserialization).
    Deserialized(Vec<(MoveTypeLayout, MoveValue)>),
    /// Concatenated BCS of the captured values, with their cached nesting depth
    /// (see `SerializedFunctionData::depth` on the VM side).
    Serialized { depth: u16, blob: Vec<u8> },
}

/// A closure (function value). The closure stores the name of the function and its
/// type instantiation, as well as the closure mask and the captured arguments.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct MoveClosure {
    pub module_id: ModuleId,
    pub fun_id: Identifier,
    pub ty_args: Vec<TypeTag>,
    pub mask: ClosureMask,
    pub captured: MoveClosureCapturedArgs,
}

impl MoveClosure {
    /// Decodes a V2 captured blob into values, given the layouts of the captured
    /// arguments (derived from the function signature). Fails if the values do not
    /// match the layouts or the blob has trailing bytes.
    pub fn deserialize_captured(
        blob: &[u8],
        layouts: Vec<MoveTypeLayout>,
    ) -> anyhow::Result<Vec<MoveValue>> {
        // A concatenation of BCS values is exactly the BCS encoding of a struct with
        // those field layouts.
        let layout = MoveTypeLayout::new_struct(MoveStructLayout::Runtime(layouts));
        match MoveValue::simple_deserialize(blob, &layout)? {
            MoveValue::Struct(MoveStruct::Runtime(values)) => Ok(values),
            _ => bail!("expected runtime struct when decoding captured arguments"),
        }
    }
}

pub(crate) struct ClosureVisitor;

impl<'d> serde::de::Visitor<'d> for ClosureVisitor {
    type Value = MoveClosure;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Closure")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let version = read_required_value::<_, u16>(&mut seq)?;
        let module_id = read_required_value::<_, ModuleId>(&mut seq)?;
        let fun_id = read_required_value::<_, Identifier>(&mut seq)?;
        let ty_args = read_required_value::<_, Vec<TypeTag>>(&mut seq)?;
        let mask = read_required_value::<_, ClosureMask>(&mut seq)?;
        let num_captured = mask.captured_count() as usize;
        let captured = match version {
            FUNCTION_DATA_SERIALIZATION_FORMAT_V1 => {
                let mut captured = vec![];
                for _ in 0..num_captured {
                    let layout = read_required_value::<_, MoveTypeLayout>(&mut seq)?;
                    match seq.next_element_seed(&layout)? {
                        Some(v) => captured.push((layout, v)),
                        None => return Err(A::Error::invalid_length(captured.len(), &self)),
                    }
                }
                MoveClosureCapturedArgs::Deserialized(captured)
            },
            FUNCTION_DATA_SERIALIZATION_FORMAT_V2 => {
                let depth = read_required_value::<_, u16>(&mut seq)?;
                let blob = read_required_value::<_, Vec<u8>>(&mut seq)?;
                // Each captured value takes at least one byte.
                if blob.len() < num_captured {
                    return Err(A::Error::custom("captured blob is too short"));
                }
                MoveClosureCapturedArgs::Serialized { depth, blob }
            },
            _ => {
                return Err(A::Error::custom(format!(
                    "unexpected function data version {}",
                    version
                )))
            },
        };
        // If the sequence length is known, check whether there are no extra values
        if matches!(seq.size_hint(), Some(remaining) if remaining != 0) {
            return Err(A::Error::invalid_length(num_captured, &self));
        }
        Ok(MoveClosure {
            module_id,
            fun_id,
            ty_args,
            mask,
            captured,
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

impl serde::Serialize for MoveClosure {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let MoveClosure {
            module_id,
            fun_id,
            ty_args,
            mask,
            captured,
        } = self;
        match captured {
            MoveClosureCapturedArgs::Deserialized(captured) => {
                let mut s = serializer.serialize_seq(Some(5 + captured.len() * 2))?;
                s.serialize_element(&FUNCTION_DATA_SERIALIZATION_FORMAT_V1)?;
                s.serialize_element(module_id)?;
                s.serialize_element(fun_id)?;
                s.serialize_element(ty_args)?;
                s.serialize_element(mask)?;
                for (l, v) in captured {
                    s.serialize_element(l)?;
                    s.serialize_element(v)?;
                }
                s.end()
            },
            MoveClosureCapturedArgs::Serialized { depth, blob } => {
                let mut s = serializer.serialize_seq(Some(7))?;
                s.serialize_element(&FUNCTION_DATA_SERIALIZATION_FORMAT_V2)?;
                s.serialize_element(module_id)?;
                s.serialize_element(fun_id)?;
                s.serialize_element(ty_args)?;
                s.serialize_element(mask)?;
                s.serialize_element(depth)?;
                s.serialize_element(blob)?;
                s.end()
            },
        }
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use crate::{account_address::AccountAddress, ident_str};

    fn make_closure(captured: MoveClosureCapturedArgs) -> MoveValue {
        MoveValue::Closure(Box::new(MoveClosure {
            module_id: ModuleId {
                address: AccountAddress::ONE,
                name: ident_str!("mod").to_owned(),
            },
            fun_id: ident_str!("func").to_owned(),
            ty_args: vec![TypeTag::Bool],
            mask: ClosureMask::new(0b111),
            captured,
        }))
    }

    fn captured_pairs() -> Vec<(MoveTypeLayout, MoveValue)> {
        vec![
            (MoveTypeLayout::U64, MoveValue::U64(2066)),
            (
                MoveTypeLayout::Vector(Box::new(MoveTypeLayout::Bool)),
                MoveValue::Vector(vec![MoveValue::Bool(false)]),
            ),
            (
                MoveTypeLayout::new_struct(MoveStructLayout::Runtime(vec![
                    MoveTypeLayout::Bool,
                    MoveTypeLayout::U8,
                ])),
                MoveValue::Struct(MoveStruct::Runtime(vec![
                    MoveValue::Bool(false),
                    MoveValue::U8(22),
                ])),
            ),
        ]
    }

    fn round_trip(value: &MoveValue) {
        let blob = value
            .simple_serialize()
            .expect("serialization must succeed");
        assert_eq!(
            value,
            &MoveValue::simple_deserialize(&blob, &MoveTypeLayout::Function)
                .expect("deserialization must succeed"),
            "deserialized value not equal to original one"
        );
    }

    #[test]
    fn function_value_serialization_v1_ok() {
        round_trip(&make_closure(MoveClosureCapturedArgs::Deserialized(
            captured_pairs(),
        )));
    }

    #[test]
    fn function_value_serialization_v2_ok() {
        // The blob is the concatenated BCS of the captured values.
        let blob = captured_pairs()
            .into_iter()
            .flat_map(|(_, v)| v.simple_serialize().unwrap())
            .collect::<Vec<u8>>();
        round_trip(&make_closure(MoveClosureCapturedArgs::Serialized {
            depth: 3,
            blob,
        }));
    }

    #[test]
    fn function_value_v2_decode_captured() {
        let (layouts, values): (Vec<_>, Vec<_>) = captured_pairs().into_iter().unzip();
        let blob = values
            .iter()
            .flat_map(|v| v.simple_serialize().unwrap())
            .collect::<Vec<u8>>();
        let decoded = MoveClosure::deserialize_captured(&blob, layouts.clone())
            .expect("decoding must succeed");
        assert_eq!(decoded, values);

        // Trailing bytes are rejected.
        let mut with_trailing = blob.clone();
        with_trailing.push(0);
        MoveClosure::deserialize_captured(&with_trailing, layouts.clone())
            .expect_err("trailing bytes must be rejected");

        // Truncated blobs are rejected.
        MoveClosure::deserialize_captured(&blob[..blob.len() - 1], layouts)
            .expect_err("truncated blob must be rejected");
    }

    #[test]
    fn function_value_serialization_bad_version() {
        // A closure with version 3 in the version slot must be rejected.
        let v2_bytes = make_closure(MoveClosureCapturedArgs::Serialized {
            depth: 0,
            blob: vec![1, 2, 3],
        })
        .simple_serialize()
        .expect("serialization must succeed");
        // The version u16 is serialized little-endian right after the seq length byte.
        let mut bad = v2_bytes;
        bad[1] = 3;
        MoveValue::simple_deserialize(&bad, &MoveTypeLayout::Function)
            .expect_err("unknown version must be rejected");
    }

    #[test]
    fn function_value_serialization_v1_golden_bytes() {
        // V1 byte stability: the encoding of decoded captured arguments must not
        // change, since it is the on-chain format.
        let value = MoveValue::Closure(Box::new(MoveClosure {
            module_id: ModuleId {
                address: AccountAddress::ONE,
                name: ident_str!("m").to_owned(),
            },
            fun_id: ident_str!("f").to_owned(),
            ty_args: vec![],
            mask: ClosureMask::new(0b1),
            captured: MoveClosureCapturedArgs::Deserialized(vec![(
                MoveTypeLayout::U8,
                MoveValue::U8(7),
            )]),
        }));
        let blob = value.simple_serialize().expect("serialization succeeds");
        let mut expected = vec![
            7, // seq length: 5 + 2 * 1
            1, 0, // version 1 (u16, little-endian)
        ];
        expected.extend(AccountAddress::ONE.to_vec()); // module address
        expected.extend([
            1, b'm', // module name
            1, b'f', // function name
            0,    // no type args
            1, 0, 0, 0, 0, 0, 0, 0, // mask (u64, little-endian)
            1, // layout: U8 (enum variant index)
            7, // value
        ]);
        assert_eq!(blob, expected);
    }

    #[test]
    fn function_value_serialization_v2_golden_bytes() {
        // V2 byte stability: the cached depth is a u16 sitting between the mask and
        // the captured blob.
        let value = MoveValue::Closure(Box::new(MoveClosure {
            module_id: ModuleId {
                address: AccountAddress::ONE,
                name: ident_str!("m").to_owned(),
            },
            fun_id: ident_str!("f").to_owned(),
            ty_args: vec![],
            mask: ClosureMask::new(0b1),
            captured: MoveClosureCapturedArgs::Serialized {
                depth: 5,
                blob: vec![7],
            },
        }));
        let blob = value.simple_serialize().expect("serialization succeeds");
        let mut expected = vec![
            7, // seq length: version, module_id, fun_id, ty_args, mask, depth, blob
            2, 0, // version 2 (u16, little-endian)
        ];
        expected.extend(AccountAddress::ONE.to_vec()); // module address
        expected.extend([
            1, b'm', // module name
            1, b'f', // function name
            0,    // no type args
            1, 0, 0, 0, 0, 0, 0, 0, // mask (u64, little-endian)
            5, 0, // depth (u16, little-endian)
            1, // blob length (one captured byte)
            7, // blob byte
        ]);
        assert_eq!(blob, expected);
    }

    #[test]
    fn function_value_serialization_v2_preserves_depth() {
        // The cached depth survives a serialize/deserialize round-trip.
        let value = make_closure(MoveClosureCapturedArgs::Serialized {
            depth: 9,
            blob: vec![1, 2, 3],
        });
        let blob = value.simple_serialize().expect("serialization succeeds");
        let decoded = MoveValue::simple_deserialize(&blob, &MoveTypeLayout::Function)
            .expect("deserialization succeeds");
        match decoded {
            MoveValue::Closure(c) => match c.captured {
                MoveClosureCapturedArgs::Serialized { depth, blob } => {
                    assert_eq!(depth, 9);
                    assert_eq!(blob, vec![1, 2, 3]);
                },
                other => panic!("expected serialized captures, got {:?}", other),
            },
            other => panic!("expected a closure, got {:?}", other),
        }
    }
}
